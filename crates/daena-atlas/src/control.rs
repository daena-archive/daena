//! Seam-wrapped, pole-safe samplers for accepted physical control fields.

use daena_physical::climate::ClimateField;
use daena_physical::hydrology::HydrologyField;
use daena_physical::tectonics::{BoundaryKind, CrustType, TectonicWorld};
use daena_physical::{Grid, PhysicalField};

use crate::detail::{nearest_cell, sample_field_mm};
use crate::projection::{clamp_lat_micro, wrap_lon_micro};

pub const CLIMATE_CLASS_ICE: i32 = 0;
pub const CLIMATE_CLASS_TUNDRA: i32 = 1;
pub const CLIMATE_CLASS_ARID: i32 = 2;
pub const CLIMATE_CLASS_GRASSLAND: i32 = 3;
pub const CLIMATE_CLASS_FOREST: i32 = 4;
pub const CONTINENTAL_INFLUENCE_PPM: i32 = 1_000_000;

#[derive(Debug, Clone)]
pub struct ControlFields {
    pub grid: Grid,
    pub elevation_mm: Vec<i32>,
    pub crust_influence_ppm: Vec<i32>,
    pub crust_class: Vec<i32>,
    pub temperature_centi_c: Vec<i32>,
    pub temperature_nh_summer_centi_c: Vec<i32>,
    pub temperature_nh_winter_centi_c: Vec<i32>,
    pub wind_east_milli: Vec<i32>,
    pub wind_north_milli: Vec<i32>,
    pub wind_east_nh_summer_milli: Vec<i32>,
    pub wind_north_nh_summer_milli: Vec<i32>,
    pub wind_east_nh_winter_milli: Vec<i32>,
    pub wind_north_nh_winter_milli: Vec<i32>,
    pub wind_divergence_ppm: Vec<i32>,
    pub wind_divergence_nh_summer_ppm: Vec<i32>,
    pub wind_divergence_nh_winter_ppm: Vec<i32>,
    pub wind_band: Vec<i32>,
    pub wind_band_nh_summer: Vec<i32>,
    pub wind_band_nh_winter: Vec<i32>,
    pub runoff_mm: Vec<i32>,
    pub precipitation_mm: Vec<i32>,
    pub climate_class: Vec<i32>,
    pub ice_thickness_mm: Vec<i32>,
    pub water_level_mm: Vec<i32>,
    pub lake_level_mm: Vec<i32>,
    pub mountain_influence_ppm: Vec<i32>,
    pub watershed_id: Vec<i32>,
    pub basin_id: Vec<i32>,
    pub lake_mask: Vec<i32>,
    pub sea_level_mm: i32,
}

impl ControlFields {
    pub fn from_accepted(
        field: &PhysicalField,
        tectonics: &TectonicWorld,
        climate: &ClimateField,
        hydrology: &HydrologyField,
    ) -> Result<Self, crate::AtlasError> {
        let count = field.grid.sample_count();
        if tectonics.crust_by_cell.len() != count
            || climate.temperature_centi_c.len() != count
            || climate.temperature_nh_summer_centi_c.len() != count
            || climate.temperature_nh_winter_centi_c.len() != count
            || climate.wind_east_milli.len() != count
            || climate.wind_north_milli.len() != count
            || climate.wind_east_nh_summer_milli.len() != count
            || climate.wind_north_nh_summer_milli.len() != count
            || climate.wind_east_nh_winter_milli.len() != count
            || climate.wind_north_nh_winter_milli.len() != count
            || climate.wind_divergence_ppm.len() != count
            || climate.wind_divergence_nh_summer_ppm.len() != count
            || climate.wind_divergence_nh_winter_ppm.len() != count
            || climate.wind_band.len() != count
            || climate.wind_band_nh_summer.len() != count
            || climate.wind_band_nh_winter.len() != count
            || climate.runoff_mm_per_year.len() != count
            || climate.precipitation_mm_per_year.len() != count
            || hydrology.ice_thickness_mm.len() != count
            || hydrology.ice_cells.len() != count
            || hydrology.water_level_mm.len() != count
            || hydrology.lake_level_mm.len() != count
            || hydrology.watershed_id.len() != count
            || hydrology.basin_by_cell.len() != count
            || hydrology.lake_cells.len() != count
        {
            return Err(crate::AtlasError::invalid(
                "control field sample count does not match the grid",
            ));
        }
        let mut crust_influence_ppm = Vec::with_capacity(count);
        let mut crust_class = Vec::with_capacity(count);
        let mut runoff_mm = Vec::with_capacity(count);
        let mut precipitation_mm = Vec::with_capacity(count);
        let mut climate_class = Vec::with_capacity(count);
        let mut ice_thickness_mm = Vec::with_capacity(count);
        let mut watershed_id = Vec::with_capacity(count);
        let mut basin_id = Vec::with_capacity(count);
        let mut lake_mask = Vec::with_capacity(count);
        for cell in 0..count {
            let continental = tectonics.crust_by_cell[cell] == CrustType::Continental;
            crust_influence_ppm.push(if continental {
                CONTINENTAL_INFLUENCE_PPM
            } else {
                0
            });
            crust_class.push(i32::from(continental));
            runoff_mm.push(i32::try_from(climate.runoff_mm_per_year[cell]).unwrap_or(i32::MAX));
            precipitation_mm
                .push(i32::try_from(climate.precipitation_mm_per_year[cell]).unwrap_or(i32::MAX));
            ice_thickness_mm
                .push(i32::try_from(hydrology.ice_thickness_mm[cell]).unwrap_or(i32::MAX));
            watershed_id.push(i32::try_from(hydrology.watershed_id[cell]).unwrap_or(i32::MAX));
            basin_id.push(i32::try_from(hydrology.basin_by_cell[cell]).unwrap_or(i32::MAX));
            lake_mask.push(i32::from(hydrology.lake_cells[cell]));
            climate_class.push(climate_class_at(
                hydrology.ice_cells[cell],
                climate.temperature_centi_c[cell],
                climate.precipitation_mm_per_year[cell],
            ));
        }
        Ok(Self {
            grid: field.grid,
            elevation_mm: field.elevations_mm.clone(),
            crust_influence_ppm,
            crust_class,
            temperature_centi_c: climate.temperature_centi_c.clone(),
            temperature_nh_summer_centi_c: climate.temperature_nh_summer_centi_c.clone(),
            temperature_nh_winter_centi_c: climate.temperature_nh_winter_centi_c.clone(),
            wind_east_milli: climate.wind_east_milli.clone(),
            wind_north_milli: climate.wind_north_milli.clone(),
            wind_east_nh_summer_milli: climate.wind_east_nh_summer_milli.clone(),
            wind_north_nh_summer_milli: climate.wind_north_nh_summer_milli.clone(),
            wind_east_nh_winter_milli: climate.wind_east_nh_winter_milli.clone(),
            wind_north_nh_winter_milli: climate.wind_north_nh_winter_milli.clone(),
            wind_divergence_ppm: climate.wind_divergence_ppm.clone(),
            wind_divergence_nh_summer_ppm: climate.wind_divergence_nh_summer_ppm.clone(),
            wind_divergence_nh_winter_ppm: climate.wind_divergence_nh_winter_ppm.clone(),
            wind_band: climate
                .wind_band
                .iter()
                .map(|value| i32::try_from(*value).unwrap_or(i32::MAX))
                .collect(),
            wind_band_nh_summer: climate
                .wind_band_nh_summer
                .iter()
                .map(|value| i32::try_from(*value).unwrap_or(i32::MAX))
                .collect(),
            wind_band_nh_winter: climate
                .wind_band_nh_winter
                .iter()
                .map(|value| i32::try_from(*value).unwrap_or(i32::MAX))
                .collect(),
            runoff_mm,
            precipitation_mm,
            climate_class,
            ice_thickness_mm,
            water_level_mm: hydrology.water_level_mm.clone(),
            lake_level_mm: hydrology.lake_level_mm.clone(),
            mountain_influence_ppm: mountain_influence_ppm(tectonics, &field.elevations_mm),
            watershed_id,
            basin_id,
            lake_mask,
            sea_level_mm: hydrology.sea_level_mm,
        })
    }

    #[must_use]
    pub fn sample_elevation(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        sample_field_mm(self.grid, &self.elevation_mm, lon_micro, lat_micro)
    }

    #[must_use]
    pub fn sample_crust_influence(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        sample_field_mm(self.grid, &self.crust_influence_ppm, lon_micro, lat_micro)
    }

    #[must_use]
    pub fn sample_temperature(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        sample_field_mm(self.grid, &self.temperature_centi_c, lon_micro, lat_micro)
    }

    #[must_use]
    pub fn sample_nh_summer_temperature(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        sample_field_mm(
            self.grid,
            &self.temperature_nh_summer_centi_c,
            lon_micro,
            lat_micro,
        )
    }

    #[must_use]
    pub fn sample_nh_winter_temperature(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        sample_field_mm(
            self.grid,
            &self.temperature_nh_winter_centi_c,
            lon_micro,
            lat_micro,
        )
    }

    #[must_use]
    pub fn sample_wind_east(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        sample_field_mm(self.grid, &self.wind_east_milli, lon_micro, lat_micro)
    }

    #[must_use]
    pub fn sample_wind_north(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        sample_field_mm(self.grid, &self.wind_north_milli, lon_micro, lat_micro)
    }

    #[must_use]
    pub fn sample_runoff(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        sample_field_mm(self.grid, &self.runoff_mm, lon_micro, lat_micro)
    }

    #[must_use]
    pub fn sample_precipitation(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        sample_field_mm(self.grid, &self.precipitation_mm, lon_micro, lat_micro)
    }

    #[must_use]
    pub fn sample_ice_thickness(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        sample_field_mm(self.grid, &self.ice_thickness_mm, lon_micro, lat_micro)
    }

    #[must_use]
    pub fn sample_water_level(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        sample_field_mm(self.grid, &self.water_level_mm, lon_micro, lat_micro)
    }

    #[must_use]
    pub fn sample_lake_level(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        sample_field_mm(self.grid, &self.lake_level_mm, lon_micro, lat_micro)
    }

    #[must_use]
    pub fn sample_mountain_influence(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        sample_field_mm(
            self.grid,
            &self.mountain_influence_ppm,
            lon_micro,
            lat_micro,
        )
    }

    #[must_use]
    pub fn sample_sea_level(&self, _lon_micro: i32, _lat_micro: i32) -> i32 {
        self.sea_level_mm
    }

    #[must_use]
    pub fn sample_climate_class(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        self.climate_class[nearest_cell(self.grid, lon_micro, lat_micro)]
    }

    #[must_use]
    pub fn sample_watershed_id(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        self.watershed_id[nearest_cell(self.grid, lon_micro, lat_micro)]
    }

    #[must_use]
    pub fn sample_basin_id(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        self.basin_id[nearest_cell(self.grid, lon_micro, lat_micro)]
    }

    #[must_use]
    pub fn sample_lake_mask(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        self.lake_mask[nearest_cell(self.grid, lon_micro, lat_micro)]
    }

    #[must_use]
    pub fn sample_crust_class(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        self.crust_class[nearest_cell(self.grid, lon_micro, lat_micro)]
    }
}

#[must_use]
pub fn climate_class_at(ice: bool, temperature_centi_c: i32, precipitation_mm: u32) -> i32 {
    if ice {
        CLIMATE_CLASS_ICE
    } else if temperature_centi_c < -500 {
        CLIMATE_CLASS_TUNDRA
    } else if precipitation_mm < 250 {
        CLIMATE_CLASS_ARID
    } else if precipitation_mm < 800 {
        CLIMATE_CLASS_GRASSLAND
    } else {
        CLIMATE_CLASS_FOREST
    }
}

#[must_use]
pub fn climate_class_name(class: i32) -> &'static str {
    match class {
        CLIMATE_CLASS_ICE => "ice",
        CLIMATE_CLASS_TUNDRA => "tundra",
        CLIMATE_CLASS_ARID => "arid",
        CLIMATE_CLASS_GRASSLAND => "grassland",
        CLIMATE_CLASS_FOREST => "forest",
        _ => "grassland",
    }
}

#[must_use]
pub fn wind_band_name(band: i32) -> &'static str {
    match band {
        0 => "hadley",
        1 => "ferrel",
        2 => "polar",
        _ => "hadley",
    }
}

#[must_use]
pub fn surface_kind(ice: bool, lake: bool, elevation_mm: i32, sea_level_mm: i32) -> &'static str {
    if ice {
        "ice"
    } else if lake {
        "lake"
    } else if elevation_mm < sea_level_mm {
        "ocean"
    } else {
        "land"
    }
}

#[must_use]
pub fn mountain_influence_ppm(tectonics: &TectonicWorld, elevations_mm: &[i32]) -> Vec<i32> {
    let count = tectonics.grid.sample_count();
    let mut influence = vec![0_i32; count];
    for boundary in &tectonics.boundaries {
        if boundary.kind != BoundaryKind::Convergent {
            continue;
        }
        let first = tectonics.crust_by_cell[boundary.first_cell];
        let second = tectonics.crust_by_cell[boundary.second_cell];
        let bump = if first == CrustType::Continental && second == CrustType::Continental {
            1_000_000
        } else {
            700_000
        };
        if first == CrustType::Continental {
            influence[boundary.first_cell] = influence[boundary.first_cell].max(bump);
        }
        if second == CrustType::Continental {
            influence[boundary.second_cell] = influence[boundary.second_cell].max(bump);
        }
    }
    for center in &tectonics.volcanic_centers {
        let intensity = i32::try_from(center.intensity_ppm).unwrap_or(1_000_000);
        influence[center.cell] = influence[center.cell].max(intensity.max(400_000));
    }
    let mut continental_elev = elevations_mm
        .iter()
        .enumerate()
        .filter(|(cell, _)| tectonics.crust_by_cell[*cell] == CrustType::Continental)
        .map(|(_, elevation)| *elevation)
        .collect::<Vec<_>>();
    continental_elev.sort_unstable();
    let quartile = continental_elev
        .get(continental_elev.len().saturating_mul(3) / 4)
        .copied()
        .unwrap_or(i32::MAX);
    for cell in 0..count {
        if tectonics.crust_by_cell[cell] == CrustType::Continental
            && elevations_mm[cell] >= quartile
        {
            influence[cell] = influence[cell].max(250_000);
        }
    }
    let mut dilated = influence.clone();
    for (cell, amount) in influence.iter().enumerate() {
        if *amount == 0 {
            continue;
        }
        for neighbor in tectonics.grid.neighbors(cell) {
            dilated[neighbor] = dilated[neighbor].max(*amount / 2);
        }
    }
    dilated
}

#[must_use]
pub fn wrapped_antimeridian_pair(lon_a: i32, lon_b: i32) -> (i32, i32) {
    let a = wrap_lon_micro(i64::from(lon_a));
    let b = wrap_lon_micro(i64::from(lon_b));
    (a, b)
}

#[must_use]
pub fn pole_safe_lat(lat_micro: i32) -> i32 {
    clamp_lat_micro(i64::from(lat_micro))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::golden_world;
    use crate::projection::{LAT_MICRO_MIN, LON_MICRO_MIN};

    fn fields() -> ControlFields {
        let world = golden_world();
        ControlFields::from_accepted(
            &world.field,
            &world.tectonics,
            &world.climate,
            &world.hydrology,
        )
        .unwrap()
    }

    #[test]
    fn bilinear_samplers_wrap_longitude_and_clamp_latitude() {
        let controls = fields();
        let west = controls.sample_elevation(LON_MICRO_MIN, 0);
        let east = controls.sample_elevation(180_000_000, 0);
        assert_eq!(west, east);
        let wrapped = controls.sample_elevation(180_000_000 + 1_000, 0);
        let expected = controls.sample_elevation(LON_MICRO_MIN + 1_000, 0);
        assert_eq!(wrapped, expected);
        assert_eq!(
            controls.sample_elevation(0, 90_000_000),
            controls.sample_elevation(0, 91_000_000)
        );
        assert_eq!(
            controls.sample_temperature(0, LAT_MICRO_MIN),
            controls.sample_temperature(0, LAT_MICRO_MIN - 1)
        );
        assert_eq!(
            controls.sample_precipitation(180_000_000, 0),
            controls.sample_precipitation(LON_MICRO_MIN, 0)
        );
        assert!(controls
            .mountain_influence_ppm
            .iter()
            .any(|value| *value > 0));
        let _ = (
            controls.sample_crust_influence(179_900_000, 89_000_000),
            controls.sample_runoff(-179_900_000, -89_000_000),
            controls.sample_ice_thickness(0, 0),
            controls.sample_water_level(12_000_000, -4_000_000),
            controls.sample_lake_level(0, 10_000_000),
            controls.sample_mountain_influence(0, 0),
            controls.sample_sea_level(0, 0),
            controls.sample_climate_class(0, 0),
            controls.sample_watershed_id(0, 0),
            controls.sample_basin_id(0, 0),
            controls.sample_lake_mask(0, 0),
            controls.sample_crust_class(0, 0),
        );
        assert_eq!(pole_safe_lat(91_000_000), 90_000_000);
        assert_eq!(
            wrapped_antimeridian_pair(179_999_000, -179_999_000).0,
            179_999_000
        );
    }

    #[test]
    fn categorical_samplers_use_nearest_cell() {
        let controls = fields();
        let cell = nearest_cell(controls.grid, 1_000, 1_000);
        assert_eq!(
            controls.sample_watershed_id(1_000, 1_000),
            controls.watershed_id[cell]
        );
        assert_eq!(
            controls.sample_climate_class(1_000, 1_000),
            controls.climate_class[cell]
        );
        assert_eq!(
            controls.sample_basin_id(1_000, 1_000),
            controls.basin_id[cell]
        );
        assert_eq!(
            controls.sample_lake_mask(1_000, 1_000),
            controls.lake_mask[cell]
        );
    }

    #[test]
    fn climate_and_surface_names_are_stable() {
        assert_eq!(climate_class_name(CLIMATE_CLASS_ICE), "ice");
        assert_eq!(climate_class_name(CLIMATE_CLASS_TUNDRA), "tundra");
        assert_eq!(climate_class_name(CLIMATE_CLASS_ARID), "arid");
        assert_eq!(climate_class_name(CLIMATE_CLASS_GRASSLAND), "grassland");
        assert_eq!(climate_class_name(CLIMATE_CLASS_FOREST), "forest");
        assert_eq!(surface_kind(true, true, 1_000, 0), "ice");
        assert_eq!(surface_kind(false, true, -1_000, 0), "lake");
        assert_eq!(surface_kind(false, false, -1_000, 0), "ocean");
        assert_eq!(surface_kind(false, false, 1_000, 0), "land");
        assert_eq!(surface_kind(false, false, 0, 0), "land");
        assert_eq!(wind_band_name(0), "hadley");
        assert_eq!(wind_band_name(1), "ferrel");
        assert_eq!(wind_band_name(2), "polar");
    }
}
