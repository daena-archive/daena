//! Star, orbit and the knobs that shift the energy-balance climate.
//!
//! Derived quantities use a 365.25-day year scaled by Kepler's law. The solar
//! constant is the Earth value `1361 W/m²` at 1 AU and one solar luminosity.

use crate::WorldError;

/// Orbit and radiative settings. Validated with [`crate::WorldSettings`].
#[derive(Clone, Copy, Debug)]
pub struct PlanetSettings {
    /// Star luminosity in solar units.
    pub star_luminosity: f64,
    /// Star mass in solar units.
    pub star_mass: f64,
    /// Semi-major axis in astronomical units.
    pub orbit_au: f64,
    /// Orbital eccentricity.
    pub eccentricity: f64,
    /// Longitude of perihelion, degrees from the northern-hemisphere spring equinox.
    pub perihelion_deg: f64,
    /// Obliquity in degrees.
    pub axial_tilt_deg: f64,
    /// Sidereal day length in hours.
    pub rotation_hours: f64,
    /// Extra trapped longwave, W/m². Positive warms.
    pub greenhouse_wm2: f64,
    /// Artistic temperature offset, °C. Positive warms.
    pub temperature_offset_c: f64,
    /// Multiplier on evaporative supply.
    pub precipitation_scale: f64,
}

impl Default for PlanetSettings {
    fn default() -> Self {
        Self {
            star_luminosity: 1.0,
            star_mass: 1.0,
            orbit_au: 1.0,
            eccentricity: 0.0167,
            perihelion_deg: 283.0,
            axial_tilt_deg: 23.44,
            rotation_hours: 24.0,
            greenhouse_wm2: 0.0,
            temperature_offset_c: 0.0,
            precipitation_scale: 1.0,
        }
    }
}

impl PlanetSettings {
    /// Reject values outside the phase B1 ranges.
    pub fn validate(&self) -> Result<(), WorldError> {
        check_range("star_luminosity", self.star_luminosity, 0.05, 20.0, true)?;
        check_range("star_mass", self.star_mass, 0.1, 5.0, true)?;
        check_range("orbit_au", self.orbit_au, 0.1, 20.0, true)?;
        check_range("eccentricity", self.eccentricity, 0.0, 0.6, true)?;
        if !self.perihelion_deg.is_finite()
            || self.perihelion_deg < 0.0
            || self.perihelion_deg >= 360.0
        {
            return Err(WorldError::InvalidSettings(format!(
                "perihelion_deg must be in [0, 360), got {}",
                self.perihelion_deg
            )));
        }
        check_range("axial_tilt_deg", self.axial_tilt_deg, 0.0, 90.0, true)?;
        check_range("rotation_hours", self.rotation_hours, 2.0, 2400.0, true)?;
        check_range("greenhouse_wm2", self.greenhouse_wm2, -60.0, 120.0, true)?;
        check_range(
            "temperature_offset_c",
            self.temperature_offset_c,
            -40.0,
            40.0,
            true,
        )?;
        check_range(
            "precipitation_scale",
            self.precipitation_scale,
            0.05,
            4.0,
            true,
        )?;
        Ok(())
    }

    /// Top-of-atmosphere flux at the mean orbital distance, W/m².
    pub fn solar_constant(&self) -> f64 {
        1361.0 * self.star_luminosity / (self.orbit_au * self.orbit_au)
    }

    /// Sidereal year in seconds, `3.15576e7 · a^1.5 / sqrt(M)`.
    pub fn year_seconds(&self) -> f64 {
        3.15576e7 * self.orbit_au.powf(1.5) / self.star_mass.sqrt()
    }

    /// Calendar month in seconds. Twelve months make one year.
    pub fn month_seconds(&self) -> f64 {
        self.year_seconds() / 12.0
    }
}

fn check_range(
    name: &str,
    value: f64,
    lo: f64,
    hi: f64,
    inclusive: bool,
) -> Result<(), WorldError> {
    let ok = value.is_finite()
        && if inclusive {
            (lo..=hi).contains(&value)
        } else {
            value >= lo && value < hi
        };
    if ok {
        Ok(())
    } else {
        Err(WorldError::InvalidSettings(format!(
            "{name} must be in [{lo}, {hi}], got {value}"
        )))
    }
}
