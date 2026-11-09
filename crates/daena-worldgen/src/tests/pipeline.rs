use std::collections::VecDeque;

use crate::grid::NONE;
use crate::{generate, NoProgress, World, WorldSettings};

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
    assert_ne!(first.elevation_hash(), other.elevation_hash());

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
