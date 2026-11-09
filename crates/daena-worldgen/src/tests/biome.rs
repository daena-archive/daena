use crate::biome::Biome;
use crate::hydrology::LakeKind;
use crate::{generate, NoProgress, PlanetSettings, World, WorldSettings};

#[test]
fn default_earth_has_the_expected_land_biomes() {
    let world = build(0.0, 1.0);
    let mut seen = [false; 26];
    for byte in &world.biome {
        if let Some(class) = Biome::from_u8(*byte) {
            if class.is_land_class() {
                seen[class as usize] = true;
            }
        }
    }
    let land: Vec<Biome> = (0..26u8)
        .filter_map(Biome::from_u8)
        .filter(|class| class.is_land_class() && seen[*class as usize])
        .collect();
    assert!(land.len() >= 8, "land biomes {land:?}");
    assert!(seen[Biome::HotDesert as usize], "missing hot desert");
    assert!(
        seen[Biome::TropicalRainforest as usize] || seen[Biome::TropicalSeasonalForest as usize],
        "missing tropical forest"
    );
    assert!(
        seen[Biome::BorealForest as usize] || seen[Biome::Tundra as usize],
        "missing boreal or tundra"
    );
}

#[test]
fn dry_world_has_more_desert_and_salt() {
    // Seed 1's two basins still overflow at 0.3× rain, so the saline share
    // stays zero. Seed 6 has a closed basin that is endorheic when dry and
    // fresh when wet. n = 64 holds only a handful of depressions.
    let dry = build_seed(6, 0.0, 0.3);
    let wet = build_seed(6, 0.0, 2.0);
    let dry_desert = desert_fraction(&dry);
    let wet_desert = desert_fraction(&wet);
    assert!(
        dry_desert > wet_desert,
        "dry desert {dry_desert} wet {wet_desert}"
    );
    let dry_salt = saline_share(&dry);
    let wet_salt = saline_share(&wet);
    assert!(
        dry_salt > wet_salt,
        "dry saline share {dry_salt} wet {wet_salt}"
    );
}

fn desert_fraction(world: &World) -> f64 {
    let mut desert = 0.0;
    let mut land = 0.0;
    for cell in 0..world.grid.cell_count() {
        if world.ocean[cell] == 1 {
            continue;
        }
        let area = world.grid.area_m2(cell as u32);
        land += area;
        if let Some(class) = Biome::from_u8(world.biome[cell]) {
            if class.is_desert() {
                desert += area;
            }
        }
    }
    if land > 0.0 {
        desert / land
    } else {
        0.0
    }
}

fn saline_share(world: &World) -> f64 {
    let mut salt = 0.0;
    let mut all = 0.0;
    for lake in &world.hydrology.lakes {
        all += lake.area_m2;
        if lake.kind != LakeKind::Fresh {
            salt += lake.area_m2;
        }
    }
    if all > 0.0 {
        salt / all
    } else {
        0.0
    }
}

fn build(offset: f64, precip: f64) -> World {
    build_seed(1, offset, precip)
}

fn build_seed(seed: u64, offset: f64, precip: f64) -> World {
    let settings = WorldSettings {
        resolution: 64,
        seed,
        planet: PlanetSettings {
            temperature_offset_c: offset,
            precipitation_scale: precip,
            ..PlanetSettings::default()
        },
        ..WorldSettings::default()
    };
    generate(&settings, &mut NoProgress).unwrap()
}
