//! Land, lake and ocean biomes from climate, ice and the drainage network.
//!
//! Classes are stable `u8` values. Ocean is classified first, then a lake cell,
//! then the land rules in the order below. Wetland and mangrove replace a
//! living land class; ice is left as ice.

use rayon::prelude::*;

use crate::grid::{cross, dot3, CubeSphere, NONE};
use crate::hydrology::{Hydrology, LakeKind};
use crate::{ClimateSummary, WorldError};

/// Biome class. The discriminant is the stored id and the colour key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Biome {
    DeepOcean = 0,
    Shelf = 1,
    Trench = 2,
    Reef = 3,
    SeaIceSeasonal = 4,
    SeaIcePerennial = 5,
    FreshLake = 6,
    SaltLake = 7,
    SaltFlat = 8,
    IceSheet = 9,
    Glacier = 10,
    Tundra = 11,
    Alpine = 12,
    BorealForest = 13,
    ColdSteppe = 14,
    ColdDesert = 15,
    TemperateRainforest = 16,
    TemperateForest = 17,
    Mediterranean = 18,
    Grassland = 19,
    HotDesert = 20,
    Savanna = 21,
    TropicalSeasonalForest = 22,
    TropicalRainforest = 23,
    Wetland = 24,
    Mangrove = 25,
}

impl Biome {
    pub fn from_u8(value: u8) -> Option<Self> {
        Some(match value {
            0 => Self::DeepOcean,
            1 => Self::Shelf,
            2 => Self::Trench,
            3 => Self::Reef,
            4 => Self::SeaIceSeasonal,
            5 => Self::SeaIcePerennial,
            6 => Self::FreshLake,
            7 => Self::SaltLake,
            8 => Self::SaltFlat,
            9 => Self::IceSheet,
            10 => Self::Glacier,
            11 => Self::Tundra,
            12 => Self::Alpine,
            13 => Self::BorealForest,
            14 => Self::ColdSteppe,
            15 => Self::ColdDesert,
            16 => Self::TemperateRainforest,
            17 => Self::TemperateForest,
            18 => Self::Mediterranean,
            19 => Self::Grassland,
            20 => Self::HotDesert,
            21 => Self::Savanna,
            22 => Self::TropicalSeasonalForest,
            23 => Self::TropicalRainforest,
            24 => Self::Wetland,
            25 => Self::Mangrove,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::DeepOcean => "deep_ocean",
            Self::Shelf => "shelf",
            Self::Trench => "trench",
            Self::Reef => "reef",
            Self::SeaIceSeasonal => "sea_ice_seasonal",
            Self::SeaIcePerennial => "sea_ice_perennial",
            Self::FreshLake => "fresh_lake",
            Self::SaltLake => "salt_lake",
            Self::SaltFlat => "salt_flat",
            Self::IceSheet => "ice_sheet",
            Self::Glacier => "glacier",
            Self::Tundra => "tundra",
            Self::Alpine => "alpine",
            Self::BorealForest => "boreal_forest",
            Self::ColdSteppe => "cold_steppe",
            Self::ColdDesert => "cold_desert",
            Self::TemperateRainforest => "temperate_rainforest",
            Self::TemperateForest => "temperate_forest",
            Self::Mediterranean => "mediterranean",
            Self::Grassland => "grassland",
            Self::HotDesert => "hot_desert",
            Self::Savanna => "savanna",
            Self::TropicalSeasonalForest => "tropical_seasonal_forest",
            Self::TropicalRainforest => "tropical_rainforest",
            Self::Wetland => "wetland",
            Self::Mangrove => "mangrove",
        }
    }

    /// Flat preview colour.
    pub fn color(self) -> [u8; 3] {
        match self {
            Self::DeepOcean => [16, 48, 112],
            Self::Shelf => [64, 150, 176],
            Self::Trench => [8, 16, 64],
            Self::Reef => [40, 190, 180],
            Self::SeaIceSeasonal => [170, 200, 214],
            Self::SeaIcePerennial => [236, 244, 248],
            Self::FreshLake => [40, 96, 210],
            Self::SaltLake => [36, 150, 156],
            Self::SaltFlat => [232, 226, 206],
            Self::IceSheet => [248, 248, 255],
            Self::Glacier => [200, 220, 232],
            Self::Tundra => [150, 168, 140],
            Self::Alpine => [140, 120, 112],
            Self::BorealForest => [28, 92, 48],
            Self::ColdSteppe => [168, 156, 96],
            Self::ColdDesert => [196, 176, 140],
            Self::TemperateRainforest => [16, 96, 40],
            Self::TemperateForest => [46, 130, 52],
            Self::Mediterranean => [150, 148, 64],
            Self::Grassland => [170, 186, 72],
            Self::HotDesert => [214, 176, 96],
            Self::Savanna => [186, 164, 52],
            Self::TropicalSeasonalForest => [52, 140, 48],
            Self::TropicalRainforest => [12, 100, 36],
            Self::Wetland => [70, 130, 90],
            Self::Mangrove => [24, 100, 80],
        }
    }

    /// Satellite-like colour for the `natural` layer.
    pub fn natural(self) -> [u8; 3] {
        match self {
            Self::DeepOcean => [14, 42, 88],
            Self::Shelf => [40, 110, 140],
            Self::Trench => [6, 22, 58],
            Self::Reef => [48, 140, 130],
            Self::SeaIceSeasonal => [190, 210, 220],
            Self::SeaIcePerennial => [240, 246, 250],
            Self::FreshLake => [28, 78, 168],
            Self::SaltLake => [30, 120, 128],
            Self::SaltFlat => [226, 220, 200],
            Self::IceSheet => [244, 246, 250],
            Self::Glacier => [220, 228, 236],
            Self::Tundra => [128, 140, 118],
            Self::Alpine => [120, 108, 96],
            Self::BorealForest => [22, 72, 36],
            Self::ColdSteppe => [150, 140, 86],
            Self::ColdDesert => [176, 158, 122],
            Self::TemperateRainforest => [14, 78, 34],
            Self::TemperateForest => [36, 104, 42],
            Self::Mediterranean => [128, 132, 64],
            Self::Grassland => [150, 158, 70],
            Self::HotDesert => [196, 164, 102],
            Self::Savanna => [166, 148, 62],
            Self::TropicalSeasonalForest => [40, 112, 40],
            Self::TropicalRainforest => [16, 82, 32],
            Self::Wetland => [54, 108, 72],
            Self::Mangrove => [20, 84, 64],
        }
    }

    pub fn is_land_class(self) -> bool {
        !matches!(
            self,
            Self::DeepOcean
                | Self::Shelf
                | Self::Trench
                | Self::Reef
                | Self::SeaIceSeasonal
                | Self::SeaIcePerennial
        )
    }

    pub fn is_desert(self) -> bool {
        matches!(self, Self::HotDesert | Self::ColdDesert)
    }

    pub fn is_tropical_forest(self) -> bool {
        matches!(
            self,
            Self::TropicalRainforest | Self::TropicalSeasonalForest
        )
    }
}

/// One biome per cell.
pub(crate) fn classify(
    grid: &CubeSphere,
    elevation: &[f32],
    ocean: &[u8],
    climate: &ClimateSummary,
    hydro: &Hydrology,
) -> Result<Vec<u8>, WorldError> {
    let n = grid.cell_count();
    if elevation.len() != n
        || ocean.len() != n
        || climate.t_annual_c.len() != n
        || climate.t_warm_c.len() != n
        || climate.t_cold_c.len() != n
        || climate.p_annual_mm.len() != n
        || climate.p_dry_month_mm.len() != n
        || climate.pet_annual_mm.len() != n
        || climate.sea_ice_months.len() != n
        || hydro.lake_id.len() != n
        || hydro.discharge_m3s.len() != n
        || hydro.ice_cover.len() != n
    {
        return Err(WorldError::Internal(
            "biome inputs do not match the terrain grid".to_string(),
        ));
    }
    let slopes: Vec<f64> = (0..n)
        .into_par_iter()
        .map(|cell| slope(grid, elevation, cell))
        .collect();
    let biome = (0..n)
        .into_par_iter()
        .map(|cell| classify_cell(grid, elevation, ocean, climate, hydro, &slopes, cell) as u8)
        .collect();
    Ok(biome)
}

fn classify_cell(
    grid: &CubeSphere,
    elevation: &[f32],
    ocean: &[u8],
    climate: &ClimateSummary,
    hydro: &Hydrology,
    slopes: &[f64],
    cell: usize,
) -> Biome {
    let h = f64::from(elevation[cell]);
    if ocean[cell] == 1 {
        return ocean_biome(grid, h, climate, cell);
    }
    if let Some(kind) = lake_kind(hydro, cell) {
        return match kind {
            LakeKind::Fresh => Biome::FreshLake,
            LakeKind::Saline => Biome::SaltLake,
            LakeKind::SaltFlat => Biome::SaltFlat,
        };
    }
    let warm = f64::from(climate.t_warm_c[cell]);
    let cold = f64::from(climate.t_cold_c[cell]);
    let annual = f64::from(climate.t_annual_c[cell]);
    let precip = f64::from(climate.p_annual_mm[cell]).max(0.0);
    let dry = f64::from(climate.p_dry_month_mm[cell]).max(0.0);
    let pet = f64::from(climate.pet_annual_mm[cell]).max(0.0);
    let aridity = if pet <= 0.0 {
        f64::INFINITY
    } else {
        precip / pet
    };
    let lat = grid.lat_lon(cell as u32).0.to_degrees().abs();
    let ice = hydro.ice_cover[cell];
    let mut class = if ice == 2 || (ice != 1 && warm < 0.0) {
        Biome::IceSheet
    } else if ice == 1 {
        Biome::Glacier
    } else if warm < 10.0 {
        if h > 1500.0 && lat < 55.0 {
            Biome::Alpine
        } else {
            Biome::Tundra
        }
    } else if cold >= 18.0 {
        tropical(aridity, dry)
    } else if cold < -3.0 {
        boreal(aridity)
    } else {
        temperate(aridity, precip, dry, annual)
    };
    if !matches!(class, Biome::IceSheet | Biome::Glacier) {
        let wet_neighbour = beside_lake(grid, hydro, cell) || hydro.discharge_m3s[cell] >= 50.0;
        if slopes[cell] < 0.0005 && aridity >= 1.2 && wet_neighbour {
            class = Biome::Wetland;
        }
        if beside_ocean(grid, ocean, cell) && h < 15.0 && cold >= 20.0 && aridity >= 0.5 {
            class = Biome::Mangrove;
        }
    }
    class
}

fn ocean_biome(grid: &CubeSphere, h: f64, climate: &ClimateSummary, cell: usize) -> Biome {
    let months = climate.sea_ice_months[cell];
    if months >= 11 {
        return Biome::SeaIcePerennial;
    }
    if (4..=10).contains(&months) {
        return Biome::SeaIceSeasonal;
    }
    let lat = grid.lat_lon(cell as u32).0.to_degrees().abs();
    let cold = f64::from(climate.t_cold_c[cell]);
    if h > -60.0 && h <= 0.0 && cold >= 20.0 && lat <= 30.0 {
        return Biome::Reef;
    }
    if h < -6000.0 {
        return Biome::Trench;
    }
    if h > -200.0 {
        return Biome::Shelf;
    }
    Biome::DeepOcean
}

fn tropical(aridity: f64, dry: f64) -> Biome {
    if aridity >= 1.0 && dry >= 60.0 {
        Biome::TropicalRainforest
    } else if aridity >= 0.65 {
        Biome::TropicalSeasonalForest
    } else if aridity >= 0.25 {
        Biome::Savanna
    } else {
        Biome::HotDesert
    }
}

fn boreal(aridity: f64) -> Biome {
    if aridity >= 0.45 {
        Biome::BorealForest
    } else if aridity >= 0.2 {
        Biome::ColdSteppe
    } else {
        Biome::ColdDesert
    }
}

fn temperate(aridity: f64, precip: f64, dry: f64, annual: f64) -> Biome {
    if aridity >= 1.4 && precip >= 1800.0 {
        Biome::TemperateRainforest
    } else if aridity >= 0.65 {
        Biome::TemperateForest
    } else if aridity >= 0.35 {
        if dry < 0.15 * precip / 12.0 {
            Biome::Mediterranean
        } else {
            Biome::Grassland
        }
    } else if aridity >= 0.12 {
        Biome::Grassland
    } else if annual >= 18.0 {
        Biome::HotDesert
    } else {
        Biome::ColdDesert
    }
}

fn lake_kind(hydro: &Hydrology, cell: usize) -> Option<LakeKind> {
    let id = hydro.lake_id[cell];
    if id == NONE {
        return None;
    }
    hydro.lakes.get(id as usize).map(|lake| lake.kind)
}

fn beside_ocean(grid: &CubeSphere, ocean: &[u8], cell: usize) -> bool {
    grid.neighbors(cell as u32)
        .into_iter()
        .any(|nb| nb != NONE && ocean[nb as usize] == 1)
}

fn beside_lake(grid: &CubeSphere, hydro: &Hydrology, cell: usize) -> bool {
    grid.neighbors(cell as u32)
        .into_iter()
        .any(|nb| nb != NONE && hydro.lake_id[nb as usize] != NONE)
}

/// Magnitude of the least-squares tangent gradient of elevation, m/m.
fn slope(grid: &CubeSphere, elevation: &[f32], cell: usize) -> f64 {
    let pos = grid.position(cell as u32);
    let (east, north) = east_north(pos);
    let centre = f64::from(elevation[cell]);
    let mut a00 = 0.0;
    let mut a01 = 0.0;
    let mut a11 = 0.0;
    let mut b0 = 0.0;
    let mut b1 = 0.0;
    for (k, nb) in grid.neighbors(cell as u32).iter().enumerate() {
        if *nb == NONE {
            continue;
        }
        let dist = f64::from(grid.neighbor_distance(cell as u32, k));
        if dist <= 0.0 {
            continue;
        }
        let pk = grid.position(*nb);
        let along = [
            pk[0] - pos[0] * dot3(pk, pos),
            pk[1] - pos[1] * dot3(pk, pos),
            pk[2] - pos[2] * dot3(pk, pos),
        ];
        let dx = dot3(along, east);
        let dy = dot3(along, north);
        let horiz = (dx * dx + dy * dy).sqrt();
        if horiz < 1e-12 {
            continue;
        }
        let xm = dx / horiz * dist;
        let ym = dy / horiz * dist;
        let dh = f64::from(elevation[*nb as usize]) - centre;
        a00 += xm * xm;
        a01 += xm * ym;
        a11 += ym * ym;
        b0 += xm * dh;
        b1 += ym * dh;
    }
    let det = a00 * a11 - a01 * a01;
    if det.abs() < 1e-6 {
        return 0.0;
    }
    let gx = (b0 * a11 - a01 * b1) / det;
    let gy = (a00 * b1 - a01 * b0) / det;
    (gx * gx + gy * gy).sqrt()
}

fn east_north(up: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    let mut east = cross([0.0, 0.0, 1.0], up);
    let mut len = (east[0] * east[0] + east[1] * east[1] + east[2] * east[2]).sqrt();
    if len < 1e-8 {
        east = cross([0.0, 1.0, 0.0], up);
        len = (east[0] * east[0] + east[1] * east[1] + east[2] * east[2]).sqrt();
    }
    if len < 1e-12 {
        return ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
    }
    east = [east[0] / len, east[1] / len, east[2] / len];
    (east, cross(up, east))
}

pub(crate) fn hash_bits(biome: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in biome {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
