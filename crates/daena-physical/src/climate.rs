//! Deterministic current-climate and runoff derivation.
//!
//! Climate is a disposable interpretation of the accepted physical field. It
//! never changes the canonical elevation/source bytes. The model is purposely
//! bounded: an energy-balance annual temperature field with latitude insolation,
//! land/ocean/ice albedo, heat diffusion, diagnostic altitude lapse,
//! flux-form upwind heat advection; two solstice energy steps with carried T/q/V/W
//! so `C_ocean ≫ C_land` lags the ocean; `maritime_factor` stays a distance diagnostic;
//! a pressure-driven wind field with Coriolis and drag; seeded
//! wind meanders stay a capped product perturbation and do not source T; a wind-driven
//! surface-ocean current field whose mixed-layer tracer is imprinted onto product T;
//! moisture transport driven by those winds with sea-surface and current
//! evaporation; saturation-limited condensation rain with orographic `V·∇h`
//! cooling of `q_sat` and leftover convergence; land evaporation from an internal
//! surface-water store `W`; humidity as remaining moisture over local
//! saturation (`q / q_sat`); cloud fraction from RH and uplift that raises
//! albedo, reduces OLR, and increases condensation; six-class surface albedo
//! (ocean / vegetated / bare / desert / snow / ice) after moisture exists;
//! latent heating that re-relaxes T/V; aridity from
//! evaporative demand; precipitation that feeds runoff volumes
//! using exact spherical cell areas; land biome classes from those climate
//! conditions; and tropical-cyclone-like storm suitability, track corridors,
//! and intensity potential. Storm genesis consumes temperature, moisture,
//! convergence, thermal pressure gradient, Coriolis, land proximity, and
//! currents. Wind shear is the seasonal (solstice) wind-vector difference, not
//! vertical shear. Drought, heat-wave, extreme-rainfall, growing-season, and
//! dry/wet-season potentials are statistics of the Stage 6 seasonal fields,
//! not new prognostic state.

#![allow(clippy::needless_range_loop)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]

use super::{
    derive_subsystem_seed, splitmix64, Grid, PhysicalError, PhysicalErrorCode, PhysicalField,
    ProgressPhase, ProgressSink, SeedDomain,
};
use crate::planetary::{
    PlanetaryConfiguration, EARTH_BOND_ALBEDO_PPM, EARTH_ECCENTRICITY_PPM,
    EARTH_RETAINED_HEAT_CENTI_C, EARTH_ROTATION_PERIOD_SECONDS, SOLAR_LUMINOSITY_PPM,
};
use rayon::prelude::*;
use std::sync::OnceLock;

pub const CLIMATE_DERIVATION_VERSION: u16 = 11;
pub const BIOME_OCEAN: u32 = 0;
pub const BIOME_ICE: u32 = 1;
pub const BIOME_TUNDRA: u32 = 2;
pub const BIOME_ALPINE: u32 = 3;
pub const BIOME_COLD_GRASSLAND: u32 = 4;
pub const BIOME_TEMPERATE_GRASSLAND: u32 = 5;
pub const BIOME_DESERT: u32 = 6;
pub const BIOME_SHRUBLAND: u32 = 7;
pub const BIOME_TEMPERATE_FOREST: u32 = 8;
pub const BIOME_TROPICAL_FOREST: u32 = 9;
pub const BIOME_CLASS_MAX: u32 = BIOME_TROPICAL_FOREST;
const ALPINE_HEIGHT_MM: i32 = 1_500_000;
const TUNDRA_WARM_CENTI_C: i32 = 1_000;
const TROPICAL_ANNUAL_CENTI_C: i32 = 1_800;
const TROPICAL_COLD_CENTI_C: i32 = 1_000;
const FOREST_PRECIPITATION_MM: u32 = 500;
const GRASSLAND_PRECIPITATION_MM: u32 = 450;
const DESERT_PRECIPITATION_MM: u32 = 250;
const SNOW_COVER_MM: u32 = 80;
const DESERT_ARIDITY_PPM: u32 = 800_000;
const SHRUBLAND_ARIDITY_PPM: u32 = 500_000;
const GRASSLAND_ARIDITY_PPM: u32 = 200_000;
const COLD_GRASSLAND_ANNUAL_CENTI_C: i32 = 500;
const COLD_GRASSLAND_WINTER_CENTI_C: i32 = -1_000;
const FOREST_HUMIDITY_PPM: u32 = 550_000;
const MARITIME_HUMIDITY_PPM: u32 = 850_000;
const UNKNOWN_BIOME_FILL: [u8; 3] = [120, 120, 124];
const STORM_MIN_SST_CENTI_C: i32 = 1_200;
const STORM_FULL_SST_CENTI_C: i32 = 1_800;
const STORM_MIN_HUMIDITY_PPM: u32 = 176_000;
const STORM_FULL_HUMIDITY_PPM: u32 = 818_000;
const STORM_SHEAR_START_MILLI: u32 = 2_500;
const STORM_SHEAR_KILL_MILLI: u32 = 12_000;
const STORM_PGRAD_START_CENTI: u32 = 200;
const STORM_PGRAD_KILL_CENTI: u32 = 900;
const STORM_DIVERGENCE_START_PPM: u32 = 40_000;
const STORM_DIVERGENCE_KILL_PPM: u32 = 350_000;
const STORM_CONVERGENCE_FULL_PPM: u32 = 300_000;
const STORM_TRACK_START_PPM: u32 = 40_000;
const STORM_TRACK_STEPS: usize = 14;
const STORM_SEED_BLOCK: u32 = 6;
const STORM_PRONE_PPM: u32 = 100_000;
const STORM_CURRENT_STEER_PPM: i32 = 650_000;
const STORM_CLIMATE_YEAR_MILLI_AT_FULL: u32 = 80_000;
const EARTH_EQUATOR_BASE_CENTI_C: i32 = 2_200;
const SST_REFERENCE_CENTI_C: i32 = 1_400;
pub const CLIMATE_WIND_BAND_COUNT: u32 = 6;
pub const WIND_BAND_HADLEY: u32 = 0;
pub const WIND_BAND_FERREL: u32 = 1;
pub const WIND_BAND_POLAR: u32 = 2;
pub const MAX_WIND_MILLI: i32 = 10_000;
pub const MAX_CURRENT_MILLI: i32 = 4_000;
const WIND_GRADIENT_REF_METRES: f64 = 1_000_000.0;
const CURRENT_WIND_COUPLING: f64 = 0.30;
const CURRENT_GEOSTROPHY: f64 = 0.42;
const CURRENT_SVERDRUP: f64 = 0.22;
const MIN_CURRENT_BASIN_CELLS: usize = 8;
const CURRENT_WESTERN_RETURN: f64 = 2.35;
const CURRENT_WESTERN_SCALE: f64 = 2.1;
const CURRENT_SMOOTH_PASSES: usize = 2;
pub const CLIMATE_MAX_TRANSPORT_ITERATIONS: u32 = 512;
const CLIMATE_MIN_TRANSPORT_ITERATIONS: u32 = 8;
const CLIMATE_TRANSPORT_TOLERANCE_MM: f64 = 2.0;
const CLIMATE_TRANSPORT_RELAXATION: f64 = 0.70;
const MOISTURE_TEMPERATURE_RELAXATION: f64 = 0.40;
const COUPLING_T_TOLERANCE_C: f64 = 0.75;
const COUPLING_Q_TOLERANCE_MM: f64 = 120.0;
const PRECIPITATION_PATH_REF_METRES: f64 = 2_500_000.0;
const MAX_CLIMATE_PRECIPITATION_MM: u32 = 100_000;
const MAX_CLIMATE_MOISTURE_MM: u32 = 100_000;
const MAX_CLIMATE_TEMPERATURE_CENTI_C: i32 = 10_000;
const SOLAR_CONSTANT_WM2: f64 = 1_361.0;
const ENERGY_BALANCE_DT_SECONDS: f64 = 7_776_000.0;
const ENERGY_BALANCE_MIN_ITERATIONS: u32 = 8;
const CLIMATE_MIN_YEARS: u32 = 2;
const YEAR_T_TOLERANCE_C: f64 = 2.0;
const SEASON_SAMPLE_COUNT: f64 = 2.0;
const SEASONAL_INSOLATION_ANOMALY: f64 = 0.35;

const LATENT_HEAT_J_PER_KG: f64 = 2.5e6;
const CLIMATE_SECONDS_PER_YEAR: f64 = 31_557_600.0;
const SATURATION_MOISTURE_PER_HPA: f64 = 90.0;
const MAGNUS_T_MIN_C: f64 = -80.0;
const MAGNUS_T_MAX_C: f64 = 60.0;
const SATURATION_LUT_MIN_CENTI: i32 = -8_000;
const SATURATION_LUT_MAX_CENTI: i32 = 6_000;
const MAX_OROGRAPHIC_COOLING_C: f64 = 25.0;
const OROGRAPHIC_KU_PPM_PER_C: f64 = 1_000.0;
const GROWING_SEASON_CENTI_C: i32 = 500;
const CLOUD_UPLIFT_SCALE: f64 = 400.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HydrologyPreset {
    Arid,
    Balanced,
    Wet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClimateSettings {
    pub global_temperature_centi_c: i32,
    pub latitude_cooling_centi_c: i32,
    pub altitude_lapse_centi_c_per_km: i32,
    pub maritime_scale_km: u32,
    pub ocean_moisture_mm_per_year: u32,
    pub moisture_decay_ppm: u32,
    pub moisture_decay_scale_km: u32,
    pub convergence_ppm: u32,
    pub orographic_precipitation_ppm: u32,
    pub olr_a_milli_wm2: u32,
    pub olr_b_milli_wm2_per_c: u32,
    pub heat_diffusivity_e9_w_per_c: u32,
    pub c_land_kj_m2_k: u32,
    pub c_ocean_kj_m2_k: u32,
    pub albedo_ocean_ppm: u32,
    pub albedo_land_ppm: u32,
    pub albedo_bare_ppm: u32,
    pub albedo_desert_ppm: u32,
    pub albedo_snow_ppm: u32,
    pub albedo_ice_ppm: u32,
    pub albedo_cloud_ppm: u32,
    pub albedo_surface_ref_ppm: u32,
    pub cloud_olr_reduction_ppm: u32,
    pub cloud_rain_ppm: u32,
    pub cloud_albedo_coupling_ppm: u32,
    pub insolation_p2_ppm: u32,
    pub insolation_polar_floor_ppm: u32,
    pub energy_balance_max_iterations: u32,
    pub energy_balance_tolerance_milli_c: u32,
    pub pressure_cell_amplitude: u32,
    pub thermal_pressure_per_c: u32,
    pub pressure_scale_height_m: u32,
    pub coriolis_coupling_ppm: u32,
    pub drag_ocean_micro: u32,
    pub drag_land_micro: u32,
    pub heat_advection_kj_m2_k: u32,
    pub ocean_heat_coupling_milli_wm2_per_c: u32,
    pub ocean_heat_diffusivity_ppm: u32,
    pub ocean_heat_advection_ppm: u32,
    pub ocean_coast_blend_ppm: u32,
    pub storm_pressure_gradient_start_centi: u32,
    pub storm_pressure_gradient_kill_centi: u32,
    pub storm_divergence_start_ppm: u32,
    pub storm_divergence_kill_ppm: u32,
    pub storm_convergence_full_ppm: u32,
    pub temperature_wind_coupling_passes: u32,
    pub condensation_ppm: u32,
    pub latent_heat_coupling_ppm: u32,
    pub moisture_temperature_coupling_passes: u32,
    pub seasonal_year_max: u32,
    pub hydrology_preset: HydrologyPreset,
    pub planetary: PlanetaryConfiguration,
}

impl ClimateSettings {
    pub fn default_for(_grid: Grid) -> Self {
        Self {
            global_temperature_centi_c: 2_200,
            latitude_cooling_centi_c: 4_200,
            altitude_lapse_centi_c_per_km: 650,
            maritime_scale_km: 3_000,
            ocean_moisture_mm_per_year: 2_000,
            moisture_decay_ppm: 940_000,
            moisture_decay_scale_km: 8_000,
            convergence_ppm: 120_000,
            orographic_precipitation_ppm: 18_000_000,
            olr_a_milli_wm2: 214_000,
            olr_b_milli_wm2_per_c: 2_090,
            heat_diffusivity_e9_w_per_c: 14_000,
            c_land_kj_m2_k: 10_000,
            c_ocean_kj_m2_k: 200_000,
            albedo_ocean_ppm: 80_000,
            albedo_land_ppm: 200_000,
            albedo_bare_ppm: 230_000,
            albedo_desert_ppm: 260_000,
            albedo_snow_ppm: 480_000,
            albedo_ice_ppm: 400_000,
            albedo_cloud_ppm: 500_000,
            albedo_surface_ref_ppm: 150_000,
            cloud_olr_reduction_ppm: 80_000,
            cloud_rain_ppm: 120_000,
            cloud_albedo_coupling_ppm: 150_000,
            insolation_p2_ppm: 477_000,
            insolation_polar_floor_ppm: 200_000,
            energy_balance_max_iterations: 768,
            energy_balance_tolerance_milli_c: 80,
            pressure_cell_amplitude: 900,
            thermal_pressure_per_c: 80,
            pressure_scale_height_m: 8_000,
            coriolis_coupling_ppm: 1_000_000,
            drag_ocean_micro: 18,
            drag_land_micro: 45,
            heat_advection_kj_m2_k: 100,
            ocean_heat_coupling_milli_wm2_per_c: 12_000,
            ocean_heat_diffusivity_ppm: 80_000,
            ocean_heat_advection_ppm: 80_000,
            ocean_coast_blend_ppm: 380_000,
            storm_pressure_gradient_start_centi: STORM_PGRAD_START_CENTI,
            storm_pressure_gradient_kill_centi: STORM_PGRAD_KILL_CENTI,
            storm_divergence_start_ppm: STORM_DIVERGENCE_START_PPM,
            storm_divergence_kill_ppm: STORM_DIVERGENCE_KILL_PPM,
            storm_convergence_full_ppm: STORM_CONVERGENCE_FULL_PPM,
            temperature_wind_coupling_passes: 2,
            condensation_ppm: 400_000,
            latent_heat_coupling_ppm: 80_000,
            moisture_temperature_coupling_passes: 8,
            seasonal_year_max: 12,
            hydrology_preset: HydrologyPreset::Balanced,
            planetary: PlanetaryConfiguration::earth_like(),
        }
    }

    pub fn validate(self) -> Result<(), PhysicalError> {
        if !(-5_000..=5_000).contains(&self.global_temperature_centi_c)
            || !(0..=10_000).contains(&self.latitude_cooling_centi_c)
            || !(0..=2_000).contains(&self.altitude_lapse_centi_c_per_km)
            || !(1..=20_000).contains(&self.maritime_scale_km)
            || !(100..=100_000).contains(&self.moisture_decay_scale_km)
        {
            return Err(PhysicalError::InvalidSettings(
                "climate temperature parameters are outside the bounded range".into(),
            ));
        }
        if self.ocean_moisture_mm_per_year == 0
            || self.ocean_moisture_mm_per_year > MAX_CLIMATE_PRECIPITATION_MM
            || !(700_000..=999_000).contains(&self.moisture_decay_ppm)
            || self.convergence_ppm > 500_000
            || self.orographic_precipitation_ppm > 50_000_000
        {
            return Err(PhysicalError::InvalidSettings(
                "climate moisture parameters are outside the bounded range".into(),
            ));
        }
        if !(50_000..=400_000).contains(&self.olr_a_milli_wm2)
            || !(500..=8_000).contains(&self.olr_b_milli_wm2_per_c)
            || !(100..=100_000).contains(&self.heat_diffusivity_e9_w_per_c)
            || !(1_000..=1_000_000).contains(&self.c_land_kj_m2_k)
            || !(10_000..=10_000_000).contains(&self.c_ocean_kj_m2_k)
            || self.c_ocean_kj_m2_k < self.c_land_kj_m2_k
            || !(20_000..=800_000).contains(&self.albedo_ocean_ppm)
            || !(20_000..=800_000).contains(&self.albedo_land_ppm)
            || !(20_000..=800_000).contains(&self.albedo_bare_ppm)
            || !(20_000..=800_000).contains(&self.albedo_desert_ppm)
            || !(20_000..=800_000).contains(&self.albedo_snow_ppm)
            || !(20_000..=800_000).contains(&self.albedo_ice_ppm)
            || !(20_000..=800_000).contains(&self.albedo_cloud_ppm)
            || !(50_000..=400_000).contains(&self.albedo_surface_ref_ppm)
            || self.albedo_surface_ref_ppm >= 1_000_000
            || self.cloud_olr_reduction_ppm > 800_000
            || self.cloud_rain_ppm > 1_000_000
            || self.cloud_albedo_coupling_ppm > 1_000_000
            || self.insolation_p2_ppm > 1_000_000
            || self.insolation_polar_floor_ppm > 500_000
            || !(1..=4_096).contains(&self.energy_balance_max_iterations)
            || !(1..=5_000).contains(&self.energy_balance_tolerance_milli_c)
            || self.ocean_heat_coupling_milli_wm2_per_c > 200_000
            || self.ocean_heat_diffusivity_ppm > 1_000_000
            || self.ocean_heat_advection_ppm > 1_000_000
            || self.ocean_coast_blend_ppm > 1_000_000
        {
            return Err(PhysicalError::InvalidSettings(
                "climate energy-balance coefficients are outside the bounded range".into(),
            ));
        }
        if !(100..=50_000).contains(&self.pressure_cell_amplitude)
            || self.thermal_pressure_per_c > 2_000
            || !(1_000..=20_000).contains(&self.pressure_scale_height_m)
            || !(100_000..=2_000_000).contains(&self.coriolis_coupling_ppm)
            || !(1..=10_000).contains(&self.drag_ocean_micro)
            || !(1..=10_000).contains(&self.drag_land_micro)
            || self.heat_advection_kj_m2_k > 1_000_000
            || self.temperature_wind_coupling_passes > 16
            || self.condensation_ppm > 1_000_000
            || self.latent_heat_coupling_ppm > 1_000_000
            || !(1..=8).contains(&self.moisture_temperature_coupling_passes)
        {
            return Err(PhysicalError::InvalidSettings(
                "climate wind and advection coefficients are outside the bounded range".into(),
            ));
        }
        if self.storm_pressure_gradient_start_centi > self.storm_pressure_gradient_kill_centi
            || self.storm_pressure_gradient_kill_centi > 10_000
            || self.storm_divergence_start_ppm > self.storm_divergence_kill_ppm
            || self.storm_divergence_kill_ppm > 1_000_000
            || self.storm_convergence_full_ppm == 0
            || self.storm_convergence_full_ppm > 1_000_000
        {
            return Err(PhysicalError::InvalidSettings(
                "climate storm coupling coefficients are outside the bounded range".into(),
            ));
        }
        if !(1..=24).contains(&self.seasonal_year_max) {
            return Err(PhysicalError::InvalidSettings(
                "climate seasonal year cap is outside the bounded range".into(),
            ));
        }
        self.planetary.validate()?;
        Ok(())
    }

    fn olr_a_wm2(self) -> f64 {
        f64::from(self.olr_a_milli_wm2) / 1_000.0
    }

    fn olr_b_wm2_per_c(self) -> f64 {
        f64::from(self.olr_b_milli_wm2_per_c) / 1_000.0
    }

    fn heat_diffusivity_w_per_c(self) -> f64 {
        f64::from(self.heat_diffusivity_e9_w_per_c) * 1e9
    }

    fn heat_capacity_j_m2_k(self, ocean: bool) -> f64 {
        let kj = if ocean {
            self.c_ocean_kj_m2_k
        } else {
            self.c_land_kj_m2_k
        };
        f64::from(kj) * 1_000.0
    }

    fn energy_balance_tolerance_c(self) -> f64 {
        f64::from(self.energy_balance_tolerance_milli_c) / 1_000.0
    }

    fn drag_per_second(self, ocean: bool, elevation_km: f64) -> f64 {
        let base = if ocean {
            self.drag_ocean_micro
        } else {
            self.drag_land_micro
        };
        let mountain = if ocean {
            0.0
        } else {
            f64::from(self.drag_land_micro) * elevation_km * 0.5
        };
        (f64::from(base) + mountain) * 1e-6
    }

    fn coriolis_scale(self) -> f64 {
        f64::from(self.coriolis_coupling_ppm) / 1_000_000.0
    }

    fn heat_advection_j_m2_k(self) -> f64 {
        f64::from(self.heat_advection_kj_m2_k) * 1_000.0
    }

    fn ocean_heat_coupling_wm2_per_c(self) -> f64 {
        f64::from(self.ocean_heat_coupling_milli_wm2_per_c) / 1_000.0
    }

    fn ocean_heat_active(self) -> bool {
        self.ocean_heat_coupling_milli_wm2_per_c > 0
    }

    fn ocean_heat_diffusivity_w_per_c(self) -> f64 {
        self.heat_diffusivity_w_per_c() * f64::from(self.ocean_heat_diffusivity_ppm) / 1_000_000.0
    }

    fn ocean_heat_advection_j_m2_k(self) -> f64 {
        self.heat_capacity_j_m2_k(true) * f64::from(self.ocean_heat_advection_ppm) / 1_000_000.0
    }

    fn ocean_coast_ocean_blend(self) -> f64 {
        f64::from(self.ocean_coast_blend_ppm) / 1_000_000.0
    }

    fn cloud_olr_factor(self, cloud: f64) -> f64 {
        (1.0 - f64::from(self.cloud_olr_reduction_ppm) / 1_000_000.0 * cloud.clamp(0.0, 1.0))
            .clamp(0.2, 1.0)
    }

    fn cloud_rain_scale(self, cloud: f64) -> f64 {
        1.0 + f64::from(self.cloud_rain_ppm) / 1_000_000.0 * cloud.clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClimateMetrics {
    pub precipitation_volume_m3_per_year: u64,
    pub runoff_volume_m3_per_year: u64,
    pub mean_temperature_centi_c: i32,
    pub minimum_temperature_centi_c: i32,
    pub maximum_temperature_centi_c: i32,
    pub mean_precipitation_mm_per_year: u32,
    pub mean_runoff_mm_per_year: u32,
    pub wettest_cell_precipitation_mm_per_year: u32,
    pub driest_land_cell_precipitation_mm_per_year: u32,
    pub transport_iterations: u32,
    pub mean_seasonal_range_centi_c: u32,
    pub minimum_seasonal_temperature_centi_c: i32,
    pub maximum_seasonal_temperature_centi_c: i32,
    pub permanently_frozen_land_ppm: u32,
    pub seasonally_frozen_land_ppm: u32,
    pub mean_wind_speed_milli: u32,
    pub itcz_latitude_milli_deg: i32,
    pub easterly_cell_ppm: u32,
    pub converging_cell_ppm: u32,
    pub mean_current_speed_milli: u32,
    pub mean_humidity_ppm: u32,
    pub mean_land_aridity_ppm: u32,
    pub mean_seasonal_precipitation_range_mm: u32,
    pub dominant_land_biome: u32,
    pub mean_ocean_storm_suitability_ppm: u32,
    pub storm_prone_ocean_ppm: u32,
    pub mean_storm_intensity_ppm: u32,
    pub mean_land_storm_track_ppm: u32,
    pub expected_storms_per_year_milli: u32,
    pub mean_land_drought_potential_ppm: u32,
    pub mean_heat_wave_potential_ppm: u32,
    pub mean_extreme_rainfall_potential_ppm: u32,
    pub mean_land_growing_season_ppm: u32,
    pub mean_land_dry_season_ppm: u32,
    pub mean_land_wet_season_ppm: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClimateField {
    pub grid: Grid,
    pub derivation_version: u16,
    pub planetary: PlanetaryConfiguration,
    pub temperature_centi_c: Vec<i32>,
    pub temperature_nh_summer_centi_c: Vec<i32>,
    pub temperature_nh_winter_centi_c: Vec<i32>,
    pub moisture_mm_per_year: Vec<u32>,
    pub precipitation_mm_per_year: Vec<u32>,
    pub runoff_mm_per_year: Vec<u32>,
    pub runoff_volume_m3_per_year: Vec<u64>,
    pub maritime_factor_ppm: Vec<u32>,
    pub wind_east_milli: Vec<i32>,
    pub wind_north_milli: Vec<i32>,
    pub wind_east_nh_summer_milli: Vec<i32>,
    pub wind_north_nh_summer_milli: Vec<i32>,
    pub wind_east_nh_winter_milli: Vec<i32>,
    pub wind_north_nh_winter_milli: Vec<i32>,
    pub wind_divergence_ppm: Vec<i32>,
    pub wind_divergence_nh_summer_ppm: Vec<i32>,
    pub wind_divergence_nh_winter_ppm: Vec<i32>,
    pub wind_band: Vec<u32>,
    pub wind_band_nh_summer: Vec<u32>,
    pub wind_band_nh_winter: Vec<u32>,
    pub current_east_milli: Vec<i32>,
    pub current_north_milli: Vec<i32>,
    pub humidity_ppm: Vec<u32>,
    pub aridity_ppm: Vec<u32>,
    pub precipitation_nh_summer_mm: Vec<u32>,
    pub precipitation_nh_winter_mm: Vec<u32>,
    pub biome_class: Vec<u32>,
    pub storm_suitability_ppm: Vec<u32>,
    pub storm_track_ppm: Vec<u32>,
    pub storm_intensity_ppm: Vec<u32>,
    pub metrics: ClimateMetrics,
}

impl ClimateField {
    pub fn validate(&self) -> Result<(), PhysicalError> {
        let expected = self.grid.sample_count();
        if self.derivation_version != CLIMATE_DERIVATION_VERSION
            || self.temperature_centi_c.len() != expected
            || self.temperature_nh_summer_centi_c.len() != expected
            || self.temperature_nh_winter_centi_c.len() != expected
            || self.moisture_mm_per_year.len() != expected
            || self.precipitation_mm_per_year.len() != expected
            || self.runoff_mm_per_year.len() != expected
            || self.runoff_volume_m3_per_year.len() != expected
            || self.maritime_factor_ppm.len() != expected
            || self.wind_east_milli.len() != expected
            || self.wind_north_milli.len() != expected
            || self.wind_east_nh_summer_milli.len() != expected
            || self.wind_north_nh_summer_milli.len() != expected
            || self.wind_east_nh_winter_milli.len() != expected
            || self.wind_north_nh_winter_milli.len() != expected
            || self.wind_divergence_ppm.len() != expected
            || self.wind_divergence_nh_summer_ppm.len() != expected
            || self.wind_divergence_nh_winter_ppm.len() != expected
            || self.wind_band.len() != expected
            || self.wind_band_nh_summer.len() != expected
            || self.wind_band_nh_winter.len() != expected
            || self.current_east_milli.len() != expected
            || self.current_north_milli.len() != expected
            || self.humidity_ppm.len() != expected
            || self.aridity_ppm.len() != expected
            || self.precipitation_nh_summer_mm.len() != expected
            || self.precipitation_nh_winter_mm.len() != expected
            || self.biome_class.len() != expected
            || self.storm_suitability_ppm.len() != expected
            || self.storm_track_ppm.len() != expected
            || self.storm_intensity_ppm.len() != expected
        {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::NumericNonFinite,
                "climate field shape or derivation version is invalid",
            ));
        }
        if self
            .temperature_centi_c
            .iter()
            .chain(self.temperature_nh_summer_centi_c.iter())
            .chain(self.temperature_nh_winter_centi_c.iter())
            .any(|value| {
                !(-MAX_CLIMATE_TEMPERATURE_CENTI_C..=MAX_CLIMATE_TEMPERATURE_CENTI_C)
                    .contains(value)
            })
            || self
                .moisture_mm_per_year
                .iter()
                .chain(self.precipitation_mm_per_year.iter())
                .chain(self.runoff_mm_per_year.iter())
                .any(|value| *value > MAX_CLIMATE_PRECIPITATION_MM)
            || self
                .maritime_factor_ppm
                .iter()
                .any(|value| *value > 1_000_000)
            || self
                .wind_east_milli
                .iter()
                .chain(self.wind_north_milli.iter())
                .chain(self.wind_east_nh_summer_milli.iter())
                .chain(self.wind_north_nh_summer_milli.iter())
                .chain(self.wind_east_nh_winter_milli.iter())
                .chain(self.wind_north_nh_winter_milli.iter())
                .any(|value| !(-MAX_WIND_MILLI..=MAX_WIND_MILLI).contains(value))
            || self
                .wind_divergence_ppm
                .iter()
                .chain(self.wind_divergence_nh_summer_ppm.iter())
                .chain(self.wind_divergence_nh_winter_ppm.iter())
                .any(|value| !(-1_000_000..=1_000_000).contains(value))
            || self
                .wind_band
                .iter()
                .chain(self.wind_band_nh_summer.iter())
                .chain(self.wind_band_nh_winter.iter())
                .any(|value| *value > WIND_BAND_POLAR)
            || self
                .current_east_milli
                .iter()
                .chain(self.current_north_milli.iter())
                .any(|value| !(-MAX_CURRENT_MILLI..=MAX_CURRENT_MILLI).contains(value))
            || self
                .humidity_ppm
                .iter()
                .chain(self.aridity_ppm.iter())
                .any(|value| *value > 1_000_000)
            || self
                .precipitation_nh_summer_mm
                .iter()
                .chain(self.precipitation_nh_winter_mm.iter())
                .any(|value| *value > MAX_CLIMATE_PRECIPITATION_MM)
            || self
                .biome_class
                .iter()
                .any(|value| *value > BIOME_CLASS_MAX)
            || self
                .storm_suitability_ppm
                .iter()
                .chain(self.storm_track_ppm.iter())
                .chain(self.storm_intensity_ppm.iter())
                .any(|value| *value > 1_000_000)
        {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::NumericNonFinite,
                "climate field contains a non-finite or unbounded value",
            ));
        }
        Ok(())
    }

    pub fn with_global_temperature_offset(&self, offset_centi_c: i32) -> Self {
        if offset_centi_c == 0 {
            return self.clone();
        }
        let mut shifted = self.clone();
        for temperatures in [
            &mut shifted.temperature_centi_c,
            &mut shifted.temperature_nh_summer_centi_c,
            &mut shifted.temperature_nh_winter_centi_c,
        ] {
            for temperature in temperatures {
                *temperature = temperature.saturating_add(offset_centi_c).clamp(
                    -MAX_CLIMATE_TEMPERATURE_CENTI_C,
                    MAX_CLIMATE_TEMPERATURE_CENTI_C,
                );
            }
        }
        shifted.metrics.mean_temperature_centi_c = shifted
            .metrics
            .mean_temperature_centi_c
            .saturating_add(offset_centi_c)
            .clamp(
                -MAX_CLIMATE_TEMPERATURE_CENTI_C,
                MAX_CLIMATE_TEMPERATURE_CENTI_C,
            );
        shifted.metrics.minimum_temperature_centi_c = shifted
            .temperature_centi_c
            .iter()
            .copied()
            .min()
            .unwrap_or(shifted.metrics.minimum_temperature_centi_c);
        shifted.metrics.maximum_temperature_centi_c = shifted
            .temperature_centi_c
            .iter()
            .copied()
            .max()
            .unwrap_or(shifted.metrics.maximum_temperature_centi_c);
        shifted
    }

    pub fn with_winds_and_moisture_for_field(
        &self,
        field: &PhysicalField,
        settings: ClimateSettings,
        seed: u32,
        retry_index: u32,
        progress: &mut dyn ProgressSink,
    ) -> Result<Self, PhysicalError> {
        field.validate().map_err(PhysicalError::InvalidSource)?;
        self.validate()?;
        settings.validate()?;
        if self.grid != field.grid {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::GeometryInvalid,
                "climate field grid does not match the physical field",
            ));
        }
        let mut next = self.clone();
        let winds = derive_winds(
            field,
            settings,
            &next.temperature_centi_c,
            &next.temperature_nh_summer_centi_c,
            &next.temperature_nh_winter_centi_c,
            seed,
            retry_index,
        );
        assign_winds(&mut next, winds);
        let restamp_temperatures = next.temperature_centi_c.clone();
        let current_geometry = CurrentGeometry::new(field);
        stamp_currents(&mut next, field, &restamp_temperatures, &current_geometry);
        let moisture = product_moisture(
            field,
            settings,
            &next,
            seed,
            retry_index,
            0,
            &current_geometry,
            progress,
        )?;
        let (runoff, runoff_volume, mut metrics) = runoff_fields(
            field,
            settings,
            &next.temperature_centi_c,
            &moisture.precipitation,
            progress,
        )?;
        metrics.transport_iterations = moisture.iterations;
        apply_seasonal_metrics(
            field,
            &next.temperature_nh_summer_centi_c,
            &next.temperature_nh_winter_centi_c,
            &mut metrics,
        );
        apply_wind_metrics(
            field,
            &next.wind_east_milli,
            &next.wind_north_milli,
            &next.wind_divergence_ppm,
            thermal_equator_latitude(field.grid, &next.temperature_centi_c),
            &mut metrics,
        );
        apply_current_metrics(
            field,
            &next.current_east_milli,
            &next.current_north_milli,
            &mut metrics,
        );
        apply_humidity_metrics(
            field,
            &moisture.humidity_ppm,
            &moisture.aridity_ppm,
            &moisture.precipitation_summer,
            &moisture.precipitation_winter,
            &mut metrics,
        );
        apply_season_length_metrics(
            field,
            &next.temperature_nh_summer_centi_c,
            &next.temperature_nh_winter_centi_c,
            &moisture.precipitation_summer,
            &moisture.precipitation_winter,
            &mut metrics,
        );
        next.moisture_mm_per_year = moisture.moisture;
        next.precipitation_mm_per_year = moisture.precipitation;
        next.precipitation_nh_summer_mm = moisture.precipitation_summer;
        next.precipitation_nh_winter_mm = moisture.precipitation_winter;
        next.humidity_ppm = moisture.humidity_ppm;
        next.aridity_ppm = moisture.aridity_ppm;
        next.runoff_mm_per_year = runoff;
        next.runoff_volume_m3_per_year = runoff_volume;
        next.biome_class = classify_biomes(
            field,
            &next.temperature_centi_c,
            &next.temperature_nh_summer_centi_c,
            &next.temperature_nh_winter_centi_c,
            &next.precipitation_mm_per_year,
            &next.humidity_ppm,
            &next.aridity_ppm,
        );
        apply_biome_metrics(field, &next.biome_class, &mut metrics);
        let storms = derive_storms(
            field,
            settings,
            &next.temperature_centi_c,
            &next.temperature_nh_summer_centi_c,
            &next.temperature_nh_winter_centi_c,
            &next.humidity_ppm,
            &next.wind_east_milli,
            &next.wind_north_milli,
            &next.wind_east_nh_summer_milli,
            &next.wind_north_nh_summer_milli,
            &next.wind_east_nh_winter_milli,
            &next.wind_north_nh_winter_milli,
            &next.wind_divergence_ppm,
            &next.wind_divergence_nh_summer_ppm,
            &next.wind_divergence_nh_winter_ppm,
            &next.current_east_milli,
            &next.current_north_milli,
            &current_geometry,
        );
        next.storm_suitability_ppm = storms.suitability_ppm;
        next.storm_track_ppm = storms.track_ppm;
        next.storm_intensity_ppm = storms.intensity_ppm;
        apply_storm_metrics(
            field,
            &next.storm_suitability_ppm,
            &next.storm_track_ppm,
            &next.storm_intensity_ppm,
            &mut metrics,
        );
        apply_extreme_metrics(
            field,
            &next.temperature_centi_c,
            &next.temperature_nh_summer_centi_c,
            &next.temperature_nh_winter_centi_c,
            &next.precipitation_mm_per_year,
            &next.precipitation_nh_summer_mm,
            &next.precipitation_nh_winter_mm,
            &next.aridity_ppm,
            &mut metrics,
        );
        next.metrics = metrics;
        next.validate_against(field)?;
        Ok(next)
    }

    pub fn validate_against(&self, field: &PhysicalField) -> Result<(), PhysicalError> {
        self.validate()?;
        field.validate().map_err(PhysicalError::InvalidSource)?;
        if self.grid != field.grid {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::GeometryInvalid,
                "climate field grid does not match the physical field",
            ));
        }
        let mut precipitation_volume = 0.0;
        let mut runoff_volume = 0.0;
        let mut wettest = 0;
        let mut driest_land = u32::MAX;
        for cell in 0..self.grid.sample_count() {
            let area = self.grid.cell_area(self.grid.row_col(cell).0);
            precipitation_volume +=
                area * f64::from(self.precipitation_mm_per_year[cell]) / 1_000.0;
            if field.elevations_mm[cell] <= field.sea_level_mm {
                if self.runoff_mm_per_year[cell] != 0 || self.runoff_volume_m3_per_year[cell] != 0 {
                    return Err(PhysicalError::Validation(
                        "climate runoff must be zero on ocean cells".into(),
                    ));
                }
            } else {
                if self.runoff_mm_per_year[cell] > self.precipitation_mm_per_year[cell] {
                    return Err(PhysicalError::Validation(
                        "climate runoff cannot exceed local precipitation".into(),
                    ));
                }
                driest_land = driest_land.min(self.precipitation_mm_per_year[cell]);
            }
            wettest = wettest.max(self.precipitation_mm_per_year[cell]);
            let expected_volume =
                round_volume(area * f64::from(self.runoff_mm_per_year[cell]) / 1_000.0)?;
            if expected_volume != self.runoff_volume_m3_per_year[cell] {
                return Err(PhysicalError::Validation(
                    "climate runoff volume does not match cell area".into(),
                ));
            }
            runoff_volume += f64::from(self.runoff_mm_per_year[cell]) * area / 1_000.0;
        }
        if precipitation_volume.round() as u64 != self.metrics.precipitation_volume_m3_per_year
            || runoff_volume.round() as u64 != self.metrics.runoff_volume_m3_per_year
            || wettest != self.metrics.wettest_cell_precipitation_mm_per_year
            || (if driest_land == u32::MAX {
                0
            } else {
                driest_land
            }) != self.metrics.driest_land_cell_precipitation_mm_per_year
        {
            return Err(PhysicalError::Validation(
                "climate metrics do not match derived fields".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
struct UnitVector {
    x: f64,
    y: f64,
    z: f64,
}

impl UnitVector {
    fn from_lon_lat(longitude: f64, latitude: f64) -> Self {
        let latitude_cos = latitude.cos();
        Self {
            x: latitude_cos * longitude.cos(),
            y: latitude_cos * longitude.sin(),
            z: latitude.sin(),
        }
    }

    fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }
}

#[derive(Debug, Clone, Copy)]
struct CellClimateGeometry {
    latitude: f64,
    maritime_factor: f64,
}

fn cell_geometry(grid: Grid, cell: usize) -> (f64, UnitVector) {
    let (row, col) = grid.row_col(cell);
    let (longitude, latitude) = grid.center_radians(row, col);
    (latitude, UnitVector::from_lon_lat(longitude, latitude))
}

fn maritime_geometry_cell(
    field: &PhysicalField,
    ocean_vectors: &[UnitVector],
    ocean_tree: Option<&KdNode>,
    maritime_scale_metres: f64,
    cell: usize,
) -> Result<CellClimateGeometry, PhysicalError> {
    let (latitude, vector) = cell_geometry(field.grid, cell);
    let distance = if field.elevations_mm[cell] <= field.sea_level_mm {
        0.0
    } else {
        nearest_ocean_distance(ocean_vectors, ocean_tree, vector) * field.grid.radius_metres as f64
    };
    let maritime_factor = (-distance / maritime_scale_metres).exp();
    if !distance.is_finite() || !maritime_factor.is_finite() {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::NumericNonFinite,
            "climate maritime distance is not finite",
        ));
    }
    Ok(CellClimateGeometry {
        latitude,
        maritime_factor,
    })
}

fn build_geometry(
    field: &PhysicalField,
    settings: ClimateSettings,
    progress: &mut dyn ProgressSink,
) -> Result<Vec<CellClimateGeometry>, PhysicalError> {
    let ocean_vectors = (0..field.grid.sample_count())
        .filter(|cell| field.elevations_mm[*cell] <= field.sea_level_mm)
        .map(|cell| cell_geometry(field.grid, cell).1)
        .collect::<Vec<_>>();
    if ocean_vectors.is_empty() {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::GeometryInvalid,
            "climate derivation requires at least one ocean cell",
        ));
    }
    let ocean_tree = KdNode::build(&ocean_vectors);
    let maritime_scale_metres = f64::from(settings.maritime_scale_km) * 1_000.0;
    progress.check_cancelled()?;
    let mut geometry = vec![
        CellClimateGeometry {
            latitude: 0.0,
            maritime_factor: 0.0,
        };
        field.grid.sample_count()
    ];
    geometry
        .par_iter_mut()
        .enumerate()
        .try_for_each(|(cell, slot)| {
            *slot = maritime_geometry_cell(
                field,
                &ocean_vectors,
                ocean_tree.as_deref(),
                maritime_scale_metres,
                cell,
            )?;
            Ok(())
        })?;
    progress.check_cancelled()?;
    Ok(geometry)
}

struct KdNode {
    index: usize,
    axis: u8,
    left: Option<Box<KdNode>>,
    right: Option<Box<KdNode>>,
}

impl KdNode {
    fn build(points: &[UnitVector]) -> Option<Box<Self>> {
        if points.is_empty() {
            return None;
        }
        let mut indices = (0..points.len()).collect::<Vec<_>>();
        Self::build_from_indices(points, &mut indices, 0)
    }

    fn build_from_indices(
        points: &[UnitVector],
        indices: &mut [usize],
        depth: usize,
    ) -> Option<Box<Self>> {
        if indices.is_empty() {
            return None;
        }
        let axis = (depth % 3) as u8;
        let mid = indices.len() / 2;
        let (left_indices, median, right_indices) =
            indices.select_nth_unstable_by(mid, |first, second| {
                coord(points[*first], axis)
                    .partial_cmp(&coord(points[*second], axis))
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| first.cmp(second))
            });
        let index = *median;
        Some(Box::new(Self {
            index,
            axis,
            left: Self::build_from_indices(points, left_indices, depth + 1),
            right: Self::build_from_indices(points, right_indices, depth + 1),
        }))
    }

    fn nearest(
        &self,
        points: &[UnitVector],
        query: UnitVector,
        best_distance_sq: &mut f64,
        best_index: &mut usize,
    ) {
        let candidate = points[self.index];
        let distance_sq = chord_squared(candidate, query);
        if distance_sq < *best_distance_sq
            || ((distance_sq - *best_distance_sq).abs() <= f64::EPSILON && self.index < *best_index)
        {
            *best_distance_sq = distance_sq;
            *best_index = self.index;
        }
        let delta = coord(query, self.axis) - coord(candidate, self.axis);
        let (first, second) = if delta <= 0.0 {
            (self.left.as_deref(), self.right.as_deref())
        } else {
            (self.right.as_deref(), self.left.as_deref())
        };
        if let Some(child) = first {
            child.nearest(points, query, best_distance_sq, best_index);
        }
        if second.is_some() && delta * delta <= *best_distance_sq {
            if let Some(child) = second {
                child.nearest(points, query, best_distance_sq, best_index);
            }
        }
    }
}

fn coord(vector: UnitVector, axis: u8) -> f64 {
    match axis {
        0 => vector.x,
        1 => vector.y,
        _ => vector.z,
    }
}

fn chord_squared(first: UnitVector, second: UnitVector) -> f64 {
    let dx = first.x - second.x;
    let dy = first.y - second.y;
    let dz = first.z - second.z;
    dx * dx + dy * dy + dz * dz
}

fn nearest_ocean_distance(
    ocean_vectors: &[UnitVector],
    tree: Option<&KdNode>,
    vector: UnitVector,
) -> f64 {
    if ocean_vectors.len() <= 48 {
        return ocean_vectors
            .iter()
            .map(|ocean| ocean.dot(vector).clamp(-1.0, 1.0).acos())
            .fold(f64::INFINITY, f64::min);
    }
    let Some(tree) = tree else {
        return f64::INFINITY;
    };
    let mut best_distance_sq = f64::INFINITY;
    let mut best_index = 0usize;
    tree.nearest(
        ocean_vectors,
        vector,
        &mut best_distance_sq,
        &mut best_index,
    );
    ocean_vectors[best_index]
        .dot(vector)
        .clamp(-1.0, 1.0)
        .acos()
}

fn solar_base_centi_c(settings: ClimateSettings) -> Result<f64, PhysicalError> {
    let planetary = settings.planetary;
    let insolation = f64::from(planetary.insolation_ppm()?);
    let eccentricity = f64::from(planetary.eccentricity_ppm) / 1_000_000.0;
    let mean_factor = 1.0 / (1.0 - eccentricity * eccentricity).sqrt();
    let absorbed =
        insolation * mean_factor * f64::from(1_000_000 - planetary.bond_albedo_ppm) / 1_000_000.0;
    let earth_eccentricity = f64::from(EARTH_ECCENTRICITY_PPM) / 1_000_000.0;
    let earth_absorbed = f64::from(SOLAR_LUMINOSITY_PPM)
        / (1.0 - earth_eccentricity * earth_eccentricity).sqrt()
        * f64::from(1_000_000 - EARTH_BOND_ALBEDO_PPM)
        / 1_000_000.0;
    let scale = (absorbed / earth_absorbed).powf(0.25);
    if !scale.is_finite() {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::NumericNonFinite,
            "solar temperature scale is not finite",
        ));
    }
    Ok(f64::from(EARTH_EQUATOR_BASE_CENTI_C) * scale
        + f64::from(planetary.retained_heat_centi_c - EARTH_RETAINED_HEAT_CENTI_C)
        + f64::from(settings.global_temperature_centi_c - EARTH_EQUATOR_BASE_CENTI_C))
}

fn solar_declination_radians(planetary: PlanetaryConfiguration, nh_summer: bool) -> f64 {
    let tilt = (f64::from(planetary.axial_tilt_milli_deg) / 1_000.0).to_radians();
    if nh_summer {
        tilt
    } else {
        -tilt
    }
}

fn seasonal_insolation_weight(latitude: f64, declination: f64, settings: ClimateSettings) -> f64 {
    let annual = annual_insolation_weight(latitude, settings);
    (annual * (1.0 + SEASONAL_INSOLATION_ANOMALY * latitude.sin() * declination.sin())).max(0.0)
}

fn clamp_temperature(value: f64) -> Result<i32, PhysicalError> {
    if !value.is_finite() {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::NumericNonFinite,
            "climate temperature is not finite",
        ));
    }
    Ok(value.round().clamp(
        -f64::from(MAX_CLIMATE_TEMPERATURE_CENTI_C),
        f64::from(MAX_CLIMATE_TEMPERATURE_CENTI_C),
    ) as i32)
}

fn altitude_lapse_centi_c(field: &PhysicalField, settings: ClimateSettings, cell: usize) -> f64 {
    let altitude_km = f64::from(
        field.elevations_mm[cell]
            .saturating_sub(field.sea_level_mm)
            .max(0),
    ) / 1_000_000.0;
    altitude_km * f64::from(settings.altitude_lapse_centi_c_per_km)
}

fn annual_insolation_weight(latitude: f64, settings: ClimateSettings) -> f64 {
    let sin_lat = latitude.sin();
    let p2 = (3.0 * sin_lat * sin_lat - 1.0) / 2.0;
    let weight = 1.0 - f64::from(settings.insolation_p2_ppm) / 1_000_000.0 * p2;
    weight.max(f64::from(settings.insolation_polar_floor_ppm) / 1_000_000.0)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SurfaceClass {
    Ocean,
    Vegetated,
    Bare,
    Desert,
    Snow,
    Ice,
}

fn classify_surface(
    is_ocean: bool,
    temperature_centi_c: i32,
    precipitation_mm: u32,
    moisture_mm: u32,
) -> SurfaceClass {
    if temperature_centi_c < 0 {
        if is_ocean {
            return SurfaceClass::Ice;
        }
        if moisture_mm >= SNOW_COVER_MM || precipitation_mm >= SNOW_COVER_MM {
            SurfaceClass::Snow
        } else {
            SurfaceClass::Ice
        }
    } else if is_ocean {
        SurfaceClass::Ocean
    } else if precipitation_mm <= DESERT_PRECIPITATION_MM {
        SurfaceClass::Desert
    } else if precipitation_mm >= GRASSLAND_PRECIPITATION_MM {
        SurfaceClass::Vegetated
    } else {
        SurfaceClass::Bare
    }
}

fn class_albedo(is_ocean: bool, icy: bool, settings: ClimateSettings) -> f64 {
    let class = if icy {
        SurfaceClass::Ice
    } else if is_ocean {
        SurfaceClass::Ocean
    } else {
        SurfaceClass::Vegetated
    };
    surface_albedo(class, settings)
}

fn surface_albedo(class: SurfaceClass, settings: ClimateSettings) -> f64 {
    let ppm = match class {
        SurfaceClass::Ocean => settings.albedo_ocean_ppm,
        SurfaceClass::Vegetated => settings.albedo_land_ppm,
        SurfaceClass::Bare => settings.albedo_bare_ppm,
        SurfaceClass::Desert => settings.albedo_desert_ppm,
        SurfaceClass::Snow => settings.albedo_snow_ppm,
        SurfaceClass::Ice => settings.albedo_ice_ppm,
    };
    f64::from(ppm) / 1_000_000.0
}

fn mixed_albedo(surface: f64, cloud: f64, settings: ClimateSettings) -> f64 {
    let cloud_albedo = f64::from(settings.albedo_cloud_ppm) / 1_000_000.0;
    let weight = (cloud.clamp(0.0, 1.0) * f64::from(settings.cloud_albedo_coupling_ppm)
        / 1_000_000.0)
        .clamp(0.0, 1.0);
    surface * (1.0 - weight) + cloud_albedo * weight
}

fn cloud_fraction(q: f64, q_sat: f64, u_oro: f64) -> f64 {
    let rh = if q_sat <= 0.0 {
        0.0
    } else {
        (q / q_sat).clamp(0.0, 1.0)
    };
    let uplift = (u_oro * CLOUD_UPLIFT_SCALE).clamp(0.0, 1.0);
    (0.70 * rh * rh + 0.30 * uplift).clamp(0.0, 1.0)
}

fn coupled_surface_albedo(
    field: &PhysicalField,
    settings: ClimateSettings,
    temperatures: &[i32],
    precipitation: &[u32],
    moisture: &[u32],
) -> Vec<f64> {
    (0..field.grid.sample_count())
        .map(|cell| {
            surface_albedo(
                classify_surface(
                    is_ocean(field, cell),
                    temperatures[cell],
                    precipitation[cell],
                    moisture[cell],
                ),
                settings,
            )
        })
        .collect()
}

fn absorbed_fraction(class: f64, settings: ClimateSettings) -> f64 {
    let bond = f64::from(settings.planetary.bond_albedo_ppm) / 1_000_000.0;
    let reference = f64::from(settings.albedo_surface_ref_ppm) / 1_000_000.0;
    ((1.0 - bond) * (1.0 - class) / (1.0 - reference).max(0.05)).clamp(0.05, 0.95)
}

fn toa_mean_wm2(settings: ClimateSettings) -> Result<f64, PhysicalError> {
    let insolation = f64::from(settings.planetary.insolation_ppm()?);
    let eccentricity = f64::from(settings.planetary.eccentricity_ppm) / 1_000_000.0;
    let mean_factor = 1.0 / (1.0 - eccentricity * eccentricity).sqrt();
    let wm2 = SOLAR_CONSTANT_WM2 / 4.0 * insolation / 1_000_000.0 * mean_factor;
    if !wm2.is_finite() {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::NumericNonFinite,
            "top-of-atmosphere insolation is not finite",
        ));
    }
    Ok(wm2)
}

fn face_wind_ms(first: i32, second: i32) -> f64 {
    0.5 * (f64::from(first) + f64::from(second)) / 1_000.0
}

fn flux_form_heat_advection(
    temperature: &[f64],
    east_wind: &[i32],
    north_wind: &[i32],
    west: usize,
    east: usize,
    south: usize,
    north: usize,
    cell: usize,
    dx: f64,
    dy: f64,
    advection: f64,
) -> (f64, f64) {
    let u_west = face_wind_ms(east_wind[west], east_wind[cell]);
    let u_east = face_wind_ms(east_wind[cell], east_wind[east]);
    let v_south = face_wind_ms(north_wind[south], north_wind[cell]);
    let v_north = face_wind_ms(north_wind[cell], north_wind[north]);
    let mut neighbors = 0.0;
    let mut diag = 0.0;
    if u_west > 0.0 {
        neighbors += u_west / dx * temperature[west];
    } else {
        diag += -u_west / dx;
    }
    if u_east > 0.0 {
        diag += u_east / dx;
    } else {
        neighbors += -u_east / dx * temperature[east];
    }
    if v_south > 0.0 {
        neighbors += v_south / dy * temperature[south];
    } else {
        diag += -v_south / dy;
    }
    if v_north > 0.0 {
        diag += v_north / dy;
    } else {
        neighbors += -v_north / dy * temperature[north];
    }
    (advection * neighbors, advection * diag)
}

fn flux_form_ocean_advection(
    temperature: &[f64],
    current_east: &[i32],
    current_north: &[i32],
    west: usize,
    east: usize,
    south: usize,
    north: usize,
    cell: usize,
    dx: f64,
    dy: f64,
    advection: f64,
    mask: &[bool],
) -> (f64, f64) {
    let mut neighbors = 0.0;
    let mut diag = 0.0;
    if mask[west] {
        let u_west = face_wind_ms(current_east[west], current_east[cell]);
        if u_west > 0.0 {
            neighbors += u_west / dx * temperature[west];
        } else {
            diag += -u_west / dx;
        }
    }
    if mask[east] {
        let u_east = face_wind_ms(current_east[cell], current_east[east]);
        if u_east > 0.0 {
            diag += u_east / dx;
        } else {
            neighbors += -u_east / dx * temperature[east];
        }
    }
    if mask[south] {
        let v_south = face_wind_ms(current_north[south], current_north[cell]);
        if v_south > 0.0 {
            neighbors += v_south / dy * temperature[south];
        } else {
            diag += -v_south / dy;
        }
    }
    if mask[north] {
        let v_north = face_wind_ms(current_north[cell], current_north[north]);
        if v_north > 0.0 {
            diag += v_north / dy;
        } else {
            neighbors += -v_north / dy * temperature[north];
        }
    }
    (advection * neighbors, advection * diag)
}

struct OceanHeat<'a> {
    temperature: &'a mut [f64],
    mask: &'a [bool],
    current_east: &'a [i32],
    current_north: &'a [i32],
}

fn step_ocean_temperature(
    field: &PhysicalField,
    settings: ClimateSettings,
    ocean: &mut OceanHeat<'_>,
    air_celsius: &[f64],
    dt_seconds: f64,
) -> Result<(), PhysicalError> {
    let sample_count = field.grid.sample_count();
    let coupling = settings.ocean_heat_coupling_wm2_per_c();
    let capacity = settings.heat_capacity_j_m2_k(true);
    let dt_over_c = dt_seconds / capacity;
    let diffusivity = settings.ocean_heat_diffusivity_w_per_c();
    let spacing = GridSpacing::new(field.grid);
    let mut next = ocean.temperature.to_vec();
    for cell in 0..sample_count {
        if !ocean.mask[cell] {
            next[cell] = air_celsius[cell];
            continue;
        }
        let (row, col) = field.grid.row_col(cell);
        let west = field.grid.index(row, wrapped_col(field.grid, col, -1));
        let east = field.grid.index(row, wrapped_col(field.grid, col, 1));
        let south = field.grid.index(clamped_row(field.grid, row, -1), col);
        let north = field.grid.index(clamped_row(field.grid, row, 1), col);
        let (dx_w, dx_e, dy_s, dy_n) = spacing.spans(row);
        let x_span = (dx_w + dx_e) * 0.5;
        let y_span = (dy_s + dy_n) * 0.5;
        let faces = [
            (west, dx_w, x_span),
            (east, dx_e, x_span),
            (south, dy_s, y_span),
            (north, dy_n, y_span),
        ];
        let mut neighbor_part = 0.0;
        let mut diag = 0.0;
        for (neighbor, distance, span) in faces {
            if ocean.mask[neighbor] {
                neighbor_part += ocean.temperature[neighbor] / distance / span;
                diag += 1.0 / distance / span;
            }
        }
        let mut numerator = ocean.temperature[cell]
            + dt_over_c * (coupling * air_celsius[cell] + diffusivity * neighbor_part);
        let mut denominator = 1.0 + dt_over_c * (coupling + diffusivity * diag);
        let (adv_neighbors, adv_diag) = flux_form_ocean_advection(
            ocean.temperature,
            ocean.current_east,
            ocean.current_north,
            west,
            east,
            south,
            north,
            cell,
            x_span,
            y_span,
            settings.ocean_heat_advection_j_m2_k(),
            ocean.mask,
        );
        numerator += dt_over_c * adv_neighbors;
        denominator += dt_over_c * adv_diag;
        let updated = numerator / denominator;
        if !updated.is_finite() {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::NumericNonFinite,
                "climate ocean-temperature tracer is not finite",
            ));
        }
        let bounded = f64::from(MAX_CLIMATE_TEMPERATURE_CENTI_C) / 100.0;
        next[cell] = updated.clamp(-bounded, bounded);
    }
    ocean.temperature.copy_from_slice(&next);
    Ok(())
}

fn imprint_ocean_heat(
    field: &PhysicalField,
    settings: ClimateSettings,
    basin: &[bool],
    ocean_celsius: &[f64],
    air_celsius: &mut [f64],
) {
    let previous = air_celsius.to_vec();
    let ocean_blend = settings.ocean_coast_ocean_blend();
    let air_blend = 1.0 - ocean_blend;
    for cell in 0..field.grid.sample_count() {
        if basin[cell] {
            air_celsius[cell] = ocean_celsius[cell];
            continue;
        }
        let (row, col) = field.grid.row_col(cell);
        let neighbors = [
            field.grid.index(row, wrapped_col(field.grid, col, -1)),
            field.grid.index(row, wrapped_col(field.grid, col, 1)),
            field.grid.index(clamped_row(field.grid, row, -1), col),
            field.grid.index(clamped_row(field.grid, row, 1), col),
        ];
        let mut sum = 0.0;
        let mut count = 0.0;
        for neighbor in neighbors {
            if basin[neighbor] {
                sum += ocean_celsius[neighbor];
                count += 1.0;
            }
        }
        if count > 0.0 {
            air_celsius[cell] = previous[cell] * air_blend + (sum / count) * ocean_blend;
        }
    }
}

#[derive(Clone, Copy)]
enum IceAlbedo<'a> {
    Live,
    Fixed(&'a [bool]),
}

fn cell_is_icy(ice: IceAlbedo<'_>, cell: usize, surface_centi_c: f64) -> bool {
    match ice {
        IceAlbedo::Live => surface_centi_c < 0.0,
        IceAlbedo::Fixed(mask) => mask[cell],
    }
}

fn ice_mask_from_temperature(
    field: &PhysicalField,
    settings: ClimateSettings,
    temperature_celsius: &[f64],
) -> Vec<bool> {
    (0..field.grid.sample_count())
        .map(|cell| {
            let lapse_c = altitude_lapse_centi_c(field, settings, cell) / 100.0;
            temperature_celsius[cell] - lapse_c < 0.0
        })
        .collect()
}

fn relax_annual_temperature(
    field: &PhysicalField,
    settings: ClimateSettings,
    geometry: &[CellClimateGeometry],
    initial_celsius: &[f64],
    winds: Option<(&[i32], &[i32])>,
    ice: IceAlbedo<'_>,
    latent_wm2: Option<&[f64]>,
    albedo: Option<&[f64]>,
    cloud: Option<&[f64]>,
    progress: &mut dyn ProgressSink,
) -> Result<Vec<f64>, PhysicalError> {
    relax_temperature(
        field,
        settings,
        geometry,
        initial_celsius,
        winds,
        ice,
        latent_wm2,
        None,
        albedo,
        cloud,
        ENERGY_BALANCE_DT_SECONDS,
        settings.energy_balance_max_iterations,
        ENERGY_BALANCE_MIN_ITERATIONS,
        progress,
    )
}

struct RelaxTemperatureInput<'a> {
    field: &'a PhysicalField,
    settings: ClimateSettings,
    geometry: &'a [CellClimateGeometry],
    temperature: &'a [f64],
    winds: Option<(&'a [i32], &'a [i32])>,
    ice: IceAlbedo<'a>,
    latent_wm2: Option<&'a [f64]>,
    insolation_weights: Option<&'a [f64]>,
    albedo: Option<&'a [f64]>,
    cloud: Option<&'a [f64]>,
    spacing: &'a GridSpacing,
    toa: f64,
    olr_a: f64,
    olr_b: f64,
    diffusivity: f64,
    advection: f64,
    q_force: f64,
    dt_land: f64,
    dt_ocean: f64,
    bounded: f64,
}

fn relax_temperature_cell(
    input: &RelaxTemperatureInput<'_>,
    cell: usize,
) -> Result<f64, PhysicalError> {
    let field = input.field;
    let settings = input.settings;
    let (row, col) = field.grid.row_col(cell);
    let west = field.grid.index(row, wrapped_col(field.grid, col, -1));
    let east = field.grid.index(row, wrapped_col(field.grid, col, 1));
    let south = field.grid.index(clamped_row(field.grid, row, -1), col);
    let north = field.grid.index(clamped_row(field.grid, row, 1), col);
    let (dx_w, dx_e, dy_s, dy_n) = input.spacing.spans(row);
    let x_span = (dx_w + dx_e) * 0.5;
    let y_span = (dy_s + dy_n) * 0.5;
    let neighbor_part = (input.temperature[west] / dx_w + input.temperature[east] / dx_e) / x_span
        + (input.temperature[south] / dy_s + input.temperature[north] / dy_n) / y_span;
    let diag = (1.0 / dx_w + 1.0 / dx_e) / x_span + (1.0 / dy_s + 1.0 / dy_n) / y_span;
    let is_ocean_cell = is_ocean(field, cell);
    let lapse_c = altitude_lapse_centi_c(field, settings, cell) / 100.0;
    let surface_centi_c = (input.temperature[cell] - lapse_c) * 100.0;
    let insolation_weight = input
        .insolation_weights
        .map(|weights| weights[cell])
        .unwrap_or_else(|| annual_insolation_weight(input.geometry[cell].latitude, settings));
    let cloud_f = input
        .cloud
        .map(|values| values[cell].clamp(0.0, 1.0))
        .unwrap_or(0.0);
    let surface = input.albedo.map(|values| values[cell]).unwrap_or_else(|| {
        class_albedo(
            is_ocean_cell,
            cell_is_icy(input.ice, cell, surface_centi_c),
            settings,
        )
    });
    let q_solar = input.toa
        * insolation_weight
        * absorbed_fraction(mixed_albedo(surface, cloud_f, settings), settings);
    let dt_over_c = if is_ocean_cell {
        input.dt_ocean
    } else {
        input.dt_land
    };
    let k_diag = input.diffusivity * diag;
    let q_latent = input.latent_wm2.map(|values| values[cell]).unwrap_or(0.0);
    let olr_cloud = settings.cloud_olr_factor(cloud_f);
    let mut numerator = input.temperature[cell]
        + dt_over_c
            * (q_solar - input.olr_a * olr_cloud
                + input.q_force
                + q_latent
                + input.diffusivity * neighbor_part);
    let mut denominator = 1.0 + dt_over_c * (input.olr_b * olr_cloud + k_diag);
    if let Some((east_wind, north_wind)) = input.winds {
        if input.advection > 0.0 {
            let (adv_neighbors, adv_diag) = flux_form_heat_advection(
                input.temperature,
                east_wind,
                north_wind,
                west,
                east,
                south,
                north,
                cell,
                x_span,
                y_span,
                input.advection,
            );
            numerator += dt_over_c * adv_neighbors;
            denominator += dt_over_c * adv_diag;
        }
    }
    let updated = numerator / denominator;
    if !updated.is_finite() {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::NumericNonFinite,
            "climate energy-balance temperature is not finite",
        ));
    }
    Ok(updated.clamp(-input.bounded, input.bounded))
}

fn relax_temperature(
    field: &PhysicalField,
    settings: ClimateSettings,
    geometry: &[CellClimateGeometry],
    initial_celsius: &[f64],
    winds: Option<(&[i32], &[i32])>,
    ice: IceAlbedo<'_>,
    latent_wm2: Option<&[f64]>,
    insolation_weights: Option<&[f64]>,
    albedo: Option<&[f64]>,
    cloud: Option<&[f64]>,
    dt_seconds: f64,
    max_iterations: u32,
    min_iterations: u32,
    progress: &mut dyn ProgressSink,
) -> Result<Vec<f64>, PhysicalError> {
    let sample_count = field.grid.sample_count();
    let toa = toa_mean_wm2(settings)?;
    let olr_a = settings.olr_a_wm2();
    let olr_b = settings.olr_b_wm2_per_c();
    let diffusivity = settings.heat_diffusivity_w_per_c();
    let advection = settings.heat_advection_j_m2_k();
    let q_force = olr_b
        * f64::from(
            settings.global_temperature_centi_c - EARTH_EQUATOR_BASE_CENTI_C
                + settings.planetary.retained_heat_centi_c
                - EARTH_RETAINED_HEAT_CENTI_C,
        )
        / 100.0;
    let mut temperature = initial_celsius.to_vec();
    let mut next = vec![0.0; sample_count];
    let spacing = GridSpacing::new(field.grid);
    let dt_land = dt_seconds / settings.heat_capacity_j_m2_k(false);
    let dt_ocean = dt_seconds / settings.heat_capacity_j_m2_k(true);
    let bounded = f64::from(MAX_CLIMATE_TEMPERATURE_CENTI_C) / 100.0;
    let mut converged = false;
    for iteration in 0..max_iterations {
        progress.check_cancelled()?;
        let input = RelaxTemperatureInput {
            field,
            settings,
            geometry,
            temperature: &temperature,
            winds,
            ice,
            latent_wm2,
            insolation_weights,
            albedo,
            cloud,
            spacing: &spacing,
            toa,
            olr_a,
            olr_b,
            diffusivity,
            advection,
            q_force,
            dt_land,
            dt_ocean,
            bounded,
        };
        let width = field.grid.width as usize;
        next.par_chunks_mut(width)
            .enumerate()
            .try_for_each(|(row, slot)| {
                for (col, cell_slot) in slot.iter_mut().enumerate() {
                    let cell = field.grid.index(row as u32, col as u32);
                    *cell_slot = relax_temperature_cell(&input, cell)?;
                }
                Ok(())
            })?;
        let abs_delta_sum = next
            .iter()
            .zip(temperature.iter())
            .map(|(updated, previous)| (updated - previous).abs())
            .sum::<f64>();
        std::mem::swap(&mut temperature, &mut next);
        let mean_abs_delta = abs_delta_sum / sample_count as f64;
        if iteration + 1 >= min_iterations
            && mean_abs_delta <= settings.energy_balance_tolerance_c()
        {
            converged = true;
            break;
        }
    }
    if !converged {
        if min_iterations <= 1 {
            return Ok(temperature);
        }
        return Err(PhysicalError::coded(
            PhysicalErrorCode::NumericNonConvergent,
            format!("climate energy balance did not converge within {max_iterations} iterations"),
        ));
    }
    Ok(temperature)
}

fn temperature_field(
    field: &PhysicalField,
    settings: ClimateSettings,
    geometry: &[CellClimateGeometry],
    current_geometry: &CurrentGeometry,
    seed: u32,
    retry_index: u32,
    progress: &mut dyn ProgressSink,
) -> Result<(Vec<i32>, Vec<i32>, Vec<i32>, Vec<u32>, Vec<f64>), PhysicalError> {
    let base = solar_base_centi_c(settings)?;
    let mut initial_celsius = Vec::with_capacity(field.grid.sample_count());
    for (cell, cell_geometry) in geometry.iter().copied().enumerate() {
        if cell % 128 == 0 {
            progress.check_cancelled()?;
        }
        let latitude_fraction =
            (cell_geometry.latitude.abs() / std::f64::consts::FRAC_PI_2).clamp(0.0, 1.0);
        let latitude_cooling =
            f64::from(settings.latitude_cooling_centi_c) * latitude_fraction.powf(1.35);
        initial_celsius.push((base - latitude_cooling) / 100.0);
    }
    let mut relaxed = relax_annual_temperature(
        field,
        settings,
        geometry,
        &initial_celsius,
        None,
        IceAlbedo::Live,
        None,
        None,
        None,
        progress,
    )?;
    let omega = omega_ratio(settings.planetary);
    let hadley = hadley_edge_radians(omega);
    let ferrel = ferrel_edge_radians(hadley);
    let wind_seed = derive_subsystem_seed(seed, retry_index, SeedDomain::Climate);
    let mut ocean_celsius = relaxed.clone();
    for _ in 0..settings.temperature_wind_coupling_passes {
        progress.check_cancelled()?;
        let ice_mask = ice_mask_from_temperature(field, settings, &relaxed);
        let sea_centi = relaxed
            .iter()
            .map(|value| clamp_temperature(value * 100.0))
            .collect::<Result<Vec<_>, _>>()?;
        let itcz = thermal_equator_latitude(field.grid, &sea_centi);
        let (east, north) = wind_components(
            field, &sea_centi, settings, itcz, hadley, ferrel, wind_seed, false,
        );
        let (current_east, current_north) = derive_currents_with(
            field,
            settings.planetary,
            &sea_centi,
            &east,
            &north,
            current_geometry,
        );
        relaxed = relax_annual_temperature(
            field,
            settings,
            geometry,
            &relaxed,
            Some((&east, &north)),
            IceAlbedo::Fixed(&ice_mask),
            None,
            None,
            None,
            progress,
        )?;
        if settings.ocean_heat_active() {
            let mut ocean_heat = OceanHeat {
                temperature: &mut ocean_celsius,
                mask: &current_geometry.ocean,
                current_east: &current_east,
                current_north: &current_north,
            };
            step_ocean_temperature(
                field,
                settings,
                &mut ocean_heat,
                &relaxed,
                ENERGY_BALANCE_DT_SECONDS,
            )?;
        }
    }
    let temperatures = apply_diagnostic_lapse(field, settings, &relaxed)?;
    Ok((
        temperatures.clone(),
        temperatures.clone(),
        temperatures,
        maritime_factor_ppm(geometry),
        ocean_celsius,
    ))
}

fn apply_diagnostic_lapse(
    field: &PhysicalField,
    settings: ClimateSettings,
    relaxed: &[f64],
) -> Result<Vec<i32>, PhysicalError> {
    let mut temperatures = Vec::with_capacity(field.grid.sample_count());
    for cell in 0..field.grid.sample_count() {
        let lapse = altitude_lapse_centi_c(field, settings, cell);
        temperatures.push(clamp_temperature(relaxed[cell] * 100.0 - lapse)?);
    }
    Ok(temperatures)
}

fn maritime_factor_ppm(geometry: &[CellClimateGeometry]) -> Vec<u32> {
    geometry
        .iter()
        .map(|cell| (cell.maritime_factor * 1_000_000.0).round() as u32)
        .collect()
}

fn stamp_solstice_temperatures(
    annual: &[i32],
    summer_anomaly: &[f64],
    winter_anomaly: &[f64],
    untilted: bool,
) -> Result<(Vec<i32>, Vec<i32>), PhysicalError> {
    if untilted {
        return Ok((annual.to_vec(), annual.to_vec()));
    }
    let mut summers = Vec::with_capacity(annual.len());
    let mut winters = Vec::with_capacity(annual.len());
    for cell in 0..annual.len() {
        summers.push(clamp_temperature(
            f64::from(annual[cell]) + summer_anomaly[cell],
        )?);
        winters.push(clamp_temperature(
            f64::from(annual[cell]) + winter_anomaly[cell],
        )?);
    }
    Ok((summers, winters))
}

fn seasonal_year_converged(years_run: u32, last_delta_c: f64, ocean_delta_c: f64) -> bool {
    years_run >= CLIMATE_MIN_YEARS
        && last_delta_c <= YEAR_T_TOLERANCE_C
        && ocean_delta_c <= YEAR_T_TOLERANCE_C
}

fn mean_abs_centi_delta_c(left: &[i32], right: &[i32]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(first, second)| (f64::from(*first) - f64::from(*second)).abs())
        .sum::<f64>()
        / left.len() as f64
        / 100.0
}

fn mean_abs_ocean_delta_c(mask: &[bool], left: &[f64], right: &[f64]) -> f64 {
    let mut sum = 0.0;
    let mut count = 0.0;
    for (cell, ocean) in mask.iter().copied().enumerate() {
        if ocean {
            sum += (left[cell] - right[cell]).abs();
            count += 1.0;
        }
    }
    if count == 0.0 {
        0.0
    } else {
        sum / count
    }
}

fn product_moisture(
    field: &PhysicalField,
    settings: ClimateSettings,
    climate: &ClimateField,
    seed: u32,
    retry_index: u32,
    prior_iterations: u32,
    current_geometry: &CurrentGeometry,
    progress: &mut dyn ProgressSink,
) -> Result<MoistureBundle, PhysicalError> {
    let climate_seed = derive_subsystem_seed(seed, retry_index, SeedDomain::Climate);
    let annual = transport_moisture_state(
        field,
        settings,
        climate_seed,
        &climate.temperature_centi_c,
        &climate.current_east_milli,
        &climate.current_north_milli,
        &climate.wind_east_milli,
        &climate.wind_north_milli,
        &climate.wind_divergence_ppm,
        &climate.wind_band,
        None,
        None,
        progress,
    )?;
    let mut iterations = prior_iterations.max(annual.iterations);
    let (precipitation_summer, precipitation_winter) =
        if settings.planetary.axial_tilt_milli_deg == 0 {
            (annual.precipitation.clone(), annual.precipitation.clone())
        } else {
            let (summer_east, summer_north) = derive_currents_with(
                field,
                settings.planetary,
                &climate.temperature_nh_summer_centi_c,
                &climate.wind_east_nh_summer_milli,
                &climate.wind_north_nh_summer_milli,
                current_geometry,
            );
            let summer = transport_moisture_state(
                field,
                settings,
                climate_seed,
                &climate.temperature_nh_summer_centi_c,
                &summer_east,
                &summer_north,
                &climate.wind_east_nh_summer_milli,
                &climate.wind_north_nh_summer_milli,
                &climate.wind_divergence_nh_summer_ppm,
                &climate.wind_band_nh_summer,
                None,
                None,
                progress,
            )?;
            let (winter_east, winter_north) = derive_currents_with(
                field,
                settings.planetary,
                &climate.temperature_nh_winter_centi_c,
                &climate.wind_east_nh_winter_milli,
                &climate.wind_north_nh_winter_milli,
                current_geometry,
            );
            let winter = transport_moisture_state(
                field,
                settings,
                climate_seed,
                &climate.temperature_nh_winter_centi_c,
                &winter_east,
                &winter_north,
                &climate.wind_east_nh_winter_milli,
                &climate.wind_north_nh_winter_milli,
                &climate.wind_divergence_nh_winter_ppm,
                &climate.wind_band_nh_winter,
                None,
                None,
                progress,
            )?;
            iterations = iterations.max(summer.iterations).max(winter.iterations);
            (summer.precipitation, winter.precipitation)
        };
    let (humidity_ppm, aridity_ppm) = humidity_and_aridity(
        &climate.temperature_centi_c,
        &annual.moisture,
        &annual.precipitation,
    );
    Ok(MoistureBundle {
        moisture: annual.moisture,
        precipitation: annual.precipitation,
        precipitation_summer,
        precipitation_winter,
        humidity_ppm,
        aridity_ppm,
        iterations,
    })
}

fn energy_celsius_from_surface(
    field: &PhysicalField,
    settings: ClimateSettings,
    surface_centi_c: &[i32],
) -> Vec<f64> {
    (0..field.grid.sample_count())
        .map(|cell| {
            let lapse = altitude_lapse_centi_c(field, settings, cell);
            (f64::from(surface_centi_c[cell]) + lapse) / 100.0
        })
        .collect()
}

fn assign_winds(climate: &mut ClimateField, winds: DerivedWinds) {
    climate.wind_east_milli = winds.east;
    climate.wind_north_milli = winds.north;
    climate.wind_east_nh_summer_milli = winds.east_summer;
    climate.wind_north_nh_summer_milli = winds.north_summer;
    climate.wind_east_nh_winter_milli = winds.east_winter;
    climate.wind_north_nh_winter_milli = winds.north_winter;
    climate.wind_divergence_ppm = winds.divergence_ppm;
    climate.wind_divergence_nh_summer_ppm = winds.divergence_summer_ppm;
    climate.wind_divergence_nh_winter_ppm = winds.divergence_winter_ppm;
    climate.wind_band = winds.band;
    climate.wind_band_nh_summer = winds.band_summer;
    climate.wind_band_nh_winter = winds.band_winter;
}

fn couple_moisture_and_latent(
    field: &PhysicalField,
    settings: ClimateSettings,
    geometry: &[CellClimateGeometry],
    climate: &mut ClimateField,
    ocean_celsius: &mut [f64],
    current_geometry: &CurrentGeometry,
    seed: u32,
    retry_index: u32,
    progress: &mut dyn ProgressSink,
) -> Result<MoistureBundle, PhysicalError> {
    let climate_seed = derive_subsystem_seed(seed, retry_index, SeedDomain::Climate);
    let sample_count = field.grid.sample_count();
    let annual_keep = climate.temperature_centi_c.clone();
    let mut relaxed = energy_celsius_from_surface(field, settings, &climate.temperature_centi_c);
    let mut q = vec![0.0; sample_count];
    let mut water = vec![0.0; sample_count];
    let mut applied_latent = vec![0.0; sample_count];
    let mut east = climate.wind_east_milli.clone();
    let mut north = climate.wind_north_milli.clone();
    let omega = omega_ratio(settings.planetary);
    let hadley = hadley_edge_radians(omega);
    let ferrel = ferrel_edge_radians(hadley);
    let wind_seed = climate_seed;
    let dt = CLIMATE_SECONDS_PER_YEAR / SEASON_SAMPLE_COUNT;
    let seasons = [true, false];
    let annual_ice = ice_mask_from_temperature(field, settings, &relaxed);
    let mut last_delta = f64::INFINITY;
    let mut last_ocean_delta = if settings.ocean_heat_active() {
        f64::INFINITY
    } else {
        0.0
    };
    let mut year_converged = false;
    let mut previous_summer: Option<Vec<i32>> = None;
    let mut previous_winter: Option<Vec<i32>> = None;
    let mut previous_ocean: Option<Vec<f64>> = None;
    let mut summer_surface = climate.temperature_centi_c.clone();
    let mut winter_surface = climate.temperature_centi_c.clone();
    let mut transport_iterations = 0u32;
    let summer_weights = geometry
        .iter()
        .map(|cell| {
            seasonal_insolation_weight(
                cell.latitude,
                solar_declination_radians(settings.planetary, true),
                settings,
            )
        })
        .collect::<Vec<_>>();
    let winter_weights = geometry
        .iter()
        .map(|cell| {
            seasonal_insolation_weight(
                cell.latitude,
                solar_declination_radians(settings.planetary, false),
                settings,
            )
        })
        .collect::<Vec<_>>();
    let mut season_albedo: Option<Vec<f64>> = None;
    let mut season_cloud: Option<Vec<f64>> = None;
    for year in 0..settings.seasonal_year_max {
        progress.check_cancelled()?;
        for nh_summer in seasons {
            progress.check_cancelled()?;
            let weights = if nh_summer {
                summer_weights.as_slice()
            } else {
                winter_weights.as_slice()
            };
            relaxed = relax_temperature(
                field,
                settings,
                geometry,
                &relaxed,
                Some((&east, &north)),
                IceAlbedo::Fixed(&annual_ice),
                Some(&applied_latent),
                Some(weights),
                season_albedo.as_deref(),
                season_cloud.as_deref(),
                dt,
                1,
                1,
                progress,
            )?;
            let surface = apply_diagnostic_lapse(field, settings, &relaxed)?;
            let itcz = thermal_equator_latitude(field.grid, &surface);
            (east, north) = wind_components(
                field, &surface, settings, itcz, hadley, ferrel, wind_seed, true,
            );
            let (current_east, current_north) = derive_currents_with(
                field,
                settings.planetary,
                &surface,
                &east,
                &north,
                current_geometry,
            );
            if settings.ocean_heat_active() {
                let mut ocean_heat = OceanHeat {
                    temperature: ocean_celsius,
                    mask: &current_geometry.ocean,
                    current_east: &current_east,
                    current_north: &current_north,
                };
                step_ocean_temperature(field, settings, &mut ocean_heat, &relaxed, dt)?;
            }
            let divergence = wind_divergence_ppm(field.grid, &east, &north);
            let band = wind_band_field(field.grid, itcz, hadley, ferrel);
            let transport = transport_moisture_state(
                field,
                settings,
                climate_seed,
                &surface,
                &current_east,
                &current_north,
                &east,
                &north,
                &divergence,
                &band,
                Some(&q),
                Some(&water),
                progress,
            )?;
            q = transport
                .moisture
                .iter()
                .map(|value| f64::from(*value))
                .collect();
            water = transport.water_mm;
            transport_iterations = transport_iterations.max(transport.iterations);
            season_albedo = Some(coupled_surface_albedo(
                field,
                settings,
                &surface,
                &transport.precipitation,
                &transport.moisture,
            ));
            season_cloud = Some(transport.cloud);
            for cell in 0..sample_count {
                let target = LATENT_HEAT_J_PER_KG
                    * (transport.condensation_mm[cell] - transport.evaporation_mm[cell])
                    / CLIMATE_SECONDS_PER_YEAR
                    * f64::from(settings.latent_heat_coupling_ppm)
                    / 1_000_000.0;
                applied_latent[cell] = applied_latent[cell]
                    * (1.0 - MOISTURE_TEMPERATURE_RELAXATION)
                    + target * MOISTURE_TEMPERATURE_RELAXATION;
            }
            if nh_summer {
                summer_surface = surface;
            } else {
                winter_surface = surface;
            }
        }
        last_delta = match (&previous_summer, &previous_winter) {
            (Some(summer), Some(winter)) => {
                0.5 * (mean_abs_centi_delta_c(summer, &summer_surface)
                    + mean_abs_centi_delta_c(winter, &winter_surface))
            }
            _ => f64::INFINITY,
        };
        last_ocean_delta = if settings.ocean_heat_active() {
            match &previous_ocean {
                Some(previous) => {
                    mean_abs_ocean_delta_c(&current_geometry.ocean, previous, ocean_celsius)
                }
                None => f64::INFINITY,
            }
        } else {
            0.0
        };
        previous_summer = Some(summer_surface.clone());
        previous_winter = Some(winter_surface.clone());
        if settings.ocean_heat_active() {
            previous_ocean = Some(ocean_celsius.to_vec());
        }
        if seasonal_year_converged(year + 1, last_delta, last_ocean_delta) {
            year_converged = true;
            break;
        }
    }
    if !year_converged {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::NumericNonConvergent,
            format!(
                "climate seasonal year loop did not converge within {} years (mean |ΔT|={last_delta:.4} C, mean |ΔT_ocean|={last_ocean_delta:.4} C)",
                settings.seasonal_year_max
            ),
        ));
    }
    climate.temperature_centi_c = annual_keep;
    let untilted = settings.planetary.axial_tilt_milli_deg == 0;
    let mut summer_anomaly = vec![0.0; sample_count];
    let mut winter_anomaly = vec![0.0; sample_count];
    if !untilted {
        for cell in 0..sample_count {
            let mean = (f64::from(summer_surface[cell]) + f64::from(winter_surface[cell])) / 2.0;
            summer_anomaly[cell] = f64::from(summer_surface[cell]) - mean;
            winter_anomaly[cell] = f64::from(winter_surface[cell]) - mean;
        }
    }
    let (summers, winters) = stamp_solstice_temperatures(
        &climate.temperature_centi_c,
        &summer_anomaly,
        &winter_anomaly,
        untilted,
    )?;
    climate.temperature_nh_summer_centi_c = summers;
    climate.temperature_nh_winter_centi_c = winters;
    relaxed = energy_celsius_from_surface(field, settings, &climate.temperature_centi_c);
    applied_latent.fill(0.0);
    let passes = settings.moisture_temperature_coupling_passes.max(1);
    let mut last_dt = f64::INFINITY;
    let mut last_dq = f64::INFINITY;
    let mut previous_moisture: Option<Vec<u32>> = None;
    let mut coupled = passes == 1;
    for pass in 0..passes {
        progress.check_cancelled()?;
        let winds = derive_winds(
            field,
            settings,
            &climate.temperature_centi_c,
            &climate.temperature_nh_summer_centi_c,
            &climate.temperature_nh_winter_centi_c,
            seed,
            retry_index,
        );
        assign_winds(climate, winds);
        let surface_centi = climate.temperature_centi_c.clone();
        let (current_east, current_north) = derive_currents_with(
            field,
            climate.planetary,
            &surface_centi,
            &climate.wind_east_milli,
            &climate.wind_north_milli,
            current_geometry,
        );
        climate.current_east_milli = current_east;
        climate.current_north_milli = current_north;
        let transport = transport_moisture_state(
            field,
            settings,
            climate_seed,
            &climate.temperature_centi_c,
            &climate.current_east_milli,
            &climate.current_north_milli,
            &climate.wind_east_milli,
            &climate.wind_north_milli,
            &climate.wind_divergence_ppm,
            &climate.wind_band,
            if pass == 0 { None } else { Some(q.as_slice()) },
            if pass == 0 {
                None
            } else {
                Some(water.as_slice())
            },
            progress,
        )?;
        transport_iterations = transport_iterations.max(transport.iterations);
        if let Some(previous) = &previous_moisture {
            last_dq = previous
                .iter()
                .zip(&transport.moisture)
                .map(|(left, right)| (f64::from(*left) - f64::from(*right)).abs())
                .sum::<f64>()
                / previous.len() as f64;
        }
        previous_moisture = Some(transport.moisture.clone());
        q = transport
            .moisture
            .iter()
            .map(|value| f64::from(*value))
            .collect();
        water = transport.water_mm;
        for cell in 0..sample_count {
            let target = LATENT_HEAT_J_PER_KG
                * (transport.condensation_mm[cell] - transport.evaporation_mm[cell])
                / CLIMATE_SECONDS_PER_YEAR
                * f64::from(settings.latent_heat_coupling_ppm)
                / 1_000_000.0;
            applied_latent[cell] = applied_latent[cell] * (1.0 - MOISTURE_TEMPERATURE_RELAXATION)
                + target * MOISTURE_TEMPERATURE_RELAXATION;
        }
        let ice_mask = ice_mask_from_temperature(field, settings, &relaxed);
        let before = relaxed.clone();
        relaxed = relax_annual_temperature(
            field,
            settings,
            geometry,
            &relaxed,
            Some((&climate.wind_east_milli, &climate.wind_north_milli)),
            IceAlbedo::Fixed(&ice_mask),
            Some(&applied_latent),
            None,
            None,
            progress,
        )?;
        last_dt = before
            .iter()
            .zip(&relaxed)
            .map(|(left, right)| (left - right).abs())
            .sum::<f64>()
            / before.len() as f64;
        climate.temperature_centi_c = apply_diagnostic_lapse(field, settings, &relaxed)?;
        if pass > 0 && last_dt <= COUPLING_T_TOLERANCE_C && last_dq <= COUPLING_Q_TOLERANCE_MM {
            coupled = true;
            break;
        }
    }
    if !coupled {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::NumericNonConvergent,
            format!(
                "climate moisture-temperature coupling did not converge within {passes} passes (mean |ΔT|={last_dt:.4} C, mean |Δq|={last_dq:.2} mm)"
            ),
        ));
    }
    if settings.ocean_heat_active() {
        imprint_ocean_heat(
            field,
            settings,
            &current_geometry.ocean,
            ocean_celsius,
            &mut relaxed,
        );
        climate.temperature_centi_c = apply_diagnostic_lapse(field, settings, &relaxed)?;
    }
    let (summers, winters) = stamp_solstice_temperatures(
        &climate.temperature_centi_c,
        &summer_anomaly,
        &winter_anomaly,
        untilted,
    )?;
    climate.temperature_nh_summer_centi_c = summers;
    climate.temperature_nh_winter_centi_c = winters;
    let winds = derive_winds(
        field,
        settings,
        &climate.temperature_centi_c,
        &climate.temperature_nh_summer_centi_c,
        &climate.temperature_nh_winter_centi_c,
        seed,
        retry_index,
    );
    assign_winds(climate, winds);
    let surface_centi = climate.temperature_centi_c.clone();
    let (current_east, current_north) = derive_currents_with(
        field,
        climate.planetary,
        &surface_centi,
        &climate.wind_east_milli,
        &climate.wind_north_milli,
        current_geometry,
    );
    climate.current_east_milli = current_east;
    climate.current_north_milli = current_north;
    product_moisture(
        field,
        settings,
        climate,
        seed,
        retry_index,
        transport_iterations,
        current_geometry,
        progress,
    )
}

fn clamp_wind(value: f64) -> i32 {
    value
        .round()
        .clamp(-f64::from(MAX_WIND_MILLI), f64::from(MAX_WIND_MILLI)) as i32
}

fn clamp_current(value: f64) -> i32 {
    value
        .round()
        .clamp(-f64::from(MAX_CURRENT_MILLI), f64::from(MAX_CURRENT_MILLI)) as i32
}

fn omega_ratio(planetary: PlanetaryConfiguration) -> f64 {
    f64::from(EARTH_ROTATION_PERIOD_SECONDS) / f64::from(planetary.rotation_period_seconds.max(1))
}

fn hadley_edge_radians(omega: f64) -> f64 {
    (30.0 / omega.max(0.04).sqrt())
        .clamp(12.0, 80.0)
        .to_radians()
}

fn ferrel_edge_radians(hadley: f64) -> f64 {
    let pole = std::f64::consts::FRAC_PI_2;
    (hadley + (pole - hadley) * 0.55).clamp(hadley + 0.08, pole - 0.04)
}

fn thermal_equator_latitude(grid: Grid, temperatures: &[i32]) -> f64 {
    let mut best = i64::MIN;
    let mut latitude = 0.0;
    for row in 0..grid.height {
        let mut sum = 0i64;
        for col in 0..grid.width {
            sum += i64::from(temperatures[grid.index(row, col)]);
        }
        if sum > best {
            best = sum;
            latitude = grid.center_radians(row, 0).1;
        }
    }
    latitude
}

fn thermal_equator_by_column(grid: Grid, temperatures: &[i32]) -> Vec<f64> {
    let width = grid.width as usize;
    let mut raw = vec![0.0; width];
    for col in 0..grid.width {
        let mut best = i64::MIN;
        let mut latitude = 0.0;
        for row in 0..grid.height {
            let temperature = i64::from(temperatures[grid.index(row, col)]);
            if temperature > best {
                best = temperature;
                latitude = grid.center_radians(row, col).1;
            }
        }
        raw[col as usize] = latitude;
    }
    let mut smooth = vec![0.0; width];
    for col in 0..width {
        let prev = raw[(col + width - 1) % width];
        let next = raw[(col + 1) % width];
        smooth[col] = (prev + raw[col] * 2.0 + next) / 4.0;
    }
    smooth
}

fn circulation_at(latitude: f64, itcz: f64, hadley: f64, ferrel: f64) -> (u32, f64, f64) {
    let phi = latitude - itcz;
    let abs_phi = phi.abs();
    let toward_itcz = if phi >= 0.0 { -1.0 } else { 1.0 };
    if abs_phi <= hadley {
        let gap = (abs_phi / hadley.max(1e-6)).clamp(0.0, 1.0);
        (WIND_BAND_HADLEY, -1.0, toward_itcz * gap)
    } else if abs_phi <= ferrel {
        (WIND_BAND_FERREL, 1.0, -toward_itcz)
    } else {
        (WIND_BAND_POLAR, -1.0, toward_itcz)
    }
}

fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    if edge1 <= edge0 {
        return if x >= edge1 { 1.0 } else { 0.0 };
    }
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn circulation_flow(latitude: f64, itcz: f64, hadley: f64, ferrel: f64) -> (f64, f64) {
    let phi = latitude - itcz;
    let abs_phi = phi.abs();
    let toward_itcz = if phi >= 0.0 { -1.0 } else { 1.0 };
    let width = 0.16;
    let hadley_w = 1.0 - smoothstep(hadley - width, hadley + width, abs_phi);
    let polar_w = smoothstep(ferrel - width, ferrel + width, abs_phi);
    let ferrel_w = (1.0 - hadley_w - polar_w).max(0.0);
    let gap = (abs_phi / hadley.max(1e-6)).clamp(0.0, 1.0);
    (
        -hadley_w + ferrel_w - polar_w,
        hadley_w * toward_itcz * gap + ferrel_w * (-toward_itcz) + polar_w * toward_itcz,
    )
}

fn wrapped_col(grid: Grid, col: u32, delta: i32) -> u32 {
    (col as i32 + delta).rem_euclid(grid.width as i32) as u32
}

fn clamped_row(grid: Grid, row: u32, delta: i32) -> u32 {
    (row as i32 + delta).clamp(0, grid.height as i32 - 1) as u32
}

fn neighbor_metres(grid: Grid, row: u32, col: u32, drow: i32, dcol: i32) -> f64 {
    let other_row = clamped_row(grid, row, drow);
    let other_col = wrapped_col(grid, col, dcol);
    grid.great_circle_distance(
        grid.center_radians(row, col),
        grid.center_radians(other_row, other_col),
    )
    .max(1.0)
}

struct GridSpacing {
    zonal: Vec<f64>,
    south: Vec<f64>,
    north: Vec<f64>,
}

impl GridSpacing {
    fn new(grid: Grid) -> Self {
        let height = grid.height as usize;
        let mut zonal = vec![1.0; height];
        let mut south = vec![1.0; height];
        let mut north = vec![1.0; height];
        for row in 0..grid.height {
            zonal[row as usize] = neighbor_metres(grid, row, 0, 0, 1);
            south[row as usize] = neighbor_metres(grid, row, 0, -1, 0);
            north[row as usize] = neighbor_metres(grid, row, 0, 1, 0);
        }
        Self {
            zonal,
            south,
            north,
        }
    }

    fn spans(&self, row: u32) -> (f64, f64, f64, f64) {
        let row = row as usize;
        (
            self.zonal[row],
            self.zonal[row],
            self.south[row],
            self.north[row],
        )
    }

    fn scale_x(&self, row: u32) -> f64 {
        spacing_scale(self.zonal[row as usize])
    }

    fn scale_y(&self, row: u32) -> f64 {
        spacing_scale(self.north[row as usize])
    }
}

struct CurrentGeometry {
    ocean: Vec<bool>,
    west_distance: Vec<u32>,
    east_distance: Vec<u32>,
}

impl CurrentGeometry {
    fn new(field: &PhysicalField) -> Self {
        Self::from_ocean(field, ocean_mask(field))
    }

    fn from_ocean(field: &PhysicalField, ocean: Vec<bool>) -> Self {
        let (west_distance, east_distance) = fetch_distances_along_rows(field.grid, &ocean);
        Self {
            ocean,
            west_distance,
            east_distance,
        }
    }
}

fn spacing_scale(metres: f64) -> f64 {
    (WIND_GRADIENT_REF_METRES / metres.max(1.0)).clamp(0.25, 2.5)
}

fn is_ocean(field: &PhysicalField, cell: usize) -> bool {
    field.elevations_mm[cell] <= field.sea_level_mm
}

fn surface_height_m(field: &PhysicalField, cell: usize) -> f64 {
    f64::from((field.elevations_mm[cell] - field.sea_level_mm).max(0)) / 1_000.0
}

fn orographic_uplift(
    field: &PhysicalField,
    cell: usize,
    zonal_upstream: usize,
    meridional_upstream: usize,
    distance: f64,
    meridional_distance: f64,
    wind_east: f64,
    wind_north: f64,
) -> f64 {
    let height = surface_height_m(field, cell);
    let dh_zonal = (height - surface_height_m(field, zonal_upstream)) / distance;
    let dh_meridional =
        (height - surface_height_m(field, meridional_upstream)) / meridional_distance;
    let u = wind_east / f64::from(MAX_WIND_MILLI);
    let v = wind_north / f64::from(MAX_WIND_MILLI);
    (u.abs() * dh_zonal + v.abs() * dh_meridional).max(0.0)
}

fn orographic_saturation_centi_c(
    temperature_centi_c: i32,
    u_oro: f64,
    orographic_precipitation_ppm: u32,
) -> i32 {
    let t_drop_c = (u_oro * f64::from(orographic_precipitation_ppm) / OROGRAPHIC_KU_PPM_PER_C)
        .clamp(0.0, MAX_OROGRAPHIC_COOLING_C);
    (f64::from(temperature_centi_c) - t_drop_c * 100.0)
        .clamp(MAGNUS_T_MIN_C * 100.0, MAGNUS_T_MAX_C * 100.0)
        .round() as i32
}

fn smooth_scalar_field(grid: Grid, values: &mut [f64]) {
    if values.len() != grid.sample_count() {
        return;
    }
    let mut tmp = values.to_vec();
    for cell in 0..grid.sample_count() {
        let (row, col) = grid.row_col(cell);
        let west = values[grid.index(row, wrapped_col(grid, col, -1))];
        let east = values[grid.index(row, wrapped_col(grid, col, 1))];
        let south = values[grid.index(clamped_row(grid, row, -1), col)];
        let north = values[grid.index(clamped_row(grid, row, 1), col)];
        tmp[cell] = values[cell] * 0.62 + (west + east + south + north) * 0.095;
    }
    values.copy_from_slice(&tmp);
}

fn transport_iteration_limit(grid: Grid) -> u32 {
    grid.width
        .max(grid.height)
        .saturating_mul(2)
        .clamp(96, CLIMATE_MAX_TRANSPORT_ITERATIONS)
}

fn ocean_mask(field: &PhysicalField) -> Vec<bool> {
    let count = field.grid.sample_count();
    let wet: Vec<bool> = (0..count)
        .map(|cell| field.elevations_mm[cell] <= field.sea_level_mm)
        .collect();
    let mut seen = vec![false; count];
    let mut components = Vec::new();
    for start in 0..count {
        if !wet[start] || seen[start] {
            continue;
        }
        let mut stack = vec![start];
        let mut component = Vec::new();
        seen[start] = true;
        while let Some(cell) = stack.pop() {
            component.push(cell);
            let (row, col) = field.grid.row_col(cell);
            let neighbors = [
                field.grid.index(row, wrapped_col(field.grid, col, -1)),
                field.grid.index(row, wrapped_col(field.grid, col, 1)),
                field.grid.index(clamped_row(field.grid, row, -1), col),
                field.grid.index(clamped_row(field.grid, row, 1), col),
            ];
            for neighbor in neighbors {
                if wet[neighbor] && !seen[neighbor] {
                    seen[neighbor] = true;
                    stack.push(neighbor);
                }
            }
        }
        components.push(component);
    }
    let mut ocean = vec![false; count];
    for component in components {
        if component.len() >= MIN_CURRENT_BASIN_CELLS {
            for cell in component {
                ocean[cell] = true;
            }
        }
    }
    ocean
}

fn ocean_temperature(temperatures: &[i32], ocean: &[bool], cell: usize, fallback: i32) -> i32 {
    if ocean[cell] {
        temperatures[cell]
    } else {
        fallback
    }
}

fn coriolis_parameter(latitude: f64, planetary: PlanetaryConfiguration) -> f64 {
    let omega = std::f64::consts::TAU / f64::from(planetary.rotation_period_seconds.max(1));
    2.0 * omega * latitude.sin()
}

fn pressure_base(latitude: f64, itcz: f64, hadley: f64, ferrel: f64, amplitude: f64) -> f64 {
    let abs_phi = (latitude - itcz).abs();
    let pole = std::f64::consts::FRAC_PI_2;
    let width = 0.16;
    let hadley_w = 1.0 - smoothstep(hadley - width, hadley + width, abs_phi);
    let polar_w = smoothstep(ferrel - width, ferrel + width, abs_phi);
    let ferrel_w = (1.0 - hadley_w - polar_w).max(0.0);
    let hadley_frac = (abs_phi / hadley.max(1e-6)).clamp(0.0, 1.0);
    let ferrel_frac = ((abs_phi - hadley) / (ferrel - hadley).max(1e-6)).clamp(0.0, 1.0);
    let polar_frac = ((abs_phi - ferrel) / (pole - ferrel).max(1e-6)).clamp(0.0, 1.0);
    let hadley_p = -amplitude * (std::f64::consts::PI * hadley_frac).cos();
    let ferrel_p = amplitude * (std::f64::consts::PI * ferrel_frac).cos();
    let polar_p = -amplitude * (std::f64::consts::PI * polar_frac).cos();
    hadley_w * hadley_p + ferrel_w * ferrel_p + polar_w * polar_p
}

fn pressure_base_dlat(latitude: f64, itcz: f64, hadley: f64, ferrel: f64, amplitude: f64) -> f64 {
    let dlat = 0.01;
    (pressure_base(latitude + dlat, itcz, hadley, ferrel, amplitude)
        - pressure_base(latitude - dlat, itcz, hadley, ferrel, amplitude))
        / (2.0 * dlat)
}

fn pressure_anomaly(
    field: &PhysicalField,
    temperatures: &[i32],
    settings: ClimateSettings,
) -> Vec<f64> {
    let amplitude = f64::from(settings.pressure_cell_amplitude);
    let thermal = f64::from(settings.thermal_pressure_per_c);
    let scale_height = f64::from(settings.pressure_scale_height_m).max(1.0);
    let mut zonal_mean = vec![0.0_f64; field.grid.height as usize];
    let mut zonal_count = vec![0.0_f64; field.grid.height as usize];
    for cell in 0..field.grid.sample_count() {
        let (row, _) = field.grid.row_col(cell);
        zonal_mean[row as usize] += f64::from(temperatures[cell]);
        zonal_count[row as usize] += 1.0;
    }
    for row in 0..field.grid.height as usize {
        zonal_mean[row] /= zonal_count[row].max(1.0);
    }
    let mut pressure = vec![0.0; field.grid.sample_count()];
    for cell in 0..field.grid.sample_count() {
        let (row, _) = field.grid.row_col(cell);
        let t_anomaly = (f64::from(temperatures[cell]) - zonal_mean[row as usize]) / 100.0;
        let elevation_m = surface_height_m(field, cell);
        let column = (-elevation_m / scale_height).exp();
        pressure[cell] = -thermal * t_anomaly + amplitude * (column - 1.0);
    }
    pressure
}

struct WindComponentInput<'a> {
    field: &'a PhysicalField,
    settings: ClimateSettings,
    anomaly: &'a [f64],
    spacing: &'a GridSpacing,
    itcz: f64,
    hadley: f64,
    ferrel: f64,
    amplitude: f64,
    radius: f64,
    waves: f64,
    phase: f64,
    wave_amp: f64,
    meanders: bool,
}

fn wind_component_cell(input: &WindComponentInput<'_>, cell: usize) -> (i32, i32) {
    let field = input.field;
    let (row, col) = field.grid.row_col(cell);
    let (longitude, latitude) = field.grid.center_radians(row, col);
    let west_cell = field.grid.index(row, wrapped_col(field.grid, col, -1));
    let east_cell = field.grid.index(row, wrapped_col(field.grid, col, 1));
    let south_cell = field.grid.index(clamped_row(field.grid, row, -1), col);
    let north_cell = field.grid.index(clamped_row(field.grid, row, 1), col);
    let (dx_w, dx_e, dy_s, dy_n) = input.spacing.spans(row);
    let dp_dx = (input.anomaly[east_cell] - input.anomaly[west_cell]) / (dx_e + dx_w);
    let dp_dy = pressure_base_dlat(
        latitude,
        input.itcz,
        input.hadley,
        input.ferrel,
        input.amplitude,
    ) / input.radius
        + (input.anomaly[north_cell] - input.anomaly[south_cell]) / (dy_n + dy_s);
    let fx = -dp_dx;
    let fy = -dp_dy;
    let elevation_km =
        (f64::from(field.elevations_mm[cell] - field.sea_level_mm) / 1_000_000.0).max(0.0);
    let drag = input
        .settings
        .drag_per_second(is_ocean(field, cell), elevation_km)
        .max(1e-6);
    let coriolis =
        coriolis_parameter(latitude, input.settings.planetary) * input.settings.coriolis_scale();
    let denom = drag * drag + coriolis * coriolis;
    let u_ms = (drag * fx + coriolis * fy) / denom;
    let v_ms = (-coriolis * fx + drag * fy) / denom;
    let mut u = u_ms * 1_000.0;
    let mut v = v_ms * 1_000.0;
    if input.meanders {
        let zonal_sign = circulation_flow(latitude, input.itcz, input.hadley, input.ferrel).0;
        let envelope = zonal_sign.max(0.0) + (-zonal_sign).max(0.0) * 0.22;
        let theta = input.waves * longitude + input.phase;
        let theta2 = (input.waves * 0.5 + 1.0) * longitude + input.phase * 1.73;
        let mut du = envelope * input.wave_amp * (theta.sin() + 0.38 * theta2.cos());
        let mut dv = envelope * input.wave_amp * (1.2 * theta.cos() + 0.45 * theta2.sin());
        let cap = 0.42 * u.hypot(v).max(450.0);
        let perturb = du.hypot(dv);
        if perturb > cap && perturb > 0.0 {
            du *= cap / perturb;
            dv *= cap / perturb;
        }
        u += du;
        v += dv;
    }
    let blocking = 1.0 / (1.0 + elevation_km * 0.6);
    let roughness = if is_ocean(field, cell) { 1.0 } else { 0.86 };
    (
        clamp_wind(u * blocking * roughness),
        clamp_wind(v * roughness),
    )
}

fn wind_components(
    field: &PhysicalField,
    temperatures: &[i32],
    settings: ClimateSettings,
    itcz: f64,
    hadley: f64,
    ferrel: f64,
    wind_seed: u64,
    meanders: bool,
) -> (Vec<i32>, Vec<i32>) {
    let anomaly = pressure_anomaly(field, temperatures, settings);
    let omega = omega_ratio(settings.planetary);
    let amplitude = f64::from(settings.pressure_cell_amplitude);
    let radius = (field.grid.radius_metres as f64).max(1.0);
    let waves = (2.0 + 3.2 * omega.clamp(0.2, 2.4)).clamp(2.0, 8.0);
    let phase = (wind_seed as f64) * (std::f64::consts::TAU / (u64::MAX as f64));
    let wave_amp = 340.0 * omega.clamp(0.3, 2.0).sqrt();
    let spacing = GridSpacing::new(field.grid);
    let input = WindComponentInput {
        field,
        settings,
        anomaly: &anomaly,
        spacing: &spacing,
        itcz,
        hadley,
        ferrel,
        amplitude,
        radius,
        waves,
        phase,
        wave_amp,
        meanders,
    };
    let count = field.grid.sample_count();
    let mut east = vec![0; count];
    let mut north = vec![0; count];
    east.par_iter_mut()
        .zip(north.par_iter_mut())
        .enumerate()
        .for_each(|(cell, (east_slot, north_slot))| {
            let (east, north) = wind_component_cell(&input, cell);
            *east_slot = east;
            *north_slot = north;
        });
    (east, north)
}

fn wind_divergence_cell(
    grid: Grid,
    spacing: &GridSpacing,
    east: &[i32],
    north: &[i32],
    cell: usize,
) -> i32 {
    let (row, col) = grid.row_col(cell);
    let west = grid.index(row, wrapped_col(grid, col, -1));
    let east_cell = grid.index(row, wrapped_col(grid, col, 1));
    let south = grid.index(clamped_row(grid, row, -1), col);
    let north_cell = grid.index(clamped_row(grid, row, 1), col);
    let scale_x = spacing.scale_x(row);
    let scale_y = spacing.scale_y(row);
    let value = f64::from(east[east_cell] - east[west]) * scale_x
        + f64::from(north[north_cell] - north[south]) * scale_y;
    (value * 80.0).round().clamp(-1_000_000.0, 1_000_000.0) as i32
}

fn wind_divergence_ppm(grid: Grid, east: &[i32], north: &[i32]) -> Vec<i32> {
    let spacing = GridSpacing::new(grid);
    let mut divergence = vec![0; grid.sample_count()];
    divergence
        .par_iter_mut()
        .enumerate()
        .for_each(|(cell, slot)| {
            *slot = wind_divergence_cell(grid, &spacing, east, north, cell);
        });
    divergence
}

fn wind_band_field(grid: Grid, itcz: f64, hadley: f64, ferrel: f64) -> Vec<u32> {
    (0..grid.sample_count())
        .map(|cell| {
            let (row, col) = grid.row_col(cell);
            circulation_at(grid.center_radians(row, col).1, itcz, hadley, ferrel).0
        })
        .collect()
}

struct DerivedWinds {
    east: Vec<i32>,
    north: Vec<i32>,
    east_summer: Vec<i32>,
    north_summer: Vec<i32>,
    east_winter: Vec<i32>,
    north_winter: Vec<i32>,
    divergence_ppm: Vec<i32>,
    divergence_summer_ppm: Vec<i32>,
    divergence_winter_ppm: Vec<i32>,
    band: Vec<u32>,
    band_summer: Vec<u32>,
    band_winter: Vec<u32>,
}

fn derive_winds(
    field: &PhysicalField,
    settings: ClimateSettings,
    annual: &[i32],
    summer: &[i32],
    winter: &[i32],
    seed: u32,
    retry_index: u32,
) -> DerivedWinds {
    let omega = omega_ratio(settings.planetary);
    let hadley = hadley_edge_radians(omega);
    let ferrel = ferrel_edge_radians(hadley);
    let itcz = thermal_equator_latitude(field.grid, annual);
    let summer_itcz = thermal_equator_latitude(field.grid, summer);
    let winter_itcz = thermal_equator_latitude(field.grid, winter);
    let wind_seed = derive_subsystem_seed(seed, retry_index, SeedDomain::Climate);
    let (east, north) = wind_components(
        field, annual, settings, itcz, hadley, ferrel, wind_seed, true,
    );
    let (east_summer, north_summer) = wind_components(
        field,
        summer,
        settings,
        summer_itcz,
        hadley,
        ferrel,
        wind_seed,
        true,
    );
    let (east_winter, north_winter) = wind_components(
        field,
        winter,
        settings,
        winter_itcz,
        hadley,
        ferrel,
        wind_seed,
        true,
    );
    let divergence_ppm = wind_divergence_ppm(field.grid, &east, &north);
    let divergence_summer_ppm = wind_divergence_ppm(field.grid, &east_summer, &north_summer);
    let divergence_winter_ppm = wind_divergence_ppm(field.grid, &east_winter, &north_winter);
    let band = wind_band_field(field.grid, itcz, hadley, ferrel);
    let band_summer = wind_band_field(field.grid, summer_itcz, hadley, ferrel);
    let band_winter = wind_band_field(field.grid, winter_itcz, hadley, ferrel);
    DerivedWinds {
        east,
        north,
        east_summer,
        north_summer,
        east_winter,
        north_winter,
        divergence_ppm,
        divergence_summer_ppm,
        divergence_winter_ppm,
        band,
        band_summer,
        band_winter,
    }
}

fn stamp_currents(
    climate: &mut ClimateField,
    field: &PhysicalField,
    temperatures: &[i32],
    current_geometry: &CurrentGeometry,
) {
    let (east, north) = derive_currents_with(
        field,
        climate.planetary,
        temperatures,
        &climate.wind_east_milli,
        &climate.wind_north_milli,
        current_geometry,
    );
    climate.current_east_milli = east;
    climate.current_north_milli = north;
    apply_current_metrics(
        field,
        &climate.current_east_milli,
        &climate.current_north_milli,
        &mut climate.metrics,
    );
}

fn fetch_distances_along_rows(grid: Grid, ocean: &[bool]) -> (Vec<u32>, Vec<u32>) {
    let width = grid.width as usize;
    let count = grid.sample_count();
    let mut west = vec![0u32; count];
    let mut east = vec![0u32; count];
    if width == 0 {
        return (west, east);
    }
    let wrap = width.saturating_mul(2);
    for row in 0..grid.height as usize {
        let start = row * width;
        let row_ocean = &ocean[start..start + width];
        if row_ocean.iter().all(|&cell| cell) {
            let full = (width as u32).saturating_sub(1);
            west[start..start + width].fill(full);
            east[start..start + width].fill(full);
            continue;
        }
        let mut run = 0u32;
        for i in 0..wrap {
            let cell = start + i % width;
            if ocean[cell] {
                west[cell] = run;
                run += 1;
            } else {
                west[cell] = 0;
                run = 0;
            }
        }
        run = 0;
        for i in (0..wrap).rev() {
            let cell = start + i % width;
            if ocean[cell] {
                east[cell] = run;
                run += 1;
            } else {
                east[cell] = 0;
                run = 0;
            }
        }
    }
    (west, east)
}

#[cfg(test)]
fn derive_currents(
    field: &PhysicalField,
    planetary: PlanetaryConfiguration,
    temperatures: &[i32],
    wind_east: &[i32],
    wind_north: &[i32],
) -> (Vec<i32>, Vec<i32>) {
    let geometry = CurrentGeometry::new(field);
    derive_currents_with(
        field,
        planetary,
        temperatures,
        wind_east,
        wind_north,
        &geometry,
    )
}

struct CurrentForceInput<'a> {
    field: &'a PhysicalField,
    ocean: &'a [bool],
    temperatures: &'a [i32],
    wind_east: &'a [i32],
    wind_north: &'a [i32],
    spacing: &'a GridSpacing,
    west_distance: &'a [u32],
    east_distance: &'a [u32],
    omega: f64,
    wind_scale: f64,
}

fn current_force_cell(input: &CurrentForceInput, cell: usize) -> (f64, f64) {
    if !input.ocean[cell] {
        return (0.0, 0.0);
    }
    let field = input.field;
    let (row, col) = field.grid.row_col(cell);
    let latitude = field.grid.center_radians(row, col).1;
    let coriolis = (latitude.sin() * input.omega).clamp(-2.4, 2.4);
    let turn = 1.08 * coriolis.tanh();
    let (sin, cos) = turn.sin_cos();
    let wind_u = f64::from(input.wind_east[cell]);
    let wind_v = f64::from(input.wind_north[cell]);
    let mut u = input.wind_scale * (wind_u * cos + wind_v * sin);
    let mut v = input.wind_scale * (-wind_u * sin + wind_v * cos);
    let west_cell = field.grid.index(row, wrapped_col(field.grid, col, -1));
    let east_cell = field.grid.index(row, wrapped_col(field.grid, col, 1));
    let south_cell = field.grid.index(clamped_row(field.grid, row, -1), col);
    let north_cell = field.grid.index(clamped_row(field.grid, row, 1), col);
    let scale_x = input.spacing.scale_x(row);
    let scale_y = input.spacing.scale_y(row);
    let here = input.temperatures[cell];
    let dtx = f64::from(
        ocean_temperature(input.temperatures, input.ocean, east_cell, here)
            - ocean_temperature(input.temperatures, input.ocean, west_cell, here),
    ) * scale_x;
    let dty = f64::from(
        ocean_temperature(input.temperatures, input.ocean, north_cell, here)
            - ocean_temperature(input.temperatures, input.ocean, south_cell, here),
    ) * scale_y;
    let geostrophy = CURRENT_GEOSTROPHY * (2.2 * coriolis.abs()).clamp(0.0, 1.0);
    let sign = if coriolis == 0.0 {
        0.0
    } else {
        coriolis.signum()
    };
    u += geostrophy * (-dty) * sign;
    v += geostrophy * dtx * sign;
    let d_u_dy = f64::from(input.wind_east[north_cell] - input.wind_east[south_cell]) * scale_y;
    let wind_curl = -d_u_dy;
    let beta = latitude.cos().abs().max(0.2);
    let sverdrup = CURRENT_SVERDRUP * input.omega.clamp(0.2, 2.2).sqrt() * wind_curl / beta;
    v += sverdrup;
    let dist_west = input.west_distance[cell];
    let dist_east = input.east_distance[cell];
    if dist_west <= dist_east && dist_west + dist_east + 1 < field.grid.width {
        let boost =
            1.0 + CURRENT_WESTERN_RETURN * (-f64::from(dist_west) / CURRENT_WESTERN_SCALE).exp();
        v -= sverdrup * boost;
        u *= 0.62 + 0.38 / boost;
    }
    (u, v)
}

fn current_smooth_cell(
    grid: Grid,
    ocean: &[bool],
    previous_east: &[f64],
    previous_north: &[f64],
    cell: usize,
) -> (f64, f64) {
    if !ocean[cell] {
        return (0.0, 0.0);
    }
    let (row, col) = grid.row_col(cell);
    let neighbors = [
        grid.index(row, wrapped_col(grid, col, -1)),
        grid.index(row, wrapped_col(grid, col, 1)),
        grid.index(clamped_row(grid, row, -1), col),
        grid.index(clamped_row(grid, row, 1), col),
    ];
    let mut sum_u = previous_east[cell] * 2.0;
    let mut sum_v = previous_north[cell] * 2.0;
    let mut weight = 2.0;
    for neighbor in neighbors {
        if ocean[neighbor] {
            sum_u += previous_east[neighbor];
            sum_v += previous_north[neighbor];
            weight += 1.0;
        }
    }
    (sum_u / weight, sum_v / weight)
}

fn current_bound_cell(
    grid: Grid,
    ocean: &[bool],
    east: f64,
    north: f64,
    cell: usize,
) -> (f64, f64) {
    if !ocean[cell] {
        return (0.0, 0.0);
    }
    let (row, col) = grid.row_col(cell);
    let west_cell = grid.index(row, wrapped_col(grid, col, -1));
    let east_cell = grid.index(row, wrapped_col(grid, col, 1));
    let south_cell = grid.index(clamped_row(grid, row, -1), col);
    let north_cell = grid.index(clamped_row(grid, row, 1), col);
    let mut east = east;
    let mut north = north;
    if !ocean[east_cell] {
        east = east.min(0.0);
    }
    if !ocean[west_cell] {
        east = east.max(0.0);
    }
    if !ocean[north_cell] {
        north = north.min(0.0);
    }
    if !ocean[south_cell] {
        north = north.max(0.0);
    }
    (east, north)
}

fn derive_currents_with(
    field: &PhysicalField,
    planetary: PlanetaryConfiguration,
    temperatures: &[i32],
    wind_east: &[i32],
    wind_north: &[i32],
    geometry: &CurrentGeometry,
) -> (Vec<i32>, Vec<i32>) {
    let omega = omega_ratio(planetary);
    let count = field.grid.sample_count();
    let ocean = &geometry.ocean;
    let mut east = vec![0.0; count];
    let mut north = vec![0.0; count];
    let spacing = GridSpacing::new(field.grid);
    let wind_scale = CURRENT_WIND_COUPLING * omega.clamp(0.2, 2.2).sqrt();
    let input = CurrentForceInput {
        field,
        ocean,
        temperatures,
        wind_east,
        wind_north,
        spacing: &spacing,
        west_distance: &geometry.west_distance,
        east_distance: &geometry.east_distance,
        omega,
        wind_scale,
    };
    east.par_iter_mut()
        .zip(north.par_iter_mut())
        .enumerate()
        .for_each(|(cell, (east_slot, north_slot))| {
            let (u, v) = current_force_cell(&input, cell);
            *east_slot = u;
            *north_slot = v;
        });
    for _ in 0..CURRENT_SMOOTH_PASSES {
        let previous_east = east.clone();
        let previous_north = north.clone();
        east.par_iter_mut()
            .zip(north.par_iter_mut())
            .enumerate()
            .for_each(|(cell, (east_slot, north_slot))| {
                let (u, v) =
                    current_smooth_cell(field.grid, ocean, &previous_east, &previous_north, cell);
                *east_slot = u;
                *north_slot = v;
            });
    }
    east.par_iter_mut()
        .zip(north.par_iter_mut())
        .enumerate()
        .for_each(|(cell, (east_slot, north_slot))| {
            let (u, v) = current_bound_cell(field.grid, ocean, *east_slot, *north_slot, cell);
            *east_slot = u;
            *north_slot = v;
        });
    (
        east.into_iter().map(clamp_current).collect(),
        north.into_iter().map(clamp_current).collect(),
    )
}

fn hydrology_parameters(preset: HydrologyPreset) -> (f64, f64, f64, f64) {
    match preset {
        HydrologyPreset::Arid => (0.75, 0.90, 0.24, 0.18),
        HydrologyPreset::Balanced => (1.0, 1.0, 0.40, 0.28),
        HydrologyPreset::Wet => (1.15, 1.015, 0.56, 0.34),
    }
}

fn land_runoff_coefficient(
    settings: ClimateSettings,
    temperature_centi_c: i32,
    precipitation_mm: f64,
    is_land: bool,
) -> f64 {
    if !is_land {
        return 0.0;
    }
    let (_, _, base_runoff, runoff_response) = hydrology_parameters(settings.hydrology_preset);
    let wetness = (precipitation_mm / 1_500.0).clamp(0.0, 1.0);
    let temperature_factor = if temperature_centi_c < 0 { 0.72 } else { 1.0 };
    ((base_runoff + wetness * runoff_response) * temperature_factor).clamp(0.0, 0.95)
}

fn close_surface_water_mm(
    settings: ClimateSettings,
    is_land: bool,
    temperature_centi_c: i32,
    precipitation_mm: f64,
    evaporation_mm: f64,
    water_mm: f64,
) -> f64 {
    if !is_land || temperature_centi_c <= 0 {
        return 0.0;
    }
    let runoff_mm = precipitation_mm
        * land_runoff_coefficient(settings, temperature_centi_c, precipitation_mm, true);
    (water_mm + precipitation_mm - evaporation_mm - runoff_mm)
        .clamp(0.0, f64::from(MAX_CLIMATE_MOISTURE_MM))
}

fn band_source_multiplier(seed: u64, band: u32, southern: bool) -> f64 {
    let token = band.saturating_add(if southern { CLIMATE_WIND_BAND_COUNT } else { 0 });
    let random = splitmix64(seed ^ u64::from(token).wrapping_mul(0x9e37_79b9_7f4a_7c15));
    0.90 + f64::from((random % 200_001) as u32) / 1_000_000.0
}

struct MoistureBundle {
    moisture: Vec<u32>,
    precipitation: Vec<u32>,
    precipitation_summer: Vec<u32>,
    precipitation_winter: Vec<u32>,
    humidity_ppm: Vec<u32>,
    aridity_ppm: Vec<u32>,
    iterations: u32,
}

struct MoistureTransport {
    moisture: Vec<u32>,
    precipitation: Vec<u32>,
    condensation_mm: Vec<f64>,
    evaporation_mm: Vec<f64>,
    water_mm: Vec<f64>,
    cloud: Vec<f64>,
    iterations: u32,
}

fn magnus_saturation_mm(t_c: f64) -> f64 {
    let t = t_c.clamp(MAGNUS_T_MIN_C, MAGNUS_T_MAX_C);
    let es = 6.112 * (17.67 * t / (t + 243.5)).exp();
    (SATURATION_MOISTURE_PER_HPA * es).clamp(80.0, f64::from(MAX_CLIMATE_MOISTURE_MM))
}

fn saturation_lut() -> &'static [f64] {
    static LUT: OnceLock<Vec<f64>> = OnceLock::new();
    LUT.get_or_init(|| {
        let count = (SATURATION_LUT_MAX_CENTI - SATURATION_LUT_MIN_CENTI + 1) as usize;
        (0..count)
            .map(|index| {
                magnus_saturation_mm(f64::from(SATURATION_LUT_MIN_CENTI + index as i32) / 100.0)
            })
            .collect()
    })
}

fn saturation_moisture_mm(temperature_centi_c: i32) -> f64 {
    let t = temperature_centi_c.clamp(SATURATION_LUT_MIN_CENTI, SATURATION_LUT_MAX_CENTI);
    saturation_lut()[(t - SATURATION_LUT_MIN_CENTI) as usize]
}

fn potential_evapotranspiration_mm(temperature_centi_c: i32) -> f64 {
    let t = f64::from(temperature_centi_c) / 100.0;
    if t <= 0.0 {
        40.0
    } else {
        (40.0 + 42.0 * t).clamp(40.0, 4_000.0)
    }
}

fn moisture_temperature_factor(temperature_centi_c: i32) -> f64 {
    (1.0 + f64::from(temperature_centi_c - SST_REFERENCE_CENTI_C) / 4_500.0).clamp(0.35, 1.75)
}

fn ocean_evaporation_mm(
    settings: ClimateSettings,
    source_multiplier: f64,
    source_factor: f64,
    temperature_centi_c: i32,
    current_east: i32,
    current_north: i32,
) -> f64 {
    let frozen = if temperature_centi_c < 0 { 0.18 } else { 1.0 };
    let sst = moisture_temperature_factor(temperature_centi_c);
    let east = f64::from(current_east);
    let north = f64::from(current_north);
    let speed = (east * east + north * north).sqrt() / f64::from(MAX_CURRENT_MILLI);
    let current = 1.0 + 0.20 * speed.clamp(0.0, 1.0);
    f64::from(settings.ocean_moisture_mm_per_year)
        * source_multiplier
        * source_factor
        * frozen
        * sst
        * current
}

fn land_evaporation_mm(
    settings: ClimateSettings,
    source_multiplier: f64,
    source_factor: f64,
    temperature_centi_c: i32,
    moisture_mm: f64,
    water_mm: f64,
    wind_east: i32,
    wind_north: i32,
) -> f64 {
    if temperature_centi_c <= 0 || water_mm <= 0.0 {
        return 0.0;
    }
    let sat = saturation_moisture_mm(temperature_centi_c).max(1.0);
    let rh = (moisture_mm / sat).clamp(0.0, 1.0);
    let east = f64::from(wind_east);
    let north = f64::from(wind_north);
    let wind = (east * east + north * north).sqrt() / f64::from(MAX_WIND_MILLI);
    let wind_factor = 0.45 + 0.55 * wind.clamp(0.0, 1.0);
    let potential = f64::from(settings.ocean_moisture_mm_per_year)
        * source_multiplier
        * source_factor
        * moisture_temperature_factor(temperature_centi_c)
        * (1.0 - rh)
        * wind_factor;
    if potential <= 0.0 {
        0.0
    } else {
        potential * water_mm / (water_mm + potential)
    }
}

fn humidity_and_aridity(
    temperatures: &[i32],
    moisture: &[u32],
    precipitation: &[u32],
) -> (Vec<u32>, Vec<u32>) {
    let humidity = moisture
        .iter()
        .zip(temperatures)
        .map(|(value, temperature)| {
            let sat = saturation_moisture_mm(*temperature).max(1.0);
            let moisture = f64::from(*value);
            ((moisture / sat) * 1_000_000.0)
                .round()
                .clamp(0.0, 1_000_000.0) as u32
        })
        .collect::<Vec<_>>();
    let aridity = precipitation
        .iter()
        .zip(temperatures)
        .map(|(value, temperature)| {
            let pet = potential_evapotranspiration_mm(*temperature).max(1.0);
            let wetness = (f64::from(*value) / pet).clamp(0.0, 1.0);
            ((1.0 - wetness) * 1_000_000.0)
                .round()
                .clamp(0.0, 1_000_000.0) as u32
        })
        .collect::<Vec<_>>();
    (humidity, aridity)
}

#[cfg(test)]
fn transport_moisture(
    field: &PhysicalField,
    settings: ClimateSettings,
    climate_seed: u64,
    temperatures: &[i32],
    current_east: &[i32],
    current_north: &[i32],
    wind_east: &[i32],
    wind_north: &[i32],
    wind_divergence_ppm: &[i32],
    wind_band: &[u32],
    progress: &mut dyn ProgressSink,
) -> Result<MoistureTransport, PhysicalError> {
    transport_moisture_state(
        field,
        settings,
        climate_seed,
        temperatures,
        current_east,
        current_north,
        wind_east,
        wind_north,
        wind_divergence_ppm,
        wind_band,
        None,
        None,
        progress,
    )
}

struct MoistureJacobiRow<'a> {
    next: &'a mut [f64],
    next_water: &'a mut [f64],
    precipitation: &'a mut [f64],
    condensation: &'a mut [f64],
    evaporation: &'a mut [f64],
    cloud: &'a mut [f64],
}

fn moisture_jacobi_row_slots<'a>(
    width: usize,
    next: &'a mut [f64],
    next_water: &'a mut [f64],
    precipitation: &'a mut [f64],
    condensation: &'a mut [f64],
    evaporation: &'a mut [f64],
    cloud: &'a mut [f64],
) -> Vec<MoistureJacobiRow<'a>> {
    next.chunks_mut(width)
        .zip(next_water.chunks_mut(width))
        .zip(precipitation.chunks_mut(width))
        .zip(condensation.chunks_mut(width))
        .zip(evaporation.chunks_mut(width))
        .zip(cloud.chunks_mut(width))
        .map(
            |(((((next, next_water), precipitation), condensation), evaporation), cloud)| {
                MoistureJacobiRow {
                    next,
                    next_water,
                    precipitation,
                    condensation,
                    evaporation,
                    cloud,
                }
            },
        )
        .collect()
}

struct MoistureJacobiInput<'a> {
    field: &'a PhysicalField,
    settings: ClimateSettings,
    climate_seed: u64,
    temperatures: &'a [i32],
    current_east: &'a [i32],
    current_north: &'a [i32],
    wind_east: &'a [i32],
    wind_north: &'a [i32],
    wind_divergence_ppm: &'a [i32],
    wind_band: &'a [u32],
    previous: &'a [f64],
    water: &'a [f64],
    row_step_metres: &'a [f64],
    meridional_step_metres: &'a [f64],
    source_multiplier: f64,
    decay_at_scale: f64,
    convergence: f64,
    condensation_k: f64,
}

fn transport_moisture_row(
    input: &MoistureJacobiInput,
    row: u32,
    out: &mut MoistureJacobiRow,
) -> f64 {
    let field = input.field;
    let settings = input.settings;
    let source_factor = band_source_multiplier(
        input.climate_seed,
        input.wind_band[field.grid.index(row, 0)],
        row < field.grid.height / 2,
    );
    let distance = input.row_step_metres[row as usize];
    let meridional_distance = input.meridional_step_metres[row as usize];
    let scale_m = f64::from(settings.moisture_decay_scale_km) * 1_000.0;
    let zonal_weight = (input.decay_at_scale * (-distance / scale_m).exp()).clamp(0.0, 0.995);
    let meridional_weight =
        (input.decay_at_scale * (-meridional_distance / scale_m).exp()).clamp(0.0, 0.995);
    let mut maximum_delta = 0.0_f64;
    for col in 0..field.grid.width {
        let cell = field.grid.index(row, col);
        let col_i = col as usize;
        let east = f64::from(input.wind_east[cell]);
        let north = f64::from(input.wind_north[cell]);
        let axis_sum = east.abs() + north.abs();
        let zonal_frac = if axis_sum <= 0.0 {
            0.5
        } else {
            east.abs() / axis_sum
        };
        let meridional_frac = 1.0 - zonal_frac;
        let upstream_col = wrapped_col(field.grid, col, if east >= 0.0 { -1 } else { 1 });
        let upstream_row = clamped_row(field.grid, row, if north >= 0.0 { -1 } else { 1 });
        let zonal_upstream = field.grid.index(row, upstream_col);
        let meridional_upstream = field.grid.index(upstream_row, col);
        let west_cell = field.grid.index(row, wrapped_col(field.grid, col, -1));
        let east_cell = field.grid.index(row, wrapped_col(field.grid, col, 1));
        let south_cell = field.grid.index(clamped_row(field.grid, row, -1), col);
        let north_cell = field.grid.index(clamped_row(field.grid, row, 1), col);
        let is_land = field.elevations_mm[cell] > field.sea_level_mm;
        let target_evaporation = if !is_land {
            ocean_evaporation_mm(
                settings,
                input.source_multiplier,
                source_factor,
                input.temperatures[cell],
                input.current_east[cell],
                input.current_north[cell],
            )
        } else {
            land_evaporation_mm(
                settings,
                input.source_multiplier,
                source_factor,
                input.temperatures[cell],
                input.previous[cell],
                input.water[cell],
                input.wind_east[cell],
                input.wind_north[cell],
            )
        };
        let cell_evaporation = out.evaporation[col_i] * (1.0 - CLIMATE_TRANSPORT_RELAXATION)
            + target_evaporation * CLIMATE_TRANSPORT_RELAXATION;
        let advected = input.previous[zonal_upstream] * zonal_weight * zonal_frac
            + input.previous[meridional_upstream] * meridional_weight * meridional_frac;
        let neighbor_mean = (input.previous[west_cell]
            + input.previous[east_cell]
            + input.previous[south_cell]
            + input.previous[north_cell])
            / 4.0;
        let spread_weight = zonal_weight * zonal_frac + meridional_weight * meridional_frac;
        let directed = ((east * east + north * north).sqrt() / 700.0).clamp(0.0, 1.0);
        let mix = 0.12 + 0.28 * (1.0 - directed);
        let incoming =
            (cell_evaporation + advected * (1.0 - mix) + neighbor_mean * spread_weight * mix)
                .clamp(0.0, f64::from(MAX_CLIMATE_MOISTURE_MM));
        let u_oro = orographic_uplift(
            field,
            cell,
            zonal_upstream,
            meridional_upstream,
            distance,
            meridional_distance,
            east,
            north,
        );
        let q_sat = saturation_moisture_mm(orographic_saturation_centi_c(
            input.temperatures[cell],
            u_oro,
            settings.orographic_precipitation_ppm,
        ));
        let cell_cloud = cloud_fraction(incoming, q_sat, u_oro);
        let convergence_boost = if input.wind_divergence_ppm[cell] < 0 {
            input.convergence
                * (-f64::from(input.wind_divergence_ppm[cell]) / 1_000_000.0).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let path_m = zonal_frac * distance + meridional_frac * meridional_distance;
        let path_scale = (path_m / PRECIPITATION_PATH_REF_METRES).clamp(0.05, 2.0);
        let condensed = input.condensation_k
            * (incoming - q_sat).max(0.0)
            * settings.cloud_rain_scale(cell_cloud);
        let after_condensation = (incoming - condensed).max(0.0);
        let leftover_fraction = (convergence_boost * path_scale).clamp(0.0, 0.65);
        let leftover_rain = after_condensation * leftover_fraction;
        let precipitated = condensed + leftover_rain;
        let remaining_q = (after_condensation - leftover_rain).max(0.0);
        let remaining = input.previous[cell] * (1.0 - CLIMATE_TRANSPORT_RELAXATION)
            + remaining_q * CLIMATE_TRANSPORT_RELAXATION;
        out.next[col_i] = remaining;
        out.precipitation[col_i] = precipitated * CLIMATE_TRANSPORT_RELAXATION
            + out.precipitation[col_i] * (1.0 - CLIMATE_TRANSPORT_RELAXATION);
        out.condensation[col_i] = condensed * CLIMATE_TRANSPORT_RELAXATION
            + out.condensation[col_i] * (1.0 - CLIMATE_TRANSPORT_RELAXATION);
        out.evaporation[col_i] = cell_evaporation * CLIMATE_TRANSPORT_RELAXATION
            + out.evaporation[col_i] * (1.0 - CLIMATE_TRANSPORT_RELAXATION);
        out.cloud[col_i] = cell_cloud * CLIMATE_TRANSPORT_RELAXATION
            + out.cloud[col_i] * (1.0 - CLIMATE_TRANSPORT_RELAXATION);
        let target_water = close_surface_water_mm(
            settings,
            is_land,
            input.temperatures[cell],
            out.precipitation[col_i],
            out.evaporation[col_i],
            input.water[cell],
        );
        out.next_water[col_i] = (input.water[cell] * (1.0 - CLIMATE_TRANSPORT_RELAXATION)
            + target_water * CLIMATE_TRANSPORT_RELAXATION)
            .clamp(0.0, f64::from(MAX_CLIMATE_MOISTURE_MM));
        maximum_delta = maximum_delta.max((remaining - input.previous[cell]).abs());
    }
    maximum_delta
}

fn transport_moisture_state(
    field: &PhysicalField,
    settings: ClimateSettings,
    climate_seed: u64,
    temperatures: &[i32],
    current_east: &[i32],
    current_north: &[i32],
    wind_east: &[i32],
    wind_north: &[i32],
    wind_divergence_ppm: &[i32],
    wind_band: &[u32],
    initial_q: Option<&[f64]>,
    initial_water: Option<&[f64]>,
    progress: &mut dyn ProgressSink,
) -> Result<MoistureTransport, PhysicalError> {
    let (source_multiplier, decay_multiplier, _, _) =
        hydrology_parameters(settings.hydrology_preset);
    let decay_at_scale = f64::from(settings.moisture_decay_ppm) / 1_000_000.0 * decay_multiplier;
    let convergence = f64::from(settings.convergence_ppm) / 1_000_000.0;
    let condensation_k = f64::from(settings.condensation_ppm) / 1_000_000.0;
    let sample_count = field.grid.sample_count();
    let mut previous = initial_q
        .map(|values| values.to_vec())
        .unwrap_or_else(|| vec![0.0; sample_count]);
    let mut next = vec![0.0; sample_count];
    let mut precipitation = vec![0.0; sample_count];
    let mut condensation = vec![0.0; sample_count];
    let mut evaporation = vec![0.0; sample_count];
    let mut cloud = vec![0.0; sample_count];
    let mut water = initial_water
        .map(|values| values.to_vec())
        .unwrap_or_else(|| vec![0.0; sample_count]);
    let mut next_water = vec![0.0; sample_count];
    let mut iterations = 0;
    let mut converged = false;
    let row_step_metres = (0..field.grid.height)
        .map(|row| {
            field
                .grid
                .great_circle_distance(
                    field.grid.center_radians(row, 0),
                    field.grid.center_radians(row, 1 % field.grid.width.max(1)),
                )
                .max(1.0)
        })
        .collect::<Vec<_>>();
    let meridional_step_metres = (0..field.grid.height)
        .map(|row| {
            let neighbor = if row + 1 < field.grid.height {
                row + 1
            } else {
                row.saturating_sub(1)
            };
            field
                .grid
                .great_circle_distance(
                    field.grid.center_radians(row, 0),
                    field.grid.center_radians(neighbor, 0),
                )
                .max(1.0)
        })
        .collect::<Vec<_>>();

    let iteration_limit = transport_iteration_limit(field.grid);
    let width = field.grid.width as usize;
    for iteration in 0..iteration_limit {
        progress.check_cancelled()?;
        let input = MoistureJacobiInput {
            field,
            settings,
            climate_seed,
            temperatures,
            current_east,
            current_north,
            wind_east,
            wind_north,
            wind_divergence_ppm,
            wind_band,
            previous: &previous,
            water: &water,
            row_step_metres: &row_step_metres,
            meridional_step_metres: &meridional_step_metres,
            source_multiplier,
            decay_at_scale,
            convergence,
            condensation_k,
        };
        let mut rows = moisture_jacobi_row_slots(
            width,
            &mut next,
            &mut next_water,
            &mut precipitation,
            &mut condensation,
            &mut evaporation,
            &mut cloud,
        );
        let maximum_delta = rows
            .par_iter_mut()
            .enumerate()
            .map(|(row, out)| transport_moisture_row(&input, row as u32, out))
            .reduce(|| 0.0, f64::max);
        std::mem::swap(&mut previous, &mut next);
        std::mem::swap(&mut water, &mut next_water);
        iterations = iteration + 1;
        if iterations >= CLIMATE_MIN_TRANSPORT_ITERATIONS
            && maximum_delta <= CLIMATE_TRANSPORT_TOLERANCE_MM
        {
            converged = true;
            break;
        }
    }
    if !converged {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::NumericNonConvergent,
            format!(
                "climate moisture transport did not converge within {iteration_limit} iterations"
            ),
        ));
    }
    smooth_scalar_field(field.grid, &mut precipitation);
    for cell in 0..sample_count {
        let is_land = field.elevations_mm[cell] > field.sea_level_mm;
        water[cell] = close_surface_water_mm(
            settings,
            is_land,
            temperatures[cell],
            precipitation[cell],
            evaporation[cell],
            water[cell],
        );
    }
    let moisture = previous
        .into_iter()
        .map(|value| value.round().clamp(0.0, f64::from(MAX_CLIMATE_MOISTURE_MM)) as u32)
        .collect::<Vec<_>>();
    let precipitation = precipitation
        .into_iter()
        .map(|value| {
            value
                .round()
                .clamp(0.0, f64::from(MAX_CLIMATE_PRECIPITATION_MM)) as u32
        })
        .collect::<Vec<_>>();
    Ok(MoistureTransport {
        moisture,
        precipitation,
        condensation_mm: condensation,
        evaporation_mm: evaporation,
        water_mm: water,
        cloud,
        iterations,
    })
}

fn runoff_fields(
    field: &PhysicalField,
    settings: ClimateSettings,
    temperatures: &[i32],
    precipitation: &[u32],
    progress: &mut dyn ProgressSink,
) -> Result<(Vec<u32>, Vec<u64>, ClimateMetrics), PhysicalError> {
    let total_area = field.grid.total_area();
    let mut runoff = Vec::with_capacity(field.grid.sample_count());
    let mut runoff_volume = Vec::with_capacity(field.grid.sample_count());
    let mut precipitation_volume = 0.0_f64;
    let mut runoff_volume_total = 0.0_f64;
    let mut temperature_area_sum = 0.0_f64;
    let mut precipitation_area_sum = 0.0_f64;
    let mut runoff_area_sum = 0.0_f64;
    let mut minimum_temperature = i32::MAX;
    let mut maximum_temperature = i32::MIN;
    let mut wettest = 0;
    let mut driest_land = u32::MAX;
    for cell in 0..field.grid.sample_count() {
        if cell % 128 == 0 {
            progress.check_cancelled()?;
        }
        let area = field.grid.cell_area(field.grid.row_col(cell).0);
        let precip = precipitation[cell];
        let precip_volume = area * f64::from(precip) / 1_000.0;
        let is_land = field.elevations_mm[cell] > field.sea_level_mm;
        let coefficient =
            land_runoff_coefficient(settings, temperatures[cell], f64::from(precip), is_land);
        let runoff_mm = (f64::from(precip) * coefficient)
            .round()
            .clamp(0.0, f64::from(MAX_CLIMATE_PRECIPITATION_MM)) as u32;
        let runoff_m3 = area * f64::from(runoff_mm) / 1_000.0;
        if !precip_volume.is_finite() || !runoff_m3.is_finite() {
            return Err(PhysicalError::coded(
                PhysicalErrorCode::NumericNonFinite,
                "climate precipitation or runoff volume is not finite",
            ));
        }
        precipitation_volume += precip_volume;
        runoff_volume_total += runoff_m3;
        temperature_area_sum += f64::from(temperatures[cell]) * area;
        precipitation_area_sum += f64::from(precip) * area;
        runoff_area_sum += f64::from(runoff_mm) * area;
        minimum_temperature = minimum_temperature.min(temperatures[cell]);
        maximum_temperature = maximum_temperature.max(temperatures[cell]);
        wettest = wettest.max(precip);
        if is_land {
            driest_land = driest_land.min(precip);
        }
        runoff.push(runoff_mm);
        runoff_volume.push(round_volume(runoff_m3)?);
    }
    if !precipitation_volume.is_finite()
        || !runoff_volume_total.is_finite()
        || precipitation_volume > u64::MAX as f64
        || runoff_volume_total > u64::MAX as f64
    {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::NumericNonFinite,
            "climate total water volumes are not finite or bounded",
        ));
    }
    Ok((
        runoff,
        runoff_volume,
        ClimateMetrics {
            precipitation_volume_m3_per_year: precipitation_volume.round() as u64,
            runoff_volume_m3_per_year: runoff_volume_total.round() as u64,
            mean_temperature_centi_c: (temperature_area_sum / total_area).round() as i32,
            minimum_temperature_centi_c: minimum_temperature,
            maximum_temperature_centi_c: maximum_temperature,
            mean_precipitation_mm_per_year: (precipitation_area_sum / total_area).round() as u32,
            mean_runoff_mm_per_year: (runoff_area_sum / total_area).round() as u32,
            wettest_cell_precipitation_mm_per_year: wettest,
            driest_land_cell_precipitation_mm_per_year: if driest_land == u32::MAX {
                0
            } else {
                driest_land
            },
            ..ClimateMetrics::default()
        },
    ))
}

fn apply_seasonal_metrics(
    field: &PhysicalField,
    summers: &[i32],
    winters: &[i32],
    metrics: &mut ClimateMetrics,
) {
    let total_area = field.grid.total_area();
    let mut range_area_sum = 0.0;
    let mut land_area = 0.0;
    let mut permanent_area = 0.0;
    let mut seasonal_area = 0.0;
    let mut minimum_seasonal = i32::MAX;
    let mut maximum_seasonal = i32::MIN;
    for cell in 0..field.grid.sample_count() {
        let area = field.grid.cell_area(field.grid.row_col(cell).0);
        let cold = summers[cell].min(winters[cell]);
        let warm = summers[cell].max(winters[cell]);
        range_area_sum += f64::from((summers[cell] - winters[cell]).unsigned_abs()) * area;
        minimum_seasonal = minimum_seasonal.min(cold);
        maximum_seasonal = maximum_seasonal.max(warm);
        if field.elevations_mm[cell] > field.sea_level_mm {
            land_area += area;
            if warm < 0 {
                permanent_area += area;
            } else if cold < 0 {
                seasonal_area += area;
            }
        }
    }
    metrics.mean_seasonal_range_centi_c = (range_area_sum / total_area).round() as u32;
    metrics.minimum_seasonal_temperature_centi_c = minimum_seasonal;
    metrics.maximum_seasonal_temperature_centi_c = maximum_seasonal;
    metrics.permanently_frozen_land_ppm = if land_area <= 0.0 {
        0
    } else {
        (permanent_area / land_area * 1_000_000.0).round() as u32
    };
    metrics.seasonally_frozen_land_ppm = if land_area <= 0.0 {
        0
    } else {
        (seasonal_area / land_area * 1_000_000.0).round() as u32
    };
}

fn sinusoid_above_fraction(first: f64, second: f64, threshold: f64) -> f64 {
    let mean = 0.5 * (first + second);
    let amplitude = 0.5 * (first - second).abs();
    if amplitude <= 1e-9 {
        return if mean > threshold { 1.0 } else { 0.0 };
    }
    let k = (threshold - mean) / amplitude;
    if k >= 1.0 {
        0.0
    } else if k <= -1.0 {
        1.0
    } else {
        0.5 - k.asin() / std::f64::consts::PI
    }
}

fn apply_season_length_metrics(
    field: &PhysicalField,
    summers: &[i32],
    winters: &[i32],
    precipitation_summer: &[u32],
    precipitation_winter: &[u32],
    metrics: &mut ClimateMetrics,
) {
    let mut land_area = 0.0;
    let mut growing_sum = 0.0;
    let mut dry_sum = 0.0;
    let mut wet_sum = 0.0;
    let growing_threshold = f64::from(GROWING_SEASON_CENTI_C);
    let dry_threshold = f64::from(DESERT_PRECIPITATION_MM);
    let wet_threshold = f64::from(FOREST_PRECIPITATION_MM);
    for cell in 0..field.grid.sample_count() {
        if is_ocean(field, cell) {
            continue;
        }
        let area = field.grid.cell_area(field.grid.row_col(cell).0);
        land_area += area;
        growing_sum += sinusoid_above_fraction(
            f64::from(summers[cell]),
            f64::from(winters[cell]),
            growing_threshold,
        ) * area;
        dry_sum +=
            (1.0 - sinusoid_above_fraction(
                f64::from(precipitation_summer[cell]),
                f64::from(precipitation_winter[cell]),
                dry_threshold,
            )) * area;
        wet_sum += sinusoid_above_fraction(
            f64::from(precipitation_summer[cell]),
            f64::from(precipitation_winter[cell]),
            wet_threshold,
        ) * area;
    }
    metrics.mean_land_growing_season_ppm = if land_area <= 0.0 {
        0
    } else {
        (growing_sum / land_area * 1_000_000.0).round() as u32
    };
    metrics.mean_land_dry_season_ppm = if land_area <= 0.0 {
        0
    } else {
        (dry_sum / land_area * 1_000_000.0).round() as u32
    };
    metrics.mean_land_wet_season_ppm = if land_area <= 0.0 {
        0
    } else {
        (wet_sum / land_area * 1_000_000.0).round() as u32
    };
}

fn apply_wind_metrics(
    field: &PhysicalField,
    east: &[i32],
    north: &[i32],
    divergence_ppm: &[i32],
    itcz_latitude: f64,
    metrics: &mut ClimateMetrics,
) {
    let total_area = field.grid.total_area();
    let mut speed_area_sum = 0.0;
    let mut easterly_area = 0.0;
    let mut converging_area = 0.0;
    for cell in 0..field.grid.sample_count() {
        let area = field.grid.cell_area(field.grid.row_col(cell).0);
        speed_area_sum += f64::from(east[cell]).hypot(f64::from(north[cell])) * area;
        if east[cell] < 0 {
            easterly_area += area;
        }
        if divergence_ppm[cell] < 0 {
            converging_area += area;
        }
    }
    metrics.mean_wind_speed_milli = (speed_area_sum / total_area).round() as u32;
    metrics.itcz_latitude_milli_deg = (itcz_latitude.to_degrees() * 1_000.0).round() as i32;
    metrics.easterly_cell_ppm = (easterly_area / total_area * 1_000_000.0).round() as u32;
    metrics.converging_cell_ppm = (converging_area / total_area * 1_000_000.0).round() as u32;
}

fn apply_humidity_metrics(
    field: &PhysicalField,
    humidity_ppm: &[u32],
    aridity_ppm: &[u32],
    precipitation_summer: &[u32],
    precipitation_winter: &[u32],
    metrics: &mut ClimateMetrics,
) {
    let total_area = field.grid.total_area();
    let mut humidity_area_sum = 0.0;
    let mut aridity_area_sum = 0.0;
    let mut land_area = 0.0;
    let mut range_area_sum = 0.0;
    for cell in 0..field.grid.sample_count() {
        let area = field.grid.cell_area(field.grid.row_col(cell).0);
        humidity_area_sum += f64::from(humidity_ppm[cell]) * area;
        range_area_sum +=
            f64::from(precipitation_summer[cell].abs_diff(precipitation_winter[cell])) * area;
        if field.elevations_mm[cell] > field.sea_level_mm {
            land_area += area;
            aridity_area_sum += f64::from(aridity_ppm[cell]) * area;
        }
    }
    metrics.mean_humidity_ppm = (humidity_area_sum / total_area).round() as u32;
    metrics.mean_land_aridity_ppm = if land_area <= 0.0 {
        0
    } else {
        (aridity_area_sum / land_area).round() as u32
    };
    metrics.mean_seasonal_precipitation_range_mm = (range_area_sum / total_area).round() as u32;
}

fn apply_current_metrics(
    field: &PhysicalField,
    east: &[i32],
    north: &[i32],
    metrics: &mut ClimateMetrics,
) {
    let ocean = ocean_mask(field);
    let mut speed_area_sum = 0.0;
    let mut ocean_area = 0.0;
    for cell in 0..field.grid.sample_count() {
        if !ocean[cell] {
            continue;
        }
        let area = field.grid.cell_area(field.grid.row_col(cell).0);
        ocean_area += area;
        speed_area_sum += f64::from(east[cell]).hypot(f64::from(north[cell])) * area;
    }
    metrics.mean_current_speed_milli = if ocean_area <= 0.0 {
        0
    } else {
        (speed_area_sum / ocean_area).round() as u32
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BiomeLegendEntry {
    pub id: u32,
    pub name: &'static str,
    pub reason: &'static str,
    pub fill: [u8; 3],
}

#[must_use]
pub fn biome_name(class: u32) -> &'static str {
    biome_entry(class).name
}

#[must_use]
pub fn biome_reason(class: u32) -> &'static str {
    biome_entry(class).reason
}

#[must_use]
pub fn biome_fill_rgb(class: u32) -> [u8; 3] {
    biome_entry(class).fill
}

#[must_use]
pub fn biome_legend() -> [BiomeLegendEntry; 10] {
    [
        biome_entry(BIOME_OCEAN),
        biome_entry(BIOME_ICE),
        biome_entry(BIOME_TUNDRA),
        biome_entry(BIOME_ALPINE),
        biome_entry(BIOME_COLD_GRASSLAND),
        biome_entry(BIOME_TEMPERATE_GRASSLAND),
        biome_entry(BIOME_DESERT),
        biome_entry(BIOME_SHRUBLAND),
        biome_entry(BIOME_TEMPERATE_FOREST),
        biome_entry(BIOME_TROPICAL_FOREST),
    ]
}

fn biome_entry(class: u32) -> BiomeLegendEntry {
    match class {
        BIOME_ICE => BiomeLegendEntry {
            id: BIOME_ICE,
            name: "permanent ice",
            reason:
                "both solstices stay below freezing; this is climate freeze, not ice-sheet cover",
            fill: [236, 244, 252],
        },
        BIOME_TUNDRA => BiomeLegendEntry {
            id: BIOME_TUNDRA,
            name: "tundra",
            reason: "the warmer solstice stays below 10 °C",
            fill: [156, 168, 148],
        },
        BIOME_ALPINE => BiomeLegendEntry {
            id: BIOME_ALPINE,
            name: "alpine",
            reason: "high land stays cold despite its latitude",
            fill: [188, 176, 156],
        },
        BIOME_COLD_GRASSLAND => BiomeLegendEntry {
            id: BIOME_COLD_GRASSLAND,
            name: "cold grassland",
            reason: "cool or strongly seasonal climate with moderate rain",
            fill: [148, 168, 92],
        },
        BIOME_TEMPERATE_GRASSLAND => BiomeLegendEntry {
            id: BIOME_TEMPERATE_GRASSLAND,
            name: "temperate grassland",
            reason: "moderate rain without enough moisture for forest",
            fill: [168, 186, 92],
        },
        BIOME_DESERT => BiomeLegendEntry {
            id: BIOME_DESERT,
            name: "desert",
            reason: "rainfall does not meet evaporative demand",
            fill: [214, 178, 96],
        },
        BIOME_SHRUBLAND => BiomeLegendEntry {
            id: BIOME_SHRUBLAND,
            name: "shrubland",
            reason: "semi-arid or leftover humidity supports shrubs rather than closed grassland",
            fill: [196, 164, 88],
        },
        BIOME_TEMPERATE_FOREST => BiomeLegendEntry {
            id: BIOME_TEMPERATE_FOREST,
            name: "temperate forest",
            reason:
                "enough rain and remaining humidity for forest outside the tropical warmth band",
            fill: [46, 120, 62],
        },
        BIOME_TROPICAL_FOREST => BiomeLegendEntry {
            id: BIOME_TROPICAL_FOREST,
            name: "tropical forest",
            reason: "warm year-round with heavy rainfall and moist air",
            fill: [18, 92, 44],
        },
        BIOME_OCEAN => BiomeLegendEntry {
            id: BIOME_OCEAN,
            name: "ocean",
            reason: "this cell is ocean, not a land biome",
            fill: [46, 110, 148],
        },
        _ => BiomeLegendEntry {
            id: class,
            name: "unclassified",
            reason: "biome class is outside the versioned legend",
            fill: UNKNOWN_BIOME_FILL,
        },
    }
}

#[must_use]
pub fn explain_biome(
    class: u32,
    elevation_mm: i32,
    sea_level_mm: i32,
    summer_centi_c: i32,
    winter_centi_c: i32,
    precipitation_mm: u32,
    humidity_ppm: u32,
    aridity_ppm: u32,
) -> String {
    let warm = summer_centi_c.max(winter_centi_c);
    let cold = summer_centi_c.min(winter_centi_c);
    let height_m = elevation_mm.saturating_sub(sea_level_mm) / 1_000;
    format!(
        "{} because {}. Warmer solstice {:.1} °C, colder {:.1} °C, {} m above sea, {} mm/year, humidity {}% of saturation, aridity {}%.",
        biome_name(class),
        biome_reason(class),
        f64::from(warm) / 100.0,
        f64::from(cold) / 100.0,
        height_m,
        precipitation_mm,
        humidity_ppm / 10_000,
        aridity_ppm / 10_000
    )
}

#[must_use]
pub fn classify_biome_cell(
    land: bool,
    elevation_mm: i32,
    sea_level_mm: i32,
    annual_centi_c: i32,
    summer_centi_c: i32,
    winter_centi_c: i32,
    precipitation_mm: u32,
    humidity_ppm: u32,
    aridity_ppm: u32,
) -> u32 {
    if !land {
        return BIOME_OCEAN;
    }
    let warm = summer_centi_c.max(winter_centi_c);
    let cold = summer_centi_c.min(winter_centi_c);
    if warm < 0 {
        return BIOME_ICE;
    }
    let highland = elevation_mm.saturating_sub(sea_level_mm) >= ALPINE_HEIGHT_MM;
    if highland && warm < TUNDRA_WARM_CENTI_C {
        return BIOME_ALPINE;
    }
    if warm < TUNDRA_WARM_CENTI_C {
        return BIOME_TUNDRA;
    }
    if aridity_ppm >= DESERT_ARIDITY_PPM || precipitation_mm < DESERT_PRECIPITATION_MM {
        if humidity_ppm >= MARITIME_HUMIDITY_PPM {
            return BIOME_SHRUBLAND;
        }
        return BIOME_DESERT;
    }
    let cold_grassland =
        annual_centi_c < COLD_GRASSLAND_ANNUAL_CENTI_C || cold < COLD_GRASSLAND_WINTER_CENTI_C;
    if aridity_ppm >= SHRUBLAND_ARIDITY_PPM || precipitation_mm < GRASSLAND_PRECIPITATION_MM {
        if cold_grassland {
            return BIOME_COLD_GRASSLAND;
        }
        return BIOME_SHRUBLAND;
    }
    let grassland = aridity_ppm >= GRASSLAND_ARIDITY_PPM
        || precipitation_mm < FOREST_PRECIPITATION_MM
        || humidity_ppm < FOREST_HUMIDITY_PPM;
    if grassland {
        if cold_grassland {
            return BIOME_COLD_GRASSLAND;
        }
        return BIOME_TEMPERATE_GRASSLAND;
    }
    if annual_centi_c >= TROPICAL_ANNUAL_CENTI_C && cold >= TROPICAL_COLD_CENTI_C {
        return BIOME_TROPICAL_FOREST;
    }
    BIOME_TEMPERATE_FOREST
}

fn classify_biomes(
    field: &PhysicalField,
    annual: &[i32],
    summers: &[i32],
    winters: &[i32],
    precipitation: &[u32],
    humidity_ppm: &[u32],
    aridity_ppm: &[u32],
) -> Vec<u32> {
    (0..field.grid.sample_count())
        .map(|cell| {
            classify_biome_cell(
                field.elevations_mm[cell] > field.sea_level_mm,
                field.elevations_mm[cell],
                field.sea_level_mm,
                annual[cell],
                summers[cell],
                winters[cell],
                precipitation[cell],
                humidity_ppm[cell],
                aridity_ppm[cell],
            )
        })
        .collect()
}

fn apply_biome_metrics(field: &PhysicalField, biome_class: &[u32], metrics: &mut ClimateMetrics) {
    let mut areas = [0.0; BIOME_CLASS_MAX as usize + 1];
    let mut land_area = 0.0;
    for cell in 0..field.grid.sample_count() {
        if field.elevations_mm[cell] <= field.sea_level_mm {
            continue;
        }
        let area = field.grid.cell_area(field.grid.row_col(cell).0);
        land_area += area;
        let class = biome_class[cell].min(BIOME_CLASS_MAX) as usize;
        areas[class] += area;
    }
    metrics.dominant_land_biome = if land_area <= 0.0 {
        BIOME_OCEAN
    } else {
        areas
            .iter()
            .enumerate()
            .skip(1)
            .max_by(|left, right| {
                left.1
                    .partial_cmp(right.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(class, _)| class as u32)
            .unwrap_or(BIOME_OCEAN)
    };
}

struct StormFields {
    suitability_ppm: Vec<u32>,
    track_ppm: Vec<u32>,
    intensity_ppm: Vec<u32>,
}

fn unit_factor(value: i64, start: i64, full: i64) -> f64 {
    if full <= start {
        return if value >= full { 1.0 } else { 0.0 };
    }
    ((value - start) as f64 / (full - start) as f64).clamp(0.0, 1.0)
}

fn storm_latitude_factor(latitude: f64, itcz: f64, hadley: f64) -> f64 {
    let abs_lat = latitude.abs().to_degrees();
    let coriolis_lat = if abs_lat < 5.0 {
        0.0
    } else if abs_lat < 8.0 {
        (abs_lat - 5.0) / 3.0
    } else {
        1.0
    };
    if coriolis_lat == 0.0 {
        return 0.0;
    }
    let scale = (hadley.to_degrees() / 30.0).clamp(0.4, 2.5);
    let abs_phi = (latitude - itcz).abs().to_degrees();
    let inner = 5.0 * scale;
    let peak0 = 12.0 * scale;
    let peak1 = 22.0 * scale;
    let outer = 32.0 * scale;
    let band = if abs_phi < inner {
        0.0
    } else if abs_phi < peak0 {
        (abs_phi - inner) / (peak0 - inner).max(1e-6)
    } else if abs_phi <= peak1 {
        1.0
    } else if abs_phi < outer {
        (outer - abs_phi) / (outer - peak1).max(1e-6)
    } else {
        0.0
    };
    coriolis_lat * band
}

fn storm_fetch_factor(east_ocean_cells: u32, west_ocean_cells: u32) -> f64 {
    let basin = east_ocean_cells.saturating_add(west_ocean_cells);
    if basin < 3 {
        return 0.08;
    }
    match east_ocean_cells {
        0 => 0.08,
        1 => 0.22,
        2..=3 => 0.55,
        4..=20 => 1.0,
        _ => 0.78,
    }
}

fn is_storm_track_seed(grid: Grid, suitability_ppm: &[u32], cell: usize) -> bool {
    if suitability_ppm[cell] < STORM_TRACK_START_PPM {
        return false;
    }
    let (row, col) = grid.row_col(cell);
    let row0 = (row / STORM_SEED_BLOCK) * STORM_SEED_BLOCK;
    let col0 = (col / STORM_SEED_BLOCK) * STORM_SEED_BLOCK;
    let row1 = (row0 + STORM_SEED_BLOCK).min(grid.height);
    let col1 = (col0 + STORM_SEED_BLOCK).min(grid.width);
    let mut best_cell = cell;
    let mut best_value = suitability_ppm[cell];
    for seed_row in row0..row1 {
        for seed_col in col0..col1 {
            let neighbor = grid.index(seed_row, seed_col);
            let value = suitability_ppm[neighbor];
            if value > best_value || (value == best_value && neighbor < best_cell) {
                best_value = value;
                best_cell = neighbor;
            }
        }
    }
    best_cell == cell
}

fn storm_shear_milli(
    east_summer: i32,
    north_summer: i32,
    east_winter: i32,
    north_winter: i32,
) -> u32 {
    // Seasonal solstice wind-vector difference. This is not vertical shear.
    let east = i64::from(east_summer) - i64::from(east_winter);
    let north = i64::from(north_summer) - i64::from(north_winter);
    ((east * east + north * north) as f64).sqrt().round() as u32
}

fn storm_coriolis_factor(planetary: PlanetaryConfiguration) -> f64 {
    (f64::from(EARTH_ROTATION_PERIOD_SECONDS) / f64::from(planetary.rotation_period_seconds.max(1)))
        .clamp(0.0, 1.0)
}

fn storm_sst_bounds(planetary: PlanetaryConfiguration) -> (i32, i32) {
    let shift = planetary.retained_heat_centi_c - EARTH_RETAINED_HEAT_CENTI_C;
    (
        STORM_MIN_SST_CENTI_C.saturating_add(shift),
        STORM_FULL_SST_CENTI_C.saturating_add(shift),
    )
}

fn storm_proximity_factor(land_distance_cells: u16) -> f64 {
    match land_distance_cells {
        0 => 0.0,
        1 => 0.35,
        2..=6 => 1.0,
        7..=12 => 0.7,
        _ => 0.45,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StormCoupling {
    pressure_gradient_start_centi: u32,
    pressure_gradient_kill_centi: u32,
    divergence_start_ppm: u32,
    divergence_kill_ppm: u32,
    convergence_full_ppm: u32,
}

impl StormCoupling {
    fn from_settings(settings: ClimateSettings) -> Self {
        Self {
            pressure_gradient_start_centi: settings.storm_pressure_gradient_start_centi,
            pressure_gradient_kill_centi: settings.storm_pressure_gradient_kill_centi,
            divergence_start_ppm: settings.storm_divergence_start_ppm,
            divergence_kill_ppm: settings.storm_divergence_kill_ppm,
            convergence_full_ppm: settings.storm_convergence_full_ppm,
        }
    }
}

impl Default for StormCoupling {
    fn default() -> Self {
        Self {
            pressure_gradient_start_centi: STORM_PGRAD_START_CENTI,
            pressure_gradient_kill_centi: STORM_PGRAD_KILL_CENTI,
            divergence_start_ppm: STORM_DIVERGENCE_START_PPM,
            divergence_kill_ppm: STORM_DIVERGENCE_KILL_PPM,
            convergence_full_ppm: STORM_CONVERGENCE_FULL_PPM,
        }
    }
}

fn storm_convergence_factor(divergence_ppm: i32, coupling: StormCoupling) -> f64 {
    let conv = unit_factor(
        i64::from(divergence_ppm).saturating_neg(),
        0,
        i64::from(coupling.convergence_full_ppm),
    );
    let div = unit_factor(
        i64::from(divergence_ppm),
        i64::from(coupling.divergence_start_ppm),
        i64::from(coupling.divergence_kill_ppm),
    );
    ((1.0 + 0.25 * conv) * (1.0 - 0.70 * div)).clamp(0.0, 1.25)
}

fn storm_pressure_gradient_factor(gradient_centi: u32, coupling: StormCoupling) -> f64 {
    1.0 - unit_factor(
        i64::from(gradient_centi),
        i64::from(coupling.pressure_gradient_start_centi),
        i64::from(coupling.pressure_gradient_kill_centi),
    )
}

fn storm_thermal_pressure_gradient_centi(
    grid: Grid,
    temperatures: &[i32],
    ocean: &[bool],
    cell: usize,
) -> u32 {
    if !ocean[cell] {
        return 0;
    }
    let (row, col) = grid.row_col(cell);
    let t = temperatures[cell];
    let mut acc = 0.0;
    let mut n = 0.0;
    for (drow, dcol) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
        let neighbor = grid.index(clamped_row(grid, row, drow), wrapped_col(grid, col, dcol));
        if neighbor == cell || !ocean[neighbor] {
            continue;
        }
        let dt = f64::from((t - temperatures[neighbor]).unsigned_abs());
        acc += dt * spacing_scale(neighbor_metres(grid, row, col, drow, dcol));
        n += 1.0;
    }
    if n <= 0.0 {
        0
    } else {
        (acc / n).round() as u32
    }
}

fn storm_seasonal_pressure_gradient_centi(
    grid: Grid,
    annual: &[i32],
    summers: &[i32],
    winters: &[i32],
    ocean: &[bool],
    cell: usize,
) -> u32 {
    storm_thermal_pressure_gradient_centi(grid, annual, ocean, cell)
        .max(storm_thermal_pressure_gradient_centi(
            grid, summers, ocean, cell,
        ))
        .max(storm_thermal_pressure_gradient_centi(
            grid, winters, ocean, cell,
        ))
}

fn storm_current_factor(east_milli: i32, north_milli: i32) -> f64 {
    let speed = ((i64::from(east_milli) * i64::from(east_milli)
        + i64::from(north_milli) * i64::from(north_milli)) as f64)
        .sqrt()
        .round() as i64;
    0.82 + 0.18 * unit_factor(speed, 200, 2_800)
}

fn land_distance_cells(field: &PhysicalField) -> Vec<u16> {
    let count = field.grid.sample_count();
    let mut distance = vec![u16::MAX; count];
    let mut queue = Vec::with_capacity(count);
    for cell in 0..count {
        if !is_ocean(field, cell) {
            distance[cell] = 0;
            queue.push(cell);
        }
    }
    let mut head = 0usize;
    while head < queue.len() {
        let cell = queue[head];
        head += 1;
        let next = distance[cell].saturating_add(1);
        for neighbor in field.grid.neighbors(cell) {
            if distance[neighbor] > next {
                distance[neighbor] = next;
                queue.push(neighbor);
            }
        }
    }
    distance
}

#[cfg(test)]
fn classify_storm_cell(
    ocean: bool,
    sst_centi_c: i32,
    humidity_ppm: u32,
    latitude: f64,
    itcz: f64,
    hadley: f64,
    shear_milli: u32,
    coriolis: f64,
    proximity: f64,
    current: f64,
    fetch: f64,
    divergence_ppm: i32,
    pressure_gradient_centi: u32,
    sst_min: i32,
    sst_full: i32,
) -> (u32, u32) {
    classify_storm_scaled(
        ocean,
        sst_centi_c,
        humidity_ppm,
        latitude,
        itcz,
        hadley,
        shear_milli,
        coriolis,
        proximity,
        current,
        fetch,
        divergence_ppm,
        pressure_gradient_centi,
        StormCoupling::default(),
        sst_min,
        sst_full,
    )
}

fn classify_storm_scaled(
    ocean: bool,
    sst_centi_c: i32,
    humidity_ppm: u32,
    latitude: f64,
    itcz: f64,
    hadley: f64,
    shear_milli: u32,
    coriolis: f64,
    proximity: f64,
    current: f64,
    fetch: f64,
    divergence_ppm: i32,
    pressure_gradient_centi: u32,
    coupling: StormCoupling,
    sst_min: i32,
    sst_full: i32,
) -> (u32, u32) {
    if !ocean || sst_centi_c < 0 {
        return (0, 0);
    }
    let sst = unit_factor(
        i64::from(sst_centi_c),
        i64::from(sst_min),
        i64::from(sst_full),
    );
    let humidity = unit_factor(
        i64::from(humidity_ppm),
        i64::from(STORM_MIN_HUMIDITY_PPM),
        i64::from(STORM_FULL_HUMIDITY_PPM),
    );
    let latitude_factor = storm_latitude_factor(latitude, itcz, hadley) * coriolis;
    let shear = unit_factor(
        i64::from(shear_milli),
        i64::from(STORM_SHEAR_START_MILLI),
        i64::from(STORM_SHEAR_KILL_MILLI),
    );
    let suitability = (sst
        * humidity
        * latitude_factor
        * (1.0 - shear)
        * storm_convergence_factor(divergence_ppm, coupling)
        * storm_pressure_gradient_factor(pressure_gradient_centi, coupling)
        * proximity
        * current
        * fetch
        * 1_000_000.0)
        .round() as u32;
    let intensity = (suitability as f64 * (0.45 + 0.55 * sst)).round() as u32;
    (suitability.min(1_000_000), intensity.min(1_000_000))
}

pub fn explain_storm(
    ocean: bool,
    sst_centi_c: i32,
    humidity_ppm: u32,
    latitude_milli_deg: i32,
    shear_milli: u32,
    suitability_ppm: u32,
    track_ppm: u32,
    intensity_ppm: u32,
) -> String {
    let zone = if !ocean {
        "landfall exposure, not formation"
    } else if suitability_ppm == 0 {
        "this ocean cell is outside the tropical-cyclone genesis band"
    } else if suitability_ppm >= 500_000 {
        "warm, moist, rotating ocean favors tropical-cyclone genesis"
    } else {
        "marginal tropical-cyclone conditions"
    };
    format!(
        "Storm suitability {}% ({zone}). Sea {:.1} °C, humidity {}% of saturation, |lat| {:.1}°, seasonal wind shear {} (solstice vector difference, not vertical), convergence and ocean thermal fronts also scale genesis, track corridor {}%, intensity potential {}%. Climatology, not a forecast.",
        suitability_ppm / 10_000,
        f64::from(sst_centi_c) / 100.0,
        humidity_ppm / 10_000,
        f64::from(latitude_milli_deg.abs()) / 1_000.0,
        shear_milli,
        track_ppm / 10_000,
        intensity_ppm / 10_000,
    )
}

pub fn storm_annual_rate_nano(suitability_ppm: u32) -> u64 {
    u64::from(suitability_ppm)
}

pub fn storm_expected_per_year_milli(mean_ocean_suitability_ppm: u32) -> u32 {
    ((u64::from(mean_ocean_suitability_ppm) * u64::from(STORM_CLIMATE_YEAR_MILLI_AT_FULL))
        / 1_000_000) as u32
}

pub fn storm_track_step(
    grid: Grid,
    cell: usize,
    east_milli: i32,
    north_milli: i32,
    current_east_milli: i32,
    current_north_milli: i32,
    latitude: f64,
) -> Option<usize> {
    let east = east_milli.saturating_add(
        ((i64::from(current_east_milli) * i64::from(STORM_CURRENT_STEER_PPM)) / 1_000_000) as i32,
    );
    let north_wind = north_milli.saturating_add(
        ((i64::from(current_north_milli) * i64::from(STORM_CURRENT_STEER_PPM)) / 1_000_000) as i32,
    );
    let poleward = if latitude >= 0.0 { 1 } else { -1 };
    let beta = (east.unsigned_abs() as i32 * 2) / 5;
    let north = north_wind.saturating_add(poleward * beta);
    let speed = east.unsigned_abs().max(north.unsigned_abs());
    if speed < 200 {
        return None;
    }
    let dcol = if east.unsigned_abs() * 2 >= speed {
        if east > 0 {
            1
        } else {
            -1
        }
    } else {
        0
    };
    let drow = if north.unsigned_abs() * 2 >= speed {
        if north > 0 {
            1
        } else {
            -1
        }
    } else {
        0
    };
    if dcol == 0 && drow == 0 {
        return None;
    }
    let (row, col) = grid.row_col(cell);
    let next_row = clamped_row(grid, row, drow);
    let next_col = wrapped_col(grid, col, dcol);
    let next = grid.index(next_row, next_col);
    if next == cell {
        None
    } else {
        Some(next)
    }
}

pub fn collect_storm_track(
    field: &PhysicalField,
    climate: &ClimateField,
    origin: usize,
) -> Vec<usize> {
    let mut cells = vec![origin];
    let mut cell = origin;
    let mut landfalls = 0u32;
    for _ in 0..STORM_TRACK_STEPS {
        let (row, _) = field.grid.row_col(cell);
        let latitude = field.grid.center_radians(row, 0).1;
        let Some(next) = storm_track_step(
            field.grid,
            cell,
            climate.wind_east_milli[cell],
            climate.wind_north_milli[cell],
            climate.current_east_milli[cell],
            climate.current_north_milli[cell],
            latitude,
        ) else {
            break;
        };
        if cells.contains(&next) {
            break;
        }
        cell = next;
        cells.push(cell);
        if !is_ocean(field, cell) {
            landfalls += 1;
            if landfalls >= 2 {
                break;
            }
        }
    }
    cells
}

pub fn follow_storm_track(field: &PhysicalField, climate: &ClimateField, origin: usize) -> usize {
    *collect_storm_track(field, climate, origin)
        .last()
        .unwrap_or(&origin)
}

fn deposit(values: &mut [u32], cell: usize, amount: u32) {
    values[cell] = values[cell].saturating_add(amount).min(1_000_000);
}

fn classify_extreme_cell(
    land: bool,
    annual_centi_c: i32,
    summer_centi_c: i32,
    winter_centi_c: i32,
    precipitation_mm: u32,
    precipitation_summer_mm: u32,
    precipitation_winter_mm: u32,
    aridity_ppm: u32,
) -> (u32, u32, u32) {
    let warm = summer_centi_c.max(winter_centi_c).max(annual_centi_c);
    let cold = summer_centi_c.min(winter_centi_c);
    let drought = if !land || warm < 0 {
        0
    } else {
        let arid = unit_factor(
            i64::from(aridity_ppm),
            i64::from(GRASSLAND_ARIDITY_PPM),
            i64::from(DESERT_ARIDITY_PPM),
        );
        let dry = 1.0
            - unit_factor(
                i64::from(precipitation_mm),
                80,
                i64::from(FOREST_PRECIPITATION_MM),
            );
        let driest_season = precipitation_summer_mm.min(precipitation_winter_mm);
        let seasonal_dry = 1.0 - unit_factor(i64::from(driest_season), 20, 200);
        ((0.50 * arid + 0.35 * dry + 0.15 * seasonal_dry) * 1_000_000.0).round() as u32
    };
    let heat_wave = if warm < 0 {
        0
    } else {
        let heat = unit_factor(i64::from(warm), 2_200, 3_800);
        let seasonal = unit_factor(
            i64::from(summer_centi_c.abs_diff(winter_centi_c)),
            400,
            2_500,
        );
        let land_weight = if land { 1.0 } else { 0.25 };
        ((0.70 * heat + 0.30 * seasonal) * land_weight * 1_000_000.0).round() as u32
    };
    let peak_rain = precipitation_mm
        .max(precipitation_summer_mm)
        .max(precipitation_winter_mm);
    let season_span = precipitation_summer_mm.abs_diff(precipitation_winter_mm);
    let extreme_rain = if cold < 0 && peak_rain < 80 {
        0
    } else {
        let peak = unit_factor(i64::from(peak_rain), 600, 2_500);
        let monsoon = unit_factor(i64::from(season_span), 80, 800);
        ((peak * (0.70 + 0.30 * monsoon)) * 1_000_000.0).round() as u32
    };
    (
        drought.min(1_000_000),
        heat_wave.min(1_000_000),
        extreme_rain.min(1_000_000),
    )
}

fn apply_extreme_metrics(
    field: &PhysicalField,
    annual: &[i32],
    summers: &[i32],
    winters: &[i32],
    precipitation: &[u32],
    precipitation_summer: &[u32],
    precipitation_winter: &[u32],
    aridity_ppm: &[u32],
    metrics: &mut ClimateMetrics,
) {
    let mut land_area = 0.0;
    let mut all_area = 0.0;
    let mut drought_sum = 0.0;
    let mut heat_sum = 0.0;
    let mut rain_sum = 0.0;
    for cell in 0..field.grid.sample_count() {
        let area = field.grid.cell_area(field.grid.row_col(cell).0);
        let land = !is_ocean(field, cell);
        let (drought, heat, rain) = classify_extreme_cell(
            land,
            annual[cell],
            summers[cell],
            winters[cell],
            precipitation[cell],
            precipitation_summer[cell],
            precipitation_winter[cell],
            aridity_ppm[cell],
        );
        all_area += area;
        heat_sum += f64::from(heat) * area;
        rain_sum += f64::from(rain) * area;
        if land {
            land_area += area;
            drought_sum += f64::from(drought) * area;
        }
    }
    metrics.mean_land_drought_potential_ppm = if land_area <= 0.0 {
        0
    } else {
        (drought_sum / land_area).round() as u32
    };
    metrics.mean_heat_wave_potential_ppm = if all_area <= 0.0 {
        0
    } else {
        (heat_sum / all_area).round() as u32
    };
    metrics.mean_extreme_rainfall_potential_ppm = if all_area <= 0.0 {
        0
    } else {
        (rain_sum / all_area).round() as u32
    };
}

fn derive_storms(
    field: &PhysicalField,
    settings: ClimateSettings,
    annual: &[i32],
    summers: &[i32],
    winters: &[i32],
    humidity_ppm: &[u32],
    wind_east: &[i32],
    wind_north: &[i32],
    wind_east_summer: &[i32],
    wind_north_summer: &[i32],
    wind_east_winter: &[i32],
    wind_north_winter: &[i32],
    wind_divergence: &[i32],
    wind_divergence_summer: &[i32],
    wind_divergence_winter: &[i32],
    current_east: &[i32],
    current_north: &[i32],
    current_geometry: &CurrentGeometry,
) -> StormFields {
    let count = field.grid.sample_count();
    let planetary = settings.planetary;
    let coupling = StormCoupling::from_settings(settings);
    let coriolis = storm_coriolis_factor(planetary);
    let hadley = hadley_edge_radians(omega_ratio(planetary));
    let (sst_min, sst_full) = storm_sst_bounds(planetary);
    let proximity = land_distance_cells(field);
    let ocean = &current_geometry.ocean;
    let east_fetch = &current_geometry.east_distance;
    let west_fetch = &current_geometry.west_distance;
    let itcz_by_col = thermal_equator_by_column(field.grid, annual);
    let mut suitability_ppm = vec![0u32; count];
    let mut track_ppm = vec![0u32; count];
    let mut intensity_ppm = vec![0u32; count];
    for cell in 0..count {
        let (row, col) = field.grid.row_col(cell);
        let latitude = field.grid.center_radians(row, col).1;
        let itcz = itcz_by_col[col as usize];
        let sst = summers[cell].max(winters[cell]).max(annual[cell]);
        let shear = storm_shear_milli(
            wind_east_summer[cell],
            wind_north_summer[cell],
            wind_east_winter[cell],
            wind_north_winter[cell],
        );
        let divergence = wind_divergence[cell]
            .min(wind_divergence_summer[cell])
            .min(wind_divergence_winter[cell]);
        let (suitability, intensity) = classify_storm_scaled(
            ocean[cell],
            sst,
            humidity_ppm[cell],
            latitude,
            itcz,
            hadley,
            shear,
            coriolis,
            storm_proximity_factor(proximity[cell]),
            storm_current_factor(current_east[cell], current_north[cell]),
            storm_fetch_factor(east_fetch[cell], west_fetch[cell]),
            divergence,
            storm_seasonal_pressure_gradient_centi(
                field.grid, annual, summers, winters, ocean, cell,
            ),
            coupling,
            sst_min,
            sst_full,
        );
        suitability_ppm[cell] = suitability;
        intensity_ppm[cell] = intensity;
    }
    for origin in 0..count {
        if !is_storm_track_seed(field.grid, &suitability_ppm, origin) {
            continue;
        }
        let mut cell = origin;
        let mut remaining = suitability_ppm[origin];
        deposit(&mut track_ppm, cell, remaining);
        let mut landfalls = 0u32;
        for _ in 0..STORM_TRACK_STEPS {
            let (row, _) = field.grid.row_col(cell);
            let latitude = field.grid.center_radians(row, 0).1;
            let Some(next) = storm_track_step(
                field.grid,
                cell,
                wind_east[cell],
                wind_north[cell],
                current_east[cell],
                current_north[cell],
                latitude,
            ) else {
                break;
            };
            cell = next;
            let land = !is_ocean(field, cell);
            remaining =
                ((u64::from(remaining) * if land { 500_000 } else { 880_000 }) / 1_000_000) as u32;
            if remaining == 0 {
                break;
            }
            deposit(&mut track_ppm, cell, remaining);
            if land {
                let landfall =
                    ((u64::from(intensity_ppm[origin]) * u64::from(remaining)) / 1_000_000) as u32;
                deposit(&mut intensity_ppm, cell, landfall);
                landfalls += 1;
                if landfalls >= 2 {
                    break;
                }
            }
        }
    }
    StormFields {
        suitability_ppm,
        track_ppm,
        intensity_ppm,
    }
}

fn apply_storm_metrics(
    field: &PhysicalField,
    suitability_ppm: &[u32],
    track_ppm: &[u32],
    intensity_ppm: &[u32],
    metrics: &mut ClimateMetrics,
) {
    let mut ocean_area = 0.0;
    let mut land_area = 0.0;
    let mut suitability_sum = 0.0;
    let mut prone_area = 0.0;
    let mut intensity_sum = 0.0;
    let mut land_track_sum = 0.0;
    let mut intensity_area = 0.0;
    for cell in 0..field.grid.sample_count() {
        let area = field.grid.cell_area(field.grid.row_col(cell).0);
        intensity_sum += f64::from(intensity_ppm[cell]) * area;
        intensity_area += area;
        if is_ocean(field, cell) {
            ocean_area += area;
            suitability_sum += f64::from(suitability_ppm[cell]) * area;
            if suitability_ppm[cell] >= STORM_PRONE_PPM {
                prone_area += area;
            }
        } else {
            land_area += area;
            land_track_sum += f64::from(track_ppm[cell]) * area;
        }
    }
    metrics.mean_ocean_storm_suitability_ppm = if ocean_area <= 0.0 {
        0
    } else {
        (suitability_sum / ocean_area).round() as u32
    };
    metrics.storm_prone_ocean_ppm = if ocean_area <= 0.0 {
        0
    } else {
        ((prone_area / ocean_area) * 1_000_000.0).round() as u32
    };
    metrics.mean_storm_intensity_ppm = if intensity_area <= 0.0 {
        0
    } else {
        (intensity_sum / intensity_area).round() as u32
    };
    metrics.mean_land_storm_track_ppm = if land_area <= 0.0 {
        0
    } else {
        (land_track_sum / land_area).round() as u32
    };
    metrics.expected_storms_per_year_milli =
        storm_expected_per_year_milli(metrics.mean_ocean_storm_suitability_ppm);
}

fn round_volume(volume_m3: f64) -> Result<u64, PhysicalError> {
    if !volume_m3.is_finite() || !(0.0..=(u64::MAX as f64)).contains(&volume_m3) {
        return Err(PhysicalError::coded(
            PhysicalErrorCode::NumericNonFinite,
            "climate cell volume is not finite or bounded",
        ));
    }
    Ok(volume_m3.round() as u64)
}

pub fn derive_current_climate(
    field: &PhysicalField,
    settings: ClimateSettings,
    seed: u32,
    retry_index: u32,
    progress: &mut dyn ProgressSink,
) -> Result<ClimateField, PhysicalError> {
    field.validate().map_err(PhysicalError::InvalidSource)?;
    settings.validate()?;
    progress.report(ProgressPhase::CalculatingClimate, 0, 4)?;
    let geometry = build_geometry(field, settings, progress)?;
    progress.report(ProgressPhase::CalculatingClimate, 1, 4)?;
    let current_geometry = CurrentGeometry::new(field);
    let (temperatures, summers, winters, maritime_factors, mut ocean_celsius) = temperature_field(
        field,
        settings,
        &geometry,
        &current_geometry,
        seed,
        retry_index,
        progress,
    )?;
    let winds = derive_winds(
        field,
        settings,
        &temperatures,
        &summers,
        &winters,
        seed,
        retry_index,
    );
    progress.report(ProgressPhase::CalculatingClimate, 2, 4)?;
    let (current_east, current_north) = derive_currents_with(
        field,
        settings.planetary,
        &temperatures,
        &winds.east,
        &winds.north,
        &current_geometry,
    );
    let mut climate = ClimateField {
        grid: field.grid,
        derivation_version: CLIMATE_DERIVATION_VERSION,
        planetary: settings.planetary,
        temperature_centi_c: temperatures,
        temperature_nh_summer_centi_c: summers,
        temperature_nh_winter_centi_c: winters,
        moisture_mm_per_year: Vec::new(),
        precipitation_mm_per_year: Vec::new(),
        runoff_mm_per_year: Vec::new(),
        runoff_volume_m3_per_year: Vec::new(),
        maritime_factor_ppm: maritime_factors,
        wind_east_milli: winds.east,
        wind_north_milli: winds.north,
        wind_east_nh_summer_milli: winds.east_summer,
        wind_north_nh_summer_milli: winds.north_summer,
        wind_east_nh_winter_milli: winds.east_winter,
        wind_north_nh_winter_milli: winds.north_winter,
        wind_divergence_ppm: winds.divergence_ppm,
        wind_divergence_nh_summer_ppm: winds.divergence_summer_ppm,
        wind_divergence_nh_winter_ppm: winds.divergence_winter_ppm,
        wind_band: winds.band,
        wind_band_nh_summer: winds.band_summer,
        wind_band_nh_winter: winds.band_winter,
        current_east_milli: current_east,
        current_north_milli: current_north,
        humidity_ppm: Vec::new(),
        aridity_ppm: Vec::new(),
        precipitation_nh_summer_mm: Vec::new(),
        precipitation_nh_winter_mm: Vec::new(),
        biome_class: Vec::new(),
        storm_suitability_ppm: Vec::new(),
        storm_track_ppm: Vec::new(),
        storm_intensity_ppm: Vec::new(),
        metrics: ClimateMetrics::default(),
    };
    let moisture = couple_moisture_and_latent(
        field,
        settings,
        &geometry,
        &mut climate,
        &mut ocean_celsius,
        &current_geometry,
        seed,
        retry_index,
        progress,
    )?;
    let (runoff, runoff_volume, mut metrics) = runoff_fields(
        field,
        settings,
        &climate.temperature_centi_c,
        &moisture.precipitation,
        progress,
    )?;
    metrics.transport_iterations = moisture.iterations;
    apply_seasonal_metrics(
        field,
        &climate.temperature_nh_summer_centi_c,
        &climate.temperature_nh_winter_centi_c,
        &mut metrics,
    );
    apply_wind_metrics(
        field,
        &climate.wind_east_milli,
        &climate.wind_north_milli,
        &climate.wind_divergence_ppm,
        thermal_equator_latitude(field.grid, &climate.temperature_centi_c),
        &mut metrics,
    );
    apply_current_metrics(
        field,
        &climate.current_east_milli,
        &climate.current_north_milli,
        &mut metrics,
    );
    apply_humidity_metrics(
        field,
        &moisture.humidity_ppm,
        &moisture.aridity_ppm,
        &moisture.precipitation_summer,
        &moisture.precipitation_winter,
        &mut metrics,
    );
    apply_season_length_metrics(
        field,
        &climate.temperature_nh_summer_centi_c,
        &climate.temperature_nh_winter_centi_c,
        &moisture.precipitation_summer,
        &moisture.precipitation_winter,
        &mut metrics,
    );
    progress.report(ProgressPhase::CalculatingClimate, 3, 4)?;
    climate.moisture_mm_per_year = moisture.moisture;
    climate.precipitation_mm_per_year = moisture.precipitation;
    climate.precipitation_nh_summer_mm = moisture.precipitation_summer;
    climate.precipitation_nh_winter_mm = moisture.precipitation_winter;
    climate.humidity_ppm = moisture.humidity_ppm;
    climate.aridity_ppm = moisture.aridity_ppm;
    climate.runoff_mm_per_year = runoff;
    climate.runoff_volume_m3_per_year = runoff_volume;
    climate.biome_class = classify_biomes(
        field,
        &climate.temperature_centi_c,
        &climate.temperature_nh_summer_centi_c,
        &climate.temperature_nh_winter_centi_c,
        &climate.precipitation_mm_per_year,
        &climate.humidity_ppm,
        &climate.aridity_ppm,
    );
    apply_biome_metrics(field, &climate.biome_class, &mut metrics);
    let storms = derive_storms(
        field,
        settings,
        &climate.temperature_centi_c,
        &climate.temperature_nh_summer_centi_c,
        &climate.temperature_nh_winter_centi_c,
        &climate.humidity_ppm,
        &climate.wind_east_milli,
        &climate.wind_north_milli,
        &climate.wind_east_nh_summer_milli,
        &climate.wind_north_nh_summer_milli,
        &climate.wind_east_nh_winter_milli,
        &climate.wind_north_nh_winter_milli,
        &climate.wind_divergence_ppm,
        &climate.wind_divergence_nh_summer_ppm,
        &climate.wind_divergence_nh_winter_ppm,
        &climate.current_east_milli,
        &climate.current_north_milli,
        &current_geometry,
    );
    climate.storm_suitability_ppm = storms.suitability_ppm;
    climate.storm_track_ppm = storms.track_ppm;
    climate.storm_intensity_ppm = storms.intensity_ppm;
    apply_storm_metrics(
        field,
        &climate.storm_suitability_ppm,
        &climate.storm_track_ppm,
        &climate.storm_intensity_ppm,
        &mut metrics,
    );
    apply_extreme_metrics(
        field,
        &climate.temperature_centi_c,
        &climate.temperature_nh_summer_centi_c,
        &climate.temperature_nh_winter_centi_c,
        &climate.precipitation_mm_per_year,
        &climate.precipitation_nh_summer_mm,
        &climate.precipitation_nh_winter_mm,
        &climate.aridity_ppm,
        &mut metrics,
    );
    climate.metrics = metrics;
    climate.validate_against(field)?;
    progress.report(ProgressPhase::CalculatingClimate, 4, 4)?;
    Ok(climate)
}

/// Analytic helper used by the climate exit gate. It deliberately uses the
/// exact spherical cell areas rather than raster-cell counts.
pub fn uniform_runoff_volume_m3_per_year(
    grid: Grid,
    precipitation_mm_per_year: u32,
    runoff_coefficient_ppm: u32,
) -> Result<u64, PhysicalError> {
    if runoff_coefficient_ppm > 1_000_000 {
        return Err(PhysicalError::InvalidSettings(
            "uniform runoff coefficient must be parts per million".into(),
        ));
    }
    let volume = grid.total_area() * f64::from(precipitation_mm_per_year) / 1_000.0
        * f64::from(runoff_coefficient_ppm)
        / 1_000_000.0;
    round_volume(volume)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NoopProgress, DEFAULT_RADIUS_METRES};

    fn field(grid: Grid, elevations_mm: Vec<i32>, sea_level_mm: i32) -> PhysicalField {
        PhysicalField {
            grid,
            seed: 831_429,
            retry_index: 0,
            target_land_fraction_ppm: 300_000,
            sea_level_mm,
            elevations_mm,
        }
    }

    fn naive_steps_along_row(grid: Grid, ocean: &[bool], col_delta: i32) -> Vec<u32> {
        let mut distance = vec![0u32; grid.sample_count()];
        for cell in 0..grid.sample_count() {
            if !ocean[cell] {
                continue;
            }
            let (row, col) = grid.row_col(cell);
            let mut steps = 0u32;
            loop {
                steps += 1;
                if steps >= grid.width {
                    break;
                }
                let other = grid.index(row, wrapped_col(grid, col, col_delta * steps as i32));
                if !ocean[other] {
                    break;
                }
            }
            distance[cell] = steps.saturating_sub(1);
        }
        distance
    }

    #[test]
    fn fetch_distances_match_circular_row_walk() {
        let grid = Grid::new(8, 4, DEFAULT_RADIUS_METRES).unwrap();
        let mut ocean = vec![false; grid.sample_count()];
        for col in 0..grid.width {
            ocean[grid.index(0, col)] = true;
        }
        for (col, open) in [false, true, true, false, true, true, true, false]
            .into_iter()
            .enumerate()
        {
            ocean[grid.index(1, col as u32)] = open;
        }
        for col in [0, 7] {
            ocean[grid.index(2, col)] = true;
        }
        let (west, east) = fetch_distances_along_rows(grid, &ocean);
        assert_eq!(west, naive_steps_along_row(grid, &ocean, -1));
        assert_eq!(east, naive_steps_along_row(grid, &ocean, 1));
        assert_eq!(west[grid.index(0, 0)], 7);
        assert_eq!(east[grid.index(2, 0)], 0);
        assert_eq!(west[grid.index(2, 0)], 1);
        assert_eq!(east[grid.index(2, 7)], 1);
        assert_eq!(west[grid.index(2, 7)], 0);
    }

    #[test]
    fn parallel_winds_match_serial_cells() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![200_000; grid.sample_count()];
        elevations[0] = -2_000_000;
        let physical = field(grid, elevations, 0);
        let settings = ClimateSettings::default_for(grid);
        let temperatures = (0..grid.sample_count())
            .map(|cell| 800 + (cell as i32 % 17) * 40)
            .collect::<Vec<_>>();
        let omega = omega_ratio(settings.planetary);
        let hadley = hadley_edge_radians(omega);
        let ferrel = ferrel_edge_radians(hadley);
        let itcz = 0.12;
        let wind_seed = 0x9e37_79b9_7f4a_7c15;
        let anomaly = pressure_anomaly(&physical, &temperatures, settings);
        let spacing = GridSpacing::new(grid);
        let amplitude = f64::from(settings.pressure_cell_amplitude);
        let radius = (grid.radius_metres as f64).max(1.0);
        let waves = (2.0 + 3.2 * omega.clamp(0.2, 2.4)).clamp(2.0, 8.0);
        let phase = (wind_seed as f64) * (std::f64::consts::TAU / (u64::MAX as f64));
        let wave_amp = 340.0 * omega.clamp(0.3, 2.0).sqrt();
        for meanders in [false, true] {
            let (east, north) = wind_components(
                &physical,
                &temperatures,
                settings,
                itcz,
                hadley,
                ferrel,
                wind_seed,
                meanders,
            );
            let input = WindComponentInput {
                field: &physical,
                settings,
                anomaly: &anomaly,
                spacing: &spacing,
                itcz,
                hadley,
                ferrel,
                amplitude,
                radius,
                waves,
                phase,
                wave_amp,
                meanders,
            };
            for cell in 0..grid.sample_count() {
                assert_eq!(wind_component_cell(&input, cell), (east[cell], north[cell]));
            }
            let parallel = wind_divergence_ppm(grid, &east, &north);
            for cell in 0..grid.sample_count() {
                assert_eq!(
                    parallel[cell],
                    wind_divergence_cell(grid, &spacing, &east, &north, cell)
                );
            }
        }
    }

    #[test]
    fn parallel_currents_match_serial_cells() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![200_000; grid.sample_count()];
        for cell in 0..grid.sample_count() {
            if cell.is_multiple_of(3) {
                elevations[cell] = -2_000_000;
            }
        }
        let physical = field(grid, elevations, 0);
        let settings = ClimateSettings::default_for(grid);
        let temperatures = (0..grid.sample_count())
            .map(|cell| 800 + (cell as i32 % 17) * 40)
            .collect::<Vec<_>>();
        let winds = derive_winds(
            &physical,
            settings,
            &temperatures,
            &temperatures,
            &temperatures,
            physical.seed,
            physical.retry_index,
        );
        let geometry = CurrentGeometry::new(&physical);
        let (east, north) = derive_currents_with(
            &physical,
            settings.planetary,
            &temperatures,
            &winds.east,
            &winds.north,
            &geometry,
        );
        let omega = omega_ratio(settings.planetary);
        let spacing = GridSpacing::new(grid);
        let input = CurrentForceInput {
            field: &physical,
            ocean: &geometry.ocean,
            temperatures: &temperatures,
            wind_east: &winds.east,
            wind_north: &winds.north,
            spacing: &spacing,
            west_distance: &geometry.west_distance,
            east_distance: &geometry.east_distance,
            omega,
            wind_scale: CURRENT_WIND_COUPLING * omega.clamp(0.2, 2.2).sqrt(),
        };
        let mut serial_east = vec![0.0; grid.sample_count()];
        let mut serial_north = vec![0.0; grid.sample_count()];
        for cell in 0..grid.sample_count() {
            let (u, v) = current_force_cell(&input, cell);
            serial_east[cell] = u;
            serial_north[cell] = v;
        }
        for _ in 0..CURRENT_SMOOTH_PASSES {
            let previous_east = serial_east.clone();
            let previous_north = serial_north.clone();
            for cell in 0..grid.sample_count() {
                let (u, v) = current_smooth_cell(
                    grid,
                    &geometry.ocean,
                    &previous_east,
                    &previous_north,
                    cell,
                );
                serial_east[cell] = u;
                serial_north[cell] = v;
            }
        }
        for cell in 0..grid.sample_count() {
            let (u, v) = current_bound_cell(
                grid,
                &geometry.ocean,
                serial_east[cell],
                serial_north[cell],
                cell,
            );
            assert_eq!(clamp_current(u), east[cell], "cell {cell} east");
            assert_eq!(clamp_current(v), north[cell], "cell {cell} north");
        }
    }

    fn count_biome(classes: &[u32], class: u32) -> usize {
        classes.iter().filter(|value| **value == class).count()
    }

    fn assert_near(left: f64, right: f64) {
        let scale = left.abs().max(right.abs()).max(1.0);
        assert!((left - right).abs() / scale < 1e-12, "{left} vs {right}");
    }

    #[test]
    fn grid_spacing_matches_neighbor_metres() {
        let grid = Grid::new(17, 9, DEFAULT_RADIUS_METRES).unwrap();
        let spacing = GridSpacing::new(grid);
        for row in 0..grid.height {
            for col in 0..grid.width {
                let (dx_w, dx_e, dy_s, dy_n) = spacing.spans(row);
                assert_near(dx_w, neighbor_metres(grid, row, col, 0, -1));
                assert_near(dx_e, neighbor_metres(grid, row, col, 0, 1));
                assert_near(dy_s, neighbor_metres(grid, row, col, -1, 0));
                assert_near(dy_n, neighbor_metres(grid, row, col, 1, 0));
                assert_near(
                    spacing.scale_x(row),
                    spacing_scale(neighbor_metres(grid, row, col, 0, 1)),
                );
                assert_near(
                    spacing.scale_y(row),
                    spacing_scale(neighbor_metres(grid, row, col, 1, 0)),
                );
            }
        }
    }

    #[test]
    fn solstice_insolation_is_higher_in_the_summer_hemisphere() {
        let settings =
            ClimateSettings::default_for(Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap());
        let north = 70.0_f64.to_radians();
        let south = -70.0_f64.to_radians();
        let tilt = 40.0_f64.to_radians();
        assert!(
            seasonal_insolation_weight(north, tilt, settings)
                > seasonal_insolation_weight(north, -tilt, settings)
        );
        assert!(
            seasonal_insolation_weight(south, -tilt, settings)
                > seasonal_insolation_weight(south, tilt, settings)
        );
        let mean = 0.5
            * (seasonal_insolation_weight(north, tilt, settings)
                + seasonal_insolation_weight(north, -tilt, settings));
        assert!((mean - annual_insolation_weight(north, settings)).abs() < 1e-9);
    }

    #[test]
    fn uniform_runoff_uses_exact_spherical_area() {
        let expected =
            (4.0 * std::f64::consts::PI * (DEFAULT_RADIUS_METRES as f64).powi(2) * 1_000.0
                / 1_000.0
                * 400_000.0
                / 1_000_000.0)
                .round() as u64;
        for grid in [
            Grid::new(32, 16, DEFAULT_RADIUS_METRES).unwrap(),
            Grid::new(64, 32, DEFAULT_RADIUS_METRES).unwrap(),
            Grid::new(128, 64, DEFAULT_RADIUS_METRES).unwrap(),
        ] {
            let actual = uniform_runoff_volume_m3_per_year(grid, 1_000, 400_000).unwrap();
            assert_eq!(
                actual, expected,
                "row-count bias at {}x{}",
                grid.width, grid.height
            );
        }
    }

    #[test]
    fn climate_moisture_ranges_are_worldlike() {
        let grid = Grid::new(32, 16, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![400_000; grid.sample_count()];
        for row in 6..10 {
            for col in 0..grid.width {
                elevations[grid.index(row, col)] = -4_000_000;
            }
        }
        for row in 0..grid.height {
            elevations[grid.index(row, 0)] = -4_000_000;
        }
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let land: Vec<usize> = (0..grid.sample_count())
            .filter(|cell| physical.elevations_mm[*cell] > 0)
            .collect();
        let land_precip: Vec<u32> = land
            .iter()
            .map(|cell| climate.precipitation_mm_per_year[*cell])
            .collect();
        let mean_p = mean_u32(&land_precip);
        let max_p = *land_precip.iter().max().unwrap();
        let coastal = grid.index(12, 1);
        let interior = grid.index(12, 16);
        assert!(
            max_p > 200 && mean_p > 20,
            "land rainfall too dry: mean={mean_p} max={max_p}"
        );
        assert!(
            climate.precipitation_mm_per_year[coastal]
                > climate.precipitation_mm_per_year[interior],
            "coastal {} interior {}",
            climate.precipitation_mm_per_year[coastal],
            climate.precipitation_mm_per_year[interior]
        );
        assert!(climate.moisture_mm_per_year[coastal] > climate.moisture_mm_per_year[interior]);
        assert!(climate.wind_east_milli[grid.index(11, 16)] > 0);
        assert!(land
            .iter()
            .any(|cell| climate.precipitation_mm_per_year[*cell] > 0));
    }

    #[test]
    fn continent_coast_is_wetter_than_interior_across_deep_ocean() {
        let grid = Grid::new(64, 32, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-4_000_000; grid.sample_count()];
        for row in 8..24 {
            for col in 16..48 {
                elevations[grid.index(row, col)] = 300_000;
            }
        }
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let coast = grid.index(16, 16);
        let inland = grid.index(16, 32);
        let inland_precip = climate.precipitation_mm_per_year[inland];
        let coast_precip = climate.precipitation_mm_per_year[coast];
        assert!(
            coast_precip > inland_precip,
            "coast should stay wetter than interior, coast={coast_precip} inland={inland_precip}"
        );
        assert!(
            coast_precip > 40,
            "coast should stay wet next to deep ocean, got {coast_precip} inland={inland_precip}"
        );
        assert!(climate.humidity_ppm[inland] <= climate.humidity_ppm[coast]);
    }

    #[test]
    fn moisture_transport_converges_on_a_preview_span() {
        let grid = Grid::new(128, 64, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-4_000_000; grid.sample_count()];
        for row in 16..48 {
            for col in 24..104 {
                elevations[grid.index(row, col)] = 400_000;
            }
        }
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        assert!(climate.metrics.transport_iterations <= transport_iteration_limit(grid));
        assert!(climate.metrics.transport_iterations >= CLIMATE_MIN_TRANSPORT_ITERATIONS);
        let inland = grid.index(32, 64);
        let coast = grid.index(32, 24);
        assert!(climate.precipitation_mm_per_year[coast] > 0);
        assert!(
            climate.moisture_mm_per_year[inland] <= climate.moisture_mm_per_year[coast],
            "inland moisture {} coast {}",
            climate.moisture_mm_per_year[inland],
            climate.moisture_mm_per_year[coast]
        );
    }

    #[test]
    fn temperature_falls_with_latitude_and_maritime_distance_is_bounded() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut progress = NoopProgress;
        let physical = field(grid, vec![0; grid.sample_count()], 1);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut progress,
        )
        .unwrap();
        assert!(
            climate.temperature_centi_c[grid.index(3, 0)]
                > climate.temperature_centi_c[grid.index(0, 0)]
        );
        assert!(climate
            .maritime_factor_ppm
            .iter()
            .all(|value| *value == 1_000_000));
        assert!(
            climate.metrics.minimum_temperature_centi_c <= climate.metrics.mean_temperature_centi_c
        );
        assert!(
            climate.metrics.mean_temperature_centi_c <= climate.metrics.maximum_temperature_centi_c
        );
    }

    #[test]
    fn seafloor_depth_does_not_warm_ocean_air() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let settings = ClimateSettings::default_for(grid);
        let shallow = field(grid, vec![-100_000; grid.sample_count()], 0);
        let deep = field(grid, vec![-4_000_000; grid.sample_count()], 0);
        let shallow_climate = derive_current_climate(
            &shallow,
            settings,
            shallow.seed,
            shallow.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let deep_climate = derive_current_climate(
            &deep,
            settings,
            deep.seed,
            deep.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        assert_eq!(
            shallow_climate.temperature_centi_c,
            deep_climate.temperature_centi_c
        );
        let mut mixed_shallow = vec![-100_000; grid.sample_count()];
        let mut mixed_deep = vec![-4_000_000; grid.sample_count()];
        for col in 0..grid.width {
            mixed_shallow[grid.index(4, col)] = 200_000;
            mixed_deep[grid.index(4, col)] = 200_000;
        }
        let land_shallow = derive_current_climate(
            &field(grid, mixed_shallow, 0),
            settings,
            831_429,
            0,
            &mut NoopProgress,
        )
        .unwrap();
        let land_deep = derive_current_climate(
            &field(grid, mixed_deep, 0),
            settings,
            831_429,
            0,
            &mut NoopProgress,
        )
        .unwrap();
        for col in 0..grid.width {
            let land = grid.index(4, col);
            assert_eq!(
                land_shallow.temperature_centi_c[land],
                land_deep.temperature_centi_c[land]
            );
            let ocean = grid.index(3, col);
            assert_eq!(
                land_shallow.temperature_centi_c[ocean],
                land_deep.temperature_centi_c[ocean]
            );
        }
    }

    #[test]
    fn altitude_lapse_cools_land_above_sea_level() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-200_000; grid.sample_count()];
        let row = 4;
        for col in 0..grid.width {
            elevations[grid.index(row, col)] = 200_000;
        }
        elevations[grid.index(row, 4)] = 200_000;
        elevations[grid.index(row, 5)] = 2_200_000;
        let physical = field(grid, elevations, 0);
        let before = physical.elevations_mm.clone();
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        assert_eq!(physical.elevations_mm, before);
        let low = grid.index(row, 4);
        let high = grid.index(row, 5);
        assert!(
            climate.temperature_centi_c[high] < climate.temperature_centi_c[low],
            "high {} low {}",
            climate.temperature_centi_c[high],
            climate.temperature_centi_c[low]
        );
    }

    #[test]
    fn interiors_are_more_seasonal_than_oceans_without_maritime_blend() {
        let grid = Grid::new(32, 16, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-4_000_000; grid.sample_count()];
        let row = 12;
        for col in 8..24 {
            elevations[grid.index(row, col)] = 200_000;
            elevations[grid.index(row - 1, col)] = 200_000;
            elevations[grid.index(row + 1, col)] = 200_000;
        }
        let climate = derive_current_climate(
            &field(grid, elevations, 0),
            ClimateSettings::default_for(grid),
            831_429,
            0,
            &mut NoopProgress,
        )
        .unwrap();
        let coast = grid.index(row, 8);
        let interior = grid.index(row, 16);
        assert!(climate.maritime_factor_ppm[coast] > climate.maritime_factor_ppm[interior]);
        let ocean = grid.index(row, 0);
        let land_range = (climate.temperature_nh_summer_centi_c[interior]
            - climate.temperature_nh_winter_centi_c[interior])
            .unsigned_abs();
        let ocean_range = (climate.temperature_nh_summer_centi_c[ocean]
            - climate.temperature_nh_winter_centi_c[ocean])
            .unsigned_abs();
        assert!(
            land_range > ocean_range,
            "interior range {land_range} ocean range {ocean_range}"
        );
    }

    #[test]
    fn maritime_scale_does_not_change_temperature() {
        let grid = Grid::new(32, 16, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-4_000_000; grid.sample_count()];
        let row = 12;
        for col in 8..24 {
            elevations[grid.index(row, col)] = 200_000;
        }
        let physical = field(grid, elevations, 0);
        let mut near = ClimateSettings::default_for(grid);
        near.maritime_scale_km = 1;
        let mut far = ClimateSettings::default_for(grid);
        far.maritime_scale_km = 20_000;
        let short = derive_current_climate(&physical, near, 831_429, 0, &mut NoopProgress).unwrap();
        let long = derive_current_climate(&physical, far, 831_429, 0, &mut NoopProgress).unwrap();
        assert_ne!(short.maritime_factor_ppm, long.maritime_factor_ppm);
        assert_eq!(short.temperature_centi_c, long.temperature_centi_c);
        assert_eq!(
            short.temperature_nh_summer_centi_c,
            long.temperature_nh_summer_centi_c
        );
        assert_eq!(
            short.temperature_nh_winter_centi_c,
            long.temperature_nh_winter_centi_c
        );
    }

    #[test]
    fn energy_balance_errors_when_iteration_cap_is_too_low() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let physical = field(grid, vec![0; grid.sample_count()], 1);
        let mut settings = ClimateSettings::default_for(grid);
        settings.energy_balance_max_iterations = 2;
        let error = derive_current_climate(
            &physical,
            settings,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap_err();
        assert_eq!(
            error.code(),
            PhysicalErrorCode::NumericNonConvergent.as_str()
        );
    }

    #[test]
    fn year_loop_errors_when_year_cap_is_too_low() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let physical = field(grid, vec![0; grid.sample_count()], 1);
        let mut settings = ClimateSettings::default_for(grid);
        settings.seasonal_year_max = 1;
        let error = derive_current_climate(
            &physical,
            settings,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap_err();
        assert_eq!(
            error.code(),
            PhysicalErrorCode::NumericNonConvergent.as_str()
        );
        assert!(error.to_string().contains("seasonal year loop"));
    }

    #[test]
    fn year_loop_rejects_delta_above_tolerance_after_min_years() {
        assert!(!seasonal_year_converged(1, 0.0, 0.0));
        assert!(!seasonal_year_converged(2, YEAR_T_TOLERANCE_C + 0.01, 0.0));
        assert!(!seasonal_year_converged(
            2,
            YEAR_T_TOLERANCE_C,
            YEAR_T_TOLERANCE_C + 0.01
        ));
        assert!(seasonal_year_converged(
            2,
            YEAR_T_TOLERANCE_C,
            YEAR_T_TOLERANCE_C
        ));
        assert!(seasonal_year_converged(12, 0.25, 0.1));
    }

    #[test]
    fn tiny_world_still_produces_precipitation() {
        let grid = Grid::new(8, 4, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[grid.index(2, 4)] = -2_000;
        let climate = derive_current_climate(
            &field(grid, elevations, 0),
            ClimateSettings::default_for(grid),
            831_429,
            0,
            &mut NoopProgress,
        )
        .unwrap();
        assert!(climate.metrics.precipitation_volume_m3_per_year > 0);
        assert!(climate.metrics.mean_land_growing_season_ppm <= 1_000_000);
    }

    #[test]
    fn climate_field_rejects_stale_derivation_version() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let physical = field(grid, vec![0; grid.sample_count()], 1);
        let mut climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        climate.derivation_version = CLIMATE_DERIVATION_VERSION.saturating_sub(1);
        assert!(climate.validate().is_err());
    }

    #[test]
    fn ice_albedo_cools_frozen_cells_relative_to_open_water() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-200_000; grid.sample_count()];
        let row = 4;
        for col in 3..7 {
            elevations[grid.index(row, col)] = 4_000_000;
        }
        let settings = ClimateSettings::default_for(grid);
        let climate = derive_current_climate(
            &field(grid, elevations, 0),
            settings,
            831_429,
            0,
            &mut NoopProgress,
        )
        .unwrap();
        let lapse = 4.0 * f64::from(settings.altitude_lapse_centi_c_per_km);
        let mountain = grid.index(row, 5);
        let ocean = grid.index(row, 12);
        assert!(climate.temperature_centi_c[mountain] < 0);
        assert!(climate.temperature_centi_c[ocean] > 0);
        assert!(
            f64::from(climate.temperature_centi_c[mountain]) + lapse
                < f64::from(climate.temperature_centi_c[ocean]),
            "mountain {} ocean {} lapse {lapse}",
            climate.temperature_centi_c[mountain],
            climate.temperature_centi_c[ocean]
        );
    }

    #[test]
    fn transport_is_periodic_across_the_antimeridian() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-2_000; grid.sample_count()];
        for row in 4..8 {
            for col in 0..grid.width {
                elevations[grid.index(row, col)] = 1_000;
            }
        }
        let physical = field(grid, elevations, 0);
        let mut progress = NoopProgress;
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut progress,
        )
        .unwrap();
        for row in 0..grid.height {
            let west = climate.precipitation_mm_per_year[grid.index(row, 0)] as i32;
            let east = climate.precipitation_mm_per_year[grid.index(row, grid.width - 1)] as i32;
            let interior = climate.precipitation_mm_per_year[grid.index(row, 1)] as i32;
            let wrap_gap = (west - east).unsigned_abs();
            let neighbor_gap = (west - interior).unsigned_abs();
            assert!(
                wrap_gap <= neighbor_gap.saturating_add(80),
                "antimeridian seam at row {row}: wrap {wrap_gap} vs neighbor {neighbor_gap}"
            );
        }
    }

    #[test]
    fn ridge_creates_windward_precipitation_and_leeward_shadow() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-2_000; grid.sample_count()];
        let row = 3;
        for col in 1..15 {
            elevations[grid.index(row, col)] = 500;
        }
        elevations[grid.index(row, 8)] = 2_000_000;
        let physical = field(grid, elevations, 0);
        let mut progress = NoopProgress;
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut progress,
        )
        .unwrap();
        let ridge = grid.index(row, 8);
        let from_west = climate.wind_east_milli[ridge] >= 0;
        let upwind_col = if from_west { 7 } else { 9 };
        let leeward_col = if from_west { 9 } else { 7 };
        let windward = climate.precipitation_mm_per_year[ridge];
        let leeward_precipitation = climate.precipitation_mm_per_year[grid.index(row, leeward_col)];
        let leeward = climate.moisture_mm_per_year[grid.index(row, leeward_col)];
        let upwind = climate.moisture_mm_per_year[grid.index(row, upwind_col)];
        assert!(windward > leeward_precipitation);
        assert!(upwind > leeward, "upwind={upwind} leeward={leeward}");
        assert!(climate.metrics.transport_iterations <= CLIMATE_MAX_TRANSPORT_ITERATIONS);
    }

    #[test]
    fn orographic_cooling_does_not_write_surface_temperature() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-2_000; grid.sample_count()];
        let row = 3;
        for col in 1..15 {
            elevations[grid.index(row, col)] = 500;
        }
        elevations[grid.index(row, 8)] = 2_000_000;
        let physical = field(grid, elevations, 0);
        let mut without_uplift = ClimateSettings::default_for(grid);
        without_uplift.orographic_precipitation_ppm = 0;
        without_uplift.latent_heat_coupling_ppm = 0;
        without_uplift.moisture_temperature_coupling_passes = 1;
        without_uplift.cloud_albedo_coupling_ppm = 0;
        without_uplift.cloud_olr_reduction_ppm = 0;
        without_uplift.cloud_rain_ppm = 0;
        without_uplift.albedo_bare_ppm = without_uplift.albedo_land_ppm;
        without_uplift.albedo_desert_ppm = without_uplift.albedo_land_ppm;
        without_uplift.albedo_snow_ppm = without_uplift.albedo_ice_ppm;
        let mut with_uplift = without_uplift;
        with_uplift.orographic_precipitation_ppm = 18_000_000;
        let baseline = derive_current_climate(
            &physical,
            without_uplift,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let cooled = derive_current_climate(
            &physical,
            with_uplift,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let ridge = grid.index(row, 8);
        assert_eq!(baseline.temperature_centi_c, cooled.temperature_centi_c);
        assert_ne!(
            baseline.precipitation_mm_per_year[ridge],
            cooled.precipitation_mm_per_year[ridge]
        );
    }

    #[test]
    fn saturation_moisture_stays_bounded_for_extreme_orographic_cooling() {
        let floor = saturation_moisture_mm((MAGNUS_T_MIN_C * 100.0) as i32);
        let exploded = saturation_moisture_mm(-50_000);
        assert!(floor >= 80.0);
        assert_eq!(exploded, floor);
        let t_eff = orographic_saturation_centi_c(2_000, 1.0, 50_000_000);
        assert!(t_eff >= (MAGNUS_T_MIN_C * 100.0) as i32);
        assert!(t_eff <= (MAGNUS_T_MAX_C * 100.0) as i32);
        assert!(t_eff >= 2_000 - (MAX_OROGRAPHIC_COOLING_C * 100.0) as i32);
    }

    #[test]
    fn saturation_lookup_matches_magnus() {
        for centi in [-4_000, -1_500, 0, 850, 2_000, 3_500] {
            let looked_up = saturation_moisture_mm(centi);
            let direct = magnus_saturation_mm(f64::from(centi) / 100.0);
            assert!(
                (looked_up - direct).abs() <= 1e-9,
                "centi {centi} lut {looked_up} magnus {direct}"
            );
        }
    }

    #[test]
    fn six_class_albedo_distinguishes_desert_snow_and_ice() {
        let settings =
            ClimateSettings::default_for(Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap());
        assert_eq!(classify_surface(true, 1_800, 0, 0), SurfaceClass::Ocean);
        assert_eq!(classify_surface(true, -200, 0, 0), SurfaceClass::Ice);
        assert_eq!(classify_surface(false, -400, 200, 200), SurfaceClass::Snow);
        assert_eq!(classify_surface(false, 2_400, 80, 40), SurfaceClass::Desert);
        assert_eq!(classify_surface(false, 1_800, 300, 200), SurfaceClass::Bare);
        assert_eq!(
            classify_surface(false, 1_800, 600, 400),
            SurfaceClass::Vegetated
        );
        assert!(
            surface_albedo(SurfaceClass::Snow, settings)
                > surface_albedo(SurfaceClass::Ice, settings)
        );
        assert!(
            surface_albedo(SurfaceClass::Desert, settings)
                > surface_albedo(SurfaceClass::Vegetated, settings)
        );
        assert!(
            surface_albedo(SurfaceClass::Bare, settings)
                > surface_albedo(SurfaceClass::Vegetated, settings)
        );
    }

    #[test]
    fn clouds_increase_albedo_and_condensation_scale() {
        let settings =
            ClimateSettings::default_for(Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap());
        let surface = surface_albedo(SurfaceClass::Ocean, settings);
        assert!(mixed_albedo(surface, 1.0, settings) > mixed_albedo(surface, 0.0, settings));
        assert!(settings.cloud_rain_scale(1.0) > settings.cloud_rain_scale(0.0));
        assert!(settings.cloud_olr_factor(1.0) < settings.cloud_olr_factor(0.0));
        assert!(cloud_fraction(2_000.0, 2_000.0, 0.0) > cloud_fraction(200.0, 2_000.0, 0.0));
        assert!(cloud_fraction(1_000.0, 2_000.0, 0.01) > cloud_fraction(1_000.0, 2_000.0, 0.0));
    }

    #[test]
    fn growing_season_is_longer_in_the_tropics_than_at_the_pole() {
        assert!(sinusoid_above_fraction(2_400.0, 2_200.0, 500.0) > 0.9);
        assert!(sinusoid_above_fraction(-2_000.0, -3_000.0, 500.0) < 0.1);
        assert!(sinusoid_above_fraction(2_000.0, -1_000.0, 500.0) > 0.3);
        assert!(sinusoid_above_fraction(2_000.0, -1_000.0, 500.0) < 0.8);
        let dry = 1.0 - sinusoid_above_fraction(80.0, 40.0, 250.0);
        let wet = sinusoid_above_fraction(1_200.0, 900.0, 500.0);
        assert!(dry > 0.9);
        assert!(wet > 0.9);
    }

    #[test]
    fn coastal_moisture_drives_interior_drying() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-2_000; grid.sample_count()];
        let row = 3;
        for col in 1..grid.width {
            elevations[grid.index(row, col)] = 500;
        }
        let physical = field(grid, elevations, 0);
        let mut progress = NoopProgress;
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut progress,
        )
        .unwrap();
        let coastal = climate.moisture_mm_per_year[grid.index(row, grid.width - 1)];
        let interior = climate.moisture_mm_per_year[grid.index(row, 8)];
        assert!(coastal > interior, "coastal={coastal} interior={interior}");
        assert!(
            climate.aridity_ppm[grid.index(row, 8)]
                > climate.aridity_ppm[grid.index(row, grid.width - 1)]
        );
    }

    fn mean_u32(values: &[u32]) -> u64 {
        values.iter().map(|value| u64::from(*value)).sum::<u64>() / values.len() as u64
    }

    #[test]
    fn warmer_sea_surface_increases_ocean_moisture() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let elevations = vec![-2_000; grid.sample_count()];
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let zeros = vec![0i32; grid.sample_count()];
        let cold = vec![-800; grid.sample_count()];
        let warm = vec![2_800; grid.sample_count()];
        let settings = ClimateSettings::default_for(grid);
        let seed = derive_subsystem_seed(physical.seed, physical.retry_index, SeedDomain::Climate);
        let cold_precip = transport_moisture(
            &physical,
            settings,
            seed,
            &cold,
            &zeros,
            &zeros,
            &climate.wind_east_milli,
            &climate.wind_north_milli,
            &climate.wind_divergence_ppm,
            &climate.wind_band,
            &mut NoopProgress,
        )
        .unwrap();
        let warm_precip = transport_moisture(
            &physical,
            settings,
            seed,
            &warm,
            &zeros,
            &zeros,
            &climate.wind_east_milli,
            &climate.wind_north_milli,
            &climate.wind_divergence_ppm,
            &climate.wind_band,
            &mut NoopProgress,
        )
        .unwrap();
        assert!(
            mean_u32(&warm_precip.precipitation) > mean_u32(&cold_precip.precipitation),
            "warm {} cold {}",
            mean_u32(&warm_precip.precipitation),
            mean_u32(&cold_precip.precipitation)
        );
    }

    #[test]
    fn surface_currents_increase_ocean_evaporation() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let elevations = vec![-2_000; grid.sample_count()];
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let zeros = vec![0i32; grid.sample_count()];
        let settings = ClimateSettings::default_for(grid);
        let seed = derive_subsystem_seed(physical.seed, physical.retry_index, SeedDomain::Climate);
        let still = transport_moisture(
            &physical,
            settings,
            seed,
            &climate.temperature_centi_c,
            &zeros,
            &zeros,
            &climate.wind_east_milli,
            &climate.wind_north_milli,
            &climate.wind_divergence_ppm,
            &climate.wind_band,
            &mut NoopProgress,
        )
        .unwrap();
        let moving = transport_moisture(
            &physical,
            settings,
            seed,
            &climate.temperature_centi_c,
            &climate.current_east_milli,
            &climate.current_north_milli,
            &climate.wind_east_milli,
            &climate.wind_north_milli,
            &climate.wind_divergence_ppm,
            &climate.wind_band,
            &mut NoopProgress,
        )
        .unwrap();
        assert!(
            mean_u32(&moving.precipitation) >= mean_u32(&still.precipitation),
            "moving {} still {}",
            mean_u32(&moving.precipitation),
            mean_u32(&still.precipitation)
        );
        assert_ne!(moving.precipitation, still.precipitation);
    }

    #[test]
    fn seasonal_precipitation_differs_when_tilt_is_large() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-4_000_000; grid.sample_count()];
        for row in 2..6 {
            for col in 4..12 {
                elevations[grid.index(row, col)] = 200_000;
            }
        }
        let physical = field(grid, elevations, 0);
        let mut settings = ClimateSettings::default_for(grid);
        settings.planetary.axial_tilt_milli_deg = 35_000;
        let climate = derive_current_climate(
            &physical,
            settings,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        assert_ne!(
            climate.precipitation_nh_summer_mm,
            climate.precipitation_nh_winter_mm
        );
        assert!(
            (0..grid.sample_count()).any(|cell| {
                physical.elevations_mm[cell] > physical.sea_level_mm
                    && climate.precipitation_nh_summer_mm[cell]
                        != climate.precipitation_nh_winter_mm[cell]
            }),
            "tilt should change precipitation on land, not only ocean"
        );
        assert!(climate.metrics.mean_seasonal_precipitation_range_mm > 0);
        assert!(climate.metrics.mean_humidity_ppm > 0);
        assert!(climate.metrics.mean_humidity_ppm <= 1_000_000);
        assert!(climate.metrics.mean_land_growing_season_ppm <= 1_000_000);
        assert!(climate.metrics.mean_land_dry_season_ppm <= 1_000_000);
        assert!(climate.metrics.mean_land_wet_season_ppm <= 1_000_000);
    }

    #[test]
    fn humidity_tracks_incoming_moisture_versus_saturation() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-2_000; grid.sample_count()];
        let row = 3;
        for col in 1..grid.width {
            elevations[grid.index(row, col)] = 500;
        }
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let coastal = grid.index(row, grid.width - 1);
        let interior = grid.index(row, 8);
        assert!(climate.humidity_ppm[coastal] <= 1_000_000);
        assert!(
            climate.humidity_ppm[coastal] > climate.humidity_ppm[interior],
            "coastal humidity {} interior {}",
            climate.humidity_ppm[coastal],
            climate.humidity_ppm[interior]
        );
        assert!(
            climate.humidity_ppm[interior] < 850_000,
            "interior humidity should not sit at saturation {}",
            climate.humidity_ppm[interior]
        );
        assert!(
            climate
                .humidity_ppm
                .iter()
                .zip(&physical.elevations_mm)
                .filter(|(_, elevation)| **elevation > 0)
                .any(|(humidity, _)| *humidity < 850_000),
            "some land should stay below saturation"
        );
        let coastal_sat = saturation_moisture_mm(climate.temperature_centi_c[coastal]).max(1.0);
        let expected = ((f64::from(climate.moisture_mm_per_year[coastal]) / coastal_sat)
            * 1_000_000.0)
            .round()
            .clamp(0.0, 1_000_000.0) as u32;
        assert_eq!(climate.humidity_ppm[coastal], expected);
    }

    #[test]
    fn condensation_warms_and_evaporation_cools_temperature() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let elevations = vec![-2_000; grid.sample_count()];
        let physical = field(grid, elevations, 0);
        let mut no_latent = ClimateSettings::default_for(grid);
        no_latent.latent_heat_coupling_ppm = 0;
        no_latent.moisture_temperature_coupling_passes = 1;
        let mut coupled = ClimateSettings::default_for(grid);
        coupled.latent_heat_coupling_ppm = 80_000;
        coupled.moisture_temperature_coupling_passes = 8;
        let dry = derive_current_climate(
            &physical,
            no_latent,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let wet = derive_current_climate(
            &physical,
            coupled,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let seed = derive_subsystem_seed(physical.seed, physical.retry_index, SeedDomain::Climate);
        let transport = transport_moisture(
            &physical,
            ClimateSettings::default_for(grid),
            seed,
            &wet.temperature_centi_c,
            &wet.current_east_milli,
            &wet.current_north_milli,
            &wet.wind_east_milli,
            &wet.wind_north_milli,
            &wet.wind_divergence_ppm,
            &wet.wind_band,
            &mut NoopProgress,
        )
        .unwrap();
        let mut condensing = 0usize;
        let mut evaporating = 0usize;
        let mut condensing_delta = 0i64;
        let mut evaporating_delta = 0i64;
        for cell in 0..grid.sample_count() {
            let net = transport.condensation_mm[cell] - transport.evaporation_mm[cell];
            let delta =
                i64::from(wet.temperature_centi_c[cell]) - i64::from(dry.temperature_centi_c[cell]);
            if net > 50.0 {
                condensing += 1;
                condensing_delta += delta;
            } else if net < -50.0 {
                evaporating += 1;
                evaporating_delta += delta;
            }
        }
        assert!(condensing > 0 || evaporating > 0);
        if condensing > 0 && evaporating > 0 {
            assert!(
                condensing_delta * evaporating as i64 > evaporating_delta * condensing as i64,
                "condensation cells should stay warmer than evaporation cells: cond {condensing_delta}/{condensing} evap {evaporating_delta}/{evaporating}"
            );
        } else if condensing > 0 {
            assert!(
                condensing_delta > 0,
                "condensation should warm: {condensing_delta} over {condensing} cells"
            );
        } else {
            assert!(
                evaporating_delta < 0,
                "evaporation should cool: {evaporating_delta} over {evaporating} cells"
            );
        }
    }

    #[test]
    fn moisture_and_precipitation_stay_non_negative() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[0] = -2_000;
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        assert!(climate
            .moisture_mm_per_year
            .iter()
            .all(|value| *value <= MAX_CLIMATE_MOISTURE_MM));
        assert!(climate
            .precipitation_mm_per_year
            .iter()
            .all(|value| *value <= MAX_CLIMATE_PRECIPITATION_MM));
        assert!(climate.humidity_ppm.iter().all(|value| *value <= 1_000_000));
    }

    #[test]
    fn frozen_seas_evaporate_less_than_open_water() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let elevations = vec![-2_000; grid.sample_count()];
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let zeros = vec![0i32; grid.sample_count()];
        let frozen = vec![-400; grid.sample_count()];
        let open = vec![200; grid.sample_count()];
        let settings = ClimateSettings::default_for(grid);
        let seed = derive_subsystem_seed(physical.seed, physical.retry_index, SeedDomain::Climate);
        let frozen_precip = transport_moisture(
            &physical,
            settings,
            seed,
            &frozen,
            &zeros,
            &zeros,
            &climate.wind_east_milli,
            &climate.wind_north_milli,
            &climate.wind_divergence_ppm,
            &climate.wind_band,
            &mut NoopProgress,
        )
        .unwrap();
        let open_precip = transport_moisture(
            &physical,
            settings,
            seed,
            &open,
            &zeros,
            &zeros,
            &climate.wind_east_milli,
            &climate.wind_north_milli,
            &climate.wind_divergence_ppm,
            &climate.wind_band,
            &mut NoopProgress,
        )
        .unwrap();
        assert!(
            mean_u32(&frozen_precip.precipitation) < mean_u32(&open_precip.precipitation),
            "frozen {} open {}",
            mean_u32(&frozen_precip.precipitation),
            mean_u32(&open_precip.precipitation)
        );
    }

    #[test]
    fn climate_derivation_does_not_modify_terrain() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[0] = -2_000;
        let physical = field(grid, elevations.clone(), 0);
        let _ = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        assert_eq!(physical.elevations_mm, elevations);
        assert_eq!(physical.sea_level_mm, 0);
    }

    #[test]
    fn solstice_winds_change_seasonal_precipitation() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-4_000_000; grid.sample_count()];
        for row in 2..6 {
            for col in 4..12 {
                elevations[grid.index(row, col)] = 200_000;
            }
        }
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        assert_ne!(
            climate.precipitation_nh_summer_mm,
            climate.precipitation_nh_winter_mm
        );
        assert_ne!(
            climate.wind_east_nh_summer_milli,
            climate.wind_east_nh_winter_milli
        );
        assert_ne!(
            climate.wind_north_nh_summer_milli,
            climate.wind_north_nh_winter_milli
        );
    }

    #[test]
    fn hydrology_presets_change_coherent_water_response() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        for col in 0..grid.width {
            elevations[grid.index(3, col)] = -2_000;
        }
        let physical = field(grid, elevations, 0);
        let mut arid_settings = ClimateSettings::default_for(grid);
        arid_settings.hydrology_preset = HydrologyPreset::Arid;
        let mut wet_settings = ClimateSettings::default_for(grid);
        wet_settings.hydrology_preset = HydrologyPreset::Wet;
        let mut progress = NoopProgress;
        let arid = derive_current_climate(
            &physical,
            arid_settings,
            physical.seed,
            physical.retry_index,
            &mut progress,
        )
        .unwrap();
        let mut progress = NoopProgress;
        let wet = derive_current_climate(
            &physical,
            wet_settings,
            physical.seed,
            physical.retry_index,
            &mut progress,
        )
        .unwrap();
        assert!(
            wet.metrics.precipitation_volume_m3_per_year
                > arid.metrics.precipitation_volume_m3_per_year
        );
        assert!(wet.metrics.runoff_volume_m3_per_year > arid.metrics.runoff_volume_m3_per_year);
    }

    #[test]
    fn climate_is_deterministic_and_runoff_is_land_only() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        for col in 0..grid.width {
            elevations[grid.index(3, col)] = -2_000;
        }
        let physical = field(grid, elevations, 0);
        let mut progress = NoopProgress;
        let first = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut progress,
        )
        .unwrap();
        let mut progress = NoopProgress;
        let second = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut progress,
        )
        .unwrap();
        assert_eq!(first, second);
        assert!(first
            .runoff_mm_per_year
            .iter()
            .enumerate()
            .all(
                |(cell, value)| physical.elevations_mm[cell] > physical.sea_level_mm || *value == 0
            ));
        assert!(first
            .runoff_mm_per_year
            .iter()
            .enumerate()
            .any(
                |(cell, value)| physical.elevations_mm[cell] > physical.sea_level_mm && *value > 0
            ));
        assert!(
            first.metrics.precipitation_volume_m3_per_year
                >= first.metrics.runoff_volume_m3_per_year
        );
        let mut invalid = first.clone();
        invalid.runoff_volume_m3_per_year[1] += 1;
        assert!(invalid.validate_against(&physical).is_err());
    }

    #[test]
    fn wet_cells_keep_evaporative_supply_after_rain() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-2_000; grid.sample_count()];
        let row = 3;
        for col in 1..grid.width {
            elevations[grid.index(row, col)] = 500;
        }
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let seed = derive_subsystem_seed(physical.seed, physical.retry_index, SeedDomain::Climate);
        let transport = transport_moisture(
            &physical,
            ClimateSettings::default_for(grid),
            seed,
            &climate.temperature_centi_c,
            &climate.current_east_milli,
            &climate.current_north_milli,
            &climate.wind_east_milli,
            &climate.wind_north_milli,
            &climate.wind_divergence_ppm,
            &climate.wind_band,
            &mut NoopProgress,
        )
        .unwrap();
        let coast = grid.index(row, grid.width - 1);
        let interior = grid.index(row, 8);
        assert!(climate.precipitation_mm_per_year[coast] > 0);
        assert!(transport.evaporation_mm[coast] > 0.0);
        assert!(
            transport.evaporation_mm[coast] > transport.evaporation_mm[interior],
            "coast E {} interior E {}",
            transport.evaporation_mm[coast],
            transport.evaporation_mm[interior]
        );
        assert!(transport.water_mm[coast] > 0.0);
    }

    #[test]
    fn dry_interiors_do_not_emit_ocean_like_moisture() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-2_000; grid.sample_count()];
        let row = 3;
        for col in 1..grid.width {
            elevations[grid.index(row, col)] = 500;
        }
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let seed = derive_subsystem_seed(physical.seed, physical.retry_index, SeedDomain::Climate);
        let transport = transport_moisture(
            &physical,
            ClimateSettings::default_for(grid),
            seed,
            &climate.temperature_centi_c,
            &climate.current_east_milli,
            &climate.current_north_milli,
            &climate.wind_east_milli,
            &climate.wind_north_milli,
            &climate.wind_divergence_ppm,
            &climate.wind_band,
            &mut NoopProgress,
        )
        .unwrap();
        let ocean = grid.index(row, 0);
        let interior = grid.index(row, 8);
        assert!(physical.elevations_mm[ocean] <= physical.sea_level_mm);
        assert!(
            transport.evaporation_mm[interior] < transport.evaporation_mm[ocean] * 0.5,
            "interior E {} ocean E {}",
            transport.evaporation_mm[interior],
            transport.evaporation_mm[ocean]
        );
    }

    #[test]
    fn surface_water_store_stays_non_negative() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[0] = -2_000;
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let seed = derive_subsystem_seed(physical.seed, physical.retry_index, SeedDomain::Climate);
        let transport = transport_moisture(
            &physical,
            ClimateSettings::default_for(grid),
            seed,
            &climate.temperature_centi_c,
            &climate.current_east_milli,
            &climate.current_north_milli,
            &climate.wind_east_milli,
            &climate.wind_north_milli,
            &climate.wind_divergence_ppm,
            &climate.wind_band,
            &mut NoopProgress,
        )
        .unwrap();
        assert!(transport.water_mm.iter().all(|value| *value >= 0.0));
        assert_eq!(transport.water_mm[0], 0.0);
        assert!(transport.water_mm[1..].iter().any(|value| *value > 0.0));
        for cell in 1..grid.sample_count() {
            if climate.temperature_centi_c[cell] <= 0 {
                assert_eq!(transport.water_mm[cell], 0.0);
            }
        }
    }

    #[test]
    fn frozen_land_stores_no_surface_water() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[0] = -2_000;
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let seed = derive_subsystem_seed(physical.seed, physical.retry_index, SeedDomain::Climate);
        let frozen = vec![-400; grid.sample_count()];
        let transport = transport_moisture(
            &physical,
            ClimateSettings::default_for(grid),
            seed,
            &frozen,
            &climate.current_east_milli,
            &climate.current_north_milli,
            &climate.wind_east_milli,
            &climate.wind_north_milli,
            &climate.wind_divergence_ppm,
            &climate.wind_band,
            &mut NoopProgress,
        )
        .unwrap();
        assert!(transport.water_mm.iter().all(|value| *value == 0.0));
        assert!(transport.evaporation_mm[1..]
            .iter()
            .all(|value| *value == 0.0));
    }

    #[test]
    fn surface_water_close_matches_budget_and_clears_ocean() {
        let settings =
            ClimateSettings::default_for(Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap());
        assert_eq!(
            close_surface_water_mm(settings, false, 1_500, 800.0, 100.0, 50.0),
            0.0
        );
        assert_eq!(
            close_surface_water_mm(settings, true, -200, 800.0, 0.0, 50.0),
            0.0
        );
        let land = close_surface_water_mm(settings, true, 1_500, 800.0, 100.0, 50.0);
        let runoff = 800.0 * land_runoff_coefficient(settings, 1_500, 800.0, true);
        assert!((land - (50.0 + 800.0 - 100.0 - runoff)).abs() < 1e-9);
        assert!(land >= 0.0);
    }

    #[test]
    fn nearest_ocean_tree_matches_brute_force_distance() {
        let grid = Grid::new(64, 32, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![1_000; grid.sample_count()];
        for (cell, elevation) in elevations.iter_mut().enumerate() {
            if cell.is_multiple_of(7) {
                *elevation = -500;
            }
        }
        let ocean_vectors = elevations
            .iter()
            .enumerate()
            .filter(|(_, elevation)| **elevation <= 0)
            .map(|(cell, _)| cell_geometry(grid, cell).1)
            .collect::<Vec<_>>();
        assert!(ocean_vectors.len() > 48);
        let tree = KdNode::build(&ocean_vectors);
        for (cell, elevation) in elevations.iter().enumerate() {
            if *elevation <= 0 {
                continue;
            }
            let vector = cell_geometry(grid, cell).1;
            let brute = ocean_vectors
                .iter()
                .map(|ocean| ocean.dot(vector).clamp(-1.0, 1.0).acos())
                .fold(f64::INFINITY, f64::min);
            let accelerated = nearest_ocean_distance(&ocean_vectors, tree.as_deref(), vector);
            assert!(
                (brute - accelerated).abs() < 1e-12,
                "cell {cell}: brute={brute} accelerated={accelerated}"
            );
        }
    }

    #[test]
    fn parallel_maritime_geometry_matches_serial_cells() {
        let grid = Grid::new(64, 32, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![1_000; grid.sample_count()];
        for (cell, elevation) in elevations.iter_mut().enumerate() {
            if cell.is_multiple_of(7) {
                *elevation = -500;
            }
        }
        let physical = field(grid, elevations, 0);
        let settings = ClimateSettings::default_for(grid);
        let parallel = build_geometry(&physical, settings, &mut NoopProgress).unwrap();
        let ocean_vectors = (0..grid.sample_count())
            .filter(|cell| physical.elevations_mm[*cell] <= physical.sea_level_mm)
            .map(|cell| cell_geometry(grid, cell).1)
            .collect::<Vec<_>>();
        let ocean_tree = KdNode::build(&ocean_vectors);
        let maritime_scale_metres = f64::from(settings.maritime_scale_km) * 1_000.0;
        for cell in 0..grid.sample_count() {
            let serial = maritime_geometry_cell(
                &physical,
                &ocean_vectors,
                ocean_tree.as_deref(),
                maritime_scale_metres,
                cell,
            )
            .unwrap();
            assert_eq!(
                parallel[cell].latitude.to_bits(),
                serial.latitude.to_bits(),
                "cell {cell} latitude"
            );
            assert_eq!(
                parallel[cell].maritime_factor.to_bits(),
                serial.maritime_factor.to_bits(),
                "cell {cell} maritime_factor"
            );
        }
    }

    #[test]
    fn closer_orbit_warms_the_annual_field() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let physical = field(grid, vec![0; grid.sample_count()], 1);
        let earth = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let mut close = ClimateSettings::default_for(grid);
        close.planetary =
            PlanetaryConfiguration::from_preset(crate::planetary::PlanetaryPreset::CloseOrbit);
        let close_climate = derive_current_climate(
            &physical,
            close,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        assert!(
            close_climate.metrics.mean_temperature_centi_c > earth.metrics.mean_temperature_centi_c
        );
    }

    #[test]
    fn higher_luminosity_warms_the_annual_field() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let physical = field(grid, vec![0; grid.sample_count()], 1);
        let earth = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let mut bright = ClimateSettings::default_for(grid);
        bright.planetary.star_luminosity_ppm = 1_500_000;
        bright.planetary.preset = crate::planetary::PlanetaryPreset::Custom;
        let bright_climate = derive_current_climate(
            &physical,
            bright,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        assert!(
            bright_climate.metrics.mean_temperature_centi_c
                > earth.metrics.mean_temperature_centi_c
        );
    }

    #[test]
    fn axial_tilt_creates_solstice_contrast() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[grid.index(3, 0)] = -2_000;
        let physical = field(grid, elevations, 0);
        let mut untilted = ClimateSettings::default_for(grid);
        untilted.planetary.axial_tilt_milli_deg = 0;
        untilted.planetary.eccentricity_ppm = 0;
        untilted.planetary.preset = crate::planetary::PlanetaryPreset::Custom;
        let none = derive_current_climate(
            &physical,
            untilted,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        assert_eq!(
            none.temperature_nh_summer_centi_c,
            none.temperature_nh_winter_centi_c
        );
        assert_eq!(none.metrics.mean_seasonal_range_centi_c, 0);

        let mut tilted = ClimateSettings::default_for(grid);
        tilted.planetary =
            PlanetaryConfiguration::from_preset(crate::planetary::PlanetaryPreset::HighTilt);
        let seasons = derive_current_climate(
            &physical,
            tilted,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let north = grid.index(5, 4);
        let south = grid.index(2, 4);
        assert!(
            seasons.temperature_nh_summer_centi_c[north]
                > seasons.temperature_nh_winter_centi_c[north],
            "north summer {} winter {}",
            seasons.temperature_nh_summer_centi_c[north],
            seasons.temperature_nh_winter_centi_c[north]
        );
        assert!(
            seasons.temperature_nh_summer_centi_c[south]
                < seasons.temperature_nh_winter_centi_c[south],
            "south summer {} winter {}",
            seasons.temperature_nh_summer_centi_c[south],
            seasons.temperature_nh_winter_centi_c[south]
        );
        assert!(seasons.metrics.mean_seasonal_range_centi_c > 0);
    }

    #[test]
    fn eccentricity_does_not_invent_solstice_contrast_without_tilt() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[0] = -2_000;
        let physical = field(grid, elevations, 0);
        let mut settings = ClimateSettings::default_for(grid);
        settings.planetary.axial_tilt_milli_deg = 0;
        settings.planetary.eccentricity_ppm = 400_000;
        settings.planetary.preset = crate::planetary::PlanetaryPreset::Custom;
        let climate = derive_current_climate(
            &physical,
            settings,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        assert_eq!(
            climate.temperature_nh_summer_centi_c,
            climate.temperature_nh_winter_centi_c
        );
        assert_eq!(climate.metrics.mean_seasonal_range_centi_c, 0);
    }

    #[test]
    fn far_orbit_marks_permanent_freeze_on_land() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[grid.index(3, 0)] = -2_000;
        let physical = field(grid, elevations, 0);
        let mut settings = ClimateSettings::default_for(grid);
        settings.planetary.semi_major_axis_milli_au = 12_000_000;
        settings.planetary.preset = crate::planetary::PlanetaryPreset::Custom;
        let climate = derive_current_climate(
            &physical,
            settings,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        assert!(climate.metrics.permanently_frozen_land_ppm > 0);
        assert!(climate.metrics.minimum_seasonal_temperature_centi_c < 0);
    }

    #[test]
    fn earth_like_winds_have_tropical_easterlies_and_midlatitude_westerlies() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[0] = -2_000;
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let tropics = grid.index(4, 4);
        let midlatitude = grid.index(5, 4);
        assert!(climate.wind_east_milli[tropics] < 0);
        assert!(climate.wind_east_milli[midlatitude] > 0);
        assert_eq!(climate.wind_band[tropics], WIND_BAND_HADLEY);
        assert_eq!(climate.wind_band[midlatitude], WIND_BAND_FERREL);
        assert!(climate.metrics.mean_wind_speed_milli > 0);
        assert!(climate.metrics.easterly_cell_ppm > 0);
        assert!(climate.metrics.easterly_cell_ppm < 1_000_000);
    }

    #[test]
    fn slow_rotation_expands_hadley_easterlies() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[0] = -2_000;
        let physical = field(grid, elevations, 0);
        let earth = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let mut slow = ClimateSettings::default_for(grid);
        slow.planetary =
            PlanetaryConfiguration::from_preset(crate::planetary::PlanetaryPreset::SlowRotating);
        let slow_climate = derive_current_climate(
            &physical,
            slow,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let midlatitude = grid.index(5, 4);
        assert!(earth.wind_east_milli[midlatitude] > 0);
        assert!(slow_climate.wind_east_milli[midlatitude] < 0);
        assert_eq!(slow_climate.wind_band[midlatitude], WIND_BAND_HADLEY);
    }

    #[test]
    fn northern_summer_shifts_itcz_north() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[0] = -2_000;
        let physical = field(grid, elevations, 0);
        let mut tilted = ClimateSettings::default_for(grid);
        tilted.planetary =
            PlanetaryConfiguration::from_preset(crate::planetary::PlanetaryPreset::HighTilt);
        let climate = derive_current_climate(
            &physical,
            tilted,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let summer_itcz = thermal_equator_latitude(grid, &climate.temperature_nh_summer_centi_c);
        let winter_itcz = thermal_equator_latitude(grid, &climate.temperature_nh_winter_centi_c);
        assert!(summer_itcz > winter_itcz);
        let tropics = grid.index(4, 4);
        assert_ne!(
            climate.wind_north_nh_summer_milli[tropics],
            climate.wind_north_nh_winter_milli[tropics]
        );
        assert!(climate
            .wind_band_nh_summer
            .iter()
            .zip(climate.wind_band_nh_winter.iter())
            .any(|(summer, winter)| summer != winter));
        assert_ne!(
            climate.wind_divergence_nh_summer_ppm[tropics],
            climate.wind_divergence_nh_winter_ppm[tropics]
        );
    }

    #[test]
    fn mountains_block_zonal_wind() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[0] = -2_000;
        let peak = grid.index(4, 8);
        elevations[peak] = 4_000_000;
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let neighbor = grid.index(4, 4);
        assert!(
            climate.wind_east_milli[peak].unsigned_abs()
                < climate.wind_east_milli[neighbor].unsigned_abs()
        );
    }

    #[test]
    fn coriolis_reverses_across_the_equator_and_vanishes_on_it() {
        let planetary = PlanetaryConfiguration::earth_like();
        assert_eq!(coriolis_parameter(0.0, planetary), 0.0);
        let north = coriolis_parameter(0.4, planetary);
        let south = coriolis_parameter(-0.4, planetary);
        assert!(north > 0.0);
        assert!(south < 0.0);
        assert!((north + south).abs() < 1e-12);
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[0] = -2_000;
        let climate = derive_current_climate(
            &field(grid, elevations, 0),
            ClimateSettings::default_for(grid),
            831_429,
            0,
            &mut NoopProgress,
        )
        .unwrap();
        assert!(climate.wind_east_milli[grid.index(4, 4)] < 0);
        let southern_hadley = (0..grid.sample_count()).filter(|&cell| {
            let (row, _) = grid.row_col(cell);
            row < grid.height / 2 && climate.wind_band[cell] == WIND_BAND_HADLEY
        });
        assert!(southern_hadley
            .clone()
            .any(|cell| climate.wind_east_milli[cell] < 0));
        assert!(climate
            .wind_east_milli
            .iter()
            .chain(climate.wind_north_milli.iter())
            .all(|value| (-MAX_WIND_MILLI..=MAX_WIND_MILLI).contains(value)));
    }

    #[test]
    fn northern_thermal_low_turns_counterclockwise() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![200_000; grid.sample_count()];
        for col in 6..10 {
            elevations[grid.index(5, col)] = -200_000;
        }
        let climate = derive_current_climate(
            &field(grid, elevations, 0),
            ClimateSettings::default_for(grid),
            831_429,
            0,
            &mut NoopProgress,
        )
        .unwrap();
        let east = grid.index(5, 10);
        let west = grid.index(5, 5);
        assert!(
            climate.wind_north_milli[east] > climate.wind_north_milli[west],
            "east {} west {}",
            climate.wind_north_milli[east],
            climate.wind_north_milli[west]
        );
    }

    #[test]
    fn extreme_pressure_amplitude_clamps_wind() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[0] = -2_000;
        let mut settings = ClimateSettings::default_for(grid);
        settings.pressure_cell_amplitude = 50_000;
        settings.drag_ocean_micro = 1;
        settings.drag_land_micro = 1;
        let climate = derive_current_climate(
            &field(grid, elevations, 0),
            settings,
            831_429,
            0,
            &mut NoopProgress,
        )
        .unwrap();
        assert!(climate
            .wind_east_milli
            .iter()
            .chain(climate.wind_north_milli.iter())
            .all(|value| (-MAX_WIND_MILLI..=MAX_WIND_MILLI).contains(value)));
        assert_eq!(clamp_wind(1_000_000.0), MAX_WIND_MILLI);
        assert_eq!(clamp_wind(-1_000_000.0), -MAX_WIND_MILLI);
    }

    #[test]
    fn heat_advection_moves_warmth_downwind() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![200_000; grid.sample_count()];
        for col in 10..14 {
            elevations[grid.index(5, col)] = -200_000;
        }
        let physical = field(grid, elevations, 0);
        let mut diffusion = ClimateSettings::default_for(grid);
        diffusion.heat_advection_kj_m2_k = 0;
        diffusion.temperature_wind_coupling_passes = 0;
        let stage1 = derive_current_climate(
            &physical,
            diffusion,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let mut advecting = ClimateSettings::default_for(grid);
        advecting.heat_advection_kj_m2_k = 800;
        advecting.temperature_wind_coupling_passes = 2;
        let coupled = derive_current_climate(
            &physical,
            advecting,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let sample = grid.index(5, 8);
        let west = grid.index(5, 9);
        let east = grid.index(5, 14);
        let u = coupled.wind_east_milli[sample];
        let v = coupled.wind_north_milli[sample];
        assert!(
            u.unsigned_abs() > v.unsigned_abs(),
            "zonal wind {} meridional {}",
            u,
            v
        );
        let (downwind, upwind) = if u < 0 { (west, east) } else { (east, west) };
        let downwind_delta =
            coupled.temperature_centi_c[downwind] - stage1.temperature_centi_c[downwind];
        let upwind_delta = coupled.temperature_centi_c[upwind] - stage1.temperature_centi_c[upwind];
        assert!(
            downwind_delta > 0,
            "downwind did not warm {downwind_delta} wind {u}"
        );
        assert!(
            downwind_delta > upwind_delta,
            "downwind {downwind_delta} upwind {upwind_delta} wind {u}"
        );
        let mut default_advecting = ClimateSettings::default_for(grid);
        default_advecting.temperature_wind_coupling_passes = 2;
        let mut default_diffusion = default_advecting;
        default_diffusion.heat_advection_kj_m2_k = 0;
        let default_stage1 = derive_current_climate(
            &physical,
            default_diffusion,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let default_coupled = derive_current_climate(
            &physical,
            default_advecting,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let default_u = default_coupled.wind_east_milli[sample];
        let (default_downwind, default_upwind) = if default_u < 0 {
            (west, east)
        } else {
            (east, west)
        };
        let default_downwind_delta = default_coupled.temperature_centi_c[default_downwind]
            - default_stage1.temperature_centi_c[default_downwind];
        let default_upwind_delta = default_coupled.temperature_centi_c[default_upwind]
            - default_stage1.temperature_centi_c[default_upwind];
        assert!(
            default_downwind_delta > 0,
            "default advection downwind did not warm {default_downwind_delta} wind {default_u}"
        );
        assert!(
            default_downwind_delta > default_upwind_delta,
            "default advection downwind {default_downwind_delta} upwind {default_upwind_delta} wind {default_u}"
        );
    }

    #[test]
    fn thermal_pressure_anomaly_flips_meridional_wind_independent_of_the_band() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let elevations = vec![200_000; grid.sample_count()];
        let physical = field(grid, elevations, 0);
        let mut settings = ClimateSettings::default_for(grid);
        settings.pressure_cell_amplitude = 100;
        let mut hot = vec![1_500; grid.sample_count()];
        let mut cold = vec![1_500; grid.sample_count()];
        let patch = grid.index(5, 8);
        hot[patch] = 2_800;
        cold[patch] = 200;
        let omega = omega_ratio(settings.planetary);
        let hadley = hadley_edge_radians(omega);
        let ferrel = ferrel_edge_radians(hadley);
        let (_hot_east, hot_north) =
            wind_components(&physical, &hot, settings, 0.0, hadley, ferrel, 0, false);
        let (_cold_east, cold_north) =
            wind_components(&physical, &cold, settings, 0.0, hadley, ferrel, 0, false);
        let east = grid.index(5, 9);
        assert!(
            hot_north[east] > cold_north[east],
            "hot {} cold {}",
            hot_north[east],
            cold_north[east]
        );
        assert_ne!(hot_north[east].signum(), cold_north[east].signum());
    }

    #[test]
    fn flat_world_winds_are_locally_smooth_inside_a_band() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[0] = -2_000;
        let climate = derive_current_climate(
            &field(grid, elevations, 0),
            ClimateSettings::default_for(grid),
            831_429,
            0,
            &mut NoopProgress,
        )
        .unwrap();
        let mut jump = 0.0;
        let mut speed = 0.0;
        let mut count = 0.0;
        for cell in 0..grid.sample_count() {
            let (row, col) = grid.row_col(cell);
            let east = grid.index(row, wrapped_col(grid, col, 1));
            if climate.wind_band[cell] != climate.wind_band[east] {
                continue;
            }
            jump +=
                f64::from((climate.wind_east_milli[cell] - climate.wind_east_milli[east]).abs());
            speed += f64::from(climate.wind_east_milli[cell].unsigned_abs());
            count += 1.0;
        }
        assert!(count > 0.0);
        assert!(jump / count < speed / count);
    }

    #[test]
    fn uniform_temperature_offset_keeps_winds_until_sea_level_changes() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[0] = -2_000;
        let physical = field(grid, elevations, 0);
        let settings = ClimateSettings::default_for(grid);
        let climate = derive_current_climate(
            &physical,
            settings,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let shifted = climate
            .with_global_temperature_offset(-500)
            .with_winds_and_moisture_for_field(
                &physical,
                settings,
                physical.seed,
                physical.retry_index,
                &mut NoopProgress,
            )
            .unwrap();
        assert_eq!(
            shifted.metrics.mean_temperature_centi_c,
            climate.metrics.mean_temperature_centi_c - 500
        );
        assert_eq!(
            shifted.temperature_centi_c,
            climate
                .temperature_centi_c
                .iter()
                .map(|value| value.saturating_add(-500))
                .collect::<Vec<_>>()
        );
        assert_eq!(shifted.humidity_ppm.len(), climate.humidity_ppm.len());
        let mut flooded = physical.clone();
        flooded.sea_level_mm = 2_000;
        let epoch = climate
            .with_winds_and_moisture_for_field(
                &flooded,
                settings,
                physical.seed,
                physical.retry_index,
                &mut NoopProgress,
            )
            .unwrap();
        assert_ne!(epoch.wind_east_milli, climate.wind_east_milli);
        assert_ne!(
            epoch.precipitation_mm_per_year,
            climate.precipitation_mm_per_year
        );
        assert_ne!(epoch.current_east_milli, climate.current_east_milli);
        assert_ne!(epoch.current_north_milli, climate.current_north_milli);
    }

    #[test]
    fn restamp_uses_caller_moisture_settings() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[0] = -2_000;
        let physical = field(grid, elevations, 0);
        let settings = ClimateSettings::default_for(grid);
        let climate = derive_current_climate(
            &physical,
            settings,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let mut wet = settings;
        wet.ocean_moisture_mm_per_year = settings.ocean_moisture_mm_per_year.saturating_mul(2);
        let default_stamp = climate
            .with_winds_and_moisture_for_field(
                &physical,
                settings,
                physical.seed,
                physical.retry_index,
                &mut NoopProgress,
            )
            .unwrap();
        let wet_stamp = climate
            .with_winds_and_moisture_for_field(
                &physical,
                wet,
                physical.seed,
                physical.retry_index,
                &mut NoopProgress,
            )
            .unwrap();
        assert_ne!(
            wet_stamp.precipitation_mm_per_year,
            default_stamp.precipitation_mm_per_year
        );
    }

    #[test]
    fn ocean_temperature_gradient_changes_currents() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let elevations = vec![-2_000; grid.sample_count()];
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let flat = vec![1_500; grid.sample_count()];
        let mut front = flat.clone();
        for cell in 0..grid.sample_count() {
            let (row, _) = grid.row_col(cell);
            front[cell] = if row + 1 > grid.height / 2 {
                4_000
            } else {
                -2_000
            };
        }
        let (front_east, _) = derive_currents(
            &physical,
            climate.planetary,
            &front,
            &climate.wind_east_milli,
            &climate.wind_north_milli,
        );
        let (flat_east, _) = derive_currents(
            &physical,
            climate.planetary,
            &flat,
            &climate.wind_east_milli,
            &climate.wind_north_milli,
        );
        let mean_abs_delta = front_east
            .iter()
            .zip(&flat_east)
            .map(|(a, b)| i64::from(*a - *b).unsigned_abs())
            .sum::<u64>()
            / u64::from(grid.sample_count() as u32);
        assert!(
            mean_abs_delta > 40,
            "geostrophy should move currents, mean abs delta {mean_abs_delta}"
        );
    }

    #[test]
    fn circulation_signs_hold_on_a_finer_grid() {
        let grid = Grid::new(32, 16, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[0] = -2_000;
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let tropics = grid.index(8, 8);
        let midlatitude = grid.index(11, 8);
        assert!(climate.wind_east_milli[tropics] < 0);
        assert!(climate.wind_east_milli[midlatitude] > 0);
        let westerlies = (0..grid.width)
            .filter(|col| climate.wind_east_milli[grid.index(11, *col)] > 0)
            .count();
        assert!(
            westerlies >= (grid.width as usize * 3) / 4,
            "ferrel westerlies should hold around the belt, saw {westerlies}"
        );
    }

    #[test]
    fn wind_meanders_depend_on_seed() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[0] = -2_000;
        let first = field(grid, elevations.clone(), 0);
        let mut second = first.clone();
        second.seed = 42;
        let climate_first = derive_current_climate(
            &first,
            ClimateSettings::default_for(grid),
            first.seed,
            first.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let climate_second = derive_current_climate(
            &second,
            ClimateSettings::default_for(grid),
            second.seed,
            second.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        assert_ne!(
            climate_first.wind_east_milli,
            climate_second.wind_east_milli
        );
        assert_ne!(
            climate_first.wind_north_milli,
            climate_second.wind_north_milli
        );
    }

    #[test]
    fn ocean_currents_are_zero_on_land() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-2_000; grid.sample_count()];
        for row in 0..grid.height {
            elevations[grid.index(row, 5)] = 2_000;
        }
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        for cell in 0..grid.sample_count() {
            if physical.elevations_mm[cell] > physical.sea_level_mm {
                assert_eq!(climate.current_east_milli[cell], 0);
                assert_eq!(climate.current_north_milli[cell], 0);
            }
        }
        assert!(climate.metrics.mean_current_speed_milli > 0);
    }

    #[test]
    fn inland_sinks_do_not_get_ocean_currents() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![2_000; grid.sample_count()];
        for row in 0..grid.height {
            elevations[grid.index(row, 0)] = -2_000;
            elevations[grid.index(row, 1)] = -2_000;
        }
        elevations[grid.index(3, 10)] = -2_000;
        elevations[grid.index(3, 11)] = -2_000;
        elevations[grid.index(4, 10)] = -2_000;
        elevations[grid.index(4, 11)] = -2_000;
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        for cell in [
            grid.index(3, 10),
            grid.index(3, 11),
            grid.index(4, 10),
            grid.index(4, 11),
        ] {
            assert_eq!(climate.current_east_milli[cell], 0);
            assert_eq!(climate.current_north_milli[cell], 0);
        }
        assert!(climate.metrics.mean_current_speed_milli > 0);
    }

    #[test]
    fn enclosed_seas_get_wind_driven_currents() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![2_000; grid.sample_count()];
        for row in 0..grid.height {
            elevations[grid.index(row, 0)] = -2_000;
            elevations[grid.index(row, 1)] = -2_000;
        }
        for row in 2..6 {
            for col in 8..12 {
                elevations[grid.index(row, col)] = -2_000;
            }
        }
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let mut enclosed_speed = 0.0;
        for row in 2..6 {
            for col in 8..12 {
                let cell = grid.index(row, col);
                enclosed_speed += f64::from(climate.current_east_milli[cell])
                    .hypot(f64::from(climate.current_north_milli[cell]));
            }
        }
        assert!(
            enclosed_speed > 0.0,
            "enclosed sea should circulate: {enclosed_speed}"
        );
    }

    #[test]
    fn western_boundary_current_is_stronger_than_the_basin_interior() {
        let grid = Grid::new(32, 16, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-2_000; grid.sample_count()];
        for row in 0..grid.height {
            elevations[grid.index(row, 6)] = 2_000;
        }
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let mut west_speed = 0.0;
        let mut interior_speed = 0.0;
        let mut samples = 0.0;
        for row in 9..12 {
            let west = grid.index(row, 7);
            let interior = grid.index(row, 18);
            west_speed += f64::from(climate.current_east_milli[west])
                .hypot(f64::from(climate.current_north_milli[west]));
            interior_speed += f64::from(climate.current_east_milli[interior])
                .hypot(f64::from(climate.current_north_milli[interior]));
            samples += 1.0;
        }
        assert!(
            west_speed / samples > interior_speed / samples * 1.15,
            "western {west_speed} vs interior {interior_speed}"
        );
        let mut nh_north = 0i64;
        let mut sh_north = 0i64;
        let mut eq_east = 0i64;
        for col in 7..10 {
            nh_north += i64::from(climate.current_north_milli[grid.index(10, col)]);
            sh_north += i64::from(climate.current_north_milli[grid.index(5, col)]);
            eq_east += i64::from(climate.current_east_milli[grid.index(8, col)]);
        }
        assert!(
            nh_north > 0,
            "NH western boundary should run north: {nh_north}"
        );
        assert!(
            sh_north < 0,
            "SH western boundary should run south: {sh_north}"
        );
        assert!(
            eq_east < 0,
            "equator should have a westward current: {eq_east}"
        );
    }

    #[test]
    fn western_boundary_coast_is_warmer_than_the_opposite_coast() {
        let grid = Grid::new(32, 16, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-2_000; grid.sample_count()];
        for row in 0..grid.height {
            for col in 6..12 {
                elevations[grid.index(row, col)] = 2_000;
            }
        }
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let mut west_coast = 0i64;
        let mut east_coast = 0i64;
        let mut west_ocean = 0i64;
        let mut east_ocean = 0i64;
        for row in 8..13 {
            let west = grid.index(row, 6);
            let east = grid.index(row, 11);
            assert_eq!(
                climate.maritime_factor_ppm[west],
                climate.maritime_factor_ppm[east]
            );
            west_coast += i64::from(climate.temperature_centi_c[west]);
            east_coast += i64::from(climate.temperature_centi_c[east]);
            west_ocean += i64::from(climate.temperature_centi_c[grid.index(row, 5)]);
            east_ocean += i64::from(climate.temperature_centi_c[grid.index(row, 12)]);
        }
        assert!(
            east_coast > west_coast,
            "western-boundary coast {east_coast} should exceed opposite coast {west_coast}"
        );
        assert!(
            east_ocean > west_ocean,
            "western-boundary ocean {east_ocean} should exceed opposite ocean {west_ocean}"
        );
        let mut decoupled = ClimateSettings::default_for(grid);
        decoupled.ocean_heat_coupling_milli_wm2_per_c = 0;
        let off = derive_current_climate(
            &physical,
            decoupled,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let mut off_west = 0i64;
        let mut off_east = 0i64;
        for row in 8..13 {
            off_west += i64::from(off.temperature_centi_c[grid.index(row, 6)]);
            off_east += i64::from(off.temperature_centi_c[grid.index(row, 11)]);
        }
        let coupled_contrast = east_coast - west_coast;
        let off_contrast = off_east - off_west;
        assert!(
            coupled_contrast > off_contrast,
            "current heat should warm the western-boundary coast: coupled {coupled_contrast} vs off {off_contrast}"
        );
    }

    #[test]
    fn slower_rotation_changes_surface_currents() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-2_000; grid.sample_count()];
        for row in 0..grid.height {
            elevations[grid.index(row, 4)] = 2_000;
        }
        let physical = field(grid, elevations, 0);
        let earth = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let mut settings = ClimateSettings::default_for(grid);
        settings.planetary.rotation_period_seconds =
            EARTH_ROTATION_PERIOD_SECONDS.saturating_mul(4);
        settings.planetary.preset = crate::planetary::PlanetaryPreset::Custom;
        let slow = derive_current_climate(
            &physical,
            settings,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        assert_ne!(
            slow.metrics.mean_current_speed_milli,
            earth.metrics.mean_current_speed_milli
        );
        assert!(slow.metrics.mean_current_speed_milli > 0);
    }

    #[test]
    fn biome_classes_are_explainable_from_climate_conditions() {
        assert_eq!(
            classify_biome_cell(false, -2_000, 0, 2_000, 2_200, 1_800, 2_000, 500_000, 0),
            BIOME_OCEAN
        );
        assert_eq!(
            classify_biome_cell(true, 200, 0, -1_200, -800, -1_600, 200, 200_000, 200_000),
            BIOME_ICE
        );
        assert_eq!(
            classify_biome_cell(true, 400, 0, -200, 400, -800, 400, 400_000, 300_000),
            BIOME_TUNDRA
        );
        assert_eq!(
            classify_biome_cell(true, 2_400_000, 0, 200, 600, -200, 600, 400_000, 250_000),
            BIOME_ALPINE
        );
        assert_eq!(
            classify_biome_cell(true, 200, 0, 2_200, 2_600, 1_800, 80, 100_000, 900_000),
            BIOME_DESERT
        );
        assert_eq!(
            classify_biome_cell(true, 200, 0, 2_200, 2_600, 1_800, 80, 850_000, 900_000),
            BIOME_SHRUBLAND
        );
        assert_eq!(
            classify_biome_cell(true, 200, 0, 1_800, 2_200, 1_400, 350, 400_000, 600_000),
            BIOME_SHRUBLAND
        );
        assert_eq!(
            classify_biome_cell(true, 200, 0, 200, 1_200, -1_400, 600, 400_000, 350_000),
            BIOME_COLD_GRASSLAND
        );
        assert_eq!(
            classify_biome_cell(true, 200, 0, 1_400, 1_800, 1_000, 600, 400_000, 350_000),
            BIOME_TEMPERATE_GRASSLAND
        );
        assert_eq!(
            classify_biome_cell(true, 200, 0, 200, 1_200, -1_400, 300, 400_000, 600_000),
            BIOME_COLD_GRASSLAND
        );
        assert_eq!(
            classify_biome_cell(true, 200, 0, 1_200, 1_800, 600, 1_200, 100_000, 80_000),
            BIOME_TEMPERATE_GRASSLAND
        );
        assert_eq!(
            classify_biome_cell(true, 200, 0, 1_200, 1_800, 600, 1_200, 600_000, 80_000),
            BIOME_TEMPERATE_FOREST
        );
        assert_eq!(
            classify_biome_cell(true, 200, 0, 2_400, 2_600, 2_200, 2_400, 700_000, 40_000),
            BIOME_TROPICAL_FOREST
        );
        assert_eq!(
            classify_biome_cell(true, 80_000, 0, 2_200, 2_400, 2_000, 500, 600_000, 100_000),
            BIOME_TROPICAL_FOREST
        );
        assert_eq!(biome_name(BIOME_DESERT), "desert");
        assert_eq!(biome_name(BIOME_ICE), "permanent ice");
        assert!(
            explain_biome(BIOME_ALPINE, 2_400_000, 0, 600, -200, 600, 400_000, 250_000)
                .contains("2400 m")
        );
        assert_eq!(biome_legend().len(), 10);
    }

    #[test]
    fn derived_biomes_cover_land_and_leave_ocean_unclassified() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[0] = -2_000;
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        assert_eq!(climate.derivation_version, CLIMATE_DERIVATION_VERSION);
        assert_eq!(climate.biome_class.len(), grid.sample_count());
        assert_eq!(climate.biome_class[0], BIOME_OCEAN);
        assert!(climate
            .biome_class
            .iter()
            .skip(1)
            .all(|class| *class != BIOME_OCEAN));
        assert_ne!(climate.metrics.dominant_land_biome, BIOME_OCEAN);
    }

    #[test]
    fn equatorial_coastal_lowlands_can_be_tropical_forest() {
        let grid = Grid::new(64, 32, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-4_000_000; grid.sample_count()];
        let equator = grid.height / 2;
        for row in equator.saturating_sub(2)..(equator + 3).min(grid.height) {
            for col in 16..40 {
                elevations[grid.index(row, col)] = 80_000;
            }
        }
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let trop = count_biome(&climate.biome_class, BIOME_TROPICAL_FOREST);
        assert!(
            trop > 0,
            "equatorial lowlands produced no tropical forest (max land T={}, max land P={})",
            climate
                .temperature_centi_c
                .iter()
                .enumerate()
                .filter(|(cell, _)| physical.elevations_mm[*cell] > physical.sea_level_mm)
                .map(|(_, temperature)| *temperature)
                .max()
                .unwrap_or(0),
            climate
                .precipitation_mm_per_year
                .iter()
                .enumerate()
                .filter(|(cell, _)| physical.elevations_mm[*cell] > physical.sea_level_mm)
                .map(|(_, precipitation)| *precipitation)
                .max()
                .unwrap_or(0)
        );
    }

    #[test]
    fn earth_like_world_covers_tropical_temperate_and_cold_grassland() {
        let world = crate::generate_world(
            crate::GenerationSettings {
                width: 64,
                height: 32,
                radius_metres: DEFAULT_RADIUS_METRES,
                target_land_fraction_ppm: 300_000,
            },
            831_429,
            0,
            &mut NoopProgress,
        )
        .unwrap();
        let trop = count_biome(&world.climate.biome_class, BIOME_TROPICAL_FOREST);
        let temperate = count_biome(&world.climate.biome_class, BIOME_TEMPERATE_FOREST);
        let cold_grass = count_biome(&world.climate.biome_class, BIOME_COLD_GRASSLAND);
        let tundra = count_biome(&world.climate.biome_class, BIOME_TUNDRA);
        assert!(
            trop > 0 && temperate > 0 && (cold_grass > 0 || tundra > 0),
            "tropical forest={trop} temperate forest={temperate} cold grassland={cold_grass} tundra={tundra}"
        );
    }

    #[test]
    fn midlatitude_coasts_can_be_temperate_forest() {
        let grid = Grid::new(64, 32, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-4_000_000; grid.sample_count()];
        for row in 17..21 {
            for col in 16..22 {
                elevations[grid.index(row, col)] = 80_000;
            }
        }
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let temperate = count_biome(&climate.biome_class, BIOME_TEMPERATE_FOREST);
        assert!(
            temperate > 0,
            "midlatitude coasts produced no temperate forest"
        );
    }

    #[test]
    fn cold_continental_interiors_can_be_cold_grassland() {
        let grid = Grid::new(64, 32, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-4_000_000; grid.sample_count()];
        for row in 21..25 {
            for col in 8..12 {
                elevations[grid.index(row, col)] = 200_000;
            }
        }
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let cold_grass = count_biome(&climate.biome_class, BIOME_COLD_GRASSLAND);
        let tundra = count_biome(&climate.biome_class, BIOME_TUNDRA);
        assert!(
            cold_grass > 0 || tundra > 0,
            "cold interior produced no cold grassland or tundra (ice={} tundra={} desert={} shrub={} temp_grass={} forest={})",
            count_biome(&climate.biome_class, BIOME_ICE),
            tundra,
            count_biome(&climate.biome_class, BIOME_DESERT),
            count_biome(&climate.biome_class, BIOME_SHRUBLAND),
            count_biome(&climate.biome_class, BIOME_TEMPERATE_GRASSLAND),
            count_biome(&climate.biome_class, BIOME_TEMPERATE_FOREST),
        );
    }

    #[test]
    fn colder_epoch_increases_frozen_biome_cover() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![500; grid.sample_count()];
        elevations[0] = -2_000;
        let physical = field(grid, elevations, 0);
        let settings = ClimateSettings::default_for(grid);
        let present = derive_current_climate(
            &physical,
            settings,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let cold = present
            .with_global_temperature_offset(-2_500)
            .with_winds_and_moisture_for_field(
                &physical,
                settings,
                physical.seed,
                physical.retry_index,
                &mut NoopProgress,
            )
            .unwrap();
        let present_ice = present
            .biome_class
            .iter()
            .filter(|class| **class == BIOME_ICE)
            .count();
        let cold_ice = cold
            .biome_class
            .iter()
            .filter(|class| **class == BIOME_ICE)
            .count();
        assert!(
            cold_ice > present_ice,
            "cold {cold_ice} present {present_ice}"
        );
    }

    #[test]
    fn storm_genesis_needs_warm_moist_rotating_ocean() {
        let hadley = 30.0_f64.to_radians();
        let (suitability, _) = classify_storm_cell(
            false, 2_800, 800_000, 0.3, 0.0, hadley, 800, 1.0, 1.0, 1.0, 1.0, 0, 0, 1_200, 1_800,
        );
        assert_eq!(suitability, 0);
        let (equator, _) = classify_storm_cell(
            true, 2_800, 800_000, 0.0, 0.0, hadley, 800, 1.0, 1.0, 1.0, 1.0, 0, 0, 1_200, 1_800,
        );
        assert_eq!(equator, 0);
        let (cold, _) = classify_storm_cell(
            true, -200, 800_000, 0.3, 0.0, hadley, 800, 1.0, 1.0, 1.0, 1.0, 0, 0, 1_200, 1_800,
        );
        assert_eq!(cold, 0);
        let (core, intensity) = classify_storm_cell(
            true, 2_800, 800_000, 0.3, 0.0, hadley, 800, 1.0, 1.0, 1.0, 1.0, 0, 0, 1_200, 1_800,
        );
        assert!(core > 500_000, "core {core}");
        assert!(intensity > 0);
        let (sheared, _) = classify_storm_cell(
            true, 2_800, 800_000, 0.3, 0.0, hadley, 12_000, 1.0, 1.0, 1.0, 1.0, 0, 0, 1_200, 1_800,
        );
        assert_eq!(sheared, 0);
        let (coastal, _) = classify_storm_cell(
            true, 2_800, 800_000, 0.3, 0.0, hadley, 800, 1.0, 0.35, 1.0, 1.0, 0, 0, 1_200, 1_800,
        );
        assert!(coastal < core);
        let (diverging, _) = classify_storm_cell(
            true, 2_800, 800_000, 0.3, 0.0, hadley, 800, 1.0, 1.0, 1.0, 1.0, 350_000, 0, 1_200,
            1_800,
        );
        let (converging, _) = classify_storm_cell(
            true, 2_800, 800_000, 0.3, 0.0, hadley, 800, 1.0, 1.0, 1.0, 1.0, -300_000, 0, 1_200,
            1_800,
        );
        assert!(diverging < core, "diverging {diverging} core {core}");
        assert!(converging > core, "converging {converging} core {core}");
        let (front, _) = classify_storm_cell(
            true, 2_800, 800_000, 0.3, 0.0, hadley, 800, 1.0, 1.0, 1.0, 1.0, 0, 900, 1_200, 1_800,
        );
        assert_eq!(front, 0);
        let shifted = storm_latitude_factor(18.0_f64.to_radians(), 10.0_f64.to_radians(), hadley);
        assert!(shifted < storm_latitude_factor(18.0_f64.to_radians(), 0.0, hadley));
        assert_eq!(
            storm_coriolis_factor(PlanetaryConfiguration {
                rotation_period_seconds: EARTH_ROTATION_PERIOD_SECONDS * 10,
                ..PlanetaryConfiguration::earth_like()
            }),
            0.1
        );
        assert!(
            explain_storm(true, 2_800, 800_000, 18_000, 800, core, 200_000, intensity)
                .contains("Climatology")
        );
    }

    #[test]
    fn derived_storms_form_on_tropical_ocean_and_track_toward_land() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![800; grid.sample_count()];
        for col in 0..grid.width {
            elevations[grid.index(3, col)] = -2_000;
            elevations[grid.index(4, col)] = -2_000;
        }
        elevations[grid.index(3, 0)] = 800;
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        assert_eq!(climate.storm_suitability_ppm.len(), grid.sample_count());
        assert_eq!(climate.storm_suitability_ppm[grid.index(3, 0)], 0);
        assert!(
            climate.storm_suitability_ppm.iter().any(|value| *value > 0),
            "suitability never left zero"
        );
        assert!(
            climate.storm_track_ppm.iter().any(|value| *value > 0),
            "tracks never left zero"
        );
        assert!(
            climate
                .storm_track_ppm
                .iter()
                .enumerate()
                .any(|(cell, value)| {
                    *value > 0 && physical.elevations_mm[cell] > physical.sea_level_mm
                }),
            "tracks never reached land"
        );
        assert!(climate.metrics.mean_ocean_storm_suitability_ppm > 0);
    }

    #[test]
    fn storm_corridors_follow_continental_heat_and_ocean_fetch() {
        let grid = Grid::new(32, 16, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![-2_000; grid.sample_count()];
        for row in 6..10 {
            for col in 10..18 {
                elevations[grid.index(row, col)] = 800;
            }
        }
        let physical = field(grid, elevations.clone(), 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let row = 9;
        let west_of_land = climate.storm_suitability_ppm[grid.index(row, 9)];
        let east_of_land = climate.storm_suitability_ppm[grid.index(row, 19)];
        assert!(
            east_of_land > west_of_land,
            "east {east_of_land} west {west_of_land} (row 9 south flank; row 6 north edge is outside coupled genesis)"
        );
        assert!(
            climate
                .storm_track_ppm
                .iter()
                .enumerate()
                .any(|(cell, value)| {
                    *value > 0 && physical.elevations_mm[cell] > physical.sea_level_mm
                }),
            "tracks never reached the continent"
        );
        let mut west_land = elevations.clone();
        for row in 6..10 {
            for col in 2..6 {
                west_land[grid.index(row, col)] = 800;
            }
        }
        let west_climate = derive_current_climate(
            &field(grid, west_land, 0),
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        assert_ne!(climate.storm_track_ppm, west_climate.storm_track_ppm);
        assert_ne!(
            climate.storm_suitability_ppm,
            west_climate.storm_suitability_ppm
        );
    }

    #[test]
    fn colder_epoch_reduces_tropical_storm_suitability() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![800; grid.sample_count()];
        for col in 0..grid.width {
            elevations[grid.index(3, col)] = -2_000;
            elevations[grid.index(4, col)] = -2_000;
        }
        let physical = field(grid, elevations, 0);
        let settings = ClimateSettings::default_for(grid);
        let present = derive_current_climate(
            &physical,
            settings,
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        let cold = present
            .with_global_temperature_offset(-2_500)
            .with_winds_and_moisture_for_field(
                &physical,
                settings,
                physical.seed,
                physical.retry_index,
                &mut NoopProgress,
            )
            .unwrap();
        assert!(
            cold.metrics.mean_ocean_storm_suitability_ppm
                < present.metrics.mean_ocean_storm_suitability_ppm,
            "cold {} present {}",
            cold.metrics.mean_ocean_storm_suitability_ppm,
            present.metrics.mean_ocean_storm_suitability_ppm
        );
        assert!(
            cold.metrics.mean_heat_wave_potential_ppm
                < present.metrics.mean_heat_wave_potential_ppm,
            "cold heat {} present heat {}",
            cold.metrics.mean_heat_wave_potential_ppm,
            present.metrics.mean_heat_wave_potential_ppm
        );
    }

    #[test]
    fn storm_and_extreme_metrics_remain_bounded() {
        let grid = Grid::new(16, 8, DEFAULT_RADIUS_METRES).unwrap();
        let mut elevations = vec![800; grid.sample_count()];
        for col in 0..grid.width {
            elevations[grid.index(3, col)] = -2_000;
            elevations[grid.index(4, col)] = -2_000;
        }
        let physical = field(grid, elevations, 0);
        let climate = derive_current_climate(
            &physical,
            ClimateSettings::default_for(grid),
            physical.seed,
            physical.retry_index,
            &mut NoopProgress,
        )
        .unwrap();
        for value in climate
            .storm_suitability_ppm
            .iter()
            .chain(climate.storm_track_ppm.iter())
            .chain(climate.storm_intensity_ppm.iter())
        {
            assert!(*value <= 1_000_000);
        }
        assert!(climate.metrics.mean_ocean_storm_suitability_ppm <= 1_000_000);
        assert!(climate.metrics.storm_prone_ocean_ppm <= 1_000_000);
        assert!(climate.metrics.mean_storm_intensity_ppm <= 1_000_000);
        assert!(climate.metrics.mean_land_storm_track_ppm <= 1_000_000);
        assert!(climate.metrics.mean_land_drought_potential_ppm <= 1_000_000);
        assert!(climate.metrics.mean_heat_wave_potential_ppm <= 1_000_000);
        assert!(climate.metrics.mean_extreme_rainfall_potential_ppm <= 1_000_000);
        assert!(climate.metrics.mean_land_drought_potential_ppm > 0);
        assert!(climate.metrics.mean_land_growing_season_ppm <= 1_000_000);
        assert!(climate.metrics.mean_land_dry_season_ppm <= 1_000_000);
        assert!(climate.metrics.mean_land_wet_season_ppm <= 1_000_000);
    }

    #[test]
    fn extreme_potentials_are_statistics_of_seasonal_climate() {
        let (arid, _, _) = classify_extreme_cell(true, 2_400, 3_200, 1_600, 80, 40, 20, 850_000);
        let (wet, _, _) =
            classify_extreme_cell(true, 2_400, 3_200, 1_600, 1_800, 2_000, 1_400, 80_000);
        assert!(arid > wet, "arid {arid} wet {wet}");
        let (_, land_heat, _) =
            classify_extreme_cell(true, 2_800, 3_600, 1_400, 400, 500, 200, 300_000);
        let (_, ocean_heat, _) =
            classify_extreme_cell(false, 2_800, 3_000, 2_600, 400, 500, 200, 300_000);
        assert!(
            land_heat > ocean_heat,
            "land {land_heat} ocean {ocean_heat}"
        );
        let (_, _, monsoon) =
            classify_extreme_cell(true, 2_400, 2_800, 2_000, 900, 2_400, 200, 200_000);
        let (_, _, even) = classify_extreme_cell(true, 2_400, 2_800, 2_000, 400, 420, 380, 200_000);
        assert!(monsoon > even, "monsoon {monsoon} even {even}");
        let (ocean_drought, _, _) =
            classify_extreme_cell(false, 2_800, 3_000, 2_600, 80, 40, 20, 900_000);
        assert_eq!(ocean_drought, 0);
    }
}
