use crate::evolution::{
    erode, fill_single_cell_pits, ErodeJob, ErodeOptions, ErosionBuffers, WaterIn,
};
use crate::grid::{CubeSphere, NONE};
use crate::noise::noise_on_sphere;
use crate::routing::{DomainIndex, RoutingScratch};
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
    let index = DomainIndex::full(h.len());
    let mut buffers = ErosionBuffers::new(h.len());
    let mut progress = NoProgress;
    let report = erode(
        ErodeJob {
            grid: &grid,
            index: &index,
            h: &mut h,
            buffers: &mut buffers,
            domain: &domain,
            uplift: &uplift,
            k_rock: &k_rock,
            scratch: &mut scratch,
            options: &options,
            water: None,
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
fn cell_and_chunk_basin_modes_agree() {
    let (grid, h0, domain) = landscape(16);
    let n = h0.len();
    let uplift = vec![2.0e-4f32; n];
    let k_rock = vec![1.0f32; n];
    let runoff_mm: Vec<f32> = (0..n)
        .map(|cell| if domain[cell] == 1 { 150.0 } else { 10.0 })
        .collect();
    let runoff_m_yr: Vec<f32> = runoff_mm.iter().map(|value| value / 1000.0).collect();
    let evap_m_yr = vec![0.0f32; n];
    let weight: Vec<f32> = runoff_mm
        .iter()
        .copied()
        .map(crate::climate::runoff_weight)
        .collect();
    let ice_m_yr: Vec<f32> = (0..n)
        .map(|cell| {
            if domain[cell] == 1 && grid.position(cell as u32)[2] > 0.4 {
                2.0e-4
            } else {
                0.0
            }
        })
        .collect();
    let water = WaterIn {
        weight: &weight,
        runoff_m_yr: &runoff_m_yr,
        evap_m_yr: &evap_m_yr,
        ice_m_yr: &ice_m_yr,
    };
    let options = ErodeOptions {
        iterations: 8,
        dt_yr: 5.0e4,
        params: ErosionParams::default(),
        record_steps: false,
        check_bounds: true,
        stage: None,
    };
    let run = |cell_mode: bool| {
        let mut h = h0.clone();
        let mut scratch = RoutingScratch::new(n);
        if !cell_mode {
            scratch.parallel_min_cells = 0;
            scratch.chunk_cells = 64;
        }
        let index = DomainIndex::full(h.len());
        let mut buffers = ErosionBuffers::new(h.len());
        let mut progress = NoProgress;
        let report = erode(
            ErodeJob {
                grid: &grid,
                index: &index,
                h: &mut h,
                buffers: &mut buffers,
                domain: &domain,
                uplift: &uplift,
                k_rock: &k_rock,
                scratch: &mut scratch,
                options: &options,
                water: Some(water),
            },
            &mut progress,
        )
        .unwrap();
        (h, report)
    };
    let (cell_h, cell_report) = run(true);
    let (chunk_h, chunk_report) = run(false);
    assert!(
        cell_report.q_ice.iter().any(|value| *value > 0.0),
        "ice path did not run"
    );
    assert_eq!(cell_h, chunk_h);
    assert_eq!(cell_report.q_ice, chunk_report.q_ice);
}

#[test]
fn active_index_matches_full_index() {
    let (grid, h0, domain) = landscape(32);
    let n = h0.len();
    let uplift = vec![2.0e-4f32; n];
    let k_rock = vec![1.0f32; n];
    let options = ErodeOptions {
        iterations: 6,
        dt_yr: 5.0e4,
        params: ErosionParams::default(),
        record_steps: false,
        check_bounds: true,
        stage: None,
    };
    let run = |active: bool| {
        let mut h = h0.clone();
        let index = if active {
            DomainIndex::active(&grid, &domain)
        } else {
            DomainIndex::full(n)
        };
        let mut scratch = RoutingScratch::with_index(&index);
        let mut buffers = ErosionBuffers::new(index.len());
        let mut progress = NoProgress;
        let report = erode(
            ErodeJob {
                grid: &grid,
                index: &index,
                h: &mut h,
                buffers: &mut buffers,
                domain: &domain,
                uplift: &uplift,
                k_rock: &k_rock,
                scratch: &mut scratch,
                options: &options,
                water: None,
            },
            &mut progress,
        )
        .unwrap();
        (h, report)
    };
    let (full_h, mut full_report) = run(false);
    let (active_h, mut active_report) = run(true);
    assert_eq!(full_h, active_h);
    let full_ice = std::mem::take(&mut full_report.q_ice);
    let active_ice = std::mem::take(&mut active_report.q_ice);
    assert_eq!(full_report, active_report);
    let active_index = DomainIndex::active(&grid, &domain);
    for (slot, &cell) in active_index.cells().iter().enumerate() {
        assert_eq!(active_ice[slot], full_ice[cell as usize], "ice at {cell}");
    }
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
    let index = DomainIndex::full(h.len());
    let mut buffers = ErosionBuffers::new(h.len());
    let mut progress = NoProgress;
    let report = erode(
        ErodeJob {
            grid: &grid,
            index: &index,
            h: &mut h,
            buffers: &mut buffers,
            domain: &domain,
            uplift: &uplift,
            k_rock: &k_rock,
            scratch: &mut scratch,
            options: &options,
            water: None,
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
    let index = DomainIndex::full(h.len());
    let mut buffers = ErosionBuffers::new(h.len());
    let mut progress = NoProgress;
    erode(
        ErodeJob {
            grid: &grid,
            index: &index,
            h: &mut h,
            buffers: &mut buffers,
            domain: &domain,
            uplift: &uplift,
            k_rock: &k_rock,
            scratch: &mut scratch,
            options: &options,
            water: None,
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

#[test]
fn sea_frame_matches_solve_and_does_not_write() {
    let grid = CubeSphere::new(8, 6_371_000.0).unwrap();
    let n = grid.cell_count();
    let mut h = vec![0.0f32; n];
    let mut domain = vec![1u8; n];
    for cell in 0..n {
        let z = grid.position(cell as u32)[2];
        h[cell] = (z * 1500.0) as f32;
        if z < 0.0 {
            domain[cell] = 0;
        }
    }
    let before = h.clone();
    let (frame, ocean) = crate::sealevel::sea_frame(&grid, &h, &domain, 0.29, &|| false).unwrap();
    assert_eq!(h, before);
    let mut shifted = before.clone();
    let solved = crate::sealevel::solve(&grid, &mut shifted, &domain, 0.29);
    assert_eq!(frame, shifted);
    assert_eq!(ocean, solved.ocean);
}
