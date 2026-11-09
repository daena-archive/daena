use crate::evolution::{erode, fill_single_cell_pits, ErodeJob, ErodeOptions};
use crate::grid::{CubeSphere, NONE};
use crate::noise::noise_on_sphere;
use crate::routing::RoutingScratch;
use crate::{ErosionParams, NoProgress};

fn landscape(n: u32) -> (CubeSphere, Vec<f32>, Vec<u8>) {
    let grid = CubeSphere::new(n, 6_371_000.0).unwrap();
    let mut h = vec![0.0f32; grid.cell_count()];
    let mut domain = vec![1u8; grid.cell_count()];
    for cell in 0..grid.cell_count() {
        let p = grid.position(cell as u32);
        if p[2] < -0.2 {
            domain[cell] = 0;
            h[cell] = -300.0;
        } else {
            let noise = noise_on_sphere(42, p, 3.5);
            h[cell] = (500.0 + 2200.0 * p[2] + 180.0 * noise).max(1.0) as f32;
        }
    }
    (grid, h, domain)
}

#[test]
fn single_cell_pits_rise_to_the_lowest_neighbour() {
    let grid = CubeSphere::new(8, 6_371_000.0).unwrap();
    let n = grid.cell_count();
    let mut h = vec![100.0f32; n];
    let mut domain = vec![1u8; n];
    let pit = grid.id(0, 3, 3) as usize;
    h[pit] = 1.0;
    let mut min_nb = f32::MAX;
    for nb in grid.neighbors(pit as u32) {
        if nb != NONE {
            min_nb = min_nb.min(h[nb as usize]);
        }
    }
    let flat = grid.neighbors(pit as u32)[0];
    assert_ne!(flat, NONE);
    h[flat as usize] = 1.0;
    let ocean = grid.id(0, 3, 5) as usize;
    domain[ocean] = 0;
    h[ocean] = 0.0;
    fill_single_cell_pits(&grid, &mut h, &domain);
    assert!(
        (h[pit] - 1.0).abs() < 1.0e-6,
        "tied neighbour is not a strict pit"
    );
    assert!((h[flat as usize] - 1.0).abs() < 1.0e-6);
    assert!((h[ocean] - 0.0).abs() < 1.0e-6);
    h[flat as usize] = 100.0;
    fill_single_cell_pits(&grid, &mut h, &domain);
    assert!(
        (h[pit] - (min_nb + 0.01)).abs() < 1.0e-5,
        "pit {} expected {}",
        h[pit],
        min_nb + 0.01
    );
}

#[test]
fn zero_uplift_does_not_raise_or_undercut() {
    let (grid, mut h, domain) = landscape(16);
    let before = h.clone();
    let uplift = vec![0.0f32; h.len()];
    let k_rock = vec![1.0f32; h.len()];
    let mut scratch = RoutingScratch::new(h.len());
    let params = ErosionParams {
        kappa: 1.0e-6,
        s_c: 5.0,
        ..ErosionParams::default()
    };
    let options = ErodeOptions {
        iterations: 1,
        dt_yr: 1.0e5,
        params,
        record_steps: false,
        check_bounds: true,
        stage: None,
    };
    let mut progress = NoProgress;
    let report = erode(
        ErodeJob {
            grid: &grid,
            h: &mut h,
            domain: &domain,
            uplift: &uplift,
            k_rock: &k_rock,
            scratch: &mut scratch,
            options: &options,
        },
        &mut progress,
    )
    .unwrap();
    assert_eq!(report.violations, 0);
    let mut raised = 0.0f32;
    for (old, new) in before.iter().zip(&h) {
        raised = raised.max(new - old);
    }
    assert!(raised <= 0.05, "max rise {raised}");
}

#[test]
fn uniform_uplift_step_shrinks() {
    let (grid, mut h, domain) = landscape(16);
    let uplift = vec![2.0e-4f32; h.len()];
    let k_rock = vec![1.0f32; h.len()];
    let mut scratch = RoutingScratch::new(h.len());
    let options = ErodeOptions {
        iterations: 12,
        dt_yr: 5.0e5,
        params: ErosionParams::default(),
        record_steps: true,
        check_bounds: false,
        stage: None,
    };
    let mut progress = NoProgress;
    let report = erode(
        ErodeJob {
            grid: &grid,
            h: &mut h,
            domain: &domain,
            uplift: &uplift,
            k_rock: &k_rock,
            scratch: &mut scratch,
            options: &options,
        },
        &mut progress,
    )
    .unwrap();
    let steps = &report.steps;
    assert_eq!(steps.len(), 12);
    let first = steps[0];
    let last = *steps.last().unwrap();
    let early = steps[..4].iter().sum::<f32>() / 4.0;
    let late = steps[8..].iter().sum::<f32>() / 4.0;
    assert!(first > 0.0, "first step {first}");
    assert!(last < first, "first {first} last {last} steps {steps:?}");
    assert!(late < early, "early {early} late {late} steps {steps:?}");
}

#[test]
fn rift_subsidence_stops_at_floor() {
    let (grid, mut h, domain) = landscape(16);
    let uplift = vec![-1.0e-3f32; h.len()];
    let k_rock = vec![1.0f32; h.len()];
    let mut scratch = RoutingScratch::new(h.len());
    let options = ErodeOptions {
        iterations: 30,
        dt_yr: 1.0e6,
        params: ErosionParams::default(),
        record_steps: false,
        check_bounds: false,
        stage: None,
    };
    let mut progress = NoProgress;
    erode(
        ErodeJob {
            grid: &grid,
            h: &mut h,
            domain: &domain,
            uplift: &uplift,
            k_rock: &k_rock,
            scratch: &mut scratch,
            options: &options,
        },
        &mut progress,
    )
    .unwrap();
    for (cell, value) in h.iter().enumerate() {
        assert!(
            *value >= -1000.0,
            "cell {cell} subsided to {value}, domain {}",
            domain[cell]
        );
    }
}
