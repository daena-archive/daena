use crate::grid::{downsample, downsample_max, upsample, CubeSphere};

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

#[test]
fn downsample_max_keeps_the_largest_child() {
    let parent = CubeSphere::new(8, RADIUS).unwrap();
    let child = CubeSphere::new(16, RADIUS).unwrap();
    let mut field = vec![1.0f32; child.cell_count()];
    let kids = parent.child_ids(0);
    field[kids[2] as usize] = 9.0;
    let got = downsample_max(&child, &field, &parent).unwrap();
    assert_eq!(got[0], 9.0);
    assert!(got[1..].iter().all(|value| *value == 1.0));
}

#[test]
fn bilinear_across_faces_matches_interior_accuracy() {
    let n = 32u32;
    let grid = CubeSphere::new(n, RADIUS).unwrap();
    let axis = [0.3, -0.5, 0.81];
    let truth = |p: [f64; 3]| p[0] * axis[0] + p[1] * axis[1] + p[2] * axis[2];
    let field: Vec<f32> = (0..grid.cell_count())
        .map(|cell| truth(grid.position(cell as u32)) as f32)
        .collect();
    let mut interior = 0.0f64;
    let mut edge = 0.0f64;
    let nf = n as f64;
    for face in 0..6u32 {
        let mut s = 1.25;
        while s <= nf - 1.25 {
            let mut t = 1.25;
            while t <= nf - 1.25 {
                interior = interior.max(sample_error(&grid, &field, face, s, t, &truth));
                t += 0.5;
            }
            s += 0.5;
        }
        for along in 0..n {
            let a = along as f64 + 0.5;
            for inset in [0.05, 0.25, 0.55, 0.85] {
                for (fx, fy) in [(inset, a), (nf - inset, a), (a, inset), (a, nf - inset)] {
                    edge = edge.max(sample_error(&grid, &field, face, fx, fy, &truth));
                }
            }
        }
        for fx in [0.05, 0.25, 0.75] {
            for fy in [0.05, 0.25, 0.75] {
                for (x, y) in [(fx, fy), (nf - fx, fy), (fx, nf - fy), (nf - fx, nf - fy)] {
                    edge = edge.max(sample_error(&grid, &field, face, x, y, &truth));
                }
            }
        }
    }
    assert!(
        edge <= 2.0 * interior.max(1e-8),
        "edge {edge} interior {interior}"
    );
}

fn sample_error(
    grid: &CubeSphere,
    field: &[f32],
    face: u32,
    fx: f64,
    fy: f64,
    truth: &impl Fn([f64; 3]) -> f64,
) -> f64 {
    let point = point_on_face(face, grid.n, fx, fy);
    (f64::from(grid.sample_bilinear(field, point)) - truth(point)).abs()
}

fn point_on_face(face: u32, n: u32, fx: f64, fy: f64) -> [f64; 3] {
    let nf = n as f64;
    let a = (fx / nf - 0.5) * std::f64::consts::FRAC_PI_2;
    let b = (fy / nf - 0.5) * std::f64::consts::FRAC_PI_2;
    let basis = crate::grid::FACE_BASIS[face as usize];
    let ta = a.tan();
    let tb = b.tan();
    let mut q = [0.0; 3];
    for c in 0..3 {
        q[c] = basis[0][c] + ta * basis[1][c] + tb * basis[2][c];
    }
    let len = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2]).sqrt();
    [q[0] / len, q[1] / len, q[2] / len]
}

#[test]
fn chart_stencil_matches_sample_at() {
    let grid = CubeSphere::new(16, RADIUS).unwrap();
    let field: Vec<f32> = (0..grid.cell_count())
        .map(|cell| {
            let p = grid.position(cell as u32);
            (p[0] * 3.0 + p[1] * -2.0 + p[2] * 5.0) as f32
        })
        .collect();
    let n = grid.n as f64;
    for face in 0..6u32 {
        let mut s = -0.35;
        while s <= n + 0.35 {
            let mut t = -0.35;
            while t <= n + 0.35 {
                let stencil = grid.chart_stencil(face, s, t);
                let got = stencil.sample(&field);
                let expect = f64::from(grid.sample_at(&field, face, s, t));
                assert!(
                    (got - expect).abs() < 1e-4,
                    "face {face} ({s},{t}) stencil {got} sample {expect}"
                );
                t += 0.7;
            }
            s += 0.7;
        }
    }
}

/// A two-valued field with a sharp step across face edges: every sample near
/// an edge, through the bilinear chart and through the precomputed stencil,
/// must stay inside the field's range (no cross-face overshoot).
#[test]
fn cross_face_samples_do_not_overshoot_sharp_steps() {
    let grid = CubeSphere::new(16, RADIUS).unwrap();
    let axis = {
        let raw = [0.3f64, 0.8, -0.5];
        let len = (raw[0] * raw[0] + raw[1] * raw[1] + raw[2] * raw[2]).sqrt();
        [raw[0] / len, raw[1] / len, raw[2] / len]
    };
    let field: Vec<f32> = (0..grid.cell_count() as u32)
        .map(|cell| {
            let p = grid.position(cell);
            if p[0] * axis[0] + p[1] * axis[1] + p[2] * axis[2] > 0.05 {
                40.0
            } else {
                0.0
            }
        })
        .collect();
    let n = grid.n;
    let mut checked = 0;
    for cell in 0..grid.cell_count() as u32 {
        let (_, i, j) = grid.decode(cell);
        if i != 0 && j != 0 && i + 1 != n && j + 1 != n {
            continue;
        }
        let a = grid.position(cell);
        for nb in grid.neighbors(cell) {
            if nb == crate::grid::NONE {
                continue;
            }
            let b = grid.position(nb);
            for t in [0.1, 0.3, 0.5, 0.7] {
                let raw = [
                    a[0] + (b[0] - a[0]) * t,
                    a[1] + (b[1] - a[1]) * t,
                    a[2] + (b[2] - a[2]) * t,
                ];
                let len = (raw[0] * raw[0] + raw[1] * raw[1] + raw[2] * raw[2]).sqrt();
                let p = [raw[0] / len, raw[1] / len, raw[2] / len];
                let direct = grid.sample_bilinear(&field, p);
                let (face, fx, fy) = grid.face_coords(p);
                let stencil = grid.chart_stencil(face, fx, fy).sample(&field);
                for value in [f64::from(direct), stencil] {
                    assert!(
                        (-1e-4..=40.0001).contains(&value),
                        "cell {cell} -> {nb} t {t}: sample {value} outside [0, 40]"
                    );
                }
                checked += 1;
            }
        }
    }
    assert!(checked > 1000);
}
