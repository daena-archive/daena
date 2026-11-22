//! sRGB albedo for the imagery style. Mixing happens in linear RGB.

use daena_physical::products::Biome;

/// Land biomes, plus the extra ground materials imagery mixes in.
/// Ocean and lake biomes are coloured by the water model, not this table.
const LAND_SRGB: [(Biome, [u8; 3]); 18] = [
    (Biome::TropicalRainforest, [34, 60, 28]),
    (Biome::TropicalSeasonalForest, [62, 78, 36]),
    (Biome::TemperateRainforest, [32, 56, 34]),
    (Biome::TemperateForest, [52, 72, 38]),
    (Biome::BorealForest, [38, 54, 36]),
    (Biome::Mangrove, [36, 62, 38]),
    (Biome::Wetland, [58, 74, 48]),
    (Biome::Grassland, [112, 116, 62]),
    (Biome::Savanna, [146, 130, 80]),
    (Biome::Mediterranean, [116, 108, 72]),
    (Biome::ColdSteppe, [128, 120, 88]),
    (Biome::HotDesert, [200, 168, 120]),
    (Biome::ColdDesert, [150, 138, 112]),
    (Biome::Tundra, [108, 102, 82]),
    (Biome::Alpine, [108, 114, 92]),
    (Biome::SaltFlat, [226, 220, 206]),
    (Biome::IceSheet, [236, 240, 245]),
    (Biome::Glacier, [236, 240, 245]),
];

pub(super) const ROCK_SRGB: [u8; 3] = [118, 110, 102];
pub(super) const SNOW_SRGB: [u8; 3] = [245, 247, 250];
pub(super) const ICE_SRGB: [u8; 3] = [236, 240, 245];
pub(super) const BEACH_SRGB: [u8; 3] = [204, 188, 152];
pub(super) const GRASS_DRY_SRGB: [u8; 3] = [150, 136, 86];
pub(super) const GRASS_LUSH_SRGB: [u8; 3] = [96, 112, 58];
pub(super) const FOREST_BOREAL_SRGB: [u8; 3] = [38, 54, 36];
pub(super) const FOREST_TEMPERATE_SRGB: [u8; 3] = [50, 70, 38];
pub(super) const FOREST_TROPICAL_SRGB: [u8; 3] = [32, 60, 28];
pub(super) const SAND_BARE_SRGB: [u8; 3] = [204, 172, 124];
pub(super) const RED_BROWN_SRGB: [u8; 3] = [170, 118, 82];
pub(super) const GREY_ROCK_SRGB: [u8; 3] = [120, 112, 104];
pub(super) const OCEAN_SHELF_SRGB: [u8; 3] = [38, 96, 116];
pub(super) const REEF_SRGB: [u8; 3] = [52, 128, 124];
pub(super) const OCEAN_MID_SRGB: [u8; 3] = [14, 50, 88];
pub(super) const OCEAN_DEEP_SRGB: [u8; 3] = [8, 24, 58];
pub(super) const LAKE_SHALLOW_SRGB: [u8; 3] = [40, 78, 84];
pub(super) const LAKE_DEEP_SRGB: [u8; 3] = [18, 42, 58];
pub(super) const SALT_LAKE_SRGB: [u8; 3] = [128, 156, 146];
pub(super) const SALT_CRUST_SRGB: [u8; 3] = [226, 220, 206];
pub(super) const HAZE_SRGB: [u8; 3] = [186, 206, 220];
/// Low grey-green ground above the summer tree line.
/// Cold open ground above or beyond the tree line.
pub(super) const TUNDRA_SRGB: [u8; 3] = [112, 106, 80];
pub(super) const ALPINE_SRGB: [u8; 3] = [108, 114, 92];

pub(super) fn land_albedo_srgb(biome: Biome) -> Option<[u8; 3]> {
    LAND_SRGB
        .iter()
        .find(|(id, _)| *id == biome)
        .map(|(_, rgb)| *rgb)
}

pub(super) fn is_ice_biome(biome: Biome) -> bool {
    matches!(biome, Biome::IceSheet | Biome::Glacier)
}

pub(super) fn srgb_channel_to_linear(channel: u8) -> f64 {
    let u = f64::from(channel) / 255.0;
    if u <= 0.04045 {
        u / 12.92
    } else {
        ((u + 0.055) / 1.055).powf(2.4)
    }
}

pub(super) fn linear_channel_to_srgb(value: f64) -> u8 {
    let u = value.clamp(0.0, 1.0);
    let encoded = if u <= 0.003_130_8 {
        12.92 * u
    } else {
        1.055 * u.powf(1.0 / 2.4) - 0.055
    };
    (encoded * 255.0).round().clamp(0.0, 255.0) as u8
}

pub(super) fn srgb_to_linear(rgb: [u8; 3]) -> [f64; 3] {
    [
        srgb_channel_to_linear(rgb[0]),
        srgb_channel_to_linear(rgb[1]),
        srgb_channel_to_linear(rgb[2]),
    ]
}

pub(super) fn linear_to_srgb(rgb: [f64; 3]) -> [u8; 3] {
    [
        linear_channel_to_srgb(rgb[0]),
        linear_channel_to_srgb(rgb[1]),
        linear_channel_to_srgb(rgb[2]),
    ]
}

/// Vegetation fraction and forest share a biome stands for.
///
/// The climate fields (`P/PET`, temperature) only nudge these. On their own
/// they disagree with the engine's biome classifier, and a forest biome would
/// paint as bare ground.
pub(super) fn vegetation_targets(id: u8) -> (f64, f64) {
    match Biome::from_u8(id) {
        Some(Biome::TropicalRainforest | Biome::TemperateRainforest) => (1.0, 1.0),
        Some(Biome::TropicalSeasonalForest) => (0.95, 0.8),
        Some(Biome::TemperateForest) => (0.95, 0.85),
        Some(Biome::BorealForest) => (0.9, 0.85),
        Some(Biome::Mangrove) => (1.0, 0.9),
        Some(Biome::Wetland) => (0.95, 0.3),
        Some(Biome::Grassland) => (0.85, 0.08),
        Some(Biome::Mediterranean) => (0.65, 0.35),
        Some(Biome::Savanna) => (0.6, 0.15),
        Some(Biome::ColdSteppe) => (0.5, 0.0),
        Some(Biome::Tundra) => (0.55, 0.0),
        Some(Biome::Alpine) => (0.35, 0.0),
        Some(Biome::ColdDesert) => (0.12, 0.0),
        Some(Biome::HotDesert) => (0.05, 0.0),
        _ => (0.0, 0.0),
    }
}

pub(super) fn biome_linear(id: u8) -> [f64; 3] {
    let biome = Biome::from_u8(id);
    let srgb = biome.and_then(land_albedo_srgb).unwrap_or(ROCK_SRGB);
    srgb_to_linear(srgb)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn palette_entries() -> impl Iterator<Item = [u8; 3]> {
        LAND_SRGB.into_iter().map(|(_, rgb)| rgb).chain([
            ICE_SRGB,
            ROCK_SRGB,
            SNOW_SRGB,
            BEACH_SRGB,
            GRASS_DRY_SRGB,
            GRASS_LUSH_SRGB,
            FOREST_BOREAL_SRGB,
            FOREST_TEMPERATE_SRGB,
            FOREST_TROPICAL_SRGB,
            SAND_BARE_SRGB,
            RED_BROWN_SRGB,
            GREY_ROCK_SRGB,
            OCEAN_SHELF_SRGB,
            REEF_SRGB,
            OCEAN_MID_SRGB,
            OCEAN_DEEP_SRGB,
            LAKE_SHALLOW_SRGB,
            LAKE_DEEP_SRGB,
            SALT_LAKE_SRGB,
            SALT_CRUST_SRGB,
            HAZE_SRGB,
            ALPINE_SRGB,
        ])
    }

    #[test]
    fn every_land_biome_round_trips_through_linear_srgb() {
        for biome in Biome::ALL {
            let Some(rgb) = land_albedo_srgb(biome) else {
                continue;
            };
            let back = linear_to_srgb(srgb_to_linear(rgb));
            for channel in 0..3 {
                let delta = i32::from(back[channel]) - i32::from(rgb[channel]);
                assert!(
                    delta.abs() <= 1,
                    "{biome:?} channel {channel} {rgb:?} -> {back:?}"
                );
            }
        }
        let land: Vec<_> = Biome::ALL
            .into_iter()
            .filter(|biome| land_albedo_srgb(*biome).is_some())
            .collect();
        assert_eq!(land.len(), 18, "every land biome id needs an albedo");
        for rgb in palette_entries() {
            let back = linear_to_srgb(srgb_to_linear(rgb));
            for channel in 0..3 {
                let delta = i32::from(back[channel]) - i32::from(rgb[channel]);
                assert!(delta.abs() <= 1, "{rgb:?} -> {back:?}");
            }
        }
    }
}
