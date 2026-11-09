//! Analytic plates, crust, boundaries and the pre-erosion surface.
//!
//! Evaluated on the tectonic grid `n_t = min(N, 256)`. Finer levels sample
//! these fields and add detail. Plate assignment warps by 0.18 at frequency 2,
//! then by 0.06 at frequency 6, before the weighted Voronoi. Continent shape
//! warps by 0.35 at frequency 1.5, then 0.12 at frequency 4, then 0.04 at
//! frequency 12, plus fBm of amplitude 0.5 with 6 octaves.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, VecDeque};

use rayon::prelude::*;

use crate::grid::{dot3, normalize, CubeSphere, NONE};
use crate::noise::{
    derive_seed, derive_seed_salt, fbm, hash_u01, random_unit, ridged, warp, SeedDomain,
};
use crate::{ContinentLayout, WorldError, WorldSettings};

/// One rigid plate.
#[derive(Clone, Debug)]
pub struct Plate {
    /// Seed site, unit vector.
    pub site: [f64; 3],
    /// Unit Euler pole.
    pub axis: [f64; 3],
    /// Angular speed, radians per year.
    pub omega_rad_yr: f64,
    /// Lower rank subducts in an ocean–ocean pair.
    pub subduction_rank: u32,
    /// Voronoi weight in radians (`0.30 · u³`).
    pub weight_rad: f64,
    /// `1` when most of the plate's area is continental crust.
    pub crust_majority: u8,
}

/// How the plates move at a shared edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundaryKind {
    /// Plates are closing.
    Convergent,
    /// Plates are opening.
    Divergent,
    /// Motion is mostly strike-slip.
    Transform,
}

/// Which side of a convergent boundary goes down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubductSide {
    /// Continent–continent collision, or not convergent.
    None,
    /// The boundary cell's own plate subducts.
    This,
    /// The neighbour plate subducts.
    Other,
}

/// One boundary cell and the first axis-neighbour on another plate.
#[derive(Clone, Debug)]
pub struct BoundaryRecord {
    /// Boundary cell.
    pub cell: u32,
    /// Neighbour on the other plate.
    pub neighbor: u32,
    /// Plate of `cell`.
    pub plate_a: u8,
    /// Plate of `neighbor`.
    pub plate_b: u8,
    /// `1` if `cell` is continental crust.
    pub crust_a: u8,
    /// `1` if `neighbor` is continental crust.
    pub crust_b: u8,
    /// Opening, closing, or shear, from the smoothed rates.
    pub kind: BoundaryKind,
    /// Smoothed closing rate in metres per year. Positive is convergent.
    pub convergence_m_yr: f32,
    /// Smoothed strike-slip speed in metres per year.
    pub shear_m_yr: f32,
    /// Closing rate before the 250 km along-boundary average.
    pub convergence_raw_m_yr: f32,
    /// Strike-slip speed before the 250 km along-boundary average.
    pub shear_raw_m_yr: f32,
    /// Which plate subducts.
    pub subducts: SubductSide,
}

/// Hotspot and the age trail behind it. Index 0 is the active volcano.
#[derive(Clone, Debug)]
pub struct Hotspot {
    /// Active centre, unit vector.
    pub origin: [f64; 3],
    /// Trail points, youngest first, including the origin.
    pub points: Vec<[f64; 3]>,
}

/// Tectonic fields on the `n` grid, kept so later phases can read causes.
#[derive(Clone, Debug)]
pub struct Tectonics {
    /// Grid resolution this was built on.
    pub n: u32,
    /// Layout after resolving [`ContinentLayout::Mixed`].
    pub resolved_layout: ContinentLayout,
    /// Target continental area fraction.
    pub crust_fraction: f64,
    /// Plate parameters. Empty plates may remain after connectivity.
    pub plates: Vec<Plate>,
    /// One record per boundary cell.
    pub boundaries: Vec<BoundaryRecord>,
    /// Hotspot tracks.
    pub hotspots: Vec<Hotspot>,
    /// Plate index per cell.
    pub plate: Vec<u8>,
    /// `1` continental, `0` oceanic.
    pub crust: Vec<u8>,
    /// Pre-erosion elevation in metres.
    pub base_m: Vec<f32>,
    /// Rock uplift in metres per year.
    pub uplift_m_yr: Vec<f32>,
    /// Erodibility multiplier.
    pub k_rock: Vec<f32>,
    /// Metres to the nearest boundary of this plate. Infinite when a plate has none.
    pub d_boundary: Vec<f32>,
    /// Metres to the nearest oceanic ridge. Infinite when there is no ridge.
    pub d_ridge: Vec<f32>,
    /// Signed metres to the crust edge. Positive on continental crust.
    pub d_edge: Vec<f32>,
    /// Index into [`Self::boundaries`], or [`NONE`].
    pub boundary_record: Vec<u32>,
    /// Smoothed kind per cell: `0` none, `1` convergent, `2` divergent, `3` transform.
    pub boundary_kind: Vec<u8>,
    /// Fragments the connectivity pass moved onto a neighbouring plate.
    pub fragments_reassigned: u32,
}

/// Build the tectonic state on a grid of resolution `n`.
pub fn build(settings: &WorldSettings, n: u32) -> Result<Tectonics, WorldError> {
    let grid = CubeSphere::new(n, settings.radius_m).map_err(WorldError::Internal)?;
    let ncells = grid.cell_count();
    let seed = settings.seed;
    let plates = make_plates(settings);
    let mut plate = assign_plates(&grid, &plates, seed);
    let fragments_reassigned =
        enforce_connectivity(&grid, &mut plate, &plates).map_err(WorldError::Internal)?;
    let (crust, resolved, crust_fraction) = make_crust(&grid, settings, seed);
    let plates = with_majority(&grid, plates, &plate, &crust);
    let boundaries = make_boundaries(&grid, &plates, &plate, &crust, settings.radius_m);
    let (d_boundary, boundary_record) = distance_on_plate(&grid, &plate, &boundaries);
    let (d_ridge, d_edge) = ridge_and_edge(&grid, &crust, &boundaries);
    let classes = class_fields(&grid, &plate, &boundaries);
    let hotspots = make_hotspots(&grid, settings, &plates, &plate);
    let mut base_m = vec![0.0f32; ncells];
    let mut uplift_m_yr = vec![0.0f32; ncells];
    let mut k_rock = vec![0.0f32; ncells];
    let ctx = FieldCtx {
        grid: &grid,
        boundaries: &boundaries,
        hotspots: &hotspots,
        crust: &crust,
        d_ridge: &d_ridge,
        d_edge: &d_edge,
        collision: &classes.collision,
        arc: &classes.arc,
        trench: &classes.trench,
        rift: &classes.rift,
        transform: &classes.transform,
        radius: settings.radius_m,
        lith_seed: derive_seed(seed, SeedDomain::Lithology),
        surface_seed: derive_seed(seed, SeedDomain::ContinentalSurface),
        bg_seed: derive_seed(seed, SeedDomain::BackgroundUplift),
        old_seed: derive_seed(seed, SeedDomain::OldOrogen),
        arc_seed: derive_seed(seed, SeedDomain::ArcRidged),
        xform_seed: derive_seed(seed, SeedDomain::TransformRidged),
        mod_seed: derive_seed(seed, SeedDomain::UpliftModulation),
    };
    base_m
        .par_iter_mut()
        .zip(uplift_m_yr.par_iter_mut())
        .zip(k_rock.par_iter_mut())
        .enumerate()
        .for_each(|(i, ((b, u), k))| {
            let (bv, uv, kv) = eval_cell(&ctx, i);
            *b = bv;
            *u = uv;
            *k = kv;
        });
    let boundary_kind = boundary_kind_field(ncells, &boundaries);
    Ok(Tectonics {
        n,
        resolved_layout: resolved,
        crust_fraction,
        plates,
        boundaries,
        hotspots,
        plate,
        crust,
        base_m,
        uplift_m_yr,
        k_rock,
        d_boundary,
        d_ridge,
        d_edge,
        boundary_record,
        boundary_kind,
        fragments_reassigned,
    })
}

struct FieldCtx<'a> {
    grid: &'a CubeSphere,
    boundaries: &'a [BoundaryRecord],
    hotspots: &'a [Hotspot],
    crust: &'a [u8],
    d_ridge: &'a [f32],
    d_edge: &'a [f32],
    collision: &'a SeedDist,
    arc: &'a SeedDist,
    trench: &'a SeedDist,
    rift: &'a SeedDist,
    transform: &'a SeedDist,
    radius: f64,
    lith_seed: u64,
    surface_seed: u64,
    bg_seed: u64,
    old_seed: u64,
    arc_seed: u64,
    xform_seed: u64,
    mod_seed: u64,
}

fn make_plates(settings: &WorldSettings) -> Vec<Plate> {
    let n = settings.plate_count as usize;
    let mut plates = Vec::with_capacity(n);
    let site_seed = derive_seed(settings.seed, SeedDomain::Plates);
    let weight_seed = derive_seed(settings.seed, SeedDomain::PlateWeights);
    let axis_seed = derive_seed(settings.seed, SeedDomain::EulerPoles);
    let speed_seed = derive_seed(settings.seed, SeedDomain::AngularSpeed);
    let rank_seed = derive_seed(settings.seed, SeedDomain::SubductionRank);
    for i in 0..n {
        let u_w = hash_u01(weight_seed, i as u64, 0);
        let u_s = hash_u01(speed_seed, i as u64, 0);
        let deg_per_myr = 0.25 + 0.75 * u_s;
        let omega = deg_per_myr * std::f64::consts::PI / 180.0 / 1.0e6 * settings.tectonic_activity;
        let rank = split_u32(rank_seed, i as u64);
        plates.push(Plate {
            site: random_unit(site_seed, i as u64),
            axis: random_unit(axis_seed, i as u64),
            omega_rad_yr: omega,
            subduction_rank: rank,
            weight_rad: 0.30 * u_w * u_w * u_w,
            crust_majority: 0,
        });
    }
    plates
}

fn split_u32(seed: u64, i: u64) -> u32 {
    let h = crate::noise::splitmix64(seed ^ i.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    (h >> 32) as u32
}

fn assign_plates(grid: &CubeSphere, plates: &[Plate], seed: u64) -> Vec<u8> {
    let warp_seed = derive_seed(seed, SeedDomain::PlateWarp);
    let warp_seed_b = derive_seed_salt(seed, SeedDomain::PlateWarp, 1);
    let mut plate = vec![0u8; grid.cell_count()];
    plate.par_iter_mut().enumerate().for_each(|(cell, slot)| {
        let p = warp(warp_seed, grid.position(cell as u32), 0.18, 2.0);
        let p = warp(warp_seed_b, p, 0.06, 6.0);
        let mut best = f64::INFINITY;
        let mut best_k = 0u8;
        for (k, plate_k) in plates.iter().enumerate() {
            let score = angle_between(p, plate_k.site) - plate_k.weight_rad;
            let replace = match score.total_cmp(&best) {
                Ordering::Less => true,
                Ordering::Equal => (k as u8) < best_k,
                Ordering::Greater => false,
            };
            if replace {
                best = score;
                best_k = k as u8;
            }
        }
        *slot = best_k;
    });
    plate
}

fn enforce_connectivity(
    grid: &CubeSphere,
    plate: &mut [u8],
    plates: &[Plate],
) -> Result<u32, String> {
    let n = grid.cell_count();
    let plate_count = plates.len();
    let mut queue = VecDeque::new();
    let mut reassigned = 0u32;
    for _iter in 0..256 {
        let mut best_ang = vec![f64::INFINITY; plate_count];
        let mut best_cell = vec![NONE; plate_count];
        for (c, &pid) in plate.iter().enumerate() {
            let p = pid as usize;
            if p >= plate_count {
                continue;
            }
            let ang = angle_between(grid.position(c as u32), plates[p].site);
            let better = match ang.total_cmp(&best_ang[p]) {
                Ordering::Less => true,
                Ordering::Equal => (c as u32) < best_cell[p],
                Ordering::Greater => false,
            };
            if better {
                best_ang[p] = ang;
                best_cell[p] = c as u32;
            }
        }
        let mut keep = vec![0u8; n];
        for (p, &seed) in best_cell.iter().enumerate() {
            if seed == NONE {
                continue;
            }
            let s = seed as usize;
            if plate[s] as usize != p {
                continue;
            }
            keep[s] = 1;
            queue.push_back(seed);
            while let Some(c) = queue.pop_front() {
                for nb in grid.neighbors(c) {
                    if nb == NONE {
                        continue;
                    }
                    let u = nb as usize;
                    if keep[u] == 0 && plate[u] as usize == p {
                        keep[u] = 1;
                        queue.push_back(nb);
                    }
                }
            }
        }
        let mut seen = keep.clone();
        let mut changed = false;
        let mut fragment = false;
        for start in 0..n {
            if seen[start] != 0 {
                continue;
            }
            fragment = true;
            let origin = plate[start];
            let mut cells = Vec::new();
            queue.clear();
            queue.push_back(start as u32);
            seen[start] = 1;
            while let Some(c) = queue.pop_front() {
                cells.push(c);
                for nb in grid.neighbors(c) {
                    if nb == NONE {
                        continue;
                    }
                    let u = nb as usize;
                    if seen[u] == 0 && plate[u] == origin {
                        seen[u] = 1;
                        queue.push_back(nb);
                    }
                }
            }
            let mut counts = vec![0u32; plate_count];
            for &c in &cells {
                for nb in grid.neighbors(c) {
                    if nb == NONE {
                        continue;
                    }
                    let u = nb as usize;
                    if keep[u] == 0 {
                        continue;
                    }
                    let other = plate[u] as usize;
                    if other != origin as usize && other < plate_count {
                        counts[other] += 1;
                    }
                }
            }
            if let Some(target) = best_count(&counts) {
                reassigned = reassigned.saturating_add(1);
                for c in cells {
                    plate[c as usize] = target as u8;
                }
                changed = true;
            }
        }
        if !fragment {
            return Ok(reassigned);
        }
        if changed {
            continue;
        }
        let Some(start) = (0..n).find(|&c| keep[c] == 0) else {
            return Ok(reassigned);
        };
        let origin = plate[start];
        let mut cells = Vec::new();
        let mut local = vec![0u8; n];
        queue.clear();
        queue.push_back(start as u32);
        local[start] = 1;
        while let Some(c) = queue.pop_front() {
            cells.push(c);
            for nb in grid.neighbors(c) {
                if nb == NONE {
                    continue;
                }
                let u = nb as usize;
                if local[u] == 0 && keep[u] == 0 && plate[u] == origin {
                    local[u] = 1;
                    queue.push_back(nb);
                }
            }
        }
        let mut counts = vec![0u32; plate_count];
        for &c in &cells {
            for nb in grid.neighbors(c) {
                if nb == NONE {
                    continue;
                }
                let other = plate[nb as usize] as usize;
                if other != origin as usize && other < plate_count {
                    counts[other] += 1;
                }
            }
        }
        let Some(target) = best_count(&counts) else {
            return Err("plate fragment has no neighbouring plate".to_string());
        };
        reassigned = reassigned.saturating_add(1);
        for c in cells {
            plate[c as usize] = target as u8;
        }
    }
    Err("plate connectivity did not settle".to_string())
}

fn best_count(counts: &[u32]) -> Option<usize> {
    let mut best: Option<(u32, usize)> = None;
    for (p, &cnt) in counts.iter().enumerate() {
        if cnt == 0 {
            continue;
        }
        match best {
            None => best = Some((cnt, p)),
            Some((bc, bp)) if cnt > bc || (cnt == bc && p < bp) => best = Some((cnt, p)),
            _ => {}
        }
    }
    best.map(|(_, p)| p)
}

struct Blob {
    centre: [f64; 3],
    radius: f64,
}

fn make_crust(
    grid: &CubeSphere,
    settings: &WorldSettings,
    seed: u64,
) -> (Vec<u8>, ContinentLayout, f64) {
    let resolved = resolve_layout(settings.continent_layout, seed);
    let blobs = layout_blobs(resolved, seed);
    let warp_seed = derive_seed(seed, SeedDomain::ContinentWarp);
    let warp_seed_b = derive_seed_salt(seed, SeedDomain::ContinentWarp, 1);
    let warp_seed_c = derive_seed_salt(seed, SeedDomain::ContinentWarp, 2);
    let fbm_seed = derive_seed(seed, SeedDomain::ContinentFbm);
    let n = grid.cell_count();
    let mut field = vec![0.0f32; n];
    field.par_iter_mut().enumerate().for_each(|(cell, slot)| {
        let p = grid.position(cell as u32);
        let warped = warp(warp_seed, p, 0.35, 1.5);
        let warped = warp(warp_seed_b, warped, 0.12, 4.0);
        let warped = warp(warp_seed_c, warped, 0.04, 12.0);
        let mut best = f64::NEG_INFINITY;
        for blob in &blobs {
            let v = 1.0 - angle_between(warped, blob.centre) / blob.radius;
            if v > best {
                best = v;
            }
        }
        let noise = fbm(fbm_seed, p, 3.0, 6, 2.0, 0.5);
        *slot = (best + 0.5 * noise) as f32;
    });
    // Continental crust that ends below sea level is shelf: +0.06 of the sphere
    // gives Earth-like shelves (about 6 %).
    let crust_fraction = (settings.land_fraction + 0.06).clamp(0.03, 0.95);
    let mut total = 0.0f64;
    let mut order = Vec::with_capacity(n);
    for c in 0..n {
        total += grid.area_m2(c as u32);
        order.push(c as u32);
    }
    order.par_sort_unstable_by(|&a, &b| {
        field[a as usize]
            .total_cmp(&field[b as usize])
            .then(a.cmp(&b))
    });
    let target = crust_fraction * total;
    let mut acc = 0.0f64;
    let mut crust = vec![0u8; n];
    for &c in order.iter().rev() {
        if acc >= target {
            break;
        }
        crust[c as usize] = 1;
        acc += grid.area_m2(c);
    }
    (crust, resolved, crust_fraction)
}

fn resolve_layout(layout: ContinentLayout, seed: u64) -> ContinentLayout {
    if layout != ContinentLayout::Mixed {
        return layout;
    }
    let u = hash_u01(derive_seed(seed, SeedDomain::LayoutChoice), 0, 0);
    if u < 1.0 / 3.0 {
        ContinentLayout::Pangea
    } else if u < 2.0 / 3.0 {
        ContinentLayout::Few
    } else {
        ContinentLayout::Many
    }
}

fn layout_blobs(layout: ContinentLayout, seed: u64) -> Vec<Blob> {
    let s = derive_seed(seed, SeedDomain::Continents);
    let mut blobs = Vec::new();
    match layout {
        ContinentLayout::Pangea | ContinentLayout::Mixed => {
            let centre = random_unit(s, 0);
            let radius = lerp(0.9, 1.1, hash_u01(s, 0, 1));
            blobs.push(Blob { centre, radius });
            let extra = 2 + (split_u32(s, 1) % 3);
            for i in 0..extra {
                let dist = lerp(0.2, 0.8, hash_u01(s, i as u64 + 2, 1));
                let dir = tangent_dir(s, i as u64 + 2, centre);
                let child = step_gc(centre, dir, dist);
                let radius = lerp(0.35, 0.6, hash_u01(s, i as u64 + 2, 2));
                blobs.push(Blob {
                    centre: child,
                    radius,
                });
            }
        }
        ContinentLayout::Few => push_blobs(&mut blobs, s, 2 + (split_u32(s, 2) % 3), 0.45, 0.8),
        ContinentLayout::Many => push_blobs(&mut blobs, s, 6 + (split_u32(s, 3) % 5), 0.25, 0.5),
    }
    blobs
}

fn push_blobs(blobs: &mut Vec<Blob>, seed: u64, count: u32, r0: f64, r1: f64) {
    for i in 0..count {
        blobs.push(Blob {
            centre: random_unit(seed, i as u64 + 10),
            radius: lerp(r0, r1, hash_u01(seed, i as u64, 3)),
        });
    }
}

fn with_majority(
    grid: &CubeSphere,
    mut plates: Vec<Plate>,
    plate: &[u8],
    crust: &[u8],
) -> Vec<Plate> {
    let mut cont = vec![0.0f64; plates.len()];
    let mut oce = vec![0.0f64; plates.len()];
    for c in 0..plate.len() {
        let p = plate[c] as usize;
        if p >= plates.len() {
            continue;
        }
        let area = grid.area_m2(c as u32);
        if crust[c] == 1 {
            cont[p] += area;
        } else {
            oce[p] += area;
        }
    }
    for (i, plate) in plates.iter_mut().enumerate() {
        plate.crust_majority = if cont[i] > oce[i] { 1 } else { 0 };
    }
    plates
}

fn make_boundaries(
    grid: &CubeSphere,
    plates: &[Plate],
    plate: &[u8],
    crust: &[u8],
    radius: f64,
) -> Vec<BoundaryRecord> {
    let n = plate.len();
    let mut seen = vec![0u32; n];
    let mut generation = 0u32;
    let mut queue = VecDeque::new();
    let mut out = Vec::new();
    for c in 0..n {
        let pa = plate[c];
        let nbs = grid.neighbors(c as u32);
        let mut chosen = NONE;
        for slot in [0usize, 2, 4, 6] {
            let nb = nbs[slot];
            if nb != NONE && plate[nb as usize] != pa {
                chosen = nb;
                break;
            }
        }
        if chosen == NONE {
            continue;
        }
        let pb = plate[chosen as usize];
        let pc = grid.position(c as u32);
        let pk = grid.position(chosen);
        let axis = tangent_toward(pc, pk);
        let n_hat = pair_normal(
            grid,
            plate,
            c as u32,
            pa,
            pb,
            &mut NormalScratch {
                seen: &mut seen,
                generation: &mut generation,
                queue: &mut queue,
            },
        )
        .unwrap_or(axis);
        let va = velocity(&plates[pa as usize], pc, radius);
        let vb = velocity(&plates[pb as usize], pc, radius);
        let vrel = [va[0] - vb[0], va[1] - vb[1], va[2] - vb[2]];
        let convergence = dot3(vrel, n_hat);
        let tangential = [
            vrel[0] - n_hat[0] * convergence,
            vrel[1] - n_hat[1] * convergence,
            vrel[2] - n_hat[2] * convergence,
        ];
        let shear = norm(tangential);
        out.push(BoundaryRecord {
            cell: c as u32,
            neighbor: chosen,
            plate_a: pa,
            plate_b: pb,
            crust_a: crust[c],
            crust_b: crust[chosen as usize],
            kind: BoundaryKind::Transform,
            convergence_m_yr: convergence as f32,
            shear_m_yr: shear as f32,
            convergence_raw_m_yr: convergence as f32,
            shear_raw_m_yr: shear as f32,
            subducts: SubductSide::None,
        });
    }
    smooth_along_boundary(grid, &mut out);
    for rec in &mut out {
        let kind = kind_from_motion(f64::from(rec.convergence_m_yr), f64::from(rec.shear_m_yr));
        rec.kind = kind;
        rec.subducts = subduct_side(
            kind,
            rec.crust_a,
            rec.crust_b,
            &plates[rec.plate_a as usize],
            &plates[rec.plate_b as usize],
            rec.plate_a,
            rec.plate_b,
        );
    }
    out
}

struct NormalScratch<'a> {
    seen: &'a mut [u32],
    generation: &'a mut u32,
    queue: &'a mut VecDeque<(u32, u8)>,
}

fn pair_normal(
    grid: &CubeSphere,
    plate: &[u8],
    cell: u32,
    plate_a: u8,
    plate_b: u8,
    scratch: &mut NormalScratch<'_>,
) -> Option<[f64; 3]> {
    *scratch.generation = scratch.generation.wrapping_add(1);
    if *scratch.generation == 0 {
        scratch.seen.fill(0);
        *scratch.generation = 1;
    }
    let gen = *scratch.generation;
    scratch.queue.clear();
    scratch.seen[cell as usize] = gen;
    scratch.queue.push_back((cell, 0));
    let mut sum_a = [0.0f64; 3];
    let mut sum_b = [0.0f64; 3];
    let mut count_a = 0u32;
    let mut count_b = 0u32;
    while let Some((current, depth)) = scratch.queue.pop_front() {
        let pid = plate[current as usize];
        let p = grid.position(current);
        if pid == plate_a {
            sum_a[0] += p[0];
            sum_a[1] += p[1];
            sum_a[2] += p[2];
            count_a += 1;
        } else if pid == plate_b {
            sum_b[0] += p[0];
            sum_b[1] += p[1];
            sum_b[2] += p[2];
            count_b += 1;
        }
        if depth >= 3 {
            continue;
        }
        for nb in grid.neighbors(current) {
            if nb == NONE || scratch.seen[nb as usize] == gen {
                continue;
            }
            scratch.seen[nb as usize] = gen;
            scratch.queue.push_back((nb, depth + 1));
        }
    }
    if count_a == 0 || count_b == 0 {
        return None;
    }
    let centroid_a = normalize(sum_a);
    let centroid_b = normalize(sum_b);
    let delta = [
        centroid_b[0] - centroid_a[0],
        centroid_b[1] - centroid_a[1],
        centroid_b[2] - centroid_a[2],
    ];
    let pc = grid.position(cell);
    let radial = dot3(delta, pc);
    let tangent = [
        delta[0] - pc[0] * radial,
        delta[1] - pc[1] * radial,
        delta[2] - pc[2] * radial,
    ];
    if norm(tangent) < 1.0e-8 {
        None
    } else {
        Some(normalize(tangent))
    }
}

fn smooth_along_boundary(grid: &CubeSphere, records: &mut [BoundaryRecord]) {
    let m = records.len();
    if m == 0 {
        return;
    }
    let mut cell_record = vec![NONE; grid.cell_count()];
    for (i, rec) in records.iter().enumerate() {
        cell_record[rec.cell as usize] = i as u32;
    }
    let mut adj = vec![Vec::<(u32, f32)>::new(); m];
    for (i, rec) in records.iter().enumerate() {
        let key = pair_key(rec.plate_a, rec.plate_b);
        let nbs = grid.neighbors(rec.cell);
        for (k, &nb) in nbs.iter().enumerate() {
            if nb == NONE {
                continue;
            }
            let j = cell_record[nb as usize];
            if j == NONE {
                continue;
            }
            let other = &records[j as usize];
            if pair_key(other.plate_a, other.plate_b) != key {
                continue;
            }
            adj[i].push((j, grid.neighbor_distance(rec.cell, k)));
        }
    }
    let mut dist = vec![f32::INFINITY; m];
    let mut stamp = vec![0u32; m];
    let mut generation = 0u32;
    let mut heap = BinaryHeap::new();
    let mut touched = Vec::new();
    let mut smoothed_c = vec![0.0f32; m];
    let mut smoothed_s = vec![0.0f32; m];
    for src in 0..m {
        generation = generation.wrapping_add(1);
        if generation == 0 {
            stamp.fill(0);
            generation = 1;
        }
        heap.clear();
        touched.clear();
        dist[src] = 0.0;
        stamp[src] = generation;
        touched.push(src);
        heap.push(std::cmp::Reverse(HeapItem {
            key: f32_key(0.0),
            cell: src as u32,
        }));
        while let Some(std::cmp::Reverse(item)) = heap.pop() {
            let i = item.cell as usize;
            if stamp[i] != generation || item.key != f32_key(dist[i]) {
                continue;
            }
            for &(j, step) in &adj[i] {
                let nd = dist[i] + step;
                if nd > SMOOTH_RADIUS_M {
                    continue;
                }
                let j = j as usize;
                let fresh = stamp[j] != generation;
                if fresh || nd < dist[j] {
                    if fresh {
                        touched.push(j);
                    }
                    dist[j] = nd;
                    stamp[j] = generation;
                    heap.push(std::cmp::Reverse(HeapItem {
                        key: f32_key(nd),
                        cell: j as u32,
                    }));
                }
            }
        }
        let mut sum_c = 0.0f64;
        let mut sum_s = 0.0f64;
        for &i in &touched {
            sum_c += f64::from(records[i].convergence_raw_m_yr);
            sum_s += f64::from(records[i].shear_raw_m_yr);
        }
        let count = touched.len() as f64;
        smoothed_c[src] = (sum_c / count) as f32;
        smoothed_s[src] = (sum_s / count) as f32;
    }
    for (rec, (conv, shear)) in records
        .iter_mut()
        .zip(smoothed_c.into_iter().zip(smoothed_s))
    {
        rec.convergence_m_yr = conv;
        rec.shear_m_yr = shear;
    }
}

fn pair_key(a: u8, b: u8) -> u16 {
    let (lo, hi) = if a < b { (a, b) } else { (b, a) };
    (u16::from(lo) << 8) | u16::from(hi)
}

fn kind_from_motion(convergence: f64, shear: f64) -> BoundaryKind {
    let speed = (convergence * convergence + shear * shear).sqrt();
    if convergence > 0.3 * speed {
        BoundaryKind::Convergent
    } else if convergence < -0.3 * speed {
        BoundaryKind::Divergent
    } else {
        BoundaryKind::Transform
    }
}

fn boundary_kind_field(n: usize, boundaries: &[BoundaryRecord]) -> Vec<u8> {
    let mut kind = vec![0u8; n];
    for rec in boundaries {
        kind[rec.cell as usize] = match rec.kind {
            BoundaryKind::Convergent => 1,
            BoundaryKind::Divergent => 2,
            BoundaryKind::Transform => 3,
        };
    }
    kind
}

const SMOOTH_RADIUS_M: f32 = 250_000.0;
const CLASS_CAP_M: f32 = 1_500_000.0;

struct SeedDist {
    dist: Vec<f32>,
    seed: Vec<u32>,
}

struct ClassFields {
    collision: SeedDist,
    arc: SeedDist,
    trench: SeedDist,
    rift: SeedDist,
    transform: SeedDist,
}

fn class_fields(grid: &CubeSphere, plate: &[u8], boundaries: &[BoundaryRecord]) -> ClassFields {
    let mut heap = BinaryHeap::new();
    ClassFields {
        collision: seed_distance(grid, plate, boundaries, &mut heap, false, is_collision),
        arc: seed_distance(grid, plate, boundaries, &mut heap, true, is_arc),
        trench: seed_distance(grid, plate, boundaries, &mut heap, true, is_trench),
        rift: seed_distance(grid, plate, boundaries, &mut heap, false, is_rift),
        transform: seed_distance(grid, plate, boundaries, &mut heap, false, is_transform),
    }
}

fn is_collision(rec: &BoundaryRecord) -> bool {
    rec.kind == BoundaryKind::Convergent && rec.crust_a == 1 && rec.crust_b == 1
}

fn is_arc(rec: &BoundaryRecord) -> bool {
    rec.kind == BoundaryKind::Convergent
        && rec.subducts == SubductSide::Other
        && !(rec.crust_a == 1 && rec.crust_b == 1)
}

fn is_trench(rec: &BoundaryRecord) -> bool {
    rec.kind == BoundaryKind::Convergent
        && rec.subducts == SubductSide::This
        && !(rec.crust_a == 1 && rec.crust_b == 1)
}

fn is_rift(rec: &BoundaryRecord) -> bool {
    rec.kind == BoundaryKind::Divergent && rec.crust_a == 1 && rec.crust_b == 1
}

fn is_transform(rec: &BoundaryRecord) -> bool {
    rec.kind == BoundaryKind::Transform
}

fn seed_distance(
    grid: &CubeSphere,
    plate: &[u8],
    boundaries: &[BoundaryRecord],
    heap: &mut BinaryHeap<std::cmp::Reverse<HeapItem>>,
    own_plate: bool,
    pred: fn(&BoundaryRecord) -> bool,
) -> SeedDist {
    let n = grid.cell_count();
    let mut dist = vec![f32::INFINITY; n];
    let mut seed = vec![NONE; n];
    heap.clear();
    for (i, rec) in boundaries.iter().enumerate() {
        if !pred(rec) {
            continue;
        }
        let c = rec.cell as usize;
        dist[c] = 0.0;
        seed[c] = i as u32;
        heap.push(std::cmp::Reverse(HeapItem {
            key: f32_key(0.0),
            cell: rec.cell,
        }));
    }
    while let Some(std::cmp::Reverse(item)) = heap.pop() {
        let c = item.cell as usize;
        if item.key != f32_key(dist[c]) {
            continue;
        }
        let src = seed[c];
        if src == NONE {
            continue;
        }
        let plate_limit = boundaries[src as usize].plate_a;
        let nbs = grid.neighbors(item.cell);
        for (k, &nb) in nbs.iter().enumerate() {
            if nb == NONE {
                continue;
            }
            if own_plate && plate[nb as usize] != plate_limit {
                continue;
            }
            let step = grid.neighbor_distance(item.cell, k);
            let nd = dist[c] + step;
            if nd > CLASS_CAP_M || nd >= dist[nb as usize] {
                continue;
            }
            dist[nb as usize] = nd;
            seed[nb as usize] = src;
            heap.push(std::cmp::Reverse(HeapItem {
                key: f32_key(nd),
                cell: nb,
            }));
        }
    }
    SeedDist { dist, seed }
}

fn seeded<'a>(
    boundaries: &'a [BoundaryRecord],
    field: &SeedDist,
    cell: usize,
) -> Option<(f64, &'a BoundaryRecord)> {
    let distance = field.dist[cell];
    if !distance.is_finite() {
        return None;
    }
    let rec = boundaries.get(field.seed[cell] as usize)?;
    Some((f64::from(distance), rec))
}

fn subduct_side(
    kind: BoundaryKind,
    crust_a: u8,
    crust_b: u8,
    plate_a: &Plate,
    plate_b: &Plate,
    id_a: u8,
    id_b: u8,
) -> SubductSide {
    if kind != BoundaryKind::Convergent {
        return SubductSide::None;
    }
    if crust_a == 0 && crust_b == 1 {
        SubductSide::This
    } else if crust_a == 1 && crust_b == 0 {
        SubductSide::Other
    } else if crust_a == 0 && crust_b == 0 {
        if plate_a.subduction_rank < plate_b.subduction_rank
            || (plate_a.subduction_rank == plate_b.subduction_rank && id_a < id_b)
        {
            SubductSide::This
        } else {
            SubductSide::Other
        }
    } else {
        SubductSide::None
    }
}

fn distance_on_plate(
    grid: &CubeSphere,
    plate: &[u8],
    boundaries: &[BoundaryRecord],
) -> (Vec<f32>, Vec<u32>) {
    let n = grid.cell_count();
    let mut dist = vec![f32::INFINITY; n];
    let mut record = vec![NONE; n];
    let mut heap = BinaryHeap::new();
    for (i, b) in boundaries.iter().enumerate() {
        let c = b.cell as usize;
        dist[c] = 0.0;
        record[c] = i as u32;
        heap.push(std::cmp::Reverse(HeapItem {
            key: f32_key(0.0),
            cell: b.cell,
        }));
    }
    while let Some(std::cmp::Reverse(item)) = heap.pop() {
        let c = item.cell as usize;
        if item.key != f32_key(dist[c]) {
            continue;
        }
        let nbs = grid.neighbors(item.cell);
        for (k, &nb) in nbs.iter().enumerate() {
            if nb == NONE || plate[nb as usize] != plate[c] {
                continue;
            }
            let step = grid.neighbor_distance(item.cell, k);
            let nd = dist[c] + step;
            if nd < dist[nb as usize] {
                dist[nb as usize] = nd;
                record[nb as usize] = record[c];
                heap.push(std::cmp::Reverse(HeapItem {
                    key: f32_key(nd),
                    cell: nb,
                }));
            }
        }
    }
    (dist, record)
}

fn ridge_and_edge(
    grid: &CubeSphere,
    crust: &[u8],
    boundaries: &[BoundaryRecord],
) -> (Vec<f32>, Vec<f32>) {
    let n = grid.cell_count();
    let mut ridge = vec![f32::INFINITY; n];
    let mut edge = vec![f32::INFINITY; n];
    let mut heap = BinaryHeap::new();
    for b in boundaries {
        if b.kind == BoundaryKind::Divergent && b.crust_a == 0 && b.crust_b == 0 {
            ridge[b.cell as usize] = 0.0;
            heap.push(std::cmp::Reverse(HeapItem {
                key: f32_key(0.0),
                cell: b.cell,
            }));
        }
    }
    relax_everywhere(grid, &mut ridge, &mut heap);
    heap.clear();
    for c in 0..n {
        let nbs = grid.neighbors(c as u32);
        let mut edge_cell = false;
        for nb in nbs {
            if nb != NONE && crust[nb as usize] != crust[c] {
                edge_cell = true;
                break;
            }
        }
        if edge_cell {
            edge[c] = 0.0;
            heap.push(std::cmp::Reverse(HeapItem {
                key: f32_key(0.0),
                cell: c as u32,
            }));
        }
    }
    relax_everywhere(grid, &mut edge, &mut heap);
    for (c, d) in edge.iter_mut().enumerate() {
        if crust[c] == 0 {
            *d = -*d;
        }
    }
    (ridge, edge)
}

fn relax_everywhere(
    grid: &CubeSphere,
    dist: &mut [f32],
    heap: &mut BinaryHeap<std::cmp::Reverse<HeapItem>>,
) {
    while let Some(std::cmp::Reverse(item)) = heap.pop() {
        let c = item.cell as usize;
        if item.key != f32_key(dist[c]) {
            continue;
        }
        let nbs = grid.neighbors(item.cell);
        for (k, &nb) in nbs.iter().enumerate() {
            if nb == NONE {
                continue;
            }
            let nd = dist[c] + grid.neighbor_distance(item.cell, k);
            if nd < dist[nb as usize] {
                dist[nb as usize] = nd;
                heap.push(std::cmp::Reverse(HeapItem {
                    key: f32_key(nd),
                    cell: nb,
                }));
            }
        }
    }
}

fn make_hotspots(
    grid: &CubeSphere,
    settings: &WorldSettings,
    plates: &[Plate],
    plate: &[u8],
) -> Vec<Hotspot> {
    let seed = derive_seed(settings.seed, SeedDomain::Hotspots);
    let trail_seed = derive_seed(settings.seed, SeedDomain::HotspotTrail);
    let u = hash_u01(seed, 0, 0);
    let count = ((2.0 + 5.0 * u) * settings.tectonic_activity)
        .round()
        .max(0.0) as u32;
    let mut hotspots = Vec::with_capacity(count as usize);
    for i in 0..count {
        let origin = random_unit(seed, i as u64 + 1);
        let npts = 4 + (split_u32(trail_seed, i as u64) % 5);
        let spacing = lerp(300_000.0, 500_000.0, hash_u01(trail_seed, i as u64, 1));
        let mut pos = origin;
        let mut points = Vec::with_capacity(npts as usize);
        for k in 0..npts {
            points.push(pos);
            if k + 1 == npts {
                break;
            }
            let cell = grid.cell_at(pos);
            let pid = plate[cell as usize] as usize;
            let vel = if pid < plates.len() {
                velocity(&plates[pid], pos, settings.radius_m)
            } else {
                [0.0, 0.0, 0.0]
            };
            let dir = opposite_tangent(pos, vel);
            pos = step_gc(pos, dir, spacing / settings.radius_m);
        }
        hotspots.push(Hotspot { origin, points });
    }
    hotspots
}

fn eval_cell(ctx: &FieldCtx<'_>, i: usize) -> (f32, f32, f32) {
    let p = ctx.grid.position(i as u32);
    let crust = ctx.crust[i];
    let d_ridge = f64::from(ctx.d_ridge[i]);
    let d_edge = f64::from(ctx.d_edge[i]);
    let ocean_floor = -2600.0 - 2900.0 * (d_ridge / 3.0e6).clamp(0.0, 1.0).sqrt();
    let mut base = if crust == 1 {
        let inland = smoothstep(0.0, 4.0e5, d_edge.max(0.0));
        let noise = fbm(ctx.surface_seed, p, 4.0, 5, 2.0, 0.5);
        (30.0 + 270.0 * inland + 120.0 * noise).max(5.0)
    } else if d_edge.is_finite() {
        let t = smoothstep(0.0, 150_000.0, d_edge.abs());
        -150.0 + (ocean_floor + 150.0) * t
    } else {
        ocean_floor
    };
    if let Some((d, rec)) = seeded(ctx.boundaries, ctx.trench, i) {
        let s_trench = (f64::from(rec.convergence_m_yr) / 0.05).clamp(0.4, 1.5);
        base += -(3500.0 * s_trench) * gauss(d, 60_000.0);
    }
    if let Some((d, rec)) = seeded(ctx.boundaries, ctx.arc, i) {
        if rec.crust_a == 0 && rec.crust_b == 0 {
            let s_trench = (f64::from(rec.convergence_m_yr) / 0.05).clamp(0.4, 1.5);
            let arc = ridged(ctx.arc_seed, p, 12.0, 5, 2.0, 0.5);
            base += 4000.0 * s_trench * bump((d - 150_000.0) / 50_000.0) * (0.35 + 0.65 * arc);
        }
    }
    add_hotspot_base(ctx, p, crust, &mut base);
    let uplift = uplift_at(ctx, i, p, crust, d_edge);
    let mut k = (0.5 * fbm(ctx.lith_seed, p, 6.0, 8, 2.0, 0.55)).exp();
    if d_edge > 800_000.0 {
        k *= 0.7;
    }
    (base as f32, uplift as f32, k as f32)
}

fn add_hotspot_base(ctx: &FieldCtx<'_>, p: [f64; 3], crust: u8, base: &mut f64) {
    for hs in ctx.hotspots {
        for (k, point) in hs.points.iter().enumerate() {
            let sigma = if crust == 0 { 70_000.0 } else { 250_000.0 };
            let amp = if crust == 0 { 4800.0 } else { 300.0 };
            let min_dot = (8.0 * sigma / ctx.radius).cos();
            if dot3(p, *point) < min_dot {
                continue;
            }
            let dist = angle_between(p, *point) * ctx.radius;
            let r2 = (dist / sigma) * (dist / sigma);
            if r2 > 64.0 {
                continue;
            }
            *base += amp * 0.62f64.powi(k as i32) * (-r2).exp();
        }
    }
}

fn uplift_at(ctx: &FieldCtx<'_>, i: usize, p: [f64; 3], crust: u8, d_edge: f64) -> f64 {
    let mut boundary = 0.0;
    if let Some((d, rec)) = seeded(ctx.boundaries, ctx.collision, i) {
        let s = (f64::from(rec.convergence_m_yr) / 0.05).clamp(0.3, 1.5);
        boundary += 4.0e-3 * s * gauss(d, 250_000.0) + 1.0e-3 * s * gauss(d, 500_000.0);
    }
    if let Some((d, rec)) = seeded(ctx.boundaries, ctx.arc, i) {
        let s = (f64::from(rec.convergence_m_yr) / 0.05).clamp(0.3, 1.5);
        if rec.crust_a == 1 && rec.crust_b == 0 {
            boundary += 3.0e-3 * s * bump((d - 200_000.0) / 120_000.0);
        } else if rec.crust_a == 0 && rec.crust_b == 0 {
            boundary += 1.5e-3 * s * bump((d - 150_000.0) / 60_000.0);
        }
    }
    if let Some((d, _)) = seeded(ctx.boundaries, ctx.rift, i) {
        boundary += -1.0e-3 * gauss(d, 40_000.0) + 0.8e-3 * bump((d - 90_000.0) / 40_000.0);
    }
    if let Some((d, _)) = seeded(ctx.boundaries, ctx.transform, i) {
        if d < 400_000.0 {
            let rg = ridged(ctx.xform_seed, p, 20.0, 4, 2.0, 0.5);
            boundary += 0.3e-3 * gauss(d, 50_000.0) * rg;
        }
    }
    if boundary.abs() > 1e-12 {
        let m = ridged(ctx.mod_seed, p, 10.0, 4, 2.0, 0.5);
        boundary *= 0.6 + 0.4 * m;
    }
    let mut background = 0.0;
    if crust == 1 {
        let f = fbm(ctx.bg_seed, p, 2.0, 5, 2.0, 0.5);
        background = (2.0e-4 * (1.0 + 0.8 * f)).clamp(0.0, 4.0e-4);
    }
    let mut old = 0.0;
    if crust == 1 && d_edge > 300_000.0 {
        let r = ridged(ctx.old_seed, p, 3.0, 5, 2.0, 0.5);
        old = 4.0e-4 * (r - 0.6).max(0.0) / 0.4;
    }
    let mut dome = 0.0;
    if crust == 1 {
        for hs in ctx.hotspots {
            for point in &hs.points {
                let min_dot = (8.0 * 250_000.0 / ctx.radius).cos();
                if dot3(p, *point) < min_dot {
                    continue;
                }
                let dist = angle_between(p, *point) * ctx.radius;
                let r2 = (dist / 250_000.0) * (dist / 250_000.0);
                if r2 <= 64.0 {
                    dome += 6.0e-4 * (-r2).exp();
                }
            }
        }
    }
    boundary + background + old + dome
}

pub(crate) fn velocity(plate: &Plate, p: [f64; 3], radius: f64) -> [f64; 3] {
    let c = crate::grid::cross(plate.axis, p);
    let s = plate.omega_rad_yr * radius;
    [c[0] * s, c[1] * s, c[2] * s]
}

fn tangent_toward(from: [f64; 3], to: [f64; 3]) -> [f64; 3] {
    let d = dot3(from, to);
    normalize([
        to[0] - from[0] * d,
        to[1] - from[1] * d,
        to[2] - from[2] * d,
    ])
}

fn opposite_tangent(p: [f64; 3], velocity: [f64; 3]) -> [f64; 3] {
    let speed = norm(velocity);
    let raw = if speed < 1e-16 {
        let alt = crate::grid::cross(p, [0.0, 0.0, 1.0]);
        if norm(alt) < 1e-8 {
            crate::grid::cross(p, [1.0, 0.0, 0.0])
        } else {
            alt
        }
    } else {
        [-velocity[0], -velocity[1], -velocity[2]]
    };
    let radial = dot3(raw, p);
    normalize([
        raw[0] - p[0] * radial,
        raw[1] - p[1] * radial,
        raw[2] - p[2] * radial,
    ])
}

fn tangent_dir(seed: u64, i: u64, p: [f64; 3]) -> [f64; 3] {
    let r = random_unit(seed, i.wrapping_add(1000));
    let radial = dot3(r, p);
    normalize([
        r[0] - p[0] * radial,
        r[1] - p[1] * radial,
        r[2] - p[2] * radial,
    ])
}

fn step_gc(p: [f64; 3], tangent: [f64; 3], angle: f64) -> [f64; 3] {
    let (s, c) = angle.sin_cos();
    normalize([
        p[0] * c + tangent[0] * s,
        p[1] * c + tangent[1] * s,
        p[2] * c + tangent[2] * s,
    ])
}

fn angle_between(a: [f64; 3], b: [f64; 3]) -> f64 {
    let c = crate::grid::cross(a, b);
    let s = (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt();
    s.atan2(dot3(a, b).clamp(-1.0, 1.0))
}

fn norm(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn smoothstep(e0: f64, e1: f64, x: f64) -> f64 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn gauss(d: f64, sigma: f64) -> f64 {
    let x = d / sigma;
    (-x * x).exp()
}

fn bump(x: f64) -> f64 {
    (-x * x).exp()
}

fn f32_key(x: f32) -> u32 {
    let bits = x.to_bits();
    if bits & 0x8000_0000 != 0 {
        !bits
    } else {
        bits ^ 0x8000_0000
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct HeapItem {
    key: u32,
    cell: u32,
}

impl Ord for HeapItem {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key.cmp(&other.key).then(self.cell.cmp(&other.cell))
    }
}

impl PartialOrd for HeapItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
