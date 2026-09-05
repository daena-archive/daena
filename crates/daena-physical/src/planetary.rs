//! Durable planetary and orbital configuration for a physical world.
//!
//! These values describe the planet's relationship with its star. They are
//! authored world configuration, not a disposable climate cache. Downstream
//! climate derivation consumes them for solar-driven temperature, solstice
//! contrast, and rotation-aware wind circulation. Existing worlds without this
//! block use the Earth-like default.
//! Terrain identity does not hash this block; derived-cache directories
//! include a planetary token.

use super::{PhysicalError, PhysicalErrorCode, DEFAULT_RADIUS_METRES};
use serde::{Deserialize, Serialize};

pub const PLANETARY_CONFIG_VERSION: u16 = 1;
pub const SOLAR_LUMINOSITY_PPM: u32 = 1_000_000;
pub const SOLAR_MASS_PPM: u32 = 1_000_000;
pub const ASTRONOMICAL_UNIT_MILLI: u32 = 1_000_000;
pub const EARTH_ECCENTRICITY_PPM: u32 = 16_710;
pub const EARTH_AXIAL_TILT_MILLI_DEG: u32 = 23_440;
pub const EARTH_ROTATION_PERIOD_SECONDS: u32 = 86_400;
pub const EARTH_RETAINED_HEAT_CENTI_C: i32 = 1_400;
pub const EARTH_BOND_ALBEDO_PPM: u32 = 306_000;
pub const EARTH_MEAN_DENSITY_KG_M3: u32 = 5_514;
const AU_METRES: f64 = 149_597_870_700.0;
const GRAVITATIONAL_CONSTANT: f64 = 6.674_30e-11;
const SOLAR_MASS_KG: f64 = 1.988_416e30;
const EARTH_GRAVITY_MS2: f64 = 9.806_65;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum PlanetaryPreset {
    #[default]
    EarthLike,
    LowTilt,
    HighTilt,
    SlowRotating,
    CloseOrbit,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlanetaryConfiguration {
    pub version: u16,
    pub preset: PlanetaryPreset,
    pub star_luminosity_ppm: u32,
    pub star_mass_ppm: u32,
    pub semi_major_axis_milli_au: u32,
    pub eccentricity_ppm: u32,
    pub axial_tilt_milli_deg: u32,
    pub rotation_period_seconds: u32,
    pub retained_heat_centi_c: i32,
    pub bond_albedo_ppm: u32,
    pub mean_density_kg_m3: u32,
    #[serde(default = "default_radius_metres")]
    pub radius_metres: u64,
}

fn default_radius_metres() -> u64 {
    DEFAULT_RADIUS_METRES
}

impl Default for PlanetaryConfiguration {
    fn default() -> Self {
        Self::earth_like()
    }
}

impl PlanetaryConfiguration {
    pub fn earth_like() -> Self {
        Self {
            version: PLANETARY_CONFIG_VERSION,
            preset: PlanetaryPreset::EarthLike,
            star_luminosity_ppm: SOLAR_LUMINOSITY_PPM,
            star_mass_ppm: SOLAR_MASS_PPM,
            semi_major_axis_milli_au: ASTRONOMICAL_UNIT_MILLI,
            eccentricity_ppm: EARTH_ECCENTRICITY_PPM,
            axial_tilt_milli_deg: EARTH_AXIAL_TILT_MILLI_DEG,
            rotation_period_seconds: EARTH_ROTATION_PERIOD_SECONDS,
            retained_heat_centi_c: EARTH_RETAINED_HEAT_CENTI_C,
            bond_albedo_ppm: EARTH_BOND_ALBEDO_PPM,
            mean_density_kg_m3: EARTH_MEAN_DENSITY_KG_M3,
            radius_metres: DEFAULT_RADIUS_METRES,
        }
    }

    pub fn from_preset(preset: PlanetaryPreset) -> Self {
        let mut configuration = Self::earth_like();
        configuration.preset = preset;
        match preset {
            PlanetaryPreset::EarthLike | PlanetaryPreset::Custom => {}
            PlanetaryPreset::LowTilt => configuration.axial_tilt_milli_deg = 5_000,
            PlanetaryPreset::HighTilt => configuration.axial_tilt_milli_deg = 40_000,
            PlanetaryPreset::SlowRotating => configuration.rotation_period_seconds = 259_200,
            PlanetaryPreset::CloseOrbit => configuration.semi_major_axis_milli_au = 700_000,
        }
        configuration
    }

    pub fn validate(self) -> Result<(), PhysicalError> {
        if self.version != PLANETARY_CONFIG_VERSION
            || !(10_000..=100_000_000).contains(&self.star_luminosity_ppm)
            || !(80_000..=8_000_000).contains(&self.star_mass_ppm)
            || !(50_000..=50_000_000).contains(&self.semi_major_axis_milli_au)
            || self.eccentricity_ppm > 800_000
            || self.axial_tilt_milli_deg > 90_000
            || !(3_600..=7_776_000).contains(&self.rotation_period_seconds)
            || !(-5_000..=5_000).contains(&self.retained_heat_centi_c)
            || !(50_000..=800_000).contains(&self.bond_albedo_ppm)
            || !(1_000..=12_000).contains(&self.mean_density_kg_m3)
            || self.radius_metres == 0
        {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::GeneratorInvalidSettings,
                "planetary configuration is outside the bounded range",
            ));
        }
        Ok(())
    }

    pub fn insolation_ppm(self) -> Result<u32, PhysicalError> {
        self.validate()?;
        let au = f64::from(self.semi_major_axis_milli_au) / f64::from(ASTRONOMICAL_UNIT_MILLI);
        let insolation = f64::from(self.star_luminosity_ppm) / (au * au);
        if !insolation.is_finite() {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::NumericNonFinite,
                "stellar insolation is not finite",
            ));
        }
        Ok(insolation.round().clamp(1.0, 4_000_000_000.0) as u32)
    }

    pub fn orbital_period_seconds(self) -> Result<u64, PhysicalError> {
        self.validate()?;
        let mass_kg = SOLAR_MASS_KG * f64::from(self.star_mass_ppm) / f64::from(SOLAR_MASS_PPM);
        let axis_metres = AU_METRES * f64::from(self.semi_major_axis_milli_au)
            / f64::from(ASTRONOMICAL_UNIT_MILLI);
        let seconds = std::f64::consts::TAU
            * (axis_metres.powi(3) / (GRAVITATIONAL_CONSTANT * mass_kg)).sqrt();
        if !seconds.is_finite() || seconds < 1.0 {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::NumericNonFinite,
                "orbital period is not finite",
            ));
        }
        Ok(seconds.round().clamp(1.0, u64::MAX as f64) as u64)
    }

    pub fn with_orbital_period_seconds(mut self, seconds: u64) -> Result<Self, PhysicalError> {
        if seconds == 0 {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::GeneratorInvalidSettings,
                "orbital period must be positive",
            ));
        }
        let mass_kg = SOLAR_MASS_KG * f64::from(self.star_mass_ppm) / f64::from(SOLAR_MASS_PPM);
        let axis_metres =
            ((seconds as f64 / std::f64::consts::TAU).powi(2) * GRAVITATIONAL_CONSTANT * mass_kg)
                .cbrt();
        if !axis_metres.is_finite() {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::NumericNonFinite,
                "orbital distance is not finite",
            ));
        }
        let milli_au = axis_metres / AU_METRES * f64::from(ASTRONOMICAL_UNIT_MILLI);
        if !milli_au.is_finite() {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::NumericNonFinite,
                "orbital distance is not finite",
            ));
        }
        self.semi_major_axis_milli_au = milli_au.round().clamp(1.0, u32::MAX as f64) as u32;
        self.preset = PlanetaryPreset::Custom;
        self.validate()?;
        Ok(self)
    }

    pub fn surface_gravity_milli_g(self) -> Result<u32, PhysicalError> {
        self.validate()?;
        let gravity = (4.0 / 3.0)
            * std::f64::consts::PI
            * GRAVITATIONAL_CONSTANT
            * f64::from(self.mean_density_kg_m3)
            * self.radius_metres as f64;
        let milli_g = gravity / EARTH_GRAVITY_MS2 * 1_000.0;
        if !milli_g.is_finite() {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::NumericNonFinite,
                "surface gravity is not finite",
            ));
        }
        Ok(milli_g.round().clamp(1.0, 100_000.0) as u32)
    }

    pub fn climate_cache_token(self) -> String {
        let mut hash = 0xcbf2_9ce4_8422_2325;
        for value in [
            u64::from(self.star_luminosity_ppm),
            u64::from(self.semi_major_axis_milli_au),
            u64::from(self.eccentricity_ppm),
            u64::from(self.axial_tilt_milli_deg),
            u64::from(self.rotation_period_seconds),
            self.retained_heat_centi_c as u64,
            u64::from(self.bond_albedo_ppm),
        ] {
            hash ^= value;
            hash = hash.wrapping_mul(0x1000_0000_01b3);
        }
        format!("p{hash:016x}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn earth_like_defaults_are_valid_and_stable() {
        let earth = PlanetaryConfiguration::earth_like();
        earth.validate().unwrap();
        assert_eq!(earth, PlanetaryConfiguration::default());
        assert_eq!(earth.insolation_ppm().unwrap(), SOLAR_LUMINOSITY_PPM);
        let year = earth.orbital_period_seconds().unwrap();
        assert!(
            (31_000_000..32_000_000).contains(&year),
            "earth-like year was {year}s"
        );
        let gravity = earth.surface_gravity_milli_g().unwrap();
        assert!(
            (980..=1_020).contains(&gravity),
            "earth-like gravity was {gravity} milli-g"
        );
        let mut denser = earth;
        denser.mean_density_kg_m3 = 8_000;
        denser.star_mass_ppm = 2_000_000;
        assert_eq!(earth.climate_cache_token(), denser.climate_cache_token());
        denser.rotation_period_seconds = 200_000;
        assert_ne!(earth.climate_cache_token(), denser.climate_cache_token());
        denser.star_luminosity_ppm = 2_000_000;
        assert_ne!(earth.climate_cache_token(), denser.climate_cache_token());
    }

    #[test]
    fn named_presets_stay_within_bounds() {
        for preset in [
            PlanetaryPreset::EarthLike,
            PlanetaryPreset::LowTilt,
            PlanetaryPreset::HighTilt,
            PlanetaryPreset::SlowRotating,
            PlanetaryPreset::CloseOrbit,
            PlanetaryPreset::Custom,
        ] {
            PlanetaryConfiguration::from_preset(preset)
                .validate()
                .unwrap();
        }
        let close = PlanetaryConfiguration::from_preset(PlanetaryPreset::CloseOrbit);
        assert!(close.insolation_ppm().unwrap() > SOLAR_LUMINOSITY_PPM);
        assert!(
            close.orbital_period_seconds().unwrap()
                < PlanetaryConfiguration::earth_like()
                    .orbital_period_seconds()
                    .unwrap()
        );
    }

    #[test]
    fn insolation_follows_luminosity_and_distance() {
        let mut brighter = PlanetaryConfiguration::earth_like();
        brighter.preset = PlanetaryPreset::Custom;
        brighter.star_luminosity_ppm = SOLAR_LUMINOSITY_PPM * 2;
        assert_eq!(brighter.insolation_ppm().unwrap(), SOLAR_LUMINOSITY_PPM * 2);

        let mut closer = PlanetaryConfiguration::earth_like();
        closer.preset = PlanetaryPreset::Custom;
        closer.semi_major_axis_milli_au = ASTRONOMICAL_UNIT_MILLI / 2;
        assert_eq!(closer.insolation_ppm().unwrap(), SOLAR_LUMINOSITY_PPM * 4);
    }

    #[test]
    fn out_of_range_values_are_rejected() {
        let mut invalid = PlanetaryConfiguration::earth_like();
        invalid.version = 99;
        assert!(invalid.validate().is_err());
        invalid = PlanetaryConfiguration::earth_like();
        invalid.eccentricity_ppm = 900_000;
        assert!(invalid.validate().is_err());
        invalid = PlanetaryConfiguration::earth_like();
        invalid.axial_tilt_milli_deg = 91_000;
        assert!(invalid.validate().is_err());
        invalid = PlanetaryConfiguration::earth_like();
        invalid.star_mass_ppm = 1;
        assert!(invalid.validate().is_err());
        invalid = PlanetaryConfiguration::earth_like();
        invalid.radius_metres = 0;
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn year_length_sets_distance_from_star_mass() {
        let earth = PlanetaryConfiguration::earth_like();
        let year = earth.orbital_period_seconds().unwrap();
        let restored = earth.with_orbital_period_seconds(year).unwrap();
        assert!(
            (i64::from(restored.semi_major_axis_milli_au) - i64::from(ASTRONOMICAL_UNIT_MILLI))
                .abs()
                < 2_000
        );
        let longer = earth
            .with_orbital_period_seconds(year.saturating_mul(2))
            .unwrap();
        assert!(longer.semi_major_axis_milli_au > earth.semi_major_axis_milli_au);
        assert_eq!(longer.preset, PlanetaryPreset::Custom);
    }
}
