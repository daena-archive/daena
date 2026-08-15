//! Whole-world equirectangular addressing.

pub const LON_MICRO_MIN: i32 = -180_000_000;
pub const LON_MICRO_SPAN: i64 = 360_000_000;
pub const LAT_MICRO_MIN: i32 = -90_000_000;
pub const LAT_MICRO_SPAN: i64 = 180_000_000;

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
    wrap_lon_micro(
        i64::from(LON_MICRO_MIN)
            + (LON_MICRO_SPAN * (i64::from(x).saturating_mul(2) + 1)) / (i64::from(width) * 2),
    )
}

pub fn pixel_center_lat_micro(y: u32, height: u32) -> i32 {
    clamp_lat_micro(
        i64::from(LAT_MICRO_MIN)
            + (LAT_MICRO_SPAN * (i64::from(y).saturating_mul(2) + 1)) / (i64::from(height) * 2),
    )
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
}
