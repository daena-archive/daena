//! Point readout for Studio inspect on terrain sessions.

use daena_physical::engine::TOOL_SURFACE_NONE;

use super::render::{direction_from_lon_lat, ground_metres_per_pixel};
use super::{AtlasTerrain, TerrainCover};
use crate::control;
use crate::overlay::AtlasSurfaceSample;

impl AtlasTerrain {
    /// Inspect readout at a point, as the terrain tiles paint it at zoom `z`.
    ///
    /// Elevation is the detailed height, so it agrees with the map. Climate and
    /// biome come from the nearest canonical cell. The terrain carries no winds,
    /// seasonal climate or storm fields: those are zero or empty.
    #[must_use]
    pub fn inspect_sample(&self, lon_micro: i32, lat_micro: i32, z: u32) -> AtlasSurfaceSample {
        let lon = (f64::from(lon_micro) / 1_000_000.0).to_radians();
        let lat = (f64::from(lat_micro) / 1_000_000.0).to_radians();
        let direction = direction_from_lon_lat(lon, lat);
        let ground_m = ground_metres_per_pixel(z, lat, self.radius_m);
        let (height_m, cover, ice) = self.sample_pixel(direction, ground_m);
        let surface = self.surface();
        let cell = surface.cell_at_direction(direction) as usize;
        let sea_level_mm = self.sea_level_mm;
        let elevation_mm = (height_m * 1000.0).round() as i32;
        let lake = cover == TerrainCover::Lake;
        let water_surface_mm = if lake {
            let (cells, weights) = surface.bilinear_stencil(super::unit(direction));
            (0..4)
                .filter(|&slot| self.lake_id[cells[slot] as usize] != TOOL_SURFACE_NONE)
                .max_by(|&a, &b| weights[a].total_cmp(&weights[b]))
                .map_or(sea_level_mm, |slot| {
                    (f64::from(self.lake_level_m[cells[slot] as usize]) * 1000.0).round() as i32
                })
        } else {
            sea_level_mm
        };
        let temperature_centi_c = surface.temperature_centi_c.get(cell).copied().unwrap_or(0);
        let precipitation_mm = surface
            .precipitation_mm
            .get(cell)
            .map_or(0, |&mm| i32::try_from(mm).unwrap_or(i32::MAX));
        let biome_class = surface.biome_class.get(cell).copied().map_or(0, i32::from);
        let climate = format!(
            "{} ({})",
            control::biome_name(biome_class),
            control::biome_group_name(control::biome_group(biome_class))
        );
        let biome_reason = format!(
            "Annual {:.1} °C, {} mm/year precipitation, elevation {:.0} m.",
            f64::from(temperature_centi_c) / 100.0,
            precipitation_mm.max(0),
            f64::from(elevation_mm.saturating_sub(sea_level_mm)) / 1_000.0,
        );
        AtlasSurfaceSample {
            lon_micro,
            lat_micro,
            elevation_mm,
            water_surface_mm,
            temperature_centi_c,
            temperature_nh_summer_centi_c: 0,
            temperature_nh_winter_centi_c: 0,
            seasonal_range_centi_c: 0,
            freeze: String::new(),
            wind_east_milli: 0,
            wind_north_milli: 0,
            wind_east_nh_summer_milli: 0,
            wind_north_nh_summer_milli: 0,
            wind_east_nh_winter_milli: 0,
            wind_north_nh_winter_milli: 0,
            wind_divergence_nano_per_s: 0,
            wind_divergence_nh_summer_nano_per_s: 0,
            wind_divergence_nh_winter_nano_per_s: 0,
            wind_band: String::new(),
            wind_band_nh_summer: String::new(),
            wind_band_nh_winter: String::new(),
            precipitation_mm,
            aridity_ppm: 0,
            precipitation_nh_summer_mm: 0,
            precipitation_nh_winter_mm: 0,
            climate,
            biome_reason,
            storm_suitability_ppm: 0,
            storm_track_ppm: 0,
            storm_intensity_ppm: 0,
            storm_reason: String::new(),
            surface: control::surface_kind(ice, lake, elevation_mm, sea_level_mm).to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::fixture;

    fn micro(direction: [f64; 3]) -> (i32, i32) {
        let lat = direction[2].clamp(-1.0, 1.0).asin().to_degrees();
        let lon = direction[1].atan2(direction[0]).to_degrees();
        (
            (lon * 1_000_000.0).round() as i32,
            (lat * 1_000_000.0).round() as i32,
        )
    }

    #[test]
    fn inspect_sample_reads_land_and_ocean_cells() {
        let terrain = fixture();
        let surface = terrain.surface();
        let land = terrain.auto_site_cell();
        let ocean = (0..surface.cell_count() as u32)
            .find(|&cell| surface.ocean(cell) && surface.elevation_mm[cell as usize] < -2_000_000)
            .expect("deep ocean cell");

        let (lon, lat) = micro(surface.direction_of(land));
        let sample = terrain.inspect_sample(lon, lat, 6);
        assert_eq!(sample.surface, "land");
        assert!(sample.elevation_mm >= sample.water_surface_mm);
        assert_eq!(
            sample.temperature_centi_c,
            surface.temperature_centi_c[land as usize]
        );
        assert!(!sample.climate.is_empty());

        let (lon, lat) = micro(surface.direction_of(ocean));
        let sample = terrain.inspect_sample(lon, lat, 6);
        assert_eq!(sample.surface, "ocean");
        assert_eq!(sample.water_surface_mm, terrain.sea_level_mm());
        assert!(sample.elevation_mm < sample.water_surface_mm);
    }
}
