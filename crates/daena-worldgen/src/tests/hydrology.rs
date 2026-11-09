use crate::grid::{CubeSphere, NONE};
use crate::hydrology::{self, LakeKind, RiverEnd};
use crate::{generate, ClimateStats, ClimateSummary, NoProgress, PlanetSettings, WorldSettings};

const RADIUS: f64 = 6_371_000.0;
const YEAR: f64 = 3.15576e7;

fn empty_stats() -> ClimateStats {
    ClimateStats {
        t_global_c: 0.0,
        t_equator_c: 0.0,
        t_polar_c: 0.0,
        p_global_mm: 0.0,
        p_equator_mm: 0.0,
        p_subtropical_land_mm: 0.0,
        spinup_years: 0,
        spinup_converged: true,
        spinup_residual_k: 0.0,
        phi_h_deg: 30.0,
    }
}

fn climate_runoff(runoff: &[f32], precip: f32, pet: f32) -> ClimateSummary {
    let n = runoff.len();
    ClimateSummary {
        t_annual_c: vec![15.0; n],
        t_warm_c: vec![20.0; n],
        t_cold_c: vec![5.0; n],
        p_annual_mm: vec![precip; n],
        p_dry_month_mm: vec![(precip / 12.0).max(0.0); n],
        pet_annual_mm: vec![pet; n],
        runoff_mm: runoff.to_vec(),
        ice_balance_mm: vec![0.0; n],
        sea_ice_months: vec![0; n],
        stats: empty_stats(),
    }
}

fn climate(n: usize, runoff: f32, precip: f32, pet: f32) -> ClimateSummary {
    ClimateSummary {
        t_annual_c: vec![15.0; n],
        t_warm_c: vec![20.0; n],
        t_cold_c: vec![5.0; n],
        p_annual_mm: vec![precip; n],
        p_dry_month_mm: vec![(precip / 12.0).max(0.0); n],
        pet_annual_mm: vec![pet; n],
        runoff_mm: vec![runoff; n],
        ice_balance_mm: vec![0.0; n],
        sea_ice_months: vec![0; n],
        stats: empty_stats(),
    }
}

/// Ocean at (0,0), a rising channel, and one pit behind a sill at 300 m.
fn bowl() -> (CubeSphere, Vec<f32>, Vec<u8>, u32) {
    let grid = CubeSphere::new(16, RADIUS).unwrap();
    let n = grid.cell_count();
    let mut h = vec![800.0f32; n];
    let mut ocean = vec![0u8; n];
    let id = |i, j| grid.id(0, i, j);
    let mouth = id(0, 0);
    ocean[mouth as usize] = 1;
    h[mouth as usize] = 0.0;
    h[id(1, 0) as usize] = 100.0;
    h[id(2, 0) as usize] = 200.0;
    let spill = id(3, 0);
    h[spill as usize] = 300.0;
    h[id(4, 0) as usize] = 40.0;
    h[id(4, 1) as usize] = 70.0;
    h[id(5, 0) as usize] = 110.0;
    h[id(5, 1) as usize] = 150.0;
    (grid, h, ocean, spill)
}

fn balance(
    grid: &CubeSphere,
    ocean: &[u8],
    runoff: &[f32],
    hydro: &crate::Hydrology,
) -> (f64, f64, f64) {
    let mut land = 0.0;
    let mut into_ocean = 0.0;
    for cell in 0..grid.cell_count() {
        let q = hydro.discharge_m3s[cell] * YEAR;
        if ocean[cell] == 1 {
            into_ocean += q;
        } else {
            land += f64::from(runoff[cell]).max(0.0) / 1000.0 * grid.area_m2(cell as u32);
        }
    }
    let evap: f64 = hydro.lakes.iter().map(|lake| lake.evaporated_m3).sum();
    (land, into_ocean, evap)
}

#[test]
fn flood_orders_receivers_and_reaches_the_ocean() {
    let (grid, h, ocean, _) = bowl();
    let field = climate(grid.cell_count(), 0.0, 0.0, 0.0);
    let hydro = hydrology::build(&grid, &h, &ocean, &field, 100.0).unwrap();
    let n = grid.cell_count();
    let mut rank = vec![0u32; n];
    for (i, &cell) in hydro.pop_order.iter().enumerate() {
        rank[cell as usize] = i as u32;
    }
    for cell in 0..n {
        if ocean[cell] == 1 {
            continue;
        }
        assert!(hydro.filled_m[cell] + 1e-3 >= h[cell]);
        let mut cursor = cell as u32;
        let mut guard = 0;
        while ocean[cursor as usize] == 0 {
            let next = hydro.receiver[cursor as usize];
            assert_ne!(next, NONE);
            assert!(rank[next as usize] < rank[cursor as usize]);
            cursor = next;
            guard += 1;
            assert!(guard < n, "cycle");
        }
    }
}

#[test]
fn closed_basin_fills_drains_or_turns_saline() {
    let (grid, h, ocean, spill) = bowl();
    let n = grid.cell_count();
    let spill_h = f64::from(h[spill as usize]);

    let pit = [
        grid.id(0, 4, 0),
        grid.id(0, 4, 1),
        grid.id(0, 5, 0),
        grid.id(0, 5, 1),
    ];
    let dry_mm = vec![0.0f32; n];
    let dry = climate_runoff(&dry_mm, 0.0, 5000.0);
    let none = hydrology::build(&grid, &h, &ocean, &dry, 100.0).unwrap();
    assert!(none.lakes.is_empty(), "zero runoff still made a lake");

    let mut wet_mm = vec![0.0f32; n];
    for cell in pit {
        wet_mm[cell as usize] = 5000.0;
    }
    let wet = climate_runoff(&wet_mm, 4000.0, 5000.0);
    let full = hydrology::build(&grid, &h, &ocean, &wet, 100.0).unwrap();
    assert_eq!(full.lakes.len(), 1);
    let lake = &full.lakes[0];
    assert_eq!(lake.kind, LakeKind::Fresh);
    assert!(
        (lake.level_m - spill_h).abs() < 1e-6,
        "level {}",
        lake.level_m
    );
    let evap = lake.area_m2 * 1.0;
    assert!((lake.evaporated_m3 - evap).abs() / lake.inflow_m3 < 1e-9);
    assert!((lake.outflow_m3 - (lake.inflow_m3 - lake.evaporated_m3)).abs() < 1e-6);

    let mut low_mm = vec![0.0f32; n];
    for cell in pit {
        low_mm[cell as usize] = 500.0;
    }
    let low = climate_runoff(&low_mm, 0.0, 1000.0);
    let salt = hydrology::build(&grid, &h, &ocean, &low, 100.0).unwrap();
    assert_eq!(salt.lakes.len(), 1);
    let lake = &salt.lakes[0];
    assert_eq!(lake.kind, LakeKind::Saline);
    assert!(
        lake.level_m > 40.0 && lake.level_m < spill_h,
        "level {}",
        lake.level_m
    );
    let target = lake.inflow_m3 / 1.0;
    let slack = (0..n)
        .filter(|cell| salt.lake_id[*cell] == lake.id)
        .map(|cell| grid.area_m2(cell as u32))
        .fold(0.0, f64::max);
    assert!(
        (lake.area_m2 - target).abs() <= slack + 1.0,
        "area {} target {target} slack {slack}",
        lake.area_m2
    );
    assert_eq!(lake.outflow_m3, 0.0);

    for (label, field, hydro) in [("wet", &wet, &full), ("salt", &low, &salt)] {
        let (land, ocean_in, evap) = balance(&grid, &ocean, &field.runoff_mm, hydro);
        let err = (land - ocean_in - evap).abs() / land.max(1.0);
        assert!(
            err < 1e-9,
            "{label} balance {err} land {land} ocean {ocean_in} evap {evap}"
        );
        assert_discharge_monotone(hydro);
        assert_rivers(&grid, &ocean, hydro);
    }
}

fn assert_discharge_monotone(hydro: &crate::Hydrology) {
    for cell in 0..hydro.receiver.len() {
        if hydro.lake_exit[cell] == 1 {
            continue;
        }
        let dest = hydro.receiver[cell] as usize;
        if dest == cell || dest >= hydro.receiver.len() {
            continue;
        }
        assert!(
            hydro.discharge_m3s[cell] <= hydro.discharge_m3s[dest] + 1e-6,
            "cell {cell} {} downstream {}",
            hydro.discharge_m3s[cell],
            hydro.discharge_m3s[dest]
        );
    }
}

fn assert_rivers(grid: &CubeSphere, ocean: &[u8], hydro: &crate::Hydrology) {
    let n = grid.cell_count();
    let mut river = vec![false; n];
    for cell in 0..n {
        river[cell] =
            ocean[cell] == 0 && hydro.lake_id[cell] == NONE && hydro.discharge_m3s[cell] >= 100.0;
    }
    for segment in &hydro.rivers {
        assert!(!segment.cells.is_empty());
        for pair in segment.cells.windows(2) {
            assert_eq!(hydro.receiver[pair[0] as usize], pair[1]);
        }
        let last = *segment.cells.last().unwrap() as usize;
        match segment.end {
            RiverEnd::Ocean => assert_eq!(ocean[last], 1),
            RiverEnd::Lake => assert_ne!(hydro.lake_id[last], NONE),
            RiverEnd::Confluence => assert!(river[last] || hydro.lake_id[last] != NONE),
        }
        for cell in &segment.cells[..segment.cells.len() - 1] {
            assert!(river[*cell as usize], "non-river in segment {}", cell);
        }
    }
}

#[test]
fn no_ocean_has_one_root() {
    let grid = CubeSphere::new(16, RADIUS).unwrap();
    let n = grid.cell_count();
    let mut h = vec![50.0f32; n];
    h[0] = 10.0;
    let ocean = vec![0u8; n];
    let field = climate(n, 10.0, 10.0, 0.0);
    let hydro = hydrology::build(&grid, &h, &ocean, &field, 1.0e12).unwrap();
    let roots: Vec<u32> = (0..n)
        .filter(|&cell| hydro.receiver[cell] == cell as u32)
        .map(|cell| cell as u32)
        .collect();
    assert_eq!(roots, vec![0]);
    for cell in 0..n {
        let mut cursor = cell as u32;
        let mut guard = 0;
        while hydro.receiver[cursor as usize] != cursor {
            cursor = hydro.receiver[cursor as usize];
            guard += 1;
            assert!(guard < n);
        }
        assert_eq!(cursor, 0);
    }
}

#[test]
fn cold_offset_holds_more_ice_and_threads_match() {
    let base = world(0.0, 1.0, 1);
    let cold = world(-15.0, 1.0, 1);
    let four = world(0.0, 1.0, 4);
    assert_eq!(base.hydrology_hash(), four.hydrology_hash());
    assert_eq!(base.biome_hash(), four.biome_hash());
    assert_eq!(base.elevation_hash(), four.elevation_hash());
    let base_ice = ice_area(&base);
    let cold_ice = ice_area(&cold);
    assert!(cold_ice > base_ice, "cold {cold_ice} base {base_ice}");
}

fn ice_area(world: &crate::World) -> f64 {
    let mut area = 0.0;
    for cell in 0..world.grid.cell_count() {
        if world.hydrology.ice_cover[cell] > 0 {
            area += world.grid.area_m2(cell as u32);
        }
    }
    area
}

fn world(offset: f64, precip: f64, threads: usize) -> crate::World {
    let settings = WorldSettings {
        resolution: 64,
        seed: 1,
        planet: PlanetSettings {
            temperature_offset_c: offset,
            precipitation_scale: precip,
            ..PlanetSettings::default()
        },
        ..WorldSettings::default()
    };
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap();
    pool.install(|| generate(&settings, &mut NoProgress).unwrap())
}
