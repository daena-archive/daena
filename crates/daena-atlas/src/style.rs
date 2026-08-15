//! Locked built-in spike palette. Declarative styles arrive in iteration 1.

pub const OCEAN_DEEP: [u8; 3] = [12, 42, 78];
pub const OCEAN_SHALLOW: [u8; 3] = [46, 110, 148];
pub const LAND_LOW: [u8; 3] = [126, 158, 92];
pub const LAND_HIGH: [u8; 3] = [196, 176, 132];
pub const LAND_PEAK: [u8; 3] = [236, 236, 228];
pub const LAKE: [u8; 3] = [54, 108, 148];
pub const ICE: [u8; 3] = [236, 244, 252];
pub const BACKGROUND: [u8; 4] = [0, 0, 0, 255];

pub fn mix_rgb(from: [u8; 3], to: [u8; 3], t_ppm: u32) -> [u8; 3] {
    let t = u32::from(t_ppm.min(1_000_000));
    let inv = 1_000_000 - t;
    [
        ((u32::from(from[0]) * inv + u32::from(to[0]) * t) / 1_000_000) as u8,
        ((u32::from(from[1]) * inv + u32::from(to[1]) * t) / 1_000_000) as u8,
        ((u32::from(from[2]) * inv + u32::from(to[2]) * t) / 1_000_000) as u8,
    ]
}

pub fn apply_shade(rgb: [u8; 3], shade_ppm: u32) -> [u8; 4] {
    let shade = shade_ppm.clamp(350_000, 1_000_000);
    [
        ((u32::from(rgb[0]) * shade) / 1_000_000) as u8,
        ((u32::from(rgb[1]) * shade) / 1_000_000) as u8,
        ((u32::from(rgb[2]) * shade) / 1_000_000) as u8,
        255,
    ]
}

pub fn hypsometric(elevation_mm: i32, sea_level_mm: i32) -> [u8; 3] {
    let relative = elevation_mm.saturating_sub(sea_level_mm);
    if relative < 0 {
        let depth = relative.unsigned_abs().min(8_000_000);
        let t = (u64::from(depth) * 1_000_000 / 8_000_000) as u32;
        mix_rgb(OCEAN_SHALLOW, OCEAN_DEEP, t)
    } else {
        let height = relative.unsigned_abs().min(6_000_000);
        if height < 1_500_000 {
            mix_rgb(
                LAND_LOW,
                LAND_HIGH,
                (u64::from(height) * 1_000_000 / 1_500_000) as u32,
            )
        } else {
            mix_rgb(
                LAND_HIGH,
                LAND_PEAK,
                (u64::from(height - 1_500_000) * 1_000_000 / 4_500_000) as u32,
            )
        }
    }
}
