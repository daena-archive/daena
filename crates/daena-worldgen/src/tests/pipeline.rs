use std::collections::VecDeque;

use crate::grid::NONE;
use crate::{generate, NoProgress, PlanetSettings, World, WorldSettings};

fn build(seed: u64, threads: usize) -> World {
    let settings = WorldSettings {
        resolution: 64,
        seed,
        ..WorldSettings::default()
    };
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap();
    pool.install(|| generate(&settings, &mut NoProgress).unwrap())
}

#[test]
fn n64_is_deterministic_and_physical() {
    let first = build(1, 1);
    let again = build(1, 1);
    let threaded = build(1, 4);
    let other = build(2, 1);
    assert_eq!(first.elevation_hash(), again.elevation_hash());
    assert_eq!(first.elevation_hash(), threaded.elevation_hash());
    assert_eq!(first.climate_hash(), threaded.climate_hash());
    assert_eq!(first.hydrology_hash(), threaded.hydrology_hash());
    assert_eq!(first.biome_hash(), threaded.biome_hash());
    assert_ne!(first.elevation_hash(), other.elevation_hash());
    assert_eq!(first.stats.levels[0].closed_basins, 0);
    assert_eq!(first.stats.levels[0].glaciated_cells, 0);
    let level_pass = first
        .climate_passes
        .iter()
        .find(|pass| pass.label == "level_64")
        .expect("level 64 climate pass");
    assert_eq!(level_pass.source_n, 32);
    assert!((1..=40).contains(&level_pass.spinup_years));
    let final_pass = first
        .climate_passes
        .iter()
        .find(|pass| pass.label == "final")
        .expect("final climate pass");
    assert_eq!(final_pass.source_n, 64);
    assert!((1..=40).contains(&final_pass.spinup_years));
    for (left, right) in first.climate_passes.iter().zip(&threaded.climate_passes) {
        assert_eq!(left.label, right.label);
        assert_eq!(left.source_n, right.source_n);
        assert_eq!(left.spinup_years, right.spinup_years);
        assert_eq!(left.converged, right.converged);
        assert_eq!(left.residual_k.to_bits(), right.residual_k.to_bits());
    }

    let target = first.settings.land_fraction;
    let got = first.stats.achieved_land_fraction;
    assert!((got - target).abs() <= 0.01, "land fraction {got}");

    assert!(first.elevation_m.iter().all(|v| v.is_finite()));
    assert!(first.base_m.iter().all(|v| v.is_finite()));
    assert!(first.uplift_m_yr.iter().all(|v| v.is_finite()));
    assert!(first.k_rock.iter().all(|v| v.is_finite()));
    assert!(first.drainage_area_m2.iter().all(|v| v.is_finite()));
    assert!(first.tectonics.base_m.iter().all(|v| v.is_finite()));
    assert!(first.tectonics.uplift_m_yr.iter().all(|v| v.is_finite()));
    assert!(first.tectonics.k_rock.iter().all(|v| v.is_finite()));
    assert!(first.sea_level_shift_m.is_finite());

    for (elev, ocean) in first.elevation_m.iter().zip(&first.ocean) {
        if *ocean == 1 {
            assert!(*elev <= 0.0, "ocean cell at {elev}");
        }
    }
    assert!(plates_are_connected(&first));
    assert!(first.elevation_m.iter().any(|h| *h > 1500.0));
    assert!(first.elevation_m.iter().any(|h| *h < -4000.0));
}

fn plates_are_connected(world: &World) -> bool {
    let n = world.plate.len();
    let mut seen = vec![0u8; n];
    let mut started = [NONE; 256];
    for cell in 0..n {
        if seen[cell] == 1 {
            continue;
        }
        let id = world.plate[cell] as usize;
        if started[id] != NONE {
            return false;
        }
        started[id] = cell as u32;
        let mut queue = VecDeque::new();
        queue.push_back(cell as u32);
        seen[cell] = 1;
        while let Some(current) = queue.pop_front() {
            for nb in world.grid.neighbors(current) {
                if nb == NONE {
                    continue;
                }
                let j = nb as usize;
                if seen[j] == 0 && world.plate[j] == world.plate[cell] {
                    seen[j] = 1;
                    queue.push_back(nb);
                }
            }
        }
    }
    true
}

fn world_climate(seed: u64, n: u32, offset: f64, precip: f64) -> World {
    generate(
        &WorldSettings {
            resolution: n,
            seed,
            planet: PlanetSettings {
                temperature_offset_c: offset,
                precipitation_scale: precip,
                ..PlanetSettings::default()
            },
            ..WorldSettings::default()
        },
        &mut NoProgress,
    )
    .unwrap()
}

#[test]
fn arid_world_closes_more_basins_than_humid() {
    let arid = world_climate(7, 64, 0.0, 0.2);
    let humid = world_climate(7, 64, 0.0, 2.0);
    let arid_n = arid.stats.levels.last().unwrap().closed_basins;
    let humid_n = humid.stats.levels.last().unwrap().closed_basins;
    assert!(
        arid_n > humid_n,
        "arid closed {arid_n} humid closed {humid_n}"
    );
}

/// Domain cells below −100 m before the sea-level shift. Cold ice cuts fjords
/// and overdeepens beds, so this count is the robust form of the fjord check.
#[test]
fn cold_world_puts_more_domain_below_minus_100() {
    let base = world_climate(7, 128, 0.0, 1.0);
    let cold = world_climate(7, 128, -15.0, 1.0);
    let base_n = deep_domain(&base);
    let cold_n = deep_domain(&cold);
    assert!(
        cold_n > base_n,
        "cold deep domain {cold_n} default {base_n}"
    );
}

fn deep_domain(world: &World) -> u32 {
    let shift = world.sea_level_shift_m;
    let mut count = 0u32;
    for cell in 0..world.elevation_m.len() {
        if world.base_m[cell] <= 0.0 {
            continue;
        }
        let pre = world.elevation_m[cell] - shift;
        if pre < -100.0 {
            count = count.saturating_add(1);
        }
    }
    count
}
