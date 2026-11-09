use crate::grid::{normalize, NONE};
use crate::noise::{derive_seed, random_unit, SeedDomain};
use crate::tectonics::volcano_field;
use crate::{generate, NoProgress, Preset, World, WorldSettings};

const RADIUS_M: f64 = 6_371_000.0;
const SPACING_M: f64 = 140_000.0;
const SIGMA_M: f64 = 55_000.0;

/// Sizes, in cells, of every connected land component.
fn land_component_sizes(world: &World) -> Vec<usize> {
    let n = world.grid.cell_count();
    let mut seen = vec![false; n];
    let mut sizes = Vec::new();
    let mut stack: Vec<u32> = Vec::new();
    for start in 0..n {
        if world.ocean[start] != 0 || seen[start] {
            continue;
        }
        seen[start] = true;
        stack.push(start as u32);
        let mut size = 0usize;
        while let Some(cell) = stack.pop() {
            size += 1;
            for nb in world.grid.neighbors(cell) {
                if nb == NONE {
                    continue;
                }
                let nb = nb as usize;
                if world.ocean[nb] == 0 && !seen[nb] {
                    seen[nb] = true;
                    stack.push(nb as u32);
                }
            }
        }
        sizes.push(size);
    }
    sizes
}

/// Ocean-ocean arcs and oceanic hotspots must leave small islands above sea level.
/// Measured at n=128 on the archipelago preset: 94 (seed 1) and 82 (seed 7) small
/// islands before arc volcanoes; 132 and 133 after, with the thin-shell volcanoes.
#[test]
fn arc_volcanoes_make_small_islands() {
    for seed in [1u64, 7] {
        let settings = WorldSettings {
            seed,
            resolution: 128,
            ..Preset::Archipelago.settings()
        };
        let world = generate(&settings, &mut NoProgress).unwrap();
        let small = land_component_sizes(&world)
            .iter()
            .filter(|&&s| s < 40)
            .count();
        assert!(
            small >= 115,
            "seed {seed}: {small} land components under 40 cells, expected at least 115"
        );
    }
}

/// Point on the unit sphere with coordinate `axis` fixed at `a`, the other two `u` and `v`.
fn point_on_plane(axis: usize, a: f64, u: f64, v: f64) -> [f64; 3] {
    let mut q = [0.0f64; 3];
    let others = match axis {
        0 => [1, 2],
        1 => [0, 2],
        _ => [0, 1],
    };
    q[axis] = a;
    q[others[0]] = u;
    q[others[1]] = v;
    q
}

/// The cone field must not jump across lattice planes: each plane is a seam otherwise.
#[test]
fn volcano_field_is_continuous_across_lattice_planes() {
    let seed = derive_seed(7, SeedDomain::ArcVolcano);
    let c = SPACING_M / RADIUS_M;
    let eps = 1.0e-9;
    let mut largest = 0.0f64;
    let mut covered = 0usize;
    for k in [-30i64, -12, 5, 17, 28, 40] {
        let a = k as f64 * c;
        let t = (1.0 - a * a).sqrt();
        for i in 0..400u32 {
            let phi = (f64::from(i) + 0.5) / 400.0 * std::f64::consts::TAU;
            let (u, v) = (t * phi.cos(), t * phi.sin());
            for axis in 0..3 {
                let lo = normalize(point_on_plane(axis, a - eps, u, v));
                let hi = normalize(point_on_plane(axis, a + eps, u, v));
                let f_lo = volcano_field(seed, lo, RADIUS_M, SPACING_M, SIGMA_M);
                let f_hi = volcano_field(seed, hi, RADIUS_M, SPACING_M, SIGMA_M);
                largest = largest.max((f_hi - f_lo).abs());
                if f_hi > 0.5 {
                    covered += 1;
                }
            }
        }
    }
    assert!(
        covered > 0,
        "no samples landed on a volcano; the check is vacuous"
    );
    assert!(
        largest < 1.0e-6,
        "largest jump across a lattice plane is {largest:e}, expected below 1e-6"
    );
}

/// Area share of the sphere where a cone exceeds 0.5 must match one site per spacing²
/// at 65 % occupancy, which is about 0.21. Accept 0.10 to 0.35.
#[test]
fn volcano_cone_area_share_is_plausible() {
    let seed = derive_seed(7, SeedDomain::ArcVolcano);
    let samples = 100_000u64;
    let covered = (0..samples)
        .filter(|&i| {
            let p = random_unit(999, i);
            volcano_field(seed, p, RADIUS_M, SPACING_M, SIGMA_M) > 0.5
        })
        .count();
    let share = covered as f64 / samples as f64;
    assert!(
        (0.10..=0.35).contains(&share),
        "cone area share is {share:.3}, expected between 0.10 and 0.35"
    );
}
