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
    /// `tan` of each cell-centre angle along one face axis.
    tan_centre: Vec<f64>,
}

fn cache() -> &'static Mutex<BTreeMap<u32, Arc<GridTables>>> {
    static CACHE: OnceLock<Mutex<BTreeMap<u32, Arc<GridTables>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// Drop every cached grid table. Tests use this to force uncached builds.
#[cfg(test)]
pub(crate) fn clear_tables_cache() {
    let map = cache();
    let mut guard = map.lock().unwrap_or_else(|e| e.into_inner());
    guard.clear();
}

fn tables_for(n: u32) -> Arc<GridTables> {
    let map = cache();
    // Never hold the lock during `build_tables`: that work runs on the rayon
    // pool, and a stolen job can call `CubeSphere::new` for another size.
    let existing = {
        let guard = map.lock().unwrap_or_else(|e| e.into_inner());
        guard.get(&n).map(Arc::clone)
    };
    if let Some(existing) = existing {
        return existing;
    }
    let built = Arc::new(build_tables(n));
    let mut guard = map.lock().unwrap_or_else(|e| e.into_inner());
    // Another thread may have built the same size while we released the lock.
    Arc::clone(guard.entry(n).or_insert(built))
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
        let ta = self.tables.tan_centre[i as usize];
        let tb = self.tables.tan_centre[j as usize];
        unit_from_tangents(face, ta, tb)
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

    /// Bilinear sample. A stencil corner past the face edge is the cell across
    /// that edge, with the same gnomonic extrapolation as the border neighbours.
    pub fn sample_bilinear(&self, field: &[f32], p: [f64; 3]) -> f32 {
        let (face, fx, fy) = self.face_coords(p);
        self.sample_at(field, face, fx, fy)
    }

    /// Bilinear sample at face coordinates from [`Self::face_coords`].
    pub(crate) fn sample_at(&self, field: &[f32], face: u32, fx: f64, fy: f64) -> f32 {
        self.sample_face(field, None, face, fx, fy).unwrap_or(0.0)
    }

    /// Nearest cell sample.
    pub fn sample_nearest(&self, field: &[f32], p: [f64; 3]) -> f32 {
        field[self.cell_at(p) as usize]
    }

    /// Bilinear cell ids and weights. Weights are non-negative and sum to 1.
    /// A corner past the face edge is the cell across that edge.
    pub fn bilinear_stencil(&self, p: [f64; 3]) -> ([u32; 4], [f64; 4]) {
        let (face, fx, fy) = self.face_coords(p);
        self.bilinear_stencil_at(face, fx, fy)
    }

    /// Catmull-Rom cell ids and weights on the face coordinates.
    ///
    /// Weights sum to 1 and may be negative, so a cell centre reproduces that
    /// cell. Off-face taps use the same cross-edge cell as [`Self::sample_face`].
    /// A tap that leaves through a cube corner does not exist: inside that
    /// corner the stencil is bilinear, and it blends across the last cell
    /// before the corner so the value and the slope stay continuous.
    pub fn bicubic_stencil(&self, p: [f64; 3]) -> ([u32; 16], [f64; 16]) {
        let (face, fx, fy) = self.face_coords(p);
        let alpha = corner_bicubic_weight(fx, fy, self.n as f64);
        if alpha <= 0.0 {
            return pack_bilinear(self.bilinear_stencil_at(face, fx, fy));
        }
        let Some((mut cells, mut weights)) = self.bicubic_stencil_at(face, fx, fy) else {
            return pack_bilinear(self.bilinear_stencil_at(face, fx, fy));
        };
        if alpha >= 1.0 {
            return (cells, weights);
        }
        for weight in &mut weights {
            *weight *= alpha;
        }
        let (bilinear_cells, bilinear_weights) = self.bilinear_stencil_at(face, fx, fy);
        let remain = 1.0 - alpha;
        for (cell, weight) in bilinear_cells.into_iter().zip(bilinear_weights) {
            let add = weight * remain;
            if add == 0.0 {
                continue;
            }
            if let Some(slot) = cells.iter().position(|id| *id == cell) {
                weights[slot] += add;
            } else if let Some(slot) = weights.iter().position(|value| *value == 0.0) {
                cells[slot] = cell;
                weights[slot] = add;
            } else {
                weights[0] += add;
            }
        }
        (cells, weights)
    }

    fn bilinear_stencil_at(&self, face: u32, fx: f64, fy: f64) -> ([u32; 4], [f64; 4]) {
        let u = fx - 0.5;
        let v = fy - 0.5;
        let i0 = floor_i32(u);
        let j0 = floor_i32(v);
        let tx = u - f64::from(i0);
        let ty = v - f64::from(j0);
        let coords = [(i0, j0), (i0 + 1, j0), (i0, j0 + 1), (i0 + 1, j0 + 1)];
        let mut weights = [
            (1.0 - tx) * (1.0 - ty),
            tx * (1.0 - ty),
            (1.0 - tx) * ty,
            tx * ty,
        ];
        let n = self.n as i32;
        let mut cells = [0u32; 4];
        let mut missing = 0.0;
        for (slot, (i, j)) in coords.into_iter().enumerate() {
            let i_out = !(0..n).contains(&i);
            let j_out = !(0..n).contains(&j);
            if i_out && j_out {
                // Three faces meet here. There is no fourth cell, and a
                // diagonal ghost is not continuous with the neighbouring chart.
                missing += weights[slot];
                weights[slot] = 0.0;
            } else {
                cells[slot] = self.chart_cell(face, i, j);
            }
        }
        if missing > 0.0 {
            let keep = 1.0 - missing;
            if keep > 0.0 {
                for weight in &mut weights {
                    *weight /= keep;
                }
            }
        }
        (cells, weights)
    }

    fn bicubic_stencil_at(&self, face: u32, fx: f64, fy: f64) -> Option<([u32; 16], [f64; 16])> {
        let u = fx - 0.5;
        let v = fy - 0.5;
        let iu = floor_i32(u);
        let iv = floor_i32(v);
        let tx = u - f64::from(iu);
        let ty = v - f64::from(iv);
        let wx = catmull_rom_weights(tx);
        let wy = catmull_rom_weights(ty);
        let mut cells = [0u32; 16];
        let mut weights = [0.0; 16];
        for jj in 0..4i32 {
            for ii in 0..4i32 {
                let id = self.bicubic_cell(face, iu - 1 + ii, iv - 1 + jj)?;
                let slot = (jj * 4 + ii) as usize;
                cells[slot] = id;
                weights[slot] = wx[ii as usize] * wy[jj as usize];
            }
        }
        Some((cells, weights))
    }

    fn chart_cell(&self, face: u32, i: i32, j: i32) -> u32 {
        let n = self.n as i32;
        if (0..n).contains(&i) && (0..n).contains(&j) {
            self.id(face, i as u32, j as u32)
        } else {
            extrapolated_cell(self.n, face, i, j)
        }
    }

    /// `None` when `(i, j)` leaves the face through a cube corner.
    fn bicubic_cell(&self, face: u32, i: i32, j: i32) -> Option<u32> {
        let n = self.n as i32;
        let i_out = !(0..n).contains(&i);
        let j_out = !(0..n).contains(&j);
        if i_out && j_out {
            None
        } else {
            Some(self.chart_cell(face, i, j))
        }
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
        self.sample_chart(face, fx, fy, &|id| {
            if let Some(m) = mask {
                if m[id as usize] == 0 {
                    return None;
                }
            }
            Some(f64::from(field[id as usize]))
        })
        .map(|value| value as f32)
    }

    /// Bilinear sample of an `f64` field. Outside corners use the same chart as [`Self::sample_bilinear`].
    pub(crate) fn sample_f64(&self, field: &[f64], p: [f64; 3]) -> f64 {
        let (face, fx, fy) = self.face_coords(p);
        self.sample_chart(face, fx, fy, &|id| Some(field[id as usize]))
            .unwrap_or(0.0)
    }

    /// Bilinear sample of an `f64` field at face coordinates the caller already
    /// has. Same arithmetic as [`Self::sample_f64`] without the coordinate
    /// lookup, for repeated samples of one point.
    pub(crate) fn sample_chart_at(&self, field: &[f64], face: u32, fx: f64, fy: f64) -> f64 {
        self.sample_chart(face, fx, fy, &|id| Some(field[id as usize]))
            .unwrap_or(0.0)
    }

    /// Face-local bilinear weights. A corner outside the face uses the cell
    /// [`extrapolated_cell`] would pick, shifted from that cell centre to the
    /// virtual node by a tangent gradient.
    pub(crate) fn sample_chart(
        &self,
        face: u32,
        fx: f64,
        fy: f64,
        value_at: &impl Fn(u32) -> Option<f64>,
    ) -> Option<f64> {
        let n = self.n as i32;
        let u = fx - 0.5;
        let v = fy - 0.5;
        let i0f = u.floor();
        let j0f = v.floor();
        let tx = u - i0f;
        let ty = v - j0f;
        let i0 = i0f as i32;
        let j0 = j0f as i32;
        let corners = [
            (i0, j0, (1.0 - tx) * (1.0 - ty)),
            (i0 + 1, j0, tx * (1.0 - ty)),
            (i0, j0 + 1, (1.0 - tx) * ty),
            (i0 + 1, j0 + 1, tx * ty),
        ];
        let mut acc = 0.0;
        let mut wsum = 0.0;
        for (i, j, weight) in corners {
            if weight == 0.0 {
                continue;
            }
            let Some(value) = self.chart_corner(face, i, j, n, value_at) else {
                continue;
            };
            acc += value * weight;
            wsum += weight;
        }
        if wsum == 0.0 {
            None
        } else {
            Some(acc / wsum)
        }
    }

    fn chart_corner(
        &self,
        face: u32,
        i: i32,
        j: i32,
        n: i32,
        value_at: &impl Fn(u32) -> Option<f64>,
    ) -> Option<f64> {
        if (0..n).contains(&i) && (0..n).contains(&j) {
            return value_at(self.id(face, i as u32, j as u32));
        }
        // The weight assumes the corner is the virtual node. That node is not
        // the centre of the cell across the edge, especially near a cube corner.
        // Shift the anchor cell's value along a tangent gradient fitted to its
        // neighbours. A masked anchor drops the corner. The shifted value is
        // clamped to the anchor neighbourhood's range (slope limiter), so a
        // sharp contrast across the edge cannot overshoot.
        let anchor = extrapolated_cell(self.n, face, i, j);
        let base = value_at(anchor)?;
        let target = virtual_centre(self.n, face, i, j);
        let shifted = base + self.tangent_step(anchor, target, base, value_at);
        let (mut lo, mut hi) = (base, base);
        for nb in self.neighbors(anchor) {
            if nb == NONE {
                continue;
            }
            if let Some(value) = value_at(nb) {
                lo = lo.min(value);
                hi = hi.max(value);
            }
        }
        Some(shifted.clamp(lo, hi))
    }

    fn tangent_step(
        &self,
        anchor: u32,
        target: [f64; 3],
        base: f64,
        value_at: &impl Fn(u32) -> Option<f64>,
    ) -> f64 {
        let centre = self.position(anchor);
        let (east, north) = local_east_north(centre);
        let mut a00 = 0.0;
        let mut a01 = 0.0;
        let mut a11 = 0.0;
        let mut b0 = 0.0;
        let mut b1 = 0.0;
        for nb in self.neighbors(anchor) {
            if nb == NONE {
                continue;
            }
            let Some(value) = value_at(nb) else {
                continue;
            };
            let pk = self.position(nb);
            let dx = dot(
                [pk[0] - centre[0], pk[1] - centre[1], pk[2] - centre[2]],
                east,
            );
            let dy = dot(
                [pk[0] - centre[0], pk[1] - centre[1], pk[2] - centre[2]],
                north,
            );
            let dh = value - base;
            a00 += dx * dx;
            a01 += dx * dy;
            a11 += dy * dy;
            b0 += dx * dh;
            b1 += dy * dh;
        }
        let det = a00 * a11 - a01 * a01;
        if det.abs() < 1e-18 {
            return 0.0;
        }
        let gx = (b0 * a11 - a01 * b1) / det;
        let gy = (a00 * b1 - a01 * b0) / det;
        let tx = dot(
            [
                target[0] - centre[0],
                target[1] - centre[1],
                target[2] - centre[2],
            ],
            east,
        );
        let ty = dot(
            [
                target[0] - centre[0],
                target[1] - centre[1],
                target[2] - centre[2],
            ],
            north,
        );
        gx * tx + gy * ty
    }

    fn border_index(&self, face: u32, i: u32, j: u32) -> usize {
        let slot = border_slot(self.n, i, j).unwrap_or(0);
        (face * border_count(self.n) + slot) as usize
    }

    /// Linear weights for [`Self::sample_at`]. An in-face corner is one cell.
    /// A corner past the edge is the anchor plus its tangent step, which is
    /// linear in the field, so the whole sample is one weighted sum.
    pub(crate) fn chart_stencil(&self, face: u32, fx: f64, fy: f64) -> ChartStencil {
        let n = self.n as i32;
        let u = fx - 0.5;
        let v = fy - 0.5;
        let i0f = u.floor();
        let j0f = v.floor();
        let tx = u - i0f;
        let ty = v - j0f;
        let i0 = i0f as i32;
        let j0 = j0f as i32;
        let in_face = (0..n).contains(&i0)
            && (0..n).contains(&(i0 + 1))
            && (0..n).contains(&j0)
            && (0..n).contains(&(j0 + 1));
        if in_face {
            return ChartStencil::Quad {
                cells: [
                    self.id(face, i0 as u32, j0 as u32),
                    self.id(face, (i0 + 1) as u32, j0 as u32),
                    self.id(face, i0 as u32, (j0 + 1) as u32),
                    self.id(face, (i0 + 1) as u32, (j0 + 1) as u32),
                ],
                weights: [
                    (1.0 - tx) * (1.0 - ty),
                    tx * (1.0 - ty),
                    (1.0 - tx) * ty,
                    tx * ty,
                ],
            };
        }
        let corners = [
            (i0, j0, (1.0 - tx) * (1.0 - ty)),
            (i0 + 1, j0, tx * (1.0 - ty)),
            (i0, j0 + 1, (1.0 - tx) * ty),
            (i0 + 1, j0 + 1, tx * ty),
        ];
        let mut terms: Vec<(u32, f64)> = Vec::new();
        let mut wsum = 0.0;
        for (i, j, weight) in corners {
            if weight == 0.0 {
                continue;
            }
            wsum += weight;
            for (cell, coeff) in self.corner_coeffs(face, i, j) {
                add_stencil_term(&mut terms, cell, weight * coeff);
            }
        }
        if wsum == 0.0 || terms.is_empty() {
            let id = self.cell_at(self.position(self.id(
                face,
                i0.clamp(0, n - 1) as u32,
                j0.clamp(0, n - 1) as u32,
            )));
            return ChartStencil::Quad {
                cells: [id, id, id, id],
                weights: [1.0, 0.0, 0.0, 0.0],
            };
        }
        let scale = 1.0 / wsum;
        if terms.len() <= 4 {
            let mut cells = [0u32; 4];
            let mut weights = [0.0; 4];
            for (k, (cell, weight)) in terms.into_iter().enumerate() {
                cells[k] = cell;
                weights[k] = weight * scale;
            }
            ChartStencil::Quad { cells, weights }
        } else {
            let mut cells = Vec::with_capacity(terms.len());
            let mut weights = Vec::with_capacity(terms.len());
            for (cell, weight) in terms {
                cells.push(cell);
                weights.push(weight * scale);
            }
            ChartStencil::Wide { cells, weights }
        }
    }

    /// Weight 1 on an in-face cell. Outside, the anchor-plus-tangent value.
    fn corner_coeffs(&self, face: u32, i: i32, j: i32) -> Vec<(u32, f64)> {
        let n = self.n as i32;
        if (0..n).contains(&i) && (0..n).contains(&j) {
            return vec![(self.id(face, i as u32, j as u32), 1.0)];
        }
        let anchor = extrapolated_cell(self.n, face, i, j);
        let target = virtual_centre(self.n, face, i, j);
        self.tangent_weights(anchor, target)
    }

    /// `base + gradient · (target − centre)`, expanded onto the anchor and its neighbours.
    fn tangent_weights(&self, anchor: u32, target: [f64; 3]) -> Vec<(u32, f64)> {
        let centre = self.position(anchor);
        let (east, north) = local_east_north(centre);
        let mut samples = Vec::new();
        let mut a00 = 0.0;
        let mut a01 = 0.0;
        let mut a11 = 0.0;
        let mut sum_dx = 0.0;
        let mut sum_dy = 0.0;
        for nb in self.neighbors(anchor) {
            if nb == NONE {
                continue;
            }
            let pk = self.position(nb);
            let delta = [pk[0] - centre[0], pk[1] - centre[1], pk[2] - centre[2]];
            let dx = dot(delta, east);
            let dy = dot(delta, north);
            a00 += dx * dx;
            a01 += dx * dy;
            a11 += dy * dy;
            sum_dx += dx;
            sum_dy += dy;
            samples.push((nb, dx, dy));
        }
        let det = a00 * a11 - a01 * a01;
        if det.abs() < 1e-18 {
            return vec![(anchor, 1.0)];
        }
        let along = [
            target[0] - centre[0],
            target[1] - centre[1],
            target[2] - centre[2],
        ];
        let tx = dot(along, east);
        let ty = dot(along, north);
        let c0 = (tx * a11 - ty * a01) / det;
        let c1 = (ty * a00 - tx * a01) / det;
        let mut terms = vec![(anchor, 1.0 - c0 * sum_dx - c1 * sum_dy)];
        for (nb, dx, dy) in samples {
            add_stencil_term(&mut terms, nb, c0 * dx + c1 * dy);
        }
        terms
    }
}

/// Bilinear (or cross-face) weights. `sample` is a dot with the field.
pub(crate) enum ChartStencil {
    /// At most four cells. Unused slots have weight 0.
    Quad { cells: [u32; 4], weights: [f64; 4] },
    /// More than four cells after a cross-face expansion.
    Wide { cells: Vec<u32>, weights: Vec<f64> },
}

impl ChartStencil {
    /// Weighted sum of `field`. A cross-face expansion can carry negative
    /// weights; that sum is clamped to the range of the stencil's cells so a
    /// sharp contrast next to a face edge cannot overshoot (slope limiter).
    pub(crate) fn sample(&self, field: &[f32]) -> f64 {
        match self {
            Self::Quad { cells, weights } => limited_sum(field, cells, weights),
            Self::Wide { cells, weights } => limited_sum(field, cells, weights),
        }
    }
}

fn limited_sum(field: &[f32], cells: &[u32], weights: &[f64]) -> f64 {
    let mut acc = 0.0;
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    let mut signed = false;
    for (cell, weight) in cells.iter().zip(weights) {
        if *weight == 0.0 {
            continue;
        }
        let value = f64::from(field[*cell as usize]);
        acc += value * *weight;
        lo = lo.min(value);
        hi = hi.max(value);
        signed |= *weight < 0.0;
    }
    if signed && lo <= hi {
        acc.clamp(lo, hi)
    } else {
        acc
    }
}

fn add_stencil_term(terms: &mut Vec<(u32, f64)>, cell: u32, weight: f64) {
    if weight == 0.0 {
        return;
    }
    for term in terms.iter_mut() {
        if term.0 == cell {
            term.1 += weight;
            return;
        }
    }
    terms.push((cell, weight));
}

/// Bilinear upsample from `parent` onto `child` (`child.n == 2 * parent.n`).
///
/// Weights inside a face are 0.75/0.25. A parent coordinate whose stencil
/// crosses a face edge blends with the cell across that edge.
pub fn upsample(
    parent: &CubeSphere,
    parent_field: &[f32],
    child: &CubeSphere,
) -> Result<Vec<f32>, String> {
    upsample_with_cancel(parent, parent_field, child, &|| false).map_err(cancel_error_text)
}

/// Area-weighted mean of the four children. `child.n` must be `2 * parent.n`.
pub fn downsample(
    child: &CubeSphere,
    field: &[f32],
    parent: &CubeSphere,
) -> Result<Vec<f32>, String> {
    downsample_with_cancel(child, field, parent, &|| false).map_err(cancel_error_text)
}

/// Maximum of the four children. `child.n` must be `2 * parent.n`.
pub fn downsample_max(
    child: &CubeSphere,
    field: &[f32],
    parent: &CubeSphere,
) -> Result<Vec<f32>, String> {
    if child.n != parent.n * 2 {
        return Err(format!(
            "downsample_max requires child.n = 2 * parent.n ({} vs {})",
            child.n, parent.n
        ));
    }
    if field.len() != child.cell_count() {
        return Err("downsample_max field length does not match child grid".to_string());
    }
    let mut out = vec![f32::MIN; parent.cell_count()];
    out.par_iter_mut().enumerate().for_each(|(cell, dest)| {
        let mut best = f32::MIN;
        for kid in parent.child_ids(cell as u32) {
            best = best.max(field[kid as usize]);
        }
        *dest = best;
    });
    Ok(out)
}

/// Cells per resample chunk. These kernels are cheap per cell, so a large
/// chunk keeps barriers few and the cancellation gap well under 100 ms.
const RESAMPLE_CHUNK: usize = 4_194_304;

/// Map a cancellation-aware error back to the public `String` form.
fn cancel_error_text(error: crate::WorldError) -> String {
    match error {
        crate::WorldError::Internal(text) => text,
        other => other.to_string(),
    }
}

/// Resample `field` from `src` onto `dst`. Resolutions must differ by a power of two.
/// Upsampling repeats bilinear doubling. Downsampling repeats the four-child mean.
pub fn resample(src: &CubeSphere, field: &[f32], dst: &CubeSphere) -> Result<Vec<f32>, String> {
    resample_with_cancel(src, field, dst, &|| false).map_err(cancel_error_text)
}

/// Resample with a cancellation check between fixed cell chunks.
pub(crate) fn resample_with_cancel(
    src: &CubeSphere,
    field: &[f32],
    dst: &CubeSphere,
    cancelled: &dyn Fn() -> bool,
) -> Result<Vec<f32>, crate::WorldError> {
    if src.n == dst.n {
        if field.len() != src.cell_count() {
            return Err(crate::WorldError::Internal(
                "resample field length mismatch".to_string(),
            ));
        }
        return Ok(field.to_vec());
    }
    if src.n > dst.n {
        if !src.n.is_multiple_of(dst.n) {
            return Err(crate::WorldError::Internal(
                "resample resolutions are not nested".to_string(),
            ));
        }
        let mut cur_grid = src.clone();
        let mut cur = field.to_vec();
        while cur_grid.n > dst.n {
            let next = CubeSphere::new(cur_grid.n / 2, cur_grid.radius_m)
                .map_err(crate::WorldError::Internal)?;
            cur = downsample_with_cancel(&cur_grid, &cur, &next, cancelled)?;
            cur_grid = next;
        }
        Ok(cur)
    } else {
        if !dst.n.is_multiple_of(src.n) {
            return Err(crate::WorldError::Internal(
                "resample resolutions are not nested".to_string(),
            ));
        }
        let mut cur_grid = src.clone();
        let mut cur = field.to_vec();
        while cur_grid.n < dst.n {
            let next = CubeSphere::new(cur_grid.n * 2, cur_grid.radius_m)
                .map_err(crate::WorldError::Internal)?;
            cur = upsample_with_cancel(&cur_grid, &cur, &next, cancelled)?;
            cur_grid = next;
        }
        Ok(cur)
    }
}

fn upsample_with_cancel(
    parent: &CubeSphere,
    parent_field: &[f32],
    child: &CubeSphere,
    cancelled: &dyn Fn() -> bool,
) -> Result<Vec<f32>, crate::WorldError> {
    if child.n != parent.n * 2 {
        return Err(crate::WorldError::Internal(format!(
            "upsample requires child.n = 2 * parent.n ({} vs {})",
            child.n, parent.n
        )));
    }
    if parent_field.len() != parent.cell_count() {
        return Err(crate::WorldError::Internal(
            "upsample field length does not match parent grid".to_string(),
        ));
    }
    let n = child.cell_count();
    let chunk = crate::scaled_chunk(RESAMPLE_CHUNK);
    let mut out = vec![0.0f32; n];
    for start in (0..n).step_by(chunk) {
        if cancelled() {
            return Err(crate::WorldError::Cancelled);
        }
        let end = (start + chunk).min(n);
        out[start..end]
            .par_iter_mut()
            .enumerate()
            .for_each(|(offset, dest)| {
                let cell = start + offset;
                let (face, i, j) = child.decode(cell as u32);
                let fx = (i as f64 + 0.5) / 2.0;
                let fy = (j as f64 + 0.5) / 2.0;
                *dest = parent
                    .sample_face(parent_field, None, face, fx, fy)
                    .unwrap_or(0.0);
            });
    }
    Ok(out)
}

fn downsample_with_cancel(
    child: &CubeSphere,
    field: &[f32],
    parent: &CubeSphere,
    cancelled: &dyn Fn() -> bool,
) -> Result<Vec<f32>, crate::WorldError> {
    if child.n != parent.n * 2 {
        return Err(crate::WorldError::Internal(format!(
            "downsample requires child.n = 2 * parent.n ({} vs {})",
            child.n, parent.n
        )));
    }
    if field.len() != child.cell_count() {
        return Err(crate::WorldError::Internal(
            "downsample field length does not match child grid".to_string(),
        ));
    }
    let n = parent.cell_count();
    let chunk = crate::scaled_chunk(RESAMPLE_CHUNK);
    let mut out = vec![0.0f32; n];
    for start in (0..n).step_by(chunk) {
        if cancelled() {
            return Err(crate::WorldError::Cancelled);
        }
        let end = (start + chunk).min(n);
        out[start..end]
            .par_iter_mut()
            .enumerate()
            .for_each(|(offset, dest)| {
                let cell = start + offset;
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
    }
    Ok(out)
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
    let nu = n as usize;
    let tan_centre: Vec<f64> = (0..n).map(|i| centre_tangent(n, i)).collect();
    let mut area_sr = vec![0.0f64; nn];
    area_sr.par_chunks_mut(nu).enumerate().for_each(|(j, row)| {
        for (i, slot) in row.iter_mut().enumerate() {
            *slot = cell_solid_angle(n, i as u32, j as u32);
        }
    });
    let mut dist_rad = vec![0.0f32; 8 * nn];
    if n >= 3 {
        dist_rad
            .par_chunks_mut(nn)
            .enumerate()
            .for_each(|(k, plane)| {
                plane.par_chunks_mut(nu).enumerate().for_each(|(j, row)| {
                    if j == 0 || j + 1 == nu {
                        return;
                    }
                    for i in 1..nu - 1 {
                        let p = unit_from_tangents(0, tan_centre[i], tan_centre[j]);
                        let ii = (i as i32 + NEIGHBOR_DI[k]) as usize;
                        let jj = (j as i32 + NEIGHBOR_DJ[k]) as usize;
                        let q = unit_from_tangents(0, tan_centre[ii], tan_centre[jj]);
                        row[i] = angular_sep(p, q) as f32;
                    }
                });
            });
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
        tan_centre,
    }
}

fn floor_i32(value: f64) -> i32 {
    let floored = value.floor();
    if floored >= f64::from(i32::MAX) {
        i32::MAX
    } else if floored <= f64::from(i32::MIN) {
        i32::MIN
    } else {
        floored as i32
    }
}

fn catmull_rom_weights(t: f64) -> [f64; 4] {
    let t2 = t * t;
    let t3 = t2 * t;
    [
        -0.5 * t3 + t2 - 0.5 * t,
        1.5 * t3 - 2.5 * t2 + 1.0,
        -1.5 * t3 + 2.0 * t2 + 0.5 * t,
        0.5 * t3 - 0.5 * t2,
    ]
}

fn smoothstep_unit(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// 0 inside the cube-corner square where a 4×4 tap is missing, 1 one cell
/// clear of that square. Flat at both ends.
fn corner_bicubic_weight(fx: f64, fy: f64, n: f64) -> f64 {
    let dx = fx.min(n - fx);
    let dy = fy.min(n - fy);
    let dist = if dx <= 1.5 && dy <= 1.5 {
        0.0
    } else if dx <= 1.5 {
        dy - 1.5
    } else if dy <= 1.5 {
        dx - 1.5
    } else {
        let ex = dx - 1.5;
        let ey = dy - 1.5;
        ex.hypot(ey)
    };
    smoothstep_unit(0.0, 1.0, dist)
}

fn pack_bilinear(stencil: ([u32; 4], [f64; 4])) -> ([u32; 16], [f64; 16]) {
    let (cells4, weights4) = stencil;
    let mut cells = [cells4[0]; 16];
    let mut weights = [0.0; 16];
    cells[..4].copy_from_slice(&cells4);
    weights[..4].copy_from_slice(&weights4);
    (cells, weights)
}

fn virtual_centre(n: u32, face: u32, i: i32, j: i32) -> [f64; 3] {
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
    normalize(q)
}

fn extrapolated_cell(n: u32, face: u32, i: i32, j: i32) -> u32 {
    cell_at_n(n, virtual_centre(n, face, i, j))
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

fn centre_tangent(n: u32, i: u32) -> f64 {
    let nf = n as f64;
    (((i as f64 + 0.5) / nf - 0.5) * std::f64::consts::FRAC_PI_2).tan()
}

fn unit_on_face(face: u32, n: u32, i: u32, j: u32) -> [f64; 3] {
    unit_from_tangents(face, centre_tangent(n, i), centre_tangent(n, j))
}

#[inline]
fn unit_from_tangents(face: u32, ta: f64, tb: f64) -> [f64; 3] {
    let basis = FACE_BASIS[face as usize];
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
fn local_east_north(up: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    let mut east = cross([0.0, 0.0, 1.0], up);
    let mut len = (east[0] * east[0] + east[1] * east[1] + east[2] * east[2]).sqrt();
    if len < 1e-8 {
        east = cross([0.0, 1.0, 0.0], up);
        len = (east[0] * east[0] + east[1] * east[1] + east[2] * east[2]).sqrt();
    }
    if len < 1e-12 {
        return ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
    }
    east = [east[0] / len, east[1] / len, east[2] / len];
    (east, cross(up, east))
}

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
