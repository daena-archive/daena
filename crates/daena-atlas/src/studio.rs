//! Interactive Atlas Studio tile math and per-pixel XYZ rendering.
//!
//! This module does not open a project, speak Tauri, or change the static
//! export renderer. Tile rows are north-origin Web Mercator XYZ.

use serde::{Deserialize, Serialize};

use crate::projection::{
    mercator_x_to_lon_micro, mercator_y_to_lat_micro, wrap_lon_micro, AtlasExtent, AtlasProjection,
    WEB_MERCATOR_MAX_LAT_MICRO,
};
use crate::render::pixel_rgba;
use crate::request::{AtlasFormat, AtlasRenderRequest, DetailLevel};
use crate::{
    AtlasError, AtlasPhase, AtlasPreparedScene, AtlasProgress, ATLAS_DETAIL_ALGORITHM_VERSION,
    ATLAS_REQUEST_SCHEMA_VERSION,
};

pub const ATLAS_STUDIO_SESSION_SCHEMA_VERSION: u32 = 1;
pub const ATLAS_STUDIO_TILE_SCHEMA_VERSION: u32 = 1;
pub const STUDIO_TILE_SIZE: u32 = 256;
pub const STUDIO_MAX_ZOOM: u32 = 8;
pub const STUDIO_MAX_DEVICE_SCALE: u32 = 2;
pub const STUDIO_TILE_HALO: u32 = 0;
pub const STUDIO_PROJECTION_ID: &str = crate::projection::WEB_MERCATOR_ID;

pub const CODE_STUDIO_REQUEST_INVALID: &str = "atlas.studio.request.invalid";
pub const CODE_STUDIO_TILE_INVALID: &str = "atlas.studio.tile.invalid";
pub const CODE_STUDIO_RESOURCE_LIMIT: &str = "atlas.studio.resource-limit";
pub const CODE_STUDIO_CANCELLED: &str = "atlas.studio.cancelled";

pub const STUDIO_SPIKE_LAYER_IDS: [&str; 4] = ["ocean", "relief", "ice", "lakes"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlasStudioSceneRequestV1 {
    pub schema_version: u32,
    pub offset_years: i64,
    pub algorithm_version: u32,
    pub level: DetailLevel,
    pub variant: u32,
    pub style_id: String,
}

impl AtlasStudioSceneRequestV1 {
    pub fn spike() -> Self {
        Self {
            schema_version: ATLAS_STUDIO_SESSION_SCHEMA_VERSION,
            offset_years: 0,
            algorithm_version: ATLAS_DETAIL_ALGORITHM_VERSION,
            level: DetailLevel::Detailed,
            variant: 0,
            style_id: crate::style::RELIEF_STYLE_ID.to_string(),
        }
    }

    pub fn normalize(self) -> Result<Self, AtlasError> {
        if self.schema_version != ATLAS_STUDIO_SESSION_SCHEMA_VERSION {
            return Err(studio_invalid("unsupported atlas studio session schema"));
        }
        if self.algorithm_version != ATLAS_DETAIL_ALGORITHM_VERSION {
            return Err(studio_invalid("unsupported atlas detail algorithm"));
        }
        let mut request = self.as_render_request(STUDIO_TILE_SIZE)?;
        request = request.normalize().map_err(map_studio_error)?;
        Ok(Self {
            schema_version: ATLAS_STUDIO_SESSION_SCHEMA_VERSION,
            offset_years: request.offset_years,
            algorithm_version: request.algorithm_version,
            level: request.level,
            variant: request.variant,
            style_id: request.style_id,
        })
    }

    pub fn as_render_request(&self, output_px: u32) -> Result<AtlasRenderRequest, AtlasError> {
        let mut request = AtlasRenderRequest {
            schema_version: ATLAS_REQUEST_SCHEMA_VERSION,
            offset_years: self.offset_years,
            algorithm_version: self.algorithm_version,
            level: self.level,
            variant: self.variant,
            style_id: self.style_id.clone(),
            width_px: output_px,
            height_px: output_px,
            dpi: 72,
            format: AtlasFormat::Png,
            projection: AtlasProjection::WebMercator,
            extent: xyz_extent(0, 0, 0)?,
            unlock_aspect: false,
            active_layer_ids: STUDIO_SPIKE_LAYER_IDS
                .iter()
                .map(|id| (*id).to_string())
                .collect(),
            time_kind: "physical-offset-year".into(),
            authored_year: None,
            binding_revision: None,
        };
        request.active_layer_ids.sort();
        Ok(request)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlasStudioTileRequestV1 {
    pub schema_version: u32,
    pub z: u32,
    pub x: u32,
    pub y: u32,
    pub tile_size: u32,
    pub device_scale: u32,
    pub request_id: String,
}

impl AtlasStudioTileRequestV1 {
    pub fn new(z: u32, x: u32, y: u32) -> Self {
        Self {
            schema_version: ATLAS_STUDIO_TILE_SCHEMA_VERSION,
            z,
            x,
            y,
            tile_size: STUDIO_TILE_SIZE,
            device_scale: 1,
            request_id: String::new(),
        }
    }

    pub fn normalize(self) -> Result<Self, AtlasError> {
        if self.schema_version != ATLAS_STUDIO_TILE_SCHEMA_VERSION {
            return Err(studio_tile_invalid("unsupported atlas studio tile schema"));
        }
        if self.device_scale == 0 || self.device_scale > STUDIO_MAX_DEVICE_SCALE {
            return Err(studio_tile_invalid(
                "atlas studio device scale must be 1 or 2",
            ));
        }
        let output = self.output_px()?;
        if output == 0 {
            return Err(AtlasError::new(
                CODE_STUDIO_RESOURCE_LIMIT,
                "atlas studio tile output overflowed",
            ));
        }
        if self.tile_size != STUDIO_TILE_SIZE {
            return Err(studio_tile_invalid("atlas studio tile size must be 256"));
        }
        let n = tile_count(self.z)?;
        if self.x >= n || self.y >= n {
            return Err(studio_tile_invalid(
                "atlas studio tile coordinate is out of range",
            ));
        }
        Ok(self)
    }

    pub fn output_px(&self) -> Result<u32, AtlasError> {
        self.tile_size
            .checked_mul(self.device_scale)
            .ok_or_else(|| {
                AtlasError::new(
                    CODE_STUDIO_RESOURCE_LIMIT,
                    "atlas studio tile output overflowed",
                )
            })
    }
}

#[derive(Debug, Clone)]
pub struct RenderedStudioTile {
    pub z: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub png: Vec<u8>,
}

fn studio_invalid(message: impl Into<String>) -> AtlasError {
    AtlasError::new(CODE_STUDIO_REQUEST_INVALID, message)
}

fn studio_tile_invalid(message: impl Into<String>) -> AtlasError {
    AtlasError::new(CODE_STUDIO_TILE_INVALID, message)
}

fn map_studio_error(error: AtlasError) -> AtlasError {
    if error.code == crate::CODE_RESOURCE_LIMIT {
        AtlasError::new(CODE_STUDIO_RESOURCE_LIMIT, error.message)
    } else if error.code == crate::CODE_RENDER_CANCELLED {
        AtlasError::new(CODE_STUDIO_CANCELLED, error.message)
    } else {
        AtlasError::new(CODE_STUDIO_REQUEST_INVALID, error.message)
    }
}

pub fn tile_count(z: u32) -> Result<u32, AtlasError> {
    if z > STUDIO_MAX_ZOOM {
        return Err(studio_tile_invalid(
            "atlas studio zoom exceeds the locked maximum",
        ));
    }
    1u32.checked_shl(z).ok_or_else(|| {
        AtlasError::new(
            CODE_STUDIO_RESOURCE_LIMIT,
            "atlas studio zoom overflowed the tile grid",
        )
    })
}

pub fn xyz_extent(z: u32, x: u32, y: u32) -> Result<AtlasExtent, AtlasError> {
    let n = tile_count(z)?;
    if x >= n || y >= n {
        return Err(studio_tile_invalid(
            "atlas studio tile coordinate is out of range",
        ));
    }
    let n_i = i64::from(n);
    let west = -180_000_000i64
        .checked_add(
            i64::from(x)
                .checked_mul(360_000_000)
                .and_then(|span| span.checked_div(n_i))
                .ok_or_else(|| {
                    AtlasError::new(CODE_STUDIO_RESOURCE_LIMIT, "tile longitude overflowed")
                })?,
        )
        .ok_or_else(|| AtlasError::new(CODE_STUDIO_RESOURCE_LIMIT, "tile longitude overflowed"))?;
    let east_unwrapped = -180_000_000i64
        .checked_add(
            i64::from(x)
                .checked_add(1)
                .and_then(|next| next.checked_mul(360_000_000))
                .and_then(|span| span.checked_div(n_i))
                .ok_or_else(|| {
                    AtlasError::new(CODE_STUDIO_RESOURCE_LIMIT, "tile longitude overflowed")
                })?,
        )
        .ok_or_else(|| AtlasError::new(CODE_STUDIO_RESOURCE_LIMIT, "tile longitude overflowed"))?;
    let east = if x + 1 == n {
        180_000_000
    } else {
        wrap_lon_micro(east_unwrapped)
    };
    let north_y = std::f64::consts::PI * (1.0 - 2.0 * f64::from(y) / f64::from(n));
    let south_y = std::f64::consts::PI * (1.0 - 2.0 * f64::from(y + 1) / f64::from(n));
    let north = mercator_y_to_lat_micro(north_y)
        .clamp(-WEB_MERCATOR_MAX_LAT_MICRO, WEB_MERCATOR_MAX_LAT_MICRO);
    let south = mercator_y_to_lat_micro(south_y)
        .clamp(-WEB_MERCATOR_MAX_LAT_MICRO, WEB_MERCATOR_MAX_LAT_MICRO);
    let extent = AtlasExtent {
        west_lon_micro: wrap_lon_micro(west),
        south_lat_micro: south.min(north - 1),
        east_lon_micro: east,
        north_lat_micro: north.max(south + 1),
    };
    extent
        .validate(AtlasProjection::WebMercator)
        .map_err(map_studio_error)?;
    Ok(extent)
}

pub fn xyz_pixel_center(
    z: u32,
    x: u32,
    y: u32,
    px: u32,
    py: u32,
    output_px: u32,
) -> Result<(i32, i32), AtlasError> {
    let n = tile_count(z)?;
    if x >= n || y >= n || px >= output_px || py >= output_px {
        return Err(studio_tile_invalid("atlas studio sample is out of range"));
    }
    let world_px = u64::from(n)
        .checked_mul(u64::from(output_px))
        .ok_or_else(|| {
            AtlasError::new(
                CODE_STUDIO_RESOURCE_LIMIT,
                "atlas studio world pixel overflowed",
            )
        })?;
    let gx = u64::from(x)
        .checked_mul(u64::from(output_px))
        .and_then(|base| base.checked_add(u64::from(px)))
        .ok_or_else(|| {
            AtlasError::new(CODE_STUDIO_RESOURCE_LIMIT, "atlas studio sample overflowed")
        })?;
    let gy = u64::from(y)
        .checked_mul(u64::from(output_px))
        .and_then(|base| base.checked_add(u64::from(py)))
        .ok_or_else(|| {
            AtlasError::new(CODE_STUDIO_RESOURCE_LIMIT, "atlas studio sample overflowed")
        })?;
    let mx = std::f64::consts::PI * (2.0 * (gx as f64 + 0.5) / world_px as f64 - 1.0);
    let my = std::f64::consts::PI * (1.0 - 2.0 * (gy as f64 + 0.5) / world_px as f64);
    let lon = mercator_x_to_lon_micro(mx, 0);
    let lat =
        mercator_y_to_lat_micro(my).clamp(-WEB_MERCATOR_MAX_LAT_MICRO, WEB_MERCATOR_MAX_LAT_MICRO);
    Ok((lon, lat))
}

pub fn flip_rgba_vertical(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, AtlasError> {
    let row_bytes = (width as usize)
        .checked_mul(4)
        .ok_or_else(|| AtlasError::new(CODE_STUDIO_RESOURCE_LIMIT, "tile row overflowed"))?;
    let expected = row_bytes
        .checked_mul(height as usize)
        .ok_or_else(|| AtlasError::new(CODE_STUDIO_RESOURCE_LIMIT, "tile buffer overflowed"))?;
    if rgba.len() != expected {
        return Err(studio_tile_invalid(
            "RGBA length does not match tile dimensions",
        ));
    }
    let mut flipped = vec![0_u8; expected];
    for y in 0..height as usize {
        let src = y * row_bytes;
        let dst = (height as usize - 1 - y) * row_bytes;
        flipped[dst..dst + row_bytes].copy_from_slice(&rgba[src..src + row_bytes]);
    }
    Ok(flipped)
}

pub fn render_studio_tile(
    scene: &AtlasPreparedScene,
    scene_request: &AtlasStudioSceneRequestV1,
    tile: &AtlasStudioTileRequestV1,
    progress: &mut dyn AtlasProgress,
) -> Result<RenderedStudioTile, AtlasError> {
    let tile = tile.clone().normalize()?;
    let scene_request = scene_request.clone().normalize()?;
    let output = tile.output_px()?;
    let pixels = (output as usize)
        .checked_mul(output as usize)
        .and_then(|count| count.checked_mul(4))
        .ok_or_else(|| {
            AtlasError::new(CODE_STUDIO_RESOURCE_LIMIT, "studio RGBA buffer overflowed")
        })?;
    let mut request = scene_request
        .as_render_request(output)
        .and_then(|request| request.normalize())
        .map_err(map_studio_error)?;
    request.extent = xyz_extent(tile.z, tile.x, tile.y)?;
    let mut buffer = vec![0_u8; pixels];
    let total = output;
    for py in 0..output {
        progress
            .report(AtlasPhase::Rendering, py, total)
            .map_err(map_studio_error)?;
        progress.check_cancelled().map_err(map_studio_error)?;
        for px in 0..output {
            let (lon, lat) = xyz_pixel_center(tile.z, tile.x, tile.y, px, py, output)?;
            let rgba = pixel_rgba(
                &scene.model,
                &scene.hydrology,
                &scene.sdf,
                &scene.style,
                &request,
                lon,
                lat,
            );
            let offset = (py as usize * output as usize + px as usize) * 4;
            buffer[offset..offset + 4].copy_from_slice(&rgba);
        }
    }
    progress.report(AtlasPhase::Rendering, total, total)?;
    let png = crate::encode::encode_png(
        &request,
        &buffer,
        &scene.export_provenance(&request),
        progress,
    )
    .map_err(map_studio_error)?;
    Ok(RenderedStudioTile {
        z: tile.z,
        x: tile.x,
        y: tile.y,
        width: output,
        height: output,
        rgba: buffer,
        png,
    })
}

pub fn render_xyz_region(
    scene: &AtlasPreparedScene,
    scene_request: &AtlasStudioSceneRequestV1,
    z: u32,
    origin_x: u32,
    origin_y: u32,
    tiles_x: u32,
    tiles_y: u32,
    output_px: u32,
    wrap_x: bool,
    progress: &mut dyn AtlasProgress,
) -> Result<Vec<u8>, AtlasError> {
    let n = tile_count(z)?;
    let width = tiles_x
        .checked_mul(output_px)
        .ok_or_else(|| AtlasError::new(CODE_STUDIO_RESOURCE_LIMIT, "region width overflowed"))?;
    let height = tiles_y
        .checked_mul(output_px)
        .ok_or_else(|| AtlasError::new(CODE_STUDIO_RESOURCE_LIMIT, "region height overflowed"))?;
    let request = scene_request
        .clone()
        .normalize()?
        .as_render_request(output_px)
        .and_then(|request| request.normalize())
        .map_err(map_studio_error)?;
    let pixels = (width as usize)
        .checked_mul(height as usize)
        .and_then(|count| count.checked_mul(4))
        .ok_or_else(|| AtlasError::new(CODE_STUDIO_RESOURCE_LIMIT, "region RGBA overflowed"))?;
    let mut buffer = vec![0_u8; pixels];
    for py in 0..height {
        progress.check_cancelled().map_err(map_studio_error)?;
        for px in 0..width {
            let mut tile_x = origin_x
                .checked_add(px / output_px)
                .ok_or_else(|| studio_tile_invalid("region tile x overflowed"))?;
            let tile_y = origin_y
                .checked_add(py / output_px)
                .ok_or_else(|| studio_tile_invalid("region tile y overflowed"))?;
            if wrap_x {
                tile_x %= n;
            } else if tile_x >= n || tile_y >= n {
                return Err(studio_tile_invalid("region tile is out of range"));
            }
            let local_x = px % output_px;
            let local_y = py % output_px;
            let (lon, lat) = xyz_pixel_center(z, tile_x, tile_y, local_x, local_y, output_px)?;
            let rgba = pixel_rgba(
                &scene.model,
                &scene.hydrology,
                &scene.sdf,
                &scene.style,
                &request,
                lon,
                lat,
            );
            let offset = (py as usize * width as usize + px as usize) * 4;
            buffer[offset..offset + 4].copy_from_slice(&rgba);
        }
    }
    Ok(buffer)
}

impl AtlasPreparedScene {
    fn export_provenance(
        &self,
        request: &AtlasRenderRequest,
    ) -> crate::provenance::AtlasRenderProvenanceV1 {
        let mut provenance = crate::provenance::AtlasRenderProvenanceV1::for_request(
            request,
            &self.identity,
            &self.source_sha256,
            &self.style_hash,
        );
        provenance.derived_drainage_version = crate::ATLAS_DERIVED_DRAINAGE_VERSION;
        provenance.tributary_count = self.drainage.tributaries.len() as u32;
        provenance
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection::{mercator_y, ProjectedView};
    use crate::{golden_world, prepare_from_source, spike_identity_from_source, NoopProgress};

    fn prepared() -> (AtlasPreparedScene, AtlasStudioSceneRequestV1) {
        let world = golden_world();
        let identity = spike_identity_from_source(&world.source);
        let scene_request = AtlasStudioSceneRequestV1::spike().normalize().unwrap();
        let request = scene_request.as_render_request(STUDIO_TILE_SIZE).unwrap();
        let scene = prepare_from_source(
            &world.source,
            &identity,
            &request.normalize().unwrap(),
            None,
            None,
            &mut NoopProgress,
        )
        .unwrap();
        (scene, scene_request)
    }

    #[test]
    fn rejects_unknown_versions_zoom_and_coordinates() {
        let mut tile = AtlasStudioTileRequestV1::new(0, 0, 0);
        tile.schema_version = 2;
        assert_eq!(tile.normalize().unwrap_err().code, CODE_STUDIO_TILE_INVALID);
        assert_eq!(
            AtlasStudioTileRequestV1::new(STUDIO_MAX_ZOOM + 1, 0, 0)
                .normalize()
                .unwrap_err()
                .code,
            CODE_STUDIO_TILE_INVALID
        );
        assert_eq!(
            AtlasStudioTileRequestV1::new(1, 2, 0)
                .normalize()
                .unwrap_err()
                .code,
            CODE_STUDIO_TILE_INVALID
        );
        let mut scale = AtlasStudioTileRequestV1::new(0, 0, 0);
        scale.device_scale = 3;
        assert_eq!(
            scale.normalize().unwrap_err().code,
            CODE_STUDIO_TILE_INVALID
        );
        let mut scene = AtlasStudioSceneRequestV1::spike();
        scene.schema_version = 9;
        assert_eq!(
            scene.normalize().unwrap_err().code,
            CODE_STUDIO_REQUEST_INVALID
        );
        let mut size = AtlasStudioTileRequestV1::new(0, 0, 0);
        size.tile_size = 512;
        assert_eq!(size.normalize().unwrap_err().code, CODE_STUDIO_TILE_INVALID);
        let mut overflow = AtlasStudioTileRequestV1::new(0, 0, 0);
        overflow.tile_size = u32::MAX;
        overflow.device_scale = 2;
        assert_eq!(
            overflow.normalize().unwrap_err().code,
            CODE_STUDIO_RESOURCE_LIMIT
        );
        let mut empty = AtlasStudioTileRequestV1::new(0, 0, 0);
        empty.tile_size = 0;
        assert_eq!(
            empty.normalize().unwrap_err().code,
            CODE_STUDIO_RESOURCE_LIMIT
        );
    }

    #[test]
    fn xyz_y_zero_is_north_and_antimeridian_belongs_to_x_zero() {
        let (north_lon, north_lat) = xyz_pixel_center(1, 0, 0, 128, 0, 256).unwrap();
        let (_, south_lat) = xyz_pixel_center(1, 0, 0, 128, 255, 256).unwrap();
        assert!(north_lat > south_lat);
        assert!(north_lat > 80_000_000);
        assert!(south_lat.abs() < 1_000_000);
        let west = xyz_extent(1, 0, 0).unwrap();
        assert_eq!(west.west_lon_micro, -180_000_000);
        let east = xyz_extent(1, 1, 0).unwrap();
        assert_eq!(east.east_lon_micro, 180_000_000);
        let (lon_west, _) = xyz_pixel_center(1, 0, 0, 0, 128, 256).unwrap();
        let (lon_east, _) = xyz_pixel_center(1, 1, 0, 255, 128, 256).unwrap();
        assert!(lon_west < -170_000_000);
        assert!(lon_east > 170_000_000);
        assert_eq!(wrap_lon_micro(i64::from(lon_west)), lon_west);
        let _ = north_lon;
        assert!(mercator_y(WEB_MERCATOR_MAX_LAT_MICRO).is_some());
    }

    #[test]
    fn adjacent_tiles_match_concatenated_region_including_antimeridian() {
        let (scene, scene_request) = prepared();
        let left = render_studio_tile(
            &scene,
            &scene_request,
            &AtlasStudioTileRequestV1::new(1, 0, 0),
            &mut NoopProgress,
        )
        .unwrap();
        let right = render_studio_tile(
            &scene,
            &scene_request,
            &AtlasStudioTileRequestV1::new(1, 1, 0),
            &mut NoopProgress,
        )
        .unwrap();
        let joined = render_xyz_region(
            &scene,
            &scene_request,
            1,
            0,
            0,
            2,
            1,
            256,
            false,
            &mut NoopProgress,
        )
        .unwrap();
        let mut concat = Vec::with_capacity(joined.len());
        for row in 0..256usize {
            let start = row * 256 * 4;
            concat.extend_from_slice(&left.rgba[start..start + 256 * 4]);
            concat.extend_from_slice(&right.rgba[start..start + 256 * 4]);
        }
        assert_eq!(concat, joined);

        let wrap_left = render_studio_tile(
            &scene,
            &scene_request,
            &AtlasStudioTileRequestV1::new(2, 3, 1),
            &mut NoopProgress,
        )
        .unwrap();
        let wrap_right = render_studio_tile(
            &scene,
            &scene_request,
            &AtlasStudioTileRequestV1::new(2, 0, 1),
            &mut NoopProgress,
        )
        .unwrap();
        let wrapped = render_xyz_region(
            &scene,
            &scene_request,
            2,
            3,
            1,
            2,
            1,
            256,
            true,
            &mut NoopProgress,
        )
        .unwrap();
        let mut wrap_concat = Vec::new();
        for row in 0..256usize {
            let start = row * 256 * 4;
            wrap_concat.extend_from_slice(&wrap_left.rgba[start..start + 256 * 4]);
            wrap_concat.extend_from_slice(&wrap_right.rgba[start..start + 256 * 4]);
        }
        assert_eq!(wrap_concat, wrapped);
    }

    #[test]
    fn north_origin_tile_matches_flipped_export_rows() {
        let (scene, scene_request) = prepared();
        let tile = render_studio_tile(
            &scene,
            &scene_request,
            &AtlasStudioTileRequestV1::new(1, 0, 1),
            &mut NoopProgress,
        )
        .unwrap();
        let mut export_request = scene_request.as_render_request(256).unwrap();
        export_request.extent = xyz_extent(1, 0, 1).unwrap();
        export_request = export_request.normalize().unwrap();
        let view =
            ProjectedView::new(export_request.projection, export_request.extent, 256, 256).unwrap();
        let mut south_origin = vec![0_u8; 256 * 256 * 4];
        for y in 0..256u32 {
            for x in 0..256u32 {
                let (lon, lat) = view.pixel_center(x, y);
                let rgba = pixel_rgba(
                    &scene.model,
                    &scene.hydrology,
                    &scene.sdf,
                    &scene.style,
                    &export_request,
                    lon,
                    lat,
                );
                let offset = (y as usize * 256 + x as usize) * 4;
                south_origin[offset..offset + 4].copy_from_slice(&rgba);
            }
        }
        let flipped = flip_rgba_vertical(&south_origin, 256, 256).unwrap();
        assert_eq!(tile.rgba, flipped);
        assert_eq!(STUDIO_TILE_HALO, 0);
        assert_eq!(tile.png[0..8], [137, 80, 78, 71, 13, 10, 26, 10]);
    }

    #[test]
    fn repeated_shuffled_and_parallel_tiles_match() {
        let (scene, scene_request) = prepared();
        let request = AtlasStudioTileRequestV1::new(2, 1, 2);
        let first =
            render_studio_tile(&scene, &scene_request, &request, &mut NoopProgress).unwrap();
        let second =
            render_studio_tile(&scene, &scene_request, &request, &mut NoopProgress).unwrap();
        assert_eq!(first.png, second.png);
        let mut scaled = request.clone();
        scaled.device_scale = 2;
        let hi = render_studio_tile(&scene, &scene_request, &scaled, &mut NoopProgress).unwrap();
        let (lon_a, lat_a) = xyz_pixel_center(2, 1, 2, 10, 20, 256).unwrap();
        let sample_a = pixel_rgba(
            &scene.model,
            &scene.hydrology,
            &scene.sdf,
            &scene.style,
            &scene_request
                .as_render_request(256)
                .unwrap()
                .normalize()
                .unwrap(),
            lon_a,
            lat_a,
        );
        let sample_b = pixel_rgba(
            &scene.model,
            &scene.hydrology,
            &scene.sdf,
            &scene.style,
            &scene_request
                .as_render_request(512)
                .unwrap()
                .normalize()
                .unwrap(),
            lon_a,
            lat_a,
        );
        assert_eq!(sample_a, sample_b);
        assert_eq!(hi.width, 512);
        assert_eq!(hi.rgba.len(), 512 * 512 * 4);

        let scene = std::sync::Arc::new(scene);
        let scene_request = std::sync::Arc::new(scene_request);
        let handles: Vec<_> = (0..4)
            .map(|seed| {
                let scene = scene.clone();
                let scene_request = scene_request.clone();
                std::thread::spawn(move || {
                    let order = [2u32, 0, 3, 1];
                    let tile = AtlasStudioTileRequestV1::new(2, order[seed % 4], 1);
                    render_studio_tile(&scene, &scene_request, &tile, &mut NoopProgress)
                        .unwrap()
                        .png
                })
            })
            .collect();
        let parallel: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();
        let serial = [
            render_studio_tile(
                &scene,
                &scene_request,
                &AtlasStudioTileRequestV1::new(2, 2, 1),
                &mut NoopProgress,
            )
            .unwrap()
            .png,
            render_studio_tile(
                &scene,
                &scene_request,
                &AtlasStudioTileRequestV1::new(2, 0, 1),
                &mut NoopProgress,
            )
            .unwrap()
            .png,
            render_studio_tile(
                &scene,
                &scene_request,
                &AtlasStudioTileRequestV1::new(2, 3, 1),
                &mut NoopProgress,
            )
            .unwrap()
            .png,
            render_studio_tile(
                &scene,
                &scene_request,
                &AtlasStudioTileRequestV1::new(2, 1, 1),
                &mut NoopProgress,
            )
            .unwrap()
            .png,
        ];
        assert_eq!(parallel, serial);
    }

    #[test]
    fn cancellation_stops_a_tile() {
        let (scene, scene_request) = prepared();
        let flag = crate::CancelFlag::default();
        flag.cancel();
        let error = render_studio_tile(
            &scene,
            &scene_request,
            &AtlasStudioTileRequestV1::new(0, 0, 0),
            &mut crate::FlagProgress { flag: &flag },
        )
        .unwrap_err();
        assert_eq!(error.code, CODE_STUDIO_CANCELLED);
    }

    #[test]
    fn residual_and_drainage_cache_hits_keep_tile_bytes() {
        let world = golden_world();
        let identity = spike_identity_from_source(&world.source);
        let scene_request = AtlasStudioSceneRequestV1::spike().normalize().unwrap();
        let request = scene_request
            .as_render_request(STUDIO_TILE_SIZE)
            .unwrap()
            .normalize()
            .unwrap();
        let root = std::env::temp_dir().join(format!(
            "daena-atlas-studio-cache-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let cache = crate::cache::AtlasDiskCache::open(&root).unwrap();
        let cold = prepare_from_source(
            &world.source,
            &identity,
            &request,
            None,
            Some(&cache),
            &mut NoopProgress,
        )
        .unwrap();
        assert_eq!(cold.residual_cache, crate::cache::CacheLookup::Miss);
        assert_eq!(cold.drainage_cache, crate::cache::CacheLookup::Miss);
        let tile = AtlasStudioTileRequestV1::new(3, 2, 1);
        let cold_tile =
            render_studio_tile(&cold, &scene_request, &tile, &mut NoopProgress).unwrap();
        let warm = prepare_from_source(
            &world.source,
            &identity,
            &request,
            None,
            Some(&cache),
            &mut NoopProgress,
        )
        .unwrap();
        assert_eq!(warm.residual_cache, crate::cache::CacheLookup::Hit);
        assert_eq!(warm.drainage_cache, crate::cache::CacheLookup::Hit);
        let warm_tile =
            render_studio_tile(&warm, &scene_request, &tile, &mut NoopProgress).unwrap();
        assert_eq!(cold_tile.png, warm_tile.png);
        let cache_bytes: u64 = std::fs::read_dir(&root)
            .unwrap()
            .filter_map(|entry| entry.ok()?.metadata().ok())
            .map(|meta| meta.len())
            .sum();
        assert!(cache_bytes > 0);
        let _ = std::fs::remove_dir_all(root);
    }
}
