use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use crate::{
    derive_epoch, derive_world, generate, generate_tectonics, ocean_volume_at, ocean_volume_curve,
    sea_level_for_ocean_volume, NoProgress, Preset, Progress, Stage, World, WorldError,
    WorldSettings,
};
fn settings(preset: Preset, seed: u64, resolution: u32) -> WorldSettings {
    WorldSettings {
        seed,
        resolution,
        ..preset.settings()
    }
}

fn hashes(world: &World) -> crate::GenerationHashes {
    world.generation_hashes()
}

fn assert_same_world(left: &World, right: &World) {
    assert_eq!(left.settings.seed, right.settings.seed);
    assert_eq!(left.settings.resolution, right.settings.resolution);
    assert_eq!(left.settings.radius_m, right.settings.radius_m);
    assert_eq!(left.settings.land_fraction, right.settings.land_fraction);
    assert_eq!(left.settings.plate_count, right.settings.plate_count);
    assert_eq!(
        left.settings.continent_layout,
        right.settings.continent_layout
    );
    assert_eq!(
        left.settings.tectonic_activity,
        right.settings.tectonic_activity
    );
    assert_eq!(left.settings.river_min_m3s, right.settings.river_min_m3s);
    assert_eq!(
        left.settings.planet.star_luminosity,
        right.settings.planet.star_luminosity
    );
    assert_eq!(
        left.settings.planet.star_mass,
        right.settings.planet.star_mass
    );
    assert_eq!(
        left.settings.planet.orbit_au,
        right.settings.planet.orbit_au
    );
    assert_eq!(
        left.settings.planet.eccentricity,
        right.settings.planet.eccentricity
    );
    assert_eq!(
        left.settings.planet.temperature_offset_c,
        right.settings.planet.temperature_offset_c
    );
    assert_eq!(left.grid.n, right.grid.n);
    assert_eq!(left.grid.radius_m, right.grid.radius_m);
    assert_eq!(left.elevation_m, right.elevation_m);
    assert_eq!(left.base_m, right.base_m);
    assert_eq!(left.uplift_m_yr, right.uplift_m_yr);
    assert_eq!(left.k_rock, right.k_rock);
    assert_eq!(left.plate, right.plate);
    assert_eq!(left.crust, right.crust);
    assert_eq!(left.ocean, right.ocean);
    assert_eq!(left.receiver, right.receiver);
    assert_eq!(left.drainage_area_m2, right.drainage_area_m2);
    assert_eq!(left.climate.t_annual_c, right.climate.t_annual_c);
    assert_eq!(left.climate.t_warm_c, right.climate.t_warm_c);
    assert_eq!(left.climate.t_cold_c, right.climate.t_cold_c);
    assert_eq!(left.climate.p_annual_mm, right.climate.p_annual_mm);
    assert_eq!(left.climate.p_dry_month_mm, right.climate.p_dry_month_mm);
    assert_eq!(left.climate.pet_annual_mm, right.climate.pet_annual_mm);
    assert_eq!(left.climate.runoff_mm, right.climate.runoff_mm);
    assert_eq!(left.climate.ice_balance_mm, right.climate.ice_balance_mm);
    assert_eq!(left.climate.sea_ice_months, right.climate.sea_ice_months);
    assert_eq!(left.climate_field.t_c, right.climate_field.t_c);
    assert_eq!(left.climate_field.p_mm, right.climate_field.p_mm);
    assert_eq!(left.climate_field.wind, right.climate_field.wind);
    assert_eq!(left.climate_field.maritime, right.climate_field.maritime);
    assert_eq!(left.hydrology, right.hydrology);
    assert_eq!(left.biome, right.biome);
    assert_eq!(left.hazards, right.hazards);
    assert_eq!(
        left.stats.achieved_land_fraction,
        right.stats.achieved_land_fraction
    );
    assert_eq!(left.stats.dry_depressions, right.stats.dry_depressions);
    assert_eq!(left.stats.elevation_min_m, right.stats.elevation_min_m);
    assert_eq!(left.stats.elevation_max_m, right.stats.elevation_max_m);
    assert_eq!(left.stats.land_p50_m, right.stats.land_p50_m);
    assert_eq!(left.stats.land_p90_m, right.stats.land_p90_m);
    assert_eq!(left.stats.land_p99_m, right.stats.land_p99_m);
    assert_eq!(left.hydrology_hash(), right.hydrology_hash());
    assert_eq!(left.climate_hash(), right.climate_hash());
    assert_eq!(left.biome_hash(), right.biome_hash());
    assert_eq!(left.hazard_hash(), right.hazard_hash());
    assert_eq!(hashes(left), hashes(right));
}

#[test]
fn final_routing_matches_full_grid_route() {
    for resolution in [64u32, 128] {
        let world = generate(&settings(Preset::Earthlike, 7, resolution), &mut NoProgress).unwrap();
        let domain: Vec<u8> = world
            .ocean
            .iter()
            .map(|&flag| u8::from(flag == 0))
            .collect();
        let mut scratch = crate::routing::RoutingScratch::new(world.elevation_m.len());
        crate::routing::route(
            &world.grid,
            &world.elevation_m,
            &domain,
            None,
            &mut scratch,
            None,
        );
        assert_eq!(world.receiver, scratch.receiver, "n={resolution}");
        let area: Vec<f32> = scratch.area.iter().map(|&value| value as f32).collect();
        assert_eq!(world.drainage_area_m2, area, "n={resolution}");
    }
}

#[test]
fn derive_world_and_epoch_zero_match_generation() {
    for (preset, seed, resolution) in [
        (Preset::Earthlike, 7, 64),
        (Preset::Earthlike, 7, 128),
        (Preset::Earthlike, 7, 256),
        (Preset::Frozen, 7, 64),
        (Preset::Frozen, 7, 128),
        (Preset::Archipelago, 7, 64),
        (Preset::Archipelago, 7, 128),
        (Preset::Desert, 7, 64),
        (Preset::Desert, 7, 128),
    ] {
        let settings = settings(preset, seed, resolution);
        let world = generate(&settings, &mut NoProgress).unwrap();
        let tectonics = generate_tectonics(&settings, &mut NoProgress).unwrap();
        let reference = world.source_elevation_mm();
        let derived = derive_world(&settings, &tectonics, &reference, &mut NoProgress).unwrap();
        assert_eq!(derived.climate_passes.len(), 1);
        assert_eq!(derived.climate_passes[0].label, "final");
        assert_same_world(&world, &derived);

        let epoch =
            derive_epoch(&settings, &tectonics, &reference, 0, 0.0, &mut NoProgress).unwrap();
        assert_same_world(&derived, &epoch);
    }
}

#[test]
fn epoch_sanity_more_land_colder_more_ice_and_deterministic() {
    let settings = settings(Preset::Earthlike, 7, 128);
    let world = generate(&settings, &mut NoProgress).unwrap();
    let tectonics = generate_tectonics(&settings, &mut NoProgress).unwrap();
    let reference = world.source_elevation_mm();
    let epoch = derive_epoch(
        &settings,
        &tectonics,
        &reference,
        -50_000,
        -4.0,
        &mut NoProgress,
    )
    .unwrap();
    let repeated = derive_epoch(
        &settings,
        &tectonics,
        &reference,
        -50_000,
        -4.0,
        &mut NoProgress,
    )
    .unwrap();
    assert!(epoch.stats.achieved_land_fraction > world.stats.achieved_land_fraction);
    assert!(epoch.climate.stats.t_global_c < world.climate.stats.t_global_c);
    let epoch_ice = epoch
        .hydrology
        .ice_cover
        .iter()
        .filter(|value| **value != 0)
        .count();
    let present_ice = world
        .hydrology
        .ice_cover
        .iter()
        .filter(|value| **value != 0)
        .count();
    assert!(
        epoch_ice > present_ice,
        "epoch {epoch_ice} present {present_ice}"
    );
    assert_eq!(hashes(&epoch), hashes(&repeated));
}

#[test]
fn cube_inventory_solver_round_trips_millimetres() {
    for resolution in [64, 256] {
        let settings = settings(Preset::Earthlike, 7, resolution);
        let world = generate(&settings, &mut NoProgress).unwrap();
        let elevation = world.source_elevation_mm();
        let curve = ocean_volume_curve(&world.grid, &elevation).unwrap();
        assert_eq!(
            ocean_volume_at(&world.grid, &elevation, 0).unwrap(),
            world.reference_water_inventory_m3()
        );
        assert_eq!(
            sea_level_for_ocean_volume(&curve, world.reference_water_inventory_m3()).unwrap(),
            0
        );
        let mut levels = vec![
            -4_000_000, -120_000, -50_000, -1_234, -1, 1, 777, 3_333, 70_000,
        ];
        let minimum = elevation.iter().copied().min().unwrap();
        let maximum = elevation.iter().copied().max().unwrap();
        let mut seed = 17u64;
        for _ in 0..50 {
            seed = crate::splitmix64(seed);
            let span = i64::from(maximum) - i64::from(minimum) + 1;
            let level = i64::from(minimum) + (seed % span as u64) as i64;
            levels.push(level as i32);
        }
        for level in levels {
            let volume = ocean_volume_at(&world.grid, &elevation, level).unwrap();
            let solved = sea_level_for_ocean_volume(&curve, volume).unwrap();
            assert!(
                (solved - level).abs() <= 1,
                "n={resolution} volume {volume} level {level} solved {solved}"
            );
        }
    }
}

#[test]
fn every_generation_hash_component_reacts_to_its_fields() {
    let settings = settings(Preset::Earthlike, 7, 64);
    let world = generate(&settings, &mut NoProgress).unwrap();
    assert!(!world.tectonics.plates.is_empty());
    assert!(!world.tectonics.boundaries.is_empty());
    assert!(!world.tectonics.hotspots.is_empty());
    assert!(!world.hydrology.lakes.is_empty());
    assert!(!world.hydrology.rivers.is_empty());
    assert!(!world.hazards.volcanic_sources.is_empty());

    fn next_layout(layout: crate::ContinentLayout) -> crate::ContinentLayout {
        use crate::ContinentLayout::*;
        match layout {
            Pangea => Few,
            Few => Many,
            Many => Mixed,
            Mixed => Pangea,
        }
    }
    fn next_boundary(kind: crate::BoundaryKind) -> crate::BoundaryKind {
        use crate::BoundaryKind::*;
        match kind {
            Convergent => Divergent,
            Divergent => Transform,
            Transform => Convergent,
        }
    }
    fn next_subduct(side: crate::SubductSide) -> crate::SubductSide {
        use crate::SubductSide::*;
        match side {
            None => This,
            This => Other,
            Other => None,
        }
    }
    fn next_lake(kind: crate::LakeKind) -> crate::LakeKind {
        use crate::LakeKind::*;
        match kind {
            Fresh => Saline,
            Saline => SaltFlat,
            SaltFlat => Fresh,
        }
    }
    fn next_river(end: crate::RiverEnd) -> crate::RiverEnd {
        use crate::RiverEnd::*;
        match end {
            Ocean => Lake,
            Lake => Confluence,
            Confluence => Ocean,
        }
    }
    fn next_origin(origin: crate::VolcanicOrigin) -> crate::VolcanicOrigin {
        use crate::VolcanicOrigin::*;
        match origin {
            SubductionArc => Hotspot,
            Hotspot => Rift,
            Rift => SubductionArc,
        }
    }
    fn next_activity(activity: crate::VolcanicActivity) -> crate::VolcanicActivity {
        use crate::VolcanicActivity::*;
        match activity {
            Dormant => Moderate,
            Moderate => Vigorous,
            Vigorous => Dormant,
        }
    }

    fn assert_component_changes(
        world: &World,
        label: &str,
        select: fn(&crate::GenerationHashes) -> u64,
        change: impl FnOnce(&mut World),
    ) {
        let before = select(&world.generation_hashes());
        let mut changed = world.clone();
        change(&mut changed);
        assert_ne!(
            before,
            select(&changed.generation_hashes()),
            "{label} did not change its hash"
        );
    }

    fn assert_component_unchanged(
        world: &World,
        label: &str,
        select: fn(&crate::GenerationHashes) -> u64,
        change: impl FnOnce(&mut World),
    ) {
        let before = select(&world.generation_hashes());
        let mut changed = world.clone();
        change(&mut changed);
        assert_eq!(
            before,
            select(&changed.generation_hashes()),
            "{label} is in the hash by contract"
        );
    }

    // Elevation is hashed as `i32` millimetres.
    assert_component_changes(
        &world,
        "elevation",
        |h| h.elevation_mm,
        |w| {
            w.elevation_m[0] += 1.0;
        },
    );

    // Tectonics: metadata, plates, boundaries, hotspots and every cell field.
    assert_component_changes(
        &world,
        "tectonics.n",
        |h| h.tectonics,
        |w| {
            w.tectonics.n += 1;
        },
    );
    assert_component_changes(
        &world,
        "tectonics.layout",
        |h| h.tectonics,
        |w| {
            w.tectonics.resolved_layout = next_layout(w.tectonics.resolved_layout);
        },
    );
    assert_component_changes(
        &world,
        "tectonics.crust_fraction",
        |h| h.tectonics,
        |w| {
            w.tectonics.crust_fraction += 0.01;
        },
    );
    for (label, change) in [
        (
            "plate.site",
            (|w: &mut World| w.tectonics.plates[0].site[0] += 0.1) as fn(&mut World),
        ),
        ("plate.axis", |w: &mut World| {
            w.tectonics.plates[0].axis[0] += 0.1;
        }),
        ("plate.omega", |w: &mut World| {
            w.tectonics.plates[0].omega_rad_yr += 0.01;
        }),
        ("plate.subduction_rank", |w: &mut World| {
            w.tectonics.plates[0].subduction_rank += 1;
        }),
        ("plate.weight", |w: &mut World| {
            w.tectonics.plates[0].weight_rad += 0.01;
        }),
        ("plate.crust_majority", |w: &mut World| {
            w.tectonics.plates[0].crust_majority ^= 1;
        }),
        ("boundary.cell", |w: &mut World| {
            w.tectonics.boundaries[0].cell += 1;
        }),
        ("boundary.neighbor", |w: &mut World| {
            w.tectonics.boundaries[0].neighbor += 1;
        }),
        ("boundary.plate_a", |w: &mut World| {
            w.tectonics.boundaries[0].plate_a ^= 1;
        }),
        ("boundary.plate_b", |w: &mut World| {
            w.tectonics.boundaries[0].plate_b ^= 1;
        }),
        ("boundary.crust_a", |w: &mut World| {
            w.tectonics.boundaries[0].crust_a ^= 1;
        }),
        ("boundary.crust_b", |w: &mut World| {
            w.tectonics.boundaries[0].crust_b ^= 1;
        }),
        ("boundary.kind", |w: &mut World| {
            w.tectonics.boundaries[0].kind = next_boundary(w.tectonics.boundaries[0].kind);
        }),
        ("boundary.convergence", |w: &mut World| {
            w.tectonics.boundaries[0].convergence_m_yr += 0.01;
        }),
        ("boundary.shear", |w: &mut World| {
            w.tectonics.boundaries[0].shear_m_yr += 0.01;
        }),
        ("boundary.convergence_raw", |w: &mut World| {
            w.tectonics.boundaries[0].convergence_raw_m_yr += 0.01;
        }),
        ("boundary.shear_raw", |w: &mut World| {
            w.tectonics.boundaries[0].shear_raw_m_yr += 0.01;
        }),
        ("boundary.subducts", |w: &mut World| {
            w.tectonics.boundaries[0].subducts = next_subduct(w.tectonics.boundaries[0].subducts);
        }),
        ("hotspot.origin", |w: &mut World| {
            w.tectonics.hotspots[0].origin[0] += 0.1;
        }),
        ("hotspot.points", |w: &mut World| {
            w.tectonics.hotspots[0].points[0][0] += 0.1;
        }),
        ("tectonics.plate", |w: &mut World| {
            w.tectonics.plate[0] = w.tectonics.plate[0].wrapping_add(1);
        }),
        ("tectonics.crust", |w: &mut World| {
            w.tectonics.crust[0] ^= 1;
        }),
        ("tectonics.base_m", |w: &mut World| {
            w.tectonics.base_m[0] += 1.0;
        }),
        ("tectonics.uplift", |w: &mut World| {
            w.tectonics.uplift_m_yr[0] += 1.0;
        }),
        ("tectonics.k_rock", |w: &mut World| {
            w.tectonics.k_rock[0] += 1.0;
        }),
        ("tectonics.d_boundary", |w: &mut World| {
            let value = &mut w.tectonics.d_boundary[0];
            *value = f32::from_bits(value.to_bits() ^ 1);
        }),
        ("tectonics.d_ridge", |w: &mut World| {
            let value = &mut w.tectonics.d_ridge[0];
            *value = f32::from_bits(value.to_bits() ^ 1);
        }),
        ("tectonics.d_edge", |w: &mut World| {
            let value = &mut w.tectonics.d_edge[0];
            *value = f32::from_bits(value.to_bits() ^ 1);
        }),
        ("tectonics.boundary_record", |w: &mut World| {
            w.tectonics.boundary_record[0] = w.tectonics.boundary_record[0].wrapping_add(1);
        }),
        ("tectonics.d_arc", |w: &mut World| {
            let value = &mut w.tectonics.d_arc[0];
            *value = f32::from_bits(value.to_bits() ^ 1);
        }),
        ("tectonics.arc_record", |w: &mut World| {
            w.tectonics.arc_record[0] = w.tectonics.arc_record[0].wrapping_add(1);
        }),
        ("tectonics.boundary_kind", |w: &mut World| {
            w.tectonics.boundary_kind[0] ^= 1;
        }),
        ("tectonics.fragments", |w: &mut World| {
            w.tectonics.fragments_reassigned += 1;
        }),
    ] {
        assert_component_changes(&world, label, |h| h.tectonics, change);
    }

    // Climate: every stored field. The report stats are not hashed.
    for (label, change) in [
        (
            "climate.t_annual",
            (|w: &mut World| w.climate.t_annual_c[0] += 1.0) as fn(&mut World),
        ),
        ("climate.t_warm", |w: &mut World| {
            w.climate.t_warm_c[0] += 1.0;
        }),
        ("climate.t_cold", |w: &mut World| {
            w.climate.t_cold_c[0] += 1.0;
        }),
        ("climate.p_annual", |w: &mut World| {
            w.climate.p_annual_mm[0] += 1.0;
        }),
        ("climate.p_dry", |w: &mut World| {
            w.climate.p_dry_month_mm[0] += 1.0;
        }),
        ("climate.pet", |w: &mut World| {
            w.climate.pet_annual_mm[0] += 1.0;
        }),
        ("climate.runoff", |w: &mut World| {
            w.climate.runoff_mm[0] += 1.0;
        }),
        ("climate.ice_balance", |w: &mut World| {
            w.climate.ice_balance_mm[0] += 1.0;
        }),
        ("climate.sea_ice", |w: &mut World| {
            w.climate.sea_ice_months[0] ^= 1;
        }),
    ] {
        assert_component_changes(&world, label, |h| h.climate, change);
    }
    assert_component_unchanged(
        &world,
        "climate.stats",
        |h| h.climate,
        |w| {
            w.climate.stats.t_global_c += 1.0;
        },
    );

    // Hydrology: lake water state, rivers, and the per-cell drainage fields.
    // `Lake::id`, `minimum_cell` and `spill_cell` are excluded: they are claim
    // metadata derived from the lake geometry, not water state.
    for (label, change) in [
        (
            "lake.kind",
            (|w: &mut World| w.hydrology.lakes[0].kind = next_lake(w.hydrology.lakes[0].kind))
                as fn(&mut World),
        ),
        ("lake.level", |w: &mut World| {
            w.hydrology.lakes[0].level_m += 1.0;
        }),
        ("lake.area", |w: &mut World| {
            w.hydrology.lakes[0].area_m2 += 1.0;
        }),
        ("lake.volume", |w: &mut World| {
            w.hydrology.lakes[0].volume_m3 += 1.0;
        }),
        ("lake.evaporated", |w: &mut World| {
            w.hydrology.lakes[0].evaporated_m3 += 1.0;
        }),
        ("lake.outflow", |w: &mut World| {
            w.hydrology.lakes[0].outflow_m3 += 1.0;
        }),
        ("lake.inflow", |w: &mut World| {
            w.hydrology.lakes[0].inflow_m3 += 1.0;
        }),
        ("river.end", |w: &mut World| {
            w.hydrology.rivers[0].end = next_river(w.hydrology.rivers[0].end);
        }),
        ("river.max_discharge", |w: &mut World| {
            w.hydrology.rivers[0].max_discharge_m3s += 1.0;
        }),
        ("river.cells", |w: &mut World| {
            w.hydrology.rivers[0].cells[0] += 1;
        }),
        ("hydrology.lake_id", |w: &mut World| {
            w.hydrology.lake_id[0] = w.hydrology.lake_id[0].wrapping_add(1);
        }),
        ("hydrology.discharge", |w: &mut World| {
            w.hydrology.discharge_m3s[0] += 1.0;
        }),
        ("hydrology.ice_cover", |w: &mut World| {
            w.hydrology.ice_cover[0] ^= 1;
        }),
        ("hydrology.receiver", |w: &mut World| {
            w.hydrology.receiver[0] = w.hydrology.receiver[0].wrapping_add(1);
        }),
    ] {
        assert_component_changes(&world, label, |h| h.hydrology, change);
    }
    assert_component_unchanged(
        &world,
        "lake.id",
        |h| h.hydrology,
        |w| {
            w.hydrology.lakes[0].id += 1;
        },
    );
    assert_component_unchanged(
        &world,
        "lake.minimum_cell",
        |h| h.hydrology,
        |w| {
            w.hydrology.lakes[0].minimum_cell += 1;
        },
    );
    assert_component_unchanged(
        &world,
        "lake.spill_cell",
        |h| h.hydrology,
        |w| {
            w.hydrology.lakes[0].spill_cell += 1;
        },
    );
    assert_component_unchanged(
        &world,
        "hydrology.filled_m",
        |h| h.hydrology,
        |w| {
            w.hydrology.filled_m[0] += 1.0;
        },
    );

    // Biome ids.
    assert_component_changes(
        &world,
        "biome",
        |h| h.biome,
        |w| {
            w.biome[0] ^= 1;
        },
    );

    // Hazards: every field, source and storm array.
    for (label, change) in [
        (
            "hazards.quake_rate",
            (|w: &mut World| w.hazards.quake_rate_km2[0] += 1.0) as fn(&mut World),
        ),
        ("hazards.quake_hazard", |w: &mut World| {
            w.hazards.quake_hazard[0] ^= 1;
        }),
        ("hazards.volcanic_hazard", |w: &mut World| {
            w.hazards.volcanic_hazard[0] ^= 1;
        }),
        ("hazards.volcanic_source.id", |w: &mut World| {
            w.hazards.volcanic_sources[0].id += 1;
        }),
        ("hazards.volcanic_source.pos", |w: &mut World| {
            w.hazards.volcanic_sources[0].pos[0] += 0.1;
        }),
        ("hazards.volcanic_source.cell", |w: &mut World| {
            w.hazards.volcanic_sources[0].cell += 1;
        }),
        ("hazards.volcanic_source.origin", |w: &mut World| {
            w.hazards.volcanic_sources[0].origin =
                next_origin(w.hazards.volcanic_sources[0].origin);
        }),
        ("hazards.volcanic_source.activity", |w: &mut World| {
            w.hazards.volcanic_sources[0].activity =
                next_activity(w.hazards.volcanic_sources[0].activity);
        }),
        ("hazards.volcanic_source.eruptions", |w: &mut World| {
            w.hazards.volcanic_sources[0].eruptions_per_yr += 1.0;
        }),
        ("hazards.storm.genesis", |w: &mut World| {
            w.hazards.storms.genesis_km2[0] += 1.0;
        }),
        ("hazards.storm.track", |w: &mut World| {
            w.hazards.storms.track_km2[0] += 1.0;
        }),
        ("hazards.storm.intensity", |w: &mut World| {
            w.hazards.storms.intensity[0] += 1.0;
        }),
        ("hazards.storm.monthly", |w: &mut World| {
            w.hazards.storms.genesis_monthly_km2[0] += 1.0;
        }),
        ("hazards.drought", |w: &mut World| {
            w.hazards.drought[0] ^= 1;
        }),
        ("hazards.heat_wave", |w: &mut World| {
            w.hazards.heat_wave[0] ^= 1;
        }),
        ("hazards.extreme_rain", |w: &mut World| {
            w.hazards.extreme_rain[0] ^= 1;
        }),
        ("hazards.flood", |w: &mut World| {
            w.hazards.flood[0] ^= 1;
        }),
    ] {
        assert_component_changes(&world, label, |h| h.hazards, change);
    }
    assert_component_unchanged(
        &world,
        "hazards.stats",
        |h| h.hazards,
        |w| {
            w.hazards.stats.quakes_per_yr += 1.0;
        },
    );

    // Sanity check the selectors themselves.
    let _ = world.generation_hashes();
}

struct CountingProgress {
    calls: AtomicU64,
    cancel_at: u64,
}

impl CountingProgress {
    fn new(cancel_at: u64) -> Self {
        Self {
            calls: AtomicU64::new(0),
            cancel_at,
        }
    }

    fn count(&self) -> u64 {
        self.calls.load(Ordering::Relaxed)
    }
}

impl Progress for CountingProgress {
    fn report(&mut self, _stage: Stage, _done: u32, _total: u32) {}

    fn cancelled(&self) -> bool {
        let call = self.calls.fetch_add(1, Ordering::Relaxed) + 1;
        call >= self.cancel_at
    }
}

fn sampled_checks(total: u64) -> Vec<u64> {
    (0..30).map(|index| 1 + index * (total - 1) / 29).collect()
}

#[test]
fn cancellation_at_sampled_checks_always_returns_cancelled() {
    let settings = settings(Preset::Earthlike, 7, 64);
    let mut counter = CountingProgress::new(u64::MAX);
    let world = generate(&settings, &mut counter).unwrap();
    let generate_calls = counter.count();
    assert!(generate_calls > 30, "generate checks: {generate_calls}");

    for k in sampled_checks(generate_calls) {
        let mut progress = CountingProgress::new(k);
        let result = generate(&settings, &mut progress);
        assert!(
            matches!(result, Err(WorldError::Cancelled)),
            "generate K={k} of {generate_calls}: {result:?}"
        );
    }

    let tectonics = generate_tectonics(&settings, &mut NoProgress).unwrap();
    let reference = world.source_elevation_mm();
    let mut counter = CountingProgress::new(u64::MAX);
    derive_world(&settings, &tectonics, &reference, &mut counter).unwrap();
    let derive_calls = counter.count();
    assert!(derive_calls > 30, "derive checks: {derive_calls}");

    for k in sampled_checks(derive_calls) {
        let mut progress = CountingProgress::new(k);
        let result = derive_world(&settings, &tectonics, &reference, &mut progress);
        assert!(
            matches!(result, Err(WorldError::Cancelled)),
            "derive K={k} of {derive_calls}: {result:?}"
        );
    }
}

struct GapProgress {
    last: Mutex<Instant>,
    stage: Mutex<Option<Stage>>,
    max: Mutex<(std::time::Duration, Option<Stage>)>,
}

impl GapProgress {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            last: Mutex::new(now),
            stage: Mutex::new(None),
            max: Mutex::new((std::time::Duration::ZERO, None)),
        }
    }

    fn max_gap(&self) -> (std::time::Duration, Option<Stage>) {
        *self.max.lock().unwrap()
    }
}

impl Progress for GapProgress {
    fn report(&mut self, stage: Stage, _done: u32, _total: u32) {
        *self.stage.lock().unwrap() = Some(stage);
    }

    fn cancelled(&self) -> bool {
        let now = Instant::now();
        let mut last = self.last.lock().unwrap();
        let gap = now.saturating_duration_since(*last);
        *last = now;
        let stage = *self.stage.lock().unwrap();
        let mut max = self.max.lock().unwrap();
        if gap > max.0 {
            *max = (gap, stage);
        }
        false
    }
}

#[test]
#[ignore = "wall-clock gap; run with --ignored --test-threads=1"]
fn cancellation_gaps_at_n512_stay_under_the_command_budget() {
    let settings = settings(Preset::Earthlike, 7, 512);
    let mut gap = GapProgress::new();
    generate(&settings, &mut gap).unwrap();
    let (download_gap, stage) = gap.max_gap();
    eprintln!(
        "generate n=512 largest cancelled() gap: {} ms ({stage:?})",
        download_gap.as_millis()
    );
    assert!(
        download_gap < std::time::Duration::from_millis(100),
        "generate gap {download_gap:?} at {stage:?}"
    );

    let tectonics = generate_tectonics(&settings, &mut NoProgress).unwrap();
    let reference = generate(&settings, &mut NoProgress)
        .unwrap()
        .source_elevation_mm();
    let mut gap = GapProgress::new();
    derive_world(&settings, &tectonics, &reference, &mut gap).unwrap();
    let (derive_gap, stage) = gap.max_gap();
    eprintln!(
        "derive n=512 largest cancelled() gap: {} ms ({stage:?})",
        derive_gap.as_millis()
    );
    assert!(
        derive_gap < std::time::Duration::from_millis(100),
        "derive gap {derive_gap:?} at {stage:?}"
    );
}

#[test]
#[ignore = "wall-clock gap; run with --ignored --test-threads=1"]
fn cancellation_gaps_at_n1024_report_cold_and_warm() {
    let settings = settings(Preset::Earthlike, 7, 1024);

    let mut cold = GapProgress::new();
    generate(&settings, &mut cold).unwrap();
    let (cold_gap, cold_stage) = cold.max_gap();

    let mut warm = GapProgress::new();
    let warm_world = generate(&settings, &mut warm).unwrap();
    let (warm_gap, warm_stage) = warm.max_gap();

    let tectonics = generate_tectonics(&settings, &mut NoProgress).unwrap();
    let reference = warm_world.source_elevation_mm();
    let mut derive = GapProgress::new();
    derive_world(&settings, &tectonics, &reference, &mut derive).unwrap();
    let (derive_gap, derive_stage) = derive.max_gap();

    eprintln!(
        "generate n=1024 cold gap: {} ms ({cold_stage:?}); warm gap: {} ms ({warm_stage:?}); derive gap: {} ms ({derive_stage:?})",
        cold_gap.as_millis(),
        warm_gap.as_millis(),
        derive_gap.as_millis()
    );
    // Report only. In the test profile the n = 1024 routing stages and the
    // debug-only routing validation sit close to the 100 ms budget, so an
    // assertion here is flaky. Release gaps are measured from this output.
}

struct DeadlineProgress {
    start: Instant,
    deadline_ms: AtomicU64,
    cancelled_at: Mutex<Option<Instant>>,
}

impl DeadlineProgress {
    fn new() -> Self {
        Self {
            start: Instant::now(),
            deadline_ms: AtomicU64::new(u64::MAX),
            cancelled_at: Mutex::new(None),
        }
    }
}

impl Progress for DeadlineProgress {
    fn report(&mut self, stage: Stage, done: u32, _total: u32) {
        if stage == Stage::Climate && done == 0 {
            let deadline = self.start.elapsed().as_millis() as u64 + 20;
            self.deadline_ms.store(deadline, Ordering::Relaxed);
        }
    }

    fn cancelled(&self) -> bool {
        let due =
            self.start.elapsed().as_millis() as u64 >= self.deadline_ms.load(Ordering::Relaxed);
        if due {
            let mut at = self.cancelled_at.lock().unwrap();
            if at.is_none() {
                *at = Some(Instant::now());
            }
        }
        due
    }
}

#[test]
#[ignore = "wall-clock deadline; run with --ignored --test-threads=1"]
fn cancellation_during_the_final_climate_stage_is_bounded() {
    let settings = settings(Preset::Earthlike, 7, 512);
    let mut progress = DeadlineProgress::new();
    let result = generate(&settings, &mut progress);
    assert!(matches!(result, Err(WorldError::Cancelled)));
    let at = progress
        .cancelled_at
        .lock()
        .unwrap()
        .expect("cancellation did not happen");
    let deadline = progress.deadline_ms.load(Ordering::Relaxed);
    let late = at
        .duration_since(progress.start)
        .saturating_sub(std::time::Duration::from_millis(deadline));
    eprintln!(
        "cancel after final-climate deadline: {} ms late",
        late.as_millis()
    );
    assert!(late < std::time::Duration::from_millis(100), "{late:?}");
}
