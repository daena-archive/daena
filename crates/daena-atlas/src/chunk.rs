//! On-demand nested octaves for Studio tiles. Coarse structure stays on the
//! prepared residual; higher zoom adds factor-8/16 cells for the viewport plus
//! a halo. Print is never built globally.

use crate::amplify::{
    coastline_remainder_delta_mm, coastline_remainder_ppm, octave_sample_mm,
    COASTLINE_SYNTHESIS_DOMAIN, HIERARCHICAL_RELIEF_DOMAIN,
};
use crate::constraint::{pins_shoreline, AtlasConstraint};
use crate::detail::{
    domain_key, lattice_lat_micro, lattice_lon_micro, sample_mask_ppm, sample_sdf_ppm,
    AtlasDetailModel, COASTAL_ENVELOPE_PPM, COASTAL_RAMP_MM,
};
use crate::erosion::lattice_index;
use crate::projection::{bilinear_i32, lat_to_row_ppm, lon_to_column_ppm};
use crate::{AtlasError, AtlasPreparedScene, ATLAS_DETAIL_ALGORITHM_VERSION};

pub const STUDIO_CHUNK_HALO: u32 = 8;
pub const STUDIO_CHUNK_DETAILED_ZOOM: u32 = 4;
pub const STUDIO_CHUNK_PRINT_ZOOM: u32 = 7;
const MAX_CHUNK_CELLS: usize = 256 * 256;

#[derive(Clone, Copy)]
struct ChunkCoast<'a> {
    sdf: &'a [i32],
    sea_level_mm: i32,
    lake_cells: &'a [bool],
    constraints: &'a [AtlasConstraint],
}

#[must_use]
pub fn studio_chunk_factor(prepared_factor: u32, z: u32) -> u32 {
    let want = if z >= STUDIO_CHUNK_PRINT_ZOOM {
        16
    } else if z >= STUDIO_CHUNK_DETAILED_ZOOM {
        8
    } else {
        4
    };
    want.max(prepared_factor)
}

#[derive(Debug, Clone)]
pub struct DetailChunk {
    pub origin_i: u32,
    pub origin_j: u32,
    pub width: u32,
    pub height: u32,
    pub lattice_width: u32,
    pub lattice_height: u32,
    pub factor: u32,
    extra_mm: Vec<i32>,
}

impl DetailChunk {
    pub fn cover(
        scene: &AtlasPreparedScene,
        z: u32,
        west_lon_micro: i32,
        east_lon_micro: i32,
        south_lat_micro: i32,
        north_lat_micro: i32,
        constraints: &[AtlasConstraint],
    ) -> Result<Option<Self>, AtlasError> {
        let prepared = scene.model.level.lattice_factor();
        let mut factor = studio_chunk_factor(prepared, z);
        if factor <= prepared {
            return Ok(None);
        }
        let coast = ChunkCoast {
            sdf: &scene.sdf,
            sea_level_mm: scene.hydrology.sea_level_mm,
            lake_cells: &scene.hydrology.lake_cells,
            constraints,
        };
        loop {
            match Self::build(
                &scene.model,
                &scene.crust_influence_ppm,
                &scene.mountain_influence_ppm,
                &scene.identity,
                factor,
                west_lon_micro,
                east_lon_micro,
                south_lat_micro,
                north_lat_micro,
                Some(coast),
            ) {
                Ok(chunk) => return Ok(Some(chunk)),
                Err(error) if error.code == crate::CODE_RESOURCE_LIMIT && factor > prepared * 2 => {
                    factor /= 2;
                }
                Err(error) if error.code == crate::CODE_RESOURCE_LIMIT => return Ok(None),
                Err(error) => return Err(error),
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn build(
        model: &AtlasDetailModel,
        crust_influence_ppm: &[i32],
        mountain_influence_ppm: &[i32],
        identity: &[u8],
        factor: u32,
        west_lon_micro: i32,
        east_lon_micro: i32,
        south_lat_micro: i32,
        north_lat_micro: i32,
        coast: Option<ChunkCoast<'_>>,
    ) -> Result<Self, AtlasError> {
        let lattice_width = model
            .grid
            .width
            .checked_mul(factor)
            .ok_or_else(|| crate::AtlasError::limit("atlas chunk lattice width overflowed"))?;
        let lattice_height = model
            .grid
            .height
            .checked_mul(factor)
            .ok_or_else(|| crate::AtlasError::limit("atlas chunk lattice height overflowed"))?;
        let (i_west, _, _) = lon_to_column_ppm(west_lon_micro, lattice_width);
        let (i_east, _, _) = lon_to_column_ppm(east_lon_micro, lattice_width);
        let (j_south, _, _) = lat_to_row_ppm(south_lat_micro, lattice_height);
        let (j_north, _, _) = lat_to_row_ppm(north_lat_micro, lattice_height);
        let span_i = (i_east + lattice_width - i_west) % lattice_width + 1;
        let j_min = j_north.min(j_south);
        let j_max = j_north.max(j_south);
        let halo = STUDIO_CHUNK_HALO;
        let origin_i = (i_west + lattice_width - halo) % lattice_width;
        let origin_j = j_min.saturating_sub(halo);
        let width = span_i
            .saturating_add(halo.saturating_mul(2))
            .min(lattice_width);
        let j_end = (j_max + 1).saturating_add(halo).min(lattice_height);
        let height = j_end.saturating_sub(origin_j);
        let count = (width as usize).saturating_mul(height as usize);
        if count == 0 || count > MAX_CHUNK_CELLS {
            return Err(crate::AtlasError::limit(
                "atlas studio chunk exceeded the in-process cell budget",
            ));
        }
        let prepared = model.level.lattice_factor();
        let key = domain_key(
            identity,
            ATLAS_DETAIL_ALGORITHM_VERSION,
            model.variant,
            HIERARCHICAL_RELIEF_DOMAIN,
        );
        let mut extra_mm = vec![0_i32; count];
        let mut octave = prepared.saturating_mul(2);
        while octave <= factor {
            for local_j in 0..height {
                let world_j = origin_j + local_j;
                let lat = lattice_lat_micro(world_j, lattice_height);
                for local_i in 0..width {
                    let world_i = (origin_i + local_i) % lattice_width;
                    let lon = lattice_lon_micro(world_i, lattice_width);
                    let cell = octave_sample_mm(
                        model.grid,
                        &model.elevations_mm,
                        crust_influence_ppm,
                        mountain_influence_ppm,
                        &key,
                        octave,
                        lon,
                        lat,
                    );
                    extra_mm[lattice_index(width, local_i, local_j)] =
                        extra_mm[lattice_index(width, local_i, local_j)].saturating_add(cell);
                }
            }
            octave = octave.saturating_mul(2);
        }
        if let Some(coast) = coast {
            let coast_key = domain_key(
                identity,
                ATLAS_DETAIL_ALGORITHM_VERSION,
                model.variant,
                COASTLINE_SYNTHESIS_DOMAIN,
            );
            for local_j in 0..height {
                let world_j = origin_j + local_j;
                let lat = lattice_lat_micro(world_j, lattice_height);
                for local_i in 0..width {
                    let world_i = (origin_i + local_i) % lattice_width;
                    let lon = lattice_lon_micro(world_i, lattice_width);
                    let index = lattice_index(width, local_i, local_j);
                    extra_mm[index] = compose_coastal_extra(
                        extra_mm[index],
                        model,
                        coast,
                        &coast_key,
                        world_i,
                        world_j,
                        lon,
                        lat,
                        lattice_width,
                        lattice_height,
                        prepared,
                        factor,
                    );
                }
            }
        }
        Ok(Self {
            origin_i,
            origin_j,
            width,
            height,
            lattice_width,
            lattice_height,
            factor,
            extra_mm,
        })
    }

    #[must_use]
    pub fn extra_at(&self, lon_micro: i32, lat_micro: i32) -> i32 {
        let (col, next_col, fx) = lon_to_column_ppm(lon_micro, self.lattice_width);
        let (row, next_row, fy) = lat_to_row_ppm(lat_micro, self.lattice_height);
        let Some(c00) = self.cell(col, row) else {
            return 0;
        };
        let Some(c10) = self.cell(next_col, row) else {
            return 0;
        };
        let Some(c01) = self.cell(col, next_row) else {
            return 0;
        };
        let Some(c11) = self.cell(next_col, next_row) else {
            return 0;
        };
        bilinear_i32(c00, c10, c01, c11, fx, fy)
    }

    fn cell(&self, world_i: u32, world_j: u32) -> Option<i32> {
        let di = (world_i + self.lattice_width - self.origin_i) % self.lattice_width;
        if di >= self.width {
            return None;
        }
        if world_j < self.origin_j {
            return None;
        }
        let dj = world_j - self.origin_j;
        if dj >= self.height {
            return None;
        }
        Some(self.extra_mm[lattice_index(self.width, di, dj)])
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn has_nonzero_extra(&self) -> bool {
        self.extra_mm.iter().any(|value| *value != 0)
    }
}

#[allow(clippy::too_many_arguments)]
fn compose_coastal_extra(
    structure_extra_mm: i32,
    model: &AtlasDetailModel,
    coast: ChunkCoast<'_>,
    coast_key: &[u8; 32],
    world_i: u32,
    world_j: u32,
    lon: i32,
    lat: i32,
    lattice_width: u32,
    lattice_height: u32,
    prepared_factor: u32,
    factor: u32,
) -> i32 {
    let sdf_ppm = sample_sdf_ppm(model.grid, coast.sdf, lon, lat);
    let abs_sdf = sdf_ppm.unsigned_abs();
    if abs_sdf > COASTAL_ENVELOPE_PPM {
        return structure_extra_mm;
    }
    if pins_shoreline(coast.constraints, lon, lat) {
        return 0;
    }
    let fade_ppm =
        ((u64::from(abs_sdf) * 1_000_000) / u64::from(COASTAL_ENVELOPE_PPM.max(1))) as i32;
    let prepared_mm = model
        .canonical_at(lon, lat)
        .saturating_add(model.residual_at(lon, lat));
    let headroom = prepared_mm
        .saturating_sub(coast.sea_level_mm)
        .unsigned_abs()
        .saturating_sub(1);
    let faded = ((i64::from(structure_extra_mm) * i64::from(fade_ppm)) / 1_000_000) as i32;
    let faded = faded.clamp(-(headroom as i32), headroom as i32);
    if sample_mask_ppm(model.grid, coast.lake_cells, lon, lat) > 500_000 {
        return faded;
    }
    let remainder = coastline_remainder_ppm(
        coast_key,
        world_i,
        world_j,
        lattice_width,
        lattice_height,
        model.grid.width,
        prepared_factor,
        factor,
    );
    faded
        .saturating_add(coastline_remainder_delta_mm(remainder, sdf_ppm))
        .clamp(-COASTAL_RAMP_MM, COASTAL_RAMP_MM)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::amplify::{build_amplification_model, octave_cell_mm, octave_sample_mm};
    use crate::control::ControlFields;
    use crate::detail::{lattice_lat_micro, lattice_lon_micro};
    use crate::golden_world;
    use crate::request::DetailLevel;
    use crate::spike_identity_from_source;
    use daena_physical::history::{
        derive_historical_world_with_planet, HistoricalForcingParameters,
    };
    use daena_physical::NoopProgress as PhysicalNoop;

    fn controls() -> (ControlFields, Vec<u8>) {
        let world = golden_world();
        let identity = spike_identity_from_source(&world.source);
        let historical = derive_historical_world_with_planet(
            &world.field,
            world.report.reference_water_inventory_m3,
            Some(&world.tectonics.crust_by_cell),
            HistoricalForcingParameters::default_for(world.field.seed, world.field.retry_index),
            0,
            world.climate.planetary,
            &mut PhysicalNoop,
        )
        .unwrap();
        let controls = ControlFields::from_accepted(
            &world.field,
            &world.tectonics,
            &historical.climate,
            &historical.hydrology,
        )
        .unwrap();
        (controls, identity)
    }

    #[test]
    fn zoom_factor_stays_on_standard_until_chunk_zoom() {
        assert_eq!(studio_chunk_factor(4, 0), 4);
        assert_eq!(studio_chunk_factor(4, 3), 4);
        assert_eq!(studio_chunk_factor(4, 4), 8);
        assert_eq!(studio_chunk_factor(4, 8), 16);
        assert_eq!(studio_chunk_factor(8, 2), 8);
    }

    #[test]
    fn chunk_cells_match_full_octave_and_overlap() {
        let (controls, identity) = controls();
        let mut cancel = || Ok(());
        let standard =
            build_amplification_model(&controls, &identity, 0, DetailLevel::Standard, &mut cancel)
                .unwrap();
        let west = 12_000_000;
        let east = 28_000_000;
        let south = -8_000_000;
        let north = 8_000_000;
        let left = DetailChunk::build(
            &standard.detail,
            &controls.crust_influence_ppm,
            &controls.mountain_influence_ppm,
            &identity,
            8,
            west,
            east,
            south,
            north,
            None,
        )
        .unwrap();
        let right = DetailChunk::build(
            &standard.detail,
            &controls.crust_influence_ppm,
            &controls.mountain_influence_ppm,
            &identity,
            8,
            20_000_000,
            36_000_000,
            south,
            north,
            None,
        )
        .unwrap();
        let i = left.origin_i + left.width / 2;
        let j = left.origin_j + left.height / 2;
        let lon = lattice_lon_micro(i, left.lattice_width);
        let lat = lattice_lat_micro(j, left.lattice_height);
        assert_eq!(left.extra_at(lon, lat), right.extra_at(lon, lat));
        let key = domain_key(
            &identity,
            ATLAS_DETAIL_ALGORITHM_VERSION,
            0,
            HIERARCHICAL_RELIEF_DOMAIN,
        );
        let expected = octave_cell_mm(
            controls.grid,
            &controls.elevation_mm,
            &controls.crust_influence_ppm,
            &controls.mountain_influence_ppm,
            &key,
            8,
            i,
            j,
            left.lattice_width,
            left.lattice_height,
        );
        assert_eq!(left.cell(i, j), Some(expected));
        assert_ne!(expected, 0);

        let print = DetailChunk::build(
            &standard.detail,
            &controls.crust_influence_ppm,
            &controls.mountain_influence_ppm,
            &identity,
            16,
            west,
            east,
            south,
            north,
            None,
        )
        .unwrap();
        let octave8 = octave_sample_mm(
            controls.grid,
            &controls.elevation_mm,
            &controls.crust_influence_ppm,
            &controls.mountain_influence_ppm,
            &key,
            8,
            lon,
            lat,
        );
        let octave16 = octave_sample_mm(
            controls.grid,
            &controls.elevation_mm,
            &controls.crust_influence_ppm,
            &controls.mountain_influence_ppm,
            &key,
            16,
            lon,
            lat,
        );
        assert_eq!(left.extra_at(lon, lat), octave8);
        assert_eq!(print.extra_at(lon, lat), octave8.saturating_add(octave16));
    }

    #[test]
    fn coast_compose_preserves_inland_octaves_and_bounds_envelope() {
        let (controls, identity) = controls();
        let mut cancel = || Ok(());
        let standard =
            build_amplification_model(&controls, &identity, 0, DetailLevel::Standard, &mut cancel)
                .unwrap();
        let sdf = crate::detail::signed_coastal_distance_ppm(
            controls.grid,
            &controls.elevation_mm,
            controls.sea_level_mm,
        );
        let lakes = vec![false; controls.grid.sample_count()];
        let coast = ChunkCoast {
            sdf: &sdf,
            sea_level_mm: controls.sea_level_mm,
            lake_cells: &lakes,
            constraints: &[],
        };
        let (west, east, south, north) = sdf
            .iter()
            .enumerate()
            .find_map(|(index, value)| {
                (value.unsigned_abs() <= COASTAL_ENVELOPE_PPM).then(|| {
                    let (row, col) = controls.grid.row_col(index);
                    let lon = crate::detail::cell_center_lon_micro(col, controls.grid.width);
                    let lat = crate::detail::cell_center_lat_micro(row, controls.grid.height);
                    (
                        lon.saturating_sub(8_000_000),
                        lon.saturating_add(8_000_000),
                        lat.saturating_sub(8_000_000),
                        lat.saturating_add(8_000_000),
                    )
                })
            })
            .expect("golden world has a coastal cell");
        let raw = DetailChunk::build(
            &standard.detail,
            &controls.crust_influence_ppm,
            &controls.mountain_influence_ppm,
            &identity,
            8,
            west,
            east,
            south,
            north,
            None,
        )
        .unwrap();
        let composed = DetailChunk::build(
            &standard.detail,
            &controls.crust_influence_ppm,
            &controls.mountain_influence_ppm,
            &identity,
            8,
            west,
            east,
            south,
            north,
            Some(coast),
        )
        .unwrap();
        let mut inland = 0_u32;
        let mut coastal = 0_u32;
        for local_j in 0..raw.height {
            let world_j = raw.origin_j + local_j;
            for local_i in 0..raw.width {
                let world_i = (raw.origin_i + local_i) % raw.lattice_width;
                let lon = lattice_lon_micro(world_i, raw.lattice_width);
                let lat = lattice_lat_micro(world_j, raw.lattice_height);
                let sdf_ppm = sample_sdf_ppm(controls.grid, &sdf, lon, lat);
                let raw_cell = raw.cell(world_i, world_j).unwrap();
                let composed_cell = composed.cell(world_i, world_j).unwrap();
                if sdf_ppm.unsigned_abs() > COASTAL_ENVELOPE_PPM {
                    assert_eq!(composed_cell, raw_cell);
                    inland += 1;
                } else {
                    assert!(composed_cell.unsigned_abs() <= COASTAL_RAMP_MM as u32);
                    coastal += 1;
                }
            }
        }
        assert!(inland > 0 && coastal > 0);
    }
}
