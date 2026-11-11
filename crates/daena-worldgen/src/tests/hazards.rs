//! Hazard field tests.

use std::collections::VecDeque;
use std::sync::OnceLock;

use crate::grid::NONE;
use crate::hazards::{flood_potential, quake_belt, VolcanicActivity, VolcanicOrigin};
use crate::{
    generate, BoundaryKind, BoundaryRecord, NoProgress, Preset, SubductSide, World, WorldSettings,
};

fn build(preset: Preset, seed: u64, n: u32) -> World {
    generate(
        &WorldSettings {
            seed,
            resolution: n,
            ..preset.settings()
        },
        &mut NoProgress,
    )
    .unwrap()
}

fn build_threads(preset: Preset, seed: u64, n: u32, threads: usize) -> World {
    let settings = WorldSettings {
        seed,
        resolution: n,
        ..preset.settings()
    };
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap();
    pool.install(|| generate(&settings, &mut NoProgress).unwrap())
}

static EARTH_256: OnceLock<World> = OnceLock::new();
static EARTH_512: OnceLock<World> = OnceLock::new();

fn earth_256() -> &'static World {
    EARTH_256.get_or_init(|| build(Preset::Earthlike, 7, 256))
}

fn earth_512() -> &'static World {
    EARTH_512.get_or_init(|| build(Preset::Earthlike, 7, 512))
}

fn boundary(kind: BoundaryKind, speed_m_yr: f32) -> BoundaryRecord {
    BoundaryRecord {
        cell: 0,
        neighbor: 1,
        plate_a: 0,
        plate_b: 1,
        crust_a: 0,
        crust_b: 1,
        kind,
        convergence_m_yr: speed_m_yr,
        shear_m_yr: 0.0,
        convergence_raw_m_yr: speed_m_yr,
        shear_raw_m_yr: 0.0,
        subducts: SubductSide::None,
    }
}

#[test]
fn earthquake_strength_orders_by_kind_speed_and_distance() {
    let convergent = boundary(BoundaryKind::Convergent, 0.05);
    let transform = boundary(BoundaryKind::Transform, 0.05);
    let divergent = boundary(BoundaryKind::Divergent, 0.05);
    let near = quake_belt(&convergent, 50.0);
    let middle = quake_belt(&transform, 50.0);
    let far = quake_belt(&divergent, 50.0);
    assert!(
        near > middle && middle > far,
        "kind ordering {near} {middle} {far}"
    );

    let fast = boundary(BoundaryKind::Convergent, 0.10);
    let slow = boundary(BoundaryKind::Convergent, 0.02);
    assert!(quake_belt(&fast, 50.0) > quake_belt(&slow, 50.0));
    assert!(quake_belt(&convergent, 0.0) > quake_belt(&convergent, 100.0));
}

#[test]
fn volcano_ids_are_unique_and_stable_across_resolution() {
    let small = earth_256();
    let large = earth_512();
    assert_eq!(small.tectonics.n, 256);
    assert_eq!(large.tectonics.n, 256);
    let key = |world: &World| -> Vec<(u64, VolcanicOrigin, VolcanicActivity, u32)> {
        world
            .hazards
            .volcanic_sources
            .iter()
            .map(|source| {
                (
                    source.id,
                    source.origin,
                    source.activity,
                    source.eruptions_per_yr.to_bits(),
                )
            })
            .collect()
    };
    assert_eq!(key(small), key(large));
    let mut ids: Vec<u64> = small
        .hazards
        .volcanic_sources
        .iter()
        .map(|source| source.id)
        .collect();
    let before = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), before, "volcanic source ids are not unique");

    let seed = crate::derive_seed(small.settings.seed, crate::SeedDomain::ArcVolcano);
    for source in &small.hazards.volcanic_sources {
        if source.origin != VolcanicOrigin::SubductionArc {
            continue;
        }
        let field = crate::tectonics::volcano_field(
            seed,
            source.pos,
            small.grid.radius_m,
            140_000.0,
            55_000.0,
        );
        assert!(field >= 0.999, "arc source off the volcano field: {field}");
    }
}

#[test]
fn storms_follow_climate_and_recurve() {
    let frozen = build(Preset::Frozen, 7, 128);
    let earth = earth_512();
    let hot = build(Preset::Hothouse, 7, 128);
    let count = |world: &World| world.hazards.stats.storms_per_yr;
    assert!(
        count(&frozen) < count(earth),
        "frozen {} earth {}",
        count(&frozen),
        count(earth)
    );
    assert!(
        count(earth) < count(&hot),
        "earth {} hot {}",
        count(earth),
        count(&hot)
    );
    assert!(count(&frozen) < 1.0, "frozen storms {}", count(&frozen));

    let grid = &earth.climate_field.grid;
    let n = grid.cell_count();
    let mut genesis_lat = 0.0;
    let mut genesis_weight = 0.0;
    let mut track_lat = 0.0;
    let mut track_weight = 0.0;
    let storms = &earth.hazards.storms;
    for (cell, ((genesis, track), intensity)) in storms
        .genesis_km2
        .iter()
        .zip(&storms.track_km2)
        .zip(&storms.intensity)
        .enumerate()
    {
        let area = grid.area_m2(cell as u32) / 1.0e6;
        let (lat, _) = grid.lat_lon(cell as u32);
        let lat = lat.to_degrees().abs();
        let genesis_value = *genesis;
        let genesis = f64::from(genesis_value) * area;
        let track = f64::from(*track) * area;
        genesis_lat += lat * genesis;
        genesis_weight += genesis;
        track_lat += lat * track;
        track_weight += track;
        if lat < 4.0 {
            assert_eq!(
                genesis_value, 0.0,
                "genesis inside 4 degrees at cell {cell}"
            );
        }
        assert!((0.0..=1.0).contains(intensity), "intensity {intensity}");
    }
    let genesis_mean = genesis_lat / genesis_weight.max(f64::MIN_POSITIVE);
    let track_mean = track_lat / track_weight.max(f64::MIN_POSITIVE);
    let phi_h = earth.climate_field.phi_h_deg;
    let mut poleward_track = 0.0;
    let mut all_track = 0.0;
    for cell in 0..n {
        let area = grid.area_m2(cell as u32) / 1.0e6;
        let (lat, _) = grid.lat_lon(cell as u32);
        let track = f64::from(earth.hazards.storms.track_km2[cell]) * area;
        all_track += track;
        if lat.to_degrees().abs() > phi_h - 5.0 {
            poleward_track += track;
        }
    }
    let poleward_share = poleward_track / all_track.max(f64::MIN_POSITIVE);
    assert!(
        track_mean > genesis_mean + 4.0,
        "mean track latitude {track_mean} genesis {genesis_mean}"
    );
    assert!(
        poleward_share >= 0.10,
        "poleward track passage share {poleward_share}"
    );

    // Land reaches the interior after the ocean. Tracks decay every land step.
    let ocean = coarse_ocean(earth);
    let mut distance = vec![u32::MAX; n];
    let mut queue = VecDeque::new();
    for (cell, is_ocean) in ocean.iter().enumerate() {
        if *is_ocean {
            distance[cell] = 0;
            queue.push_back(cell as u32);
        }
    }
    while let Some(cell) = queue.pop_front() {
        for nb in grid.neighbors(cell) {
            if nb == NONE || distance[nb as usize] != u32::MAX {
                continue;
            }
            distance[nb as usize] = distance[cell as usize] + 1;
            queue.push_back(nb);
        }
    }
    let mut coast = (0.0, 0.0);
    let mut interior = (0.0, 0.0);
    for (cell, depth) in distance.iter().enumerate() {
        let area = grid.area_m2(cell as u32);
        let track = f64::from(earth.hazards.storms.track_km2[cell]);
        match *depth {
            1 => {
                coast.0 += track * area;
                coast.1 += area;
            }
            3..=u32::MAX => {
                interior.0 += track * area;
                interior.1 += area;
            }
            _ => {}
        }
    }
    let coast_mean = coast.0 / coast.1.max(f64::MIN_POSITIVE);
    let interior_mean = interior.0 / interior.1.max(f64::MIN_POSITIVE);
    assert!(
        coast_mean > interior_mean,
        "coast track {coast_mean} interior {interior_mean}"
    );
}

fn coarse_ocean(world: &World) -> Vec<bool> {
    let coarse = &world.climate_field.grid;
    let fine = &world.grid;
    let scale = fine.n / coarse.n;
    (0..coarse.cell_count())
        .map(|cell| {
            let (face, i, j) = coarse.decode(cell as u32);
            let mut ocean = 0u32;
            for dj in 0..scale {
                for di in 0..scale {
                    let id = fine.id(face, i * scale + di, j * scale + dj);
                    if id != NONE && world.ocean[id as usize] == 1 {
                        ocean += 1;
                    }
                }
            }
            ocean * 2 >= scale * scale
        })
        .collect()
}

#[test]
fn extremes_and_floods_are_bounded_and_ordered() {
    let earth = earth_512();
    let desert = build(Preset::Desert, 7, 128);
    let hot = build(Preset::Hothouse, 7, 128);
    let frozen = build(Preset::Frozen, 7, 128);
    assert!(
        desert.hazards.stats.mean_land_drought > earth.hazards.stats.mean_land_drought,
        "desert drought {} earth {}",
        desert.hazards.stats.mean_land_drought,
        earth.hazards.stats.mean_land_drought
    );
    // A third of Earthlike land has P/PET below 0.2 (about a fifth on Earth),
    // so its land mean sits above Earth's.
    assert!(
        (0.40..=0.65).contains(&earth.hazards.stats.mean_land_drought),
        "earth drought {}",
        earth.hazards.stats.mean_land_drought
    );
    assert!(
        hot.hazards.stats.mean_heat > frozen.hazards.stats.mean_heat,
        "hot heat {} frozen {}",
        hot.hazards.stats.mean_heat,
        frozen.hazards.stats.mean_heat
    );
    for cell in 0..earth.grid.cell_count() {
        if earth.ocean[cell] == 1 {
            continue;
        }
        let precip = f64::from(earth.climate.p_annual_mm[cell]);
        let pet = f64::from(earth.climate.pet_annual_mm[cell]);
        if pet > 0.0 && precip / pet >= 1.0 {
            assert!(
                f64::from(earth.hazards.drought[cell]) / 255.0 <= 0.2,
                "humid drought at cell {cell}"
            );
        }
    }
    for cell in 0..earth.grid.cell_count() {
        if earth.ocean[cell] == 1 {
            assert_eq!(earth.hazards.drought[cell], 0);
            assert_eq!(earth.hazards.flood[cell], 0);
        }
        assert!(earth.hazards.quake_rate_km2[cell].is_finite());
        assert!(earth.hazards.quake_rate_km2[cell] >= 0.0);
    }
    for cell in 0..earth.climate_field.grid.cell_count() {
        assert!(earth.hazards.storms.genesis_km2[cell] >= 0.0);
        assert!(earth.hazards.storms.track_km2[cell] >= 0.0);
        let intensity = earth.hazards.storms.intensity[cell];
        assert!((0.0..=1.0).contains(&intensity));
    }

    assert!(flood_potential(0.0, 500.0, 0.5) >= flood_potential(6.0, 500.0, 0.5));
    assert!(flood_potential(6.0, 500.0, 0.5) >= flood_potential(12.0, 500.0, 0.5));
    assert!(flood_potential(0.0, 5000.0, 0.5) > flood_potential(0.0, 20.0, 0.5));
}

#[test]
fn hazards_are_deterministic_across_pools() {
    let one = build_threads(Preset::Earthlike, 7, 128, 1);
    let four = build_threads(Preset::Earthlike, 7, 128, 4);
    assert_eq!(one.hazards, four.hazards);
    assert_eq!(
        one.hazards.stats.hazard_hash,
        four.hazards.stats.hazard_hash
    );
    assert_eq!(one.hazards.stats.hazard_hash, one.hazards.hash_bits());
}

#[test]
fn hazard_totals_are_resolution_independent() {
    let small = earth_256();
    let large = earth_512();
    let close = |left: f64, right: f64| {
        let scale = left.abs().max(right.abs()).max(1.0e-12);
        ((left - right).abs() / scale) <= 0.05
    };
    assert!(close(
        small.hazards.stats.quakes_per_yr,
        large.hazards.stats.quakes_per_yr
    ));
    assert!(close(
        small.hazards.stats.eruptions_per_yr,
        large.hazards.stats.eruptions_per_yr
    ));
    assert!(close(
        small.hazards.stats.storms_per_yr,
        large.hazards.stats.storms_per_yr
    ));
}

#[test]
fn hazard_calibration_bands_hold() {
    let earth = earth_512();
    let stats = &earth.hazards.stats;
    assert!((1_000.0..=2_500.0).contains(&stats.quakes_per_yr));
    assert!(stats.quake_near_convergent_share >= 0.60);
    assert!((30.0..=100.0).contains(&stats.eruptions_per_yr));
    assert!(stats.arc_eruption_share >= 0.50);
    assert!((60..=300).contains(&stats.non_dormant_sources));
    assert!((60.0..=110.0).contains(&stats.storms_per_yr));
}

#[test]
fn boundary_records_cover_both_sides() {
    let earth = earth_512();
    let tect = &earth.tectonics;
    for rec in &tect.boundaries {
        let index = tect.boundary_record[rec.neighbor as usize];
        assert_ne!(index, NONE, "neighbour has no boundary record");
        let back = &tect.boundaries[index as usize];
        assert_eq!(back.cell, rec.neighbor, "record chain is not reciprocal");
    }
}

#[test]
fn quake_density_is_smooth_along_belts() {
    let earth = earth_512();
    let rate = &earth.hazards.quake_rate_km2;
    let max = rate.iter().copied().fold(0.0f32, f32::max);
    let threshold = 0.1 * max;
    let mut worst = 0.0f32;
    for cell in 0..earth.grid.cell_count() {
        if rate[cell] < threshold {
            continue;
        }
        for neighbor in earth.grid.neighbors(cell as u32) {
            if neighbor == NONE {
                continue;
            }
            let other = rate[neighbor as usize];
            if other < threshold {
                continue;
            }
            let ratio = if rate[cell] > other {
                rate[cell] / other
            } else {
                other / rate[cell]
            };
            worst = worst.max(ratio);
        }
    }
    assert!(worst <= 2.6, "neighbour density ratio {worst}");
}
