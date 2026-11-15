//! Seam-wrapped, pole-safe samplers for accepted physical control fields.

use daena_physical::engine::{AtlasInputs, TectonicBoundaryKind, TectonicView, ToolSurface};
use daena_physical::products::{self, HazardField, VolcanicActivity};
use daena_physical::Grid;

use crate::detail::{nearest_cell, sample_cell_field_mm};
use crate::projection::{clamp_lat_micro, wrap_lon_micro};

/// Atlas style group ids. The engine's 26 biomes map onto these ten groups in
/// one explicit adapter, so the bundled styles keep their palette.
pub const CLIMATE_CLASS_OCEAN: i32 = 0;
pub const CLIMATE_CLASS_ICE: i32 = 1;
pub const CLIMATE_CLASS_TUNDRA: i32 = 2;
pub const CLIMATE_CLASS_ALPINE: i32 = 3;
pub const CLIMATE_CLASS_COLD_GRASSLAND: i32 = 4;
pub const CLIMATE_CLASS_GRASSLAND: i32 = 5;
pub const CLIMATE_CLASS_ARID: i32 = 6;
pub const CLIMATE_CLASS_SHRUBLAND: i32 = 7;
pub const CLIMATE_CLASS_FOREST: i32 = 8;
pub const CLIMATE_CLASS_TROPICAL_FOREST: i32 = 9;
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
    pub wind_divergence_nano_per_s: Vec<i32>,
    pub wind_divergence_nh_summer_nano_per_s: Vec<i32>,
    pub wind_divergence_nh_winter_nano_per_s: Vec<i32>,
    pub wind_band: Vec<i32>,
    pub wind_band_nh_summer: Vec<i32>,
    pub wind_band_nh_winter: Vec<i32>,
    pub runoff_mm: Vec<i32>,
    pub precipitation_mm: Vec<i32>,
    pub aridity_ppm: Vec<i32>,
    pub precipitation_nh_summer_mm: Vec<i32>,
    pub precipitation_nh_winter_mm: Vec<i32>,
    pub storm_suitability_ppm: Vec<i32>,
    pub storm_track_ppm: Vec<i32>,
    pub storm_intensity_ppm: Vec<i32>,
    /// Engine biome id `0..=25` at the view cell.
    pub climate_class: Vec<i32>,
    pub ice_mask: Vec<i32>,
    pub water_level_mm: Vec<i32>,
    pub lake_level_mm: Vec<i32>,
    pub mountain_influence_ppm: Vec<i32>,
    pub watershed_id: Vec<i32>,
    pub basin_id: Vec<i32>,
    pub lake_mask: Vec<i32>,
    pub sea_level_mm: i32,
}

fn to_i32(values: impl IntoIterator<Item = u32>) -> Vec<i32> {
    values
        .into_iter()
        .map(|value| i32::try_from(value).unwrap_or(i32::MAX))
        .collect()
}

impl ControlFields {
    pub fn from_atlas_inputs(inputs: &AtlasInputs) -> Result<Self, crate::AtlasError> {
        let field = &inputs.view;
        let climate = &inputs.climate;
        let hydrology = &inputs.hydrology;
        let count = field.grid.sample_count();
        if inputs.tectonics.continental.len() != count
            || climate.temperature_centi_c.len() != count
            || climate.temperature_nh_summer_centi_c.len() != count
            || climate.temperature_nh_winter_centi_c.len() != count
            || climate.wind_east_milli.len() != count
            || climate.wind_north_milli.len() != count
            || climate.wind_east_nh_summer_milli.len() != count
            || climate.wind_north_nh_summer_milli.len() != count
            || climate.wind_east_nh_winter_milli.len() != count
            || climate.wind_north_nh_winter_milli.len() != count
            || climate.wind_divergence_nano_per_s.len() != count
            || climate.wind_divergence_nh_summer_nano_per_s.len() != count
            || climate.wind_divergence_nh_winter_nano_per_s.len() != count
            || climate.wind_band.len() != count
            || climate.wind_band_nh_summer.len() != count
            || climate.wind_band_nh_winter.len() != count
            || climate.runoff_mm_per_year.len() != count
            || climate.precipitation_mm_per_year.len() != count
            || climate.aridity_ppm.len() != count
            || climate.precipitation_nh_summer_mm.len() != count
            || climate.precipitation_nh_winter_mm.len() != count
            || climate.biome_class.len() != count
            || climate.storm_suitability_ppm.len() != count
            || climate.storm_track_ppm.len() != count
            || climate.storm_intensity_ppm.len() != count
            || hydrology.ice_cells.len() != count
            || hydrology.water_level_mm.len() != count
            || hydrology.lake_level_mm.len() != count
            || hydrology.watershed_id.len() != count
            || hydrology.basin_by_cell.len() != count
            || hydrology.lake_cells.len() != count
            || hydrology.grid != field.grid
        {
            return Err(crate::AtlasError::invalid(
                "control field sample count does not match the grid",
            ));
        }
        let crust_influence_ppm = inputs
            .tectonics
            .continental
            .iter()
            .map(|continental| {
                if *continental {
                    CONTINENTAL_INFLUENCE_PPM
                } else {
                    0
                }
            })
            .collect::<Vec<_>>();
        let crust_class = inputs
            .tectonics
            .continental
            .iter()
            .map(|continental| i32::from(*continental))
            .collect::<Vec<_>>();
        let runoff_mm = to_i32(climate.runoff_mm_per_year.iter().copied());
        let precipitation_mm = to_i32(climate.precipitation_mm_per_year.iter().copied());
        let aridity_ppm = to_i32(climate.aridity_ppm.iter().copied());
        let precipitation_nh_summer_mm = to_i32(climate.precipitation_nh_summer_mm.iter().copied());
        let precipitation_nh_winter_mm = to_i32(climate.precipitation_nh_winter_mm.iter().copied());
        let storm_suitability_ppm = to_i32(climate.storm_suitability_ppm.iter().copied());
        let storm_track_ppm = to_i32(climate.storm_track_ppm.iter().copied());
        let storm_intensity_ppm = to_i32(climate.storm_intensity_ppm.iter().copied());
        let climate_class = to_i32(climate.biome_class.iter().copied());
        let ice_mask = hydrology
            .ice_cells
            .iter()
            .map(|ice| i32::from(*ice))
            .collect::<Vec<_>>();
        let watershed_id = to_i32(hydrology.watershed_id.iter().copied());
        let basin_id = to_i32(hydrology.basin_by_cell.iter().copied());
        let lake_mask = hydrology
            .lake_cells
            .iter()
            .map(|lake| i32::from(*lake))
            .collect::<Vec<_>>();
        Ok(Self {
            grid: field.grid,
            elevation_mm: field.elevations_mm.clone(),
            mountain_influence_ppm: mountain_influence_ppm(
                field.grid,
                &inputs.tectonics,
                &inputs.hazards,
                &inputs.surface,
                &field.elevations_mm,
            ),
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
            wind_divergence_nano_per_s: climate.wind_divergence_nano_per_s.clone(),
            wind_divergence_nh_summer_nano_per_s: climate
                .wind_divergence_nh_summer_nano_per_s
                .clone(),
            wind_divergence_nh_winter_nano_per_s: climate
                .wind_divergence_nh_winter_nano_per_s
                .clone(),
            wind_band: to_i32(climate.wind_band.iter().copied()),
            wind_band_nh_summer: to_i32(climate.wind_band_nh_summer.iter().copied()),
            wind_band_nh_winter: to_i32(climate.wind_band_nh_winter.iter().copied()),
            runoff_mm,
            precipitation_mm,
            aridity_ppm,
            precipitation_nh_summer_mm,
            precipitation_nh_winter_mm,
            storm_suitability_ppm,
            storm_track_ppm,
            storm_intensity_ppm,
            climate_class,
            ice_mask,
            water_level_mm: hydrology.water_level_mm.clone(),
            lake_level_mm: hydrology.lake_level_mm.clone(),
            watershed_id,
            basin_id,
            lake_mask,
            sea_level_mm: hydrology.sea_level_mm,
        })
    }

    #[must_use]
    pub fn sample_elevation(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        sample_cell_field_mm(self.grid, &self.elevation_mm, lon_micro, lat_micro)
    }

    #[must_use]
    pub fn sample_crust_influence(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        sample_cell_field_mm(self.grid, &self.crust_influence_ppm, lon_micro, lat_micro)
    }

    #[must_use]
    pub fn sample_temperature(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        sample_cell_field_mm(self.grid, &self.temperature_centi_c, lon_micro, lat_micro)
    }

    #[must_use]
    pub fn sample_nh_summer_temperature(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        sample_cell_field_mm(
            self.grid,
            &self.temperature_nh_summer_centi_c,
            lon_micro,
            lat_micro,
        )
    }

    #[must_use]
    pub fn sample_nh_winter_temperature(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        sample_cell_field_mm(
            self.grid,
            &self.temperature_nh_winter_centi_c,
            lon_micro,
            lat_micro,
        )
    }

    #[must_use]
    pub fn sample_wind_east(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        sample_cell_field_mm(self.grid, &self.wind_east_milli, lon_micro, lat_micro)
    }

    #[must_use]
    pub fn sample_wind_north(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        sample_cell_field_mm(self.grid, &self.wind_north_milli, lon_micro, lat_micro)
    }

    #[must_use]
    pub fn sample_runoff(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        sample_cell_field_mm(self.grid, &self.runoff_mm, lon_micro, lat_micro)
    }

    #[must_use]
    pub fn sample_precipitation(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        sample_cell_field_mm(self.grid, &self.precipitation_mm, lon_micro, lat_micro)
    }

    #[must_use]
    pub fn sample_aridity(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        sample_cell_field_mm(self.grid, &self.aridity_ppm, lon_micro, lat_micro)
    }

    #[must_use]
    pub fn sample_precipitation_nh_summer(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        sample_cell_field_mm(
            self.grid,
            &self.precipitation_nh_summer_mm,
            lon_micro,
            lat_micro,
        )
    }

    #[must_use]
    pub fn sample_precipitation_nh_winter(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        sample_cell_field_mm(
            self.grid,
            &self.precipitation_nh_winter_mm,
            lon_micro,
            lat_micro,
        )
    }

    #[must_use]
    pub fn sample_ice_mask(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        self.ice_mask[nearest_cell(self.grid, lon_micro, lat_micro)]
    }

    #[must_use]
    pub fn sample_water_level(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        sample_cell_field_mm(self.grid, &self.water_level_mm, lon_micro, lat_micro)
    }

    #[must_use]
    pub fn sample_lake_level(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        sample_cell_field_mm(self.grid, &self.lake_level_mm, lon_micro, lat_micro)
    }

    #[must_use]
    pub fn sample_mountain_influence(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        sample_cell_field_mm(
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
    pub fn sample_biome_class(&self, lon_micro: i32, lat_micro: i32) -> i32 {
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

/// Display name of one engine biome id.
#[must_use]
pub fn biome_name(class: i32) -> &'static str {
    products::biome_name(u32::try_from(class).unwrap_or(u32::MAX))
}

/// The ten Atlas style groups for the engine's 26 biomes.
#[must_use]
pub fn biome_group(class: i32) -> i32 {
    match class {
        0..=7 => CLIMATE_CLASS_OCEAN,
        8 | 15 | 20 => CLIMATE_CLASS_ARID,
        9 | 10 => CLIMATE_CLASS_ICE,
        11 => CLIMATE_CLASS_TUNDRA,
        12 => CLIMATE_CLASS_ALPINE,
        13 | 16 | 17 => CLIMATE_CLASS_FOREST,
        14 => CLIMATE_CLASS_COLD_GRASSLAND,
        18 => CLIMATE_CLASS_SHRUBLAND,
        19 | 21 | 24 => CLIMATE_CLASS_GRASSLAND,
        22 | 23 | 25 => CLIMATE_CLASS_TROPICAL_FOREST,
        _ => CLIMATE_CLASS_OCEAN,
    }
}

#[must_use]
pub fn biome_group_name(group: i32) -> &'static str {
    match group {
        CLIMATE_CLASS_ICE => "ice",
        CLIMATE_CLASS_TUNDRA => "tundra",
        CLIMATE_CLASS_ALPINE => "alpine",
        CLIMATE_CLASS_COLD_GRASSLAND => "cold grassland",
        CLIMATE_CLASS_GRASSLAND => "temperate grassland",
        CLIMATE_CLASS_ARID => "desert",
        CLIMATE_CLASS_SHRUBLAND => "shrubland",
        CLIMATE_CLASS_FOREST => "temperate forest",
        CLIMATE_CLASS_TROPICAL_FOREST => "tropical forest",
        _ => "ocean",
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

/// Mountain influence from engine plate boundaries, volcanic sources and the
/// continental elevation quartile. Coordinates map to view cells; many
/// canonical records can share one view cell, and `max` keeps the strongest.
#[must_use]
pub fn mountain_influence_ppm(
    grid: Grid,
    tectonics: &TectonicView,
    hazards: &HazardField,
    surface: &ToolSurface,
    elevations_mm: &[i32],
) -> Vec<i32> {
    let count = grid.sample_count();
    let mut influence = vec![0_i32; count];
    for boundary in &tectonics.boundaries {
        if boundary.kind != TectonicBoundaryKind::Convergent {
            continue;
        }
        let first = nearest_cell(grid, boundary.first[0], boundary.first[1]);
        let second = nearest_cell(grid, boundary.second[0], boundary.second[1]);
        let bump = if boundary.first_continental && boundary.second_continental {
            1_000_000
        } else {
            700_000
        };
        if boundary.first_continental {
            influence[first] = influence[first].max(bump);
        }
        if boundary.second_continental {
            influence[second] = influence[second].max(bump);
        }
    }
    for source in &hazards.volcanic_sources {
        let [lon, lat] = surface.center_microdegrees(source.cell);
        let cell = nearest_cell(grid, lon, lat);
        let bump = match source.activity_class {
            VolcanicActivity::Vigorous => 1_000_000,
            VolcanicActivity::Moderate => 700_000,
            VolcanicActivity::Dormant => 400_000,
        };
        influence[cell] = influence[cell].max(bump);
    }
    let mut continental_elev = elevations_mm
        .iter()
        .enumerate()
        .filter(|(cell, _)| tectonics.continental.get(*cell).copied().unwrap_or(false))
        .map(|(_, elevation)| *elevation)
        .collect::<Vec<_>>();
    continental_elev.sort_unstable();
    let quartile = continental_elev
        .get(continental_elev.len().saturating_mul(3) / 4)
        .copied()
        .unwrap_or(i32::MAX);
    for (cell, elevation) in elevations_mm.iter().enumerate() {
        if tectonics.continental.get(cell).copied().unwrap_or(false) && *elevation >= quartile {
            influence[cell] = influence[cell].max(250_000);
        }
    }
    let mut dilated = influence.clone();
    for (cell, amount) in influence.iter().enumerate() {
        if *amount == 0 {
            continue;
        }
        for neighbor in grid.neighbors(cell) {
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
        ControlFields::from_atlas_inputs(&golden_world().inputs).unwrap()
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
        assert_eq!(
            controls.sample_aridity(180_000_000, 0),
            controls.sample_aridity(LON_MICRO_MIN, 0)
        );
        assert_eq!(
            controls.sample_precipitation_nh_summer(180_000_000, 0),
            controls.sample_precipitation_nh_summer(LON_MICRO_MIN, 0)
        );
        assert_eq!(
            controls.sample_precipitation_nh_winter(180_000_000, 0),
            controls.sample_precipitation_nh_winter(LON_MICRO_MIN, 0)
        );
        assert!(controls
            .mountain_influence_ppm
            .iter()
            .any(|value| *value > 0));
        let _ = (
            controls.sample_crust_influence(179_900_000, 89_000_000),
            controls.sample_runoff(-179_900_000, -89_000_000),
            controls.sample_ice_mask(0, 0),
            controls.sample_water_level(12_000_000, -4_000_000),
            controls.sample_lake_level(0, 10_000_000),
            controls.sample_mountain_influence(0, 0),
            controls.sample_sea_level(0, 0),
            controls.sample_biome_class(0, 0),
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
            controls.sample_biome_class(1_000, 1_000),
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
        assert_eq!(
            controls.sample_crust_class(1_000, 1_000),
            controls.crust_class[cell]
        );
    }

    #[test]
    fn engine_biomes_map_onto_the_ten_atlas_groups() {
        assert_eq!(biome_group(0), CLIMATE_CLASS_OCEAN);
        assert_eq!(biome_group(7), CLIMATE_CLASS_OCEAN);
        assert_eq!(biome_group(8), CLIMATE_CLASS_ARID);
        assert_eq!(biome_group(15), CLIMATE_CLASS_ARID);
        assert_eq!(biome_group(20), CLIMATE_CLASS_ARID);
        assert_eq!(biome_group(9), CLIMATE_CLASS_ICE);
        assert_eq!(biome_group(10), CLIMATE_CLASS_ICE);
        assert_eq!(biome_group(11), CLIMATE_CLASS_TUNDRA);
        assert_eq!(biome_group(12), CLIMATE_CLASS_ALPINE);
        assert_eq!(biome_group(13), CLIMATE_CLASS_FOREST);
        assert_eq!(biome_group(16), CLIMATE_CLASS_FOREST);
        assert_eq!(biome_group(17), CLIMATE_CLASS_FOREST);
        assert_eq!(biome_group(14), CLIMATE_CLASS_COLD_GRASSLAND);
        assert_eq!(biome_group(18), CLIMATE_CLASS_SHRUBLAND);
        assert_eq!(biome_group(19), CLIMATE_CLASS_GRASSLAND);
        assert_eq!(biome_group(21), CLIMATE_CLASS_GRASSLAND);
        assert_eq!(biome_group(24), CLIMATE_CLASS_GRASSLAND);
        assert_eq!(biome_group(22), CLIMATE_CLASS_TROPICAL_FOREST);
        assert_eq!(biome_group(23), CLIMATE_CLASS_TROPICAL_FOREST);
        assert_eq!(biome_group(25), CLIMATE_CLASS_TROPICAL_FOREST);
        assert_eq!(biome_group(99), CLIMATE_CLASS_OCEAN);
    }

    #[test]
    fn climate_and_surface_names_are_stable() {
        assert_eq!(biome_name(9), "ice sheet");
        assert_eq!(biome_name(11), "tundra");
        assert_eq!(biome_name(20), "hot desert");
        assert_eq!(biome_name(19), "grassland");
        assert_eq!(biome_name(17), "temperate forest");
        assert_eq!(biome_name(12), "alpine");
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
