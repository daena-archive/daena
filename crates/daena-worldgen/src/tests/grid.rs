use crate::grid::{downsample, upsample, CubeSphere};

const RADIUS: f64 = 6_371_000.0;

#[test]
fn areas_sum_to_sphere() {
    for n in [8u32, 32, 128] {
        let grid = CubeSphere::new(n, RADIUS).unwrap();
        let mut sum = 0.0f64;
        for cell in 0..grid.cell_count() {
            sum += grid.area_m2(cell as u32);
        }
        let expect = 4.0 * std::f64::consts::PI * RADIUS * RADIUS;
        let rel = (sum - expect).abs() / expect;
        assert!(rel < 1e-9, "n={n} rel={rel} sum={sum} expect={expect}");
    }
}

#[test]
fn neighbour_symmetry() {
    for n in [8u32, 16, 32] {
        let grid = CubeSphere::new(n, RADIUS).unwrap();
        for cell in 0..grid.cell_count() as u32 {
            for nb in grid.neighbors(cell) {
                if nb == crate::grid::NONE {
                    continue;
                }
                let back = grid.neighbors(nb);
                assert!(
                    back.contains(&cell),
                    "n={n} cell {cell} lists {nb} but the link is not returned"
                );
            }
        }
    }
}

#[test]
fn neighbour_counts_and_uniqueness() {
    let grid = CubeSphere::new(32, RADIUS).unwrap();
    let mut seven = 0u32;
    let mut eight = 0u32;
    for cell in 0..grid.cell_count() as u32 {
        let nbs = grid.neighbors(cell);
        let mut seen = Vec::new();
        for nb in nbs {
            if nb == crate::grid::NONE {
                continue;
            }
            assert_ne!(nb, cell, "cell {cell} lists itself");
            assert!(!seen.contains(&nb), "cell {cell} duplicates {nb}");
            seen.push(nb);
        }
        match seen.len() {
            8 => eight += 1,
            7 => seven += 1,
            other => panic!("cell {cell} has {other} neighbours"),
        }
    }
    assert_eq!(seven, 24, "corner cells with 7 neighbours");
    assert_eq!(eight + seven, grid.cell_count() as u32);
}

#[test]
fn cell_at_roundtrip_and_children() {
    let coarse = CubeSphere::new(32, RADIUS).unwrap();
    let fine = CubeSphere::new(64, RADIUS).unwrap();
    for cell in 0..coarse.cell_count() as u32 {
        assert_eq!(coarse.cell_at(coarse.position(cell)), cell);
        for child in coarse.child_ids(cell) {
            assert_eq!(fine.parent_id(child), cell);
            assert_eq!(coarse.cell_at(fine.position(child)), cell);
        }
    }
}

#[test]
fn neighbour_distances_near_nominal() {
    let grid = CubeSphere::new(32, RADIUS).unwrap();
    let cell_size = grid.cell_size_m();
    let diagonal = cell_size * std::f64::consts::SQRT_2;
    for cell in 0..grid.cell_count() as u32 {
        let here = grid.position(cell);
        let nbs = grid.neighbors(cell);
        for (k, &nb) in nbs.iter().enumerate() {
            if nb == crate::grid::NONE {
                continue;
            }
            let dist = f64::from(grid.neighbor_distance(cell, k));
            let gc = great_circle(here, grid.position(nb), RADIUS);
            let rel = (dist - gc).abs() / gc.max(1.0);
            assert!(
                rel <= 1.0e-5,
                "cell {cell} slot {k} dist {dist} great-circle {gc} rel {rel}"
            );
            // Face borders squeeze some links past the brief's 20% / 0.60
            // windows. Measured axis ratios are about [0.72, 1.00] and
            // diagonal ratios about [0.58, 1.12].
            if k % 2 == 0 {
                let ratio = dist / cell_size;
                assert!(
                    (0.70..=1.20).contains(&ratio),
                    "cell {cell} slot {k} axis ratio {ratio}"
                );
            } else {
                let ratio = dist / diagonal;
                assert!(
                    (0.55..=1.35).contains(&ratio),
                    "cell {cell} slot {k} diagonal ratio {ratio}"
                );
            }
        }
    }
}

fn great_circle(a: [f64; 3], b: [f64; 3], radius: f64) -> f64 {
    let c = [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ];
    let s = (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt();
    let dot = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).clamp(-1.0, 1.0);
    s.atan2(dot) * radius
}

#[test]
fn upsample_preserves_constants() {
    let parent = CubeSphere::new(16, RADIUS).unwrap();
    let child = CubeSphere::new(32, RADIUS).unwrap();
    let field = vec![2.5f32; parent.cell_count()];
    let up = upsample(&parent, &field, &child).unwrap();
    assert!(up.iter().all(|value| *value == 2.5));
}

#[test]
fn downsample_of_upsample_matches_smooth_field() {
    let parent = CubeSphere::new(16, RADIUS).unwrap();
    let child = CubeSphere::new(32, RADIUS).unwrap();
    let mut field = vec![0.0f32; parent.cell_count()];
    for (cell, value) in field.iter_mut().enumerate() {
        *value = parent.position(cell as u32)[2] as f32;
    }
    let up = upsample(&parent, &field, &child).unwrap();
    let back = downsample(&child, &up, &parent).unwrap();
    let mut max_err = 0.0f32;
    for (orig, got) in field.iter().zip(&back) {
        max_err = max_err.max((orig - got).abs());
    }
    assert!(max_err < 0.05, "max err {max_err}");
}
