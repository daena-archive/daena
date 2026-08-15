//! Equirectangular and Web Mercator addressing, extents, and clipping.

use serde::{Deserialize, Serialize};

use crate::AtlasError;

pub const LON_MICRO_MIN: i32 = -180_000_000;
pub const LON_MICRO_SPAN: i64 = 360_000_000;
pub const LAT_MICRO_MIN: i32 = -90_000_000;
pub const LAT_MICRO_SPAN: i64 = 180_000_000;
pub const WEB_MERCATOR_MAX_LAT_MICRO: i32 = 85_051_129;
pub const EQUIRECTANGULAR_ID: &str = "equirectangular";
pub const WEB_MERCATOR_ID: &str = "web-mercator";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AtlasProjection {
    Equirectangular,
    WebMercator,
}

impl AtlasProjection {
    pub fn parse(value: &str) -> Result<Self, AtlasError> {
        match value {
            EQUIRECTANGULAR_ID => Ok(Self::Equirectangular),
            WEB_MERCATOR_ID => Ok(Self::WebMercator),
            other => Err(AtlasError::invalid(format!(
                "unsupported atlas projection: {other}"
            ))),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Equirectangular => EQUIRECTANGULAR_ID,
            Self::WebMercator => WEB_MERCATOR_ID,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlasExtent {
    pub west_lon_micro: i32,
    pub south_lat_micro: i32,
    pub east_lon_micro: i32,
    pub north_lat_micro: i32,
}

impl Default for AtlasExtent {
    fn default() -> Self {
        Self::world()
    }
}

impl AtlasExtent {
    pub const fn world() -> Self {
        Self {
            west_lon_micro: LON_MICRO_MIN,
            south_lat_micro: LAT_MICRO_MIN,
            east_lon_micro: 180_000_000,
            north_lat_micro: 90_000_000,
        }
    }

    pub fn is_world(self) -> bool {
        self.west_lon_micro == LON_MICRO_MIN
            && self.east_lon_micro == 180_000_000
            && self.south_lat_micro == LAT_MICRO_MIN
            && self.north_lat_micro == 90_000_000
    }

    pub fn validate(self, projection: AtlasProjection) -> Result<(), AtlasError> {
        if self.north_lat_micro <= self.south_lat_micro {
            return Err(AtlasError::invalid(
                "atlas extent latitude range is empty or inverted",
            ));
        }
        if !(LAT_MICRO_MIN..=90_000_000).contains(&self.south_lat_micro)
            || !(LAT_MICRO_MIN..=90_000_000).contains(&self.north_lat_micro)
        {
            return Err(AtlasError::invalid("atlas extent latitude is out of range"));
        }
        let _ = wrap_lon_micro(i64::from(self.west_lon_micro));
        if self.east_lon_micro != 180_000_000 {
            let _ = wrap_lon_micro(i64::from(self.east_lon_micro));
        }
        if self.lon_span_micro() <= 0 {
            return Err(AtlasError::invalid("atlas extent longitude range is empty"));
        }
        if projection == AtlasProjection::WebMercator {
            if self.south_lat_micro < -WEB_MERCATOR_MAX_LAT_MICRO
                || self.north_lat_micro > WEB_MERCATOR_MAX_LAT_MICRO
            {
                return Err(AtlasError::invalid(
                    "web-mercator extent exceeds ±85.051129°",
                ));
            }
            if self.is_world() {
                return Err(AtlasError::invalid(
                    "web-mercator requires a regional extent inside its latitude limit",
                ));
            }
        }
        Ok(())
    }

    pub fn lon_span_micro(self) -> i64 {
        let west = wrap_lon_micro(i64::from(self.west_lon_micro));
        if self.east_lon_micro == 180_000_000 {
            return i64::from(180_000_000) - i64::from(west);
        }
        let east = wrap_lon_micro(i64::from(self.east_lon_micro));
        let mut span = i64::from(east) - i64::from(west);
        if span <= 0 {
            span += LON_MICRO_SPAN;
        }
        span
    }

    pub fn lat_span_micro(self) -> i64 {
        i64::from(self.north_lat_micro) - i64::from(self.south_lat_micro)
    }

    pub fn central_meridian_micro(self) -> i32 {
        wrap_lon_micro(
            i64::from(wrap_lon_micro(i64::from(self.west_lon_micro))) + self.lon_span_micro() / 2,
        )
    }

    pub fn contains_lon(self, lon_micro: i32) -> bool {
        let lon = wrap_lon_micro(i64::from(lon_micro));
        let west = wrap_lon_micro(i64::from(self.west_lon_micro));
        if self.east_lon_micro == 180_000_000 && west == LON_MICRO_MIN {
            return true;
        }
        let offset = (i64::from(lon) - i64::from(west)).rem_euclid(LON_MICRO_SPAN);
        offset < self.lon_span_micro()
    }

    pub fn contains_lat(self, lat_micro: i32) -> bool {
        let lat = clamp_lat_micro(i64::from(lat_micro));
        (lat >= self.south_lat_micro && lat < self.north_lat_micro)
            || (lat == self.north_lat_micro && lat == 90_000_000)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectedView {
    pub projection: AtlasProjection,
    pub extent: AtlasExtent,
    pub width: u32,
    pub height: u32,
}

impl ProjectedView {
    pub fn new(
        projection: AtlasProjection,
        extent: AtlasExtent,
        width: u32,
        height: u32,
    ) -> Result<Self, AtlasError> {
        extent.validate(projection)?;
        if width == 0 || height == 0 {
            return Err(AtlasError::invalid("output dimensions must be positive"));
        }
        Ok(Self {
            projection,
            extent,
            width,
            height,
        })
    }

    pub fn pixel_center(self, x: u32, y: u32) -> (i32, i32) {
        match self.projection {
            AtlasProjection::Equirectangular => (
                self.equirect_lon(x, self.width),
                self.equirect_lat(y, self.height),
            ),
            AtlasProjection::WebMercator => self.mercator_pixel_center(x, y),
        }
    }

    pub fn project(self, lon_micro: i32, lat_micro: i32) -> Option<(i32, i32)> {
        if !self.extent.contains_lon(lon_micro) || !self.extent.contains_lat(lat_micro) {
            return None;
        }
        match self.projection {
            AtlasProjection::Equirectangular => {
                let west = wrap_lon_micro(i64::from(self.extent.west_lon_micro));
                let offset = (i64::from(wrap_lon_micro(i64::from(lon_micro))) - i64::from(west))
                    .rem_euclid(LON_MICRO_SPAN);
                let x = (offset * i64::from(self.width) / self.extent.lon_span_micro()) as i32;
                let y = ((i64::from(clamp_lat_micro(i64::from(lat_micro)))
                    - i64::from(self.extent.south_lat_micro))
                    * i64::from(self.height)
                    / self.extent.lat_span_micro()) as i32;
                Some((x, y))
            }
            AtlasProjection::WebMercator => {
                let (x0, y0, x_span, y_span) = self.mercator_window();
                let xw = mercator_x(lon_micro, self.extent.central_meridian_micro());
                let yw = mercator_y(lat_micro)?;
                let x = ((xw - x0) * f64::from(self.width) / x_span).round() as i32;
                let y = ((yw - y0) * f64::from(self.height) / y_span).round() as i32;
                Some((x, y))
            }
        }
    }

    pub fn aspect_num_den(self) -> (u128, u128) {
        match self.projection {
            AtlasProjection::Equirectangular => (
                self.extent.lon_span_micro() as u128,
                self.extent.lat_span_micro() as u128,
            ),
            AtlasProjection::WebMercator => {
                let (_, _, x_span, y_span) = self.mercator_window();
                let x = (x_span * 1_000_000_000.0).round().max(1.0) as u128;
                let y = (y_span * 1_000_000_000.0).round().max(1.0) as u128;
                (x, y)
            }
        }
    }

    fn equirect_lon(self, x: u32, width: u32) -> i32 {
        let west = wrap_lon_micro(i64::from(self.extent.west_lon_micro));
        wrap_lon_micro(
            i64::from(west)
                + (self.extent.lon_span_micro() * (i64::from(x).saturating_mul(2) + 1))
                    / (i64::from(width) * 2),
        )
    }

    fn equirect_lat(self, y: u32, height: u32) -> i32 {
        clamp_lat_micro(
            i64::from(self.extent.south_lat_micro)
                + (self.extent.lat_span_micro() * (i64::from(y).saturating_mul(2) + 1))
                    / (i64::from(height) * 2),
        )
    }

    fn mercator_window(self) -> (f64, f64, f64, f64) {
        let meridian = self.extent.central_meridian_micro();
        let x0 = mercator_x(self.extent.west_lon_micro, meridian);
        let x1 = if self.extent.east_lon_micro == 180_000_000 {
            mercator_x(179_999_999, meridian)
        } else {
            mercator_x(self.extent.east_lon_micro, meridian)
        };
        let y0 = mercator_y(self.extent.south_lat_micro).unwrap_or(0.0);
        let y1 = mercator_y(self.extent.north_lat_micro).unwrap_or(0.0);
        (
            x0,
            y0,
            (x1 - x0).abs().max(f64::EPSILON),
            (y1 - y0).abs().max(f64::EPSILON),
        )
    }

    fn mercator_pixel_center(self, x: u32, y: u32) -> (i32, i32) {
        let (x0, y0, x_span, y_span) = self.mercator_window();
        let mx = x0 + x_span * (f64::from(x) + 0.5) / f64::from(self.width);
        let my = y0 + y_span * (f64::from(y) + 0.5) / f64::from(self.height);
        (
            mercator_x_to_lon_micro(mx, self.extent.central_meridian_micro()),
            mercator_y_to_lat_micro(my),
        )
    }
}

pub fn wrap_lon_micro(value: i64) -> i32 {
    let span = LON_MICRO_SPAN;
    let mut wrapped = (value - i64::from(LON_MICRO_MIN)).rem_euclid(span);
    if wrapped == span {
        wrapped = 0;
    }
    (wrapped + i64::from(LON_MICRO_MIN)) as i32
}

pub fn clamp_lat_micro(value: i64) -> i32 {
    value.clamp(i64::from(LAT_MICRO_MIN), 90_000_000) as i32
}

pub fn pixel_center_lon_micro(x: u32, width: u32) -> i32 {
    ProjectedView {
        projection: AtlasProjection::Equirectangular,
        extent: AtlasExtent::world(),
        width,
        height: 1,
    }
    .equirect_lon(x, width)
}

pub fn pixel_center_lat_micro(y: u32, height: u32) -> i32 {
    ProjectedView {
        projection: AtlasProjection::Equirectangular,
        extent: AtlasExtent::world(),
        width: 1,
        height,
    }
    .equirect_lat(y, height)
}

pub fn lon_to_column_ppm(lon_micro: i32, width: u32) -> (u32, u32, u32) {
    let width = width.max(1);
    let span = i64::from(width) * 1_000_000;
    let position = (i64::from(wrap_lon_micro(i64::from(lon_micro))) - i64::from(LON_MICRO_MIN))
        * span
        / LON_MICRO_SPAN;
    let col = ((position / 1_000_000) as u32) % width;
    let frac = (position.rem_euclid(1_000_000)) as u32;
    let next = (col + 1) % width;
    (col, next, frac)
}

pub fn lat_to_row_ppm(lat_micro: i32, height: u32) -> (u32, u32, u32) {
    let height = height.max(1);
    let last = height - 1;
    let position = (i64::from(clamp_lat_micro(i64::from(lat_micro))) - i64::from(LAT_MICRO_MIN))
        * i64::from(height)
        * 1_000_000
        / LAT_MICRO_SPAN;
    let row = ((position / 1_000_000) as u32).min(last);
    let frac = if row == last {
        0
    } else {
        (position.rem_euclid(1_000_000)) as u32
    };
    let next = (row + 1).min(last);
    (row, next, frac)
}

pub fn bilinear_i32(c00: i32, c10: i32, c01: i32, c11: i32, frac_x: u32, frac_y: u32) -> i32 {
    let fx = i128::from(frac_x.min(1_000_000));
    let fy = i128::from(frac_y.min(1_000_000));
    let ix = 1_000_000 - fx;
    let iy = 1_000_000 - fy;
    let top = i128::from(c00) * ix + i128::from(c10) * fx;
    let bottom = i128::from(c01) * ix + i128::from(c11) * fx;
    ((top * iy + bottom * fy) / 1_000_000 / 1_000_000) as i32
}

pub(crate) fn mercator_x(lon_micro: i32, central_meridian_micro: i32) -> f64 {
    let dlon = wrap_lon_micro(i64::from(lon_micro) - i64::from(central_meridian_micro));
    std::f64::consts::PI * f64::from(dlon) / 180_000_000.0
}

pub(crate) fn mercator_y(lat_micro: i32) -> Option<f64> {
    if lat_micro.abs() > WEB_MERCATOR_MAX_LAT_MICRO {
        return None;
    }
    let lat = (f64::from(lat_micro) / 1_000_000.0).to_radians();
    Some((std::f64::consts::FRAC_PI_4 + lat / 2.0).tan().ln())
}

pub(crate) fn mercator_x_to_lon_micro(x: f64, central_meridian_micro: i32) -> i32 {
    let dlon = (x / std::f64::consts::PI * 180_000_000.0).round() as i64;
    wrap_lon_micro(dlon + i64::from(central_meridian_micro))
}

pub(crate) fn mercator_y_to_lat_micro(y: f64) -> i32 {
    let lat = y.tanh().asin().to_degrees() * 1_000_000.0;
    clamp_lat_micro(lat.round() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn longitude_wraps_and_poles_clamp() {
        assert_eq!(wrap_lon_micro(180_000_000), -180_000_000);
        assert_eq!(wrap_lon_micro(-180_000_001), 179_999_999);
        assert_eq!(clamp_lat_micro(100_000_000), 90_000_000);
        assert_eq!(clamp_lat_micro(-100_000_000), -90_000_000);
        let (col, next, _) = lon_to_column_ppm(179_999_000, 8);
        assert_eq!(next, (col + 1) % 8);
    }

    #[test]
    fn world_equirectangular_pixel_centers_match_iteration_1() {
        assert_eq!(
            pixel_center_lon_micro(0, 64),
            ProjectedView::new(
                AtlasProjection::Equirectangular,
                AtlasExtent::world(),
                64,
                32
            )
            .unwrap()
            .pixel_center(0, 0)
            .0
        );
        assert_eq!(
            pixel_center_lat_micro(0, 32),
            LAT_MICRO_MIN + (LAT_MICRO_SPAN / 64) as i32
        );
    }

    #[test]
    fn web_mercator_round_trips_within_one_microdegree() {
        let fixtures = [
            (0, 0),
            (10_000_000, 10_000_000),
            (-45_000_000, 30_000_000),
            (170_000_000, -20_000_000),
            (0, WEB_MERCATOR_MAX_LAT_MICRO),
        ];
        for (lon, lat) in fixtures {
            let x = mercator_x(lon, 0);
            let y = mercator_y(lat).unwrap();
            assert!((mercator_x_to_lon_micro(x, 0) - lon).abs() <= 1);
            assert!((mercator_y_to_lat_micro(y) - lat).abs() <= 1);
        }
        assert!(mercator_y(90_000_000).is_none());
        assert!(AtlasExtent::world()
            .validate(AtlasProjection::WebMercator)
            .is_err());
    }

    #[test]
    fn antimeridian_extent_is_not_empty() {
        let extent = AtlasExtent {
            west_lon_micro: 170_000_000,
            south_lat_micro: -10_000_000,
            east_lon_micro: -170_000_000,
            north_lat_micro: 10_000_000,
        };
        extent.validate(AtlasProjection::Equirectangular).unwrap();
        assert_eq!(extent.lon_span_micro(), 20_000_000);
        assert!(extent.contains_lon(179_000_000));
        assert!(extent.contains_lon(-179_000_000));
        assert!(!extent.contains_lon(0));
    }
}
