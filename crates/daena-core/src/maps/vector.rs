use crate::error::CoreError;
use serde::de::{self, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fmt;
use std::path::Path;
use uuid::Uuid;

pub const VECTOR_SOURCE_FORMAT: &str = "daena-geojson";
pub const VECTOR_MIME: &str = "application/geo+json";
pub const VECTOR_FILENAME: &str = "map.geojson";
pub const VECTOR_MAX_BYTES: usize = 16 * 1024 * 1024;
pub const VECTOR_MAX_FEATURES: usize = 20_000;
pub const VECTOR_MAX_POSITIONS: usize = 200_000;
pub const VECTOR_MAX_FEATURE_POSITIONS: usize = 20_000;
pub const VECTOR_MAX_RINGS: usize = 256;
pub const VECTOR_MAX_LAYERS: usize = 64;
pub const VECTOR_MAX_PROPERTY_BYTES: usize = 2 * 1024;
pub const VECTOR_MAX_CUSTOM_KEYS: usize = 32;
pub const VECTOR_MAX_LABEL_TEXT: usize = 256;
pub const VECTOR_CENTER_Y_MIN: f64 = 0.0;
pub const VECTOR_CENTER_Y_MAX: f64 = 1.0;
const SCALE: i64 = 1_000_000;
const LAT_LIMIT: i64 = 90_000_000;
const LON_LIMIT: i64 = 180_000_000;
const ANTIMERIDIAN: i64 = 180_000_000;

pub const CODE_SOURCE_INVALID: &str = "vector.source.invalid";
pub const CODE_UNSUPPORTED_VERSION: &str = "vector.source.unsupported-version";
pub const CODE_GEOMETRY_INVALID: &str = "vector.geometry.invalid";
pub const CODE_ANTIMERIDIAN: &str = "vector.geometry.antimeridian";
pub const CODE_LIMIT: &str = "vector.limit.exceeded";
pub const CODE_LAYER_MISSING: &str = "vector.layer.missing";
pub const CODE_GENERATOR: &str = "vector.generator.invalid-settings";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Micro(pub i64, pub i64);

#[derive(Debug, Clone, PartialEq, Eq)]
enum Geometry {
    Point(Micro),
    MultiPoint(Vec<Micro>),
    LineString(Vec<Micro>),
    MultiLineString(Vec<Vec<Micro>>),
    Polygon {
        exterior: Vec<Micro>,
        holes: Vec<Vec<Micro>>,
    },
    MultiPolygon(Vec<(Vec<Micro>, Vec<Vec<Micro>>)>),
}

#[derive(Debug, Clone, PartialEq)]
struct Feature {
    id: String,
    layer_id: String,
    semantic_type: String,
    name: Option<String>,
    style: Option<Value>,
    label: Option<Value>,
    custom: serde_json::Map<String, Value>,
    geometry: Geometry,
}

pub type FeatureBounds = (String, String, String, f64, f64, f64, f64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceProfile {
    Candidate,
    Committed,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum VectorSpace {
    #[default]
    Geographic,
    Planar {
        extent: [f64; 4],
        is_image: bool,
    },
}

impl VectorSpace {
    pub fn from_coordinate_space(space: &crate::maps::MapCoordinateSpace) -> Self {
        match space {
            crate::maps::MapCoordinateSpace::Geographic { extent, .. } => {
                // Geographic is still geographic even if custom extent, but we treat
                // any geographic as Geographic to preserve antimeridian handling.
                let _ = extent;
                Self::Geographic
            }
            crate::maps::MapCoordinateSpace::Image { extent, .. } => Self::Planar {
                extent: *extent,
                is_image: true,
            },
            crate::maps::MapCoordinateSpace::World { extent, .. } => {
                // Default world extent [-180,-90,180,90] behaves like Geographic for backwards
                // compatibility and to preserve the specific longitude error messages.
                const WORLD_EXTENT: [f64; 4] = [-180.0, -90.0, 180.0, 90.0];
                if *extent == WORLD_EXTENT {
                    Self::Geographic
                } else {
                    Self::Planar {
                        extent: *extent,
                        is_image: false,
                    }
                }
            }
        }
    }

    pub fn from_descriptor_value(value: &Value) -> Self {
        if let Some(space_value) = value.get("coordinateSpace") {
            if let Ok(space) =
                serde_json::from_value::<crate::maps::MapCoordinateSpace>(space_value.clone())
            {
                return Self::from_coordinate_space(&space);
            }
        }
        // Fallback: infer from provider? For backwards compat, treat missing as Geographic.
        Self::Geographic
    }

    pub fn is_geographic(&self) -> bool {
        matches!(self, Self::Geographic)
    }
}

#[must_use]
pub fn lon_lat_to_normalized(longitude: f64, latitude: f64) -> (f64, f64) {
    ((longitude + 180.0) / 360.0, (90.0 - latitude) / 180.0)
}

#[must_use]
pub fn planar_to_normalized(x: f64, y: f64, extent: &[f64; 4], is_image: bool) -> (f64, f64) {
    let [min_x, min_y, max_x, max_y] = *extent;
    let width = (max_x - min_x).max(f64::EPSILON);
    let height = (max_y - min_y).max(f64::EPSILON);
    let nx = (x - min_x) / width;
    let ny = if is_image {
        (y - min_y) / height
    } else {
        (max_y - y) / height
    };
    (nx.clamp(0.0, 1.0), ny.clamp(0.0, 1.0))
}

pub fn fail(code: &str, path: &str, detail: impl fmt::Display) -> CoreError {
    if path.is_empty() {
        CoreError::Validation(format!("{code}: {detail}"))
    } else {
        CoreError::Validation(format!("{code}: {path}: {detail}"))
    }
}

pub fn path_fail(
    fs_path: &Path,
    code: &str,
    json_path: &str,
    detail: impl fmt::Display,
) -> CoreError {
    CoreError::Validation(format!(
        "{} [{code}] {json_path}: {detail}",
        fs_path.display()
    ))
}

fn to_micro(value: f64) -> i64 {
    let scaled = value * (SCALE as f64);
    let rounded = scaled.round();
    if rounded == 0.0 {
        0
    } else {
        rounded as i64
    }
}

fn format_micro(value: i64) -> String {
    let sign = if value < 0 { "-" } else { "" };
    let abs = value.unsigned_abs();
    let whole = abs / 1_000_000;
    let frac = abs % 1_000_000;
    if frac == 0 {
        format!("{sign}{whole}")
    } else {
        let mut digits = format!("{frac:06}");
        while digits.ends_with('0') {
            digits.pop();
        }
        format!("{sign}{whole}.{digits}")
    }
}

pub fn parse_strict_json(bytes: &[u8]) -> Result<Value, CoreError> {
    if bytes.len() > VECTOR_MAX_BYTES {
        return Err(fail(CODE_LIMIT, "$", "source asset exceeds 16 MiB"));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| fail(CODE_SOURCE_INVALID, "$", "source is not valid UTF-8"))?;
    let mut deserializer = serde_json::Deserializer::from_str(text);
    let value = StrictValue
        .deserialize(&mut deserializer)
        .map_err(|error| fail(CODE_SOURCE_INVALID, "$", error))?;
    deserializer
        .end()
        .map_err(|error| fail(CODE_SOURCE_INVALID, "$", error))?;
    Ok(value)
}

struct StrictValue;

impl<'de> de::DeserializeSeed<'de> for StrictValue {
    type Value = Value;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        deserializer.deserialize_any(StrictVisitor)
    }
}

struct StrictVisitor;

impl<'de> Visitor<'de> for StrictVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Value, E> {
        if !value.is_finite() {
            return Err(de::Error::custom("non-finite numbers are not allowed"));
        }
        Ok(Value::Number(
            serde_json::Number::from_f64(value)
                .ok_or_else(|| de::Error::custom("non-finite numbers are not allowed"))?,
        ))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Value, E> {
        Ok(Value::String(value.to_owned()))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Value, E> {
        Ok(Value::String(value))
    }

    fn visit_none<E: de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_unit<E: de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = seq.next_element_seed(StrictValue)? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let mut object = serde_json::Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if object.contains_key(&key) {
                return Err(de::Error::custom(format!("duplicate object key `{key}`")));
            }
            object.insert(key, map.next_value_seed(StrictValue)?);
        }
        Ok(Value::Object(object))
    }
}

fn object_keys<'a>(
    value: &'a Value,
    path: &str,
    allowed: &[&str],
) -> Result<&'a serde_json::Map<String, Value>, CoreError> {
    let object = value
        .as_object()
        .ok_or_else(|| fail(CODE_SOURCE_INVALID, path, "expected an object"))?;
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(fail(
                CODE_SOURCE_INVALID,
                path,
                format!("unknown member `{key}`"),
            ));
        }
    }
    Ok(object)
}

fn require_type(
    object: &serde_json::Map<String, Value>,
    path: &str,
    expected: &str,
) -> Result<(), CoreError> {
    match object.get("type").and_then(Value::as_str) {
        Some(found) if found == expected => Ok(()),
        Some(_) => Err(fail(
            CODE_SOURCE_INVALID,
            &format!("{path}.type"),
            format!("must be {expected}"),
        )),
        None => Err(fail(
            CODE_SOURCE_INVALID,
            &format!("{path}.type"),
            "is required",
        )),
    }
}

fn as_f64(value: &Value, path: &str) -> Result<f64, CoreError> {
    value.as_f64().ok_or_else(|| {
        fail(
            CODE_SOURCE_INVALID,
            path,
            "coordinates must be finite numbers",
        )
    })
}

fn parse_position(value: &Value, path: &str, space: &VectorSpace) -> Result<Micro, CoreError> {
    let pair = value
        .as_array()
        .ok_or_else(|| fail(CODE_GEOMETRY_INVALID, path, "position must be an array"))?;
    if pair.len() != 2 {
        return Err(fail(
            CODE_GEOMETRY_INVALID,
            path,
            "position must be [longitude, latitude]",
        ));
    }
    let x = as_f64(&pair[0], &format!("{path}[0]"))?;
    let y = as_f64(&pair[1], &format!("{path}[1]"))?;
    let coord = Micro(to_micro(x), to_micro(y));
    match space {
        VectorSpace::Geographic => {
            if coord.0.abs() > LON_LIMIT {
                return Err(fail(
                    CODE_GEOMETRY_INVALID,
                    path,
                    "longitude must be in [-180, 180]",
                ));
            }
            if coord.1.abs() > LAT_LIMIT {
                return Err(fail(
                    CODE_GEOMETRY_INVALID,
                    path,
                    "latitude is outside the Daena world extent",
                ));
            }
        }
        VectorSpace::Planar {
            extent,
            is_image: _,
        } => {
            // For planar spaces (image and calibrated world), coordinates are in map units
            // (pixels or world units). Validate they are finite and within the declared extent
            // with a tiny epsilon to tolerate floating-point rounding during editing.
            let [min_x, min_y, max_x, max_y] = *extent;
            let width = max_x - min_x;
            let height = max_y - min_y;
            if width > 0.0 && height > 0.0 {
                const PLANAR_ABS_LIMIT: f64 = 10_000_000.0;
                if x.abs() > PLANAR_ABS_LIMIT || y.abs() > PLANAR_ABS_LIMIT {
                    return Err(fail(
                        CODE_GEOMETRY_INVALID,
                        path,
                        "coordinates are outside the supported planar range",
                    ));
                }
                // Small epsilon relative to extent size to allow rounding; no generous margin
                // so that features clearly outside the map are rejected.
                let eps_x = (width * 1e-9).max(1e-6);
                let eps_y = (height * 1e-9).max(1e-6);
                if x < min_x - eps_x || x > max_x + eps_x || y < min_y - eps_y || y > max_y + eps_y
                {
                    return Err(fail(
                        CODE_GEOMETRY_INVALID,
                        path,
                        "coordinates are outside the map extent",
                    ));
                }
            }
        }
    }
    Ok(coord)
}

fn parse_line(values: &[Value], path: &str, space: &VectorSpace) -> Result<Vec<Micro>, CoreError> {
    if values.len() < 2 {
        return Err(fail(
            CODE_GEOMETRY_INVALID,
            path,
            "LineString requires at least two positions",
        ));
    }
    values
        .iter()
        .enumerate()
        .map(|(index, value)| parse_position(value, &format!("{path}[{index}]"), space))
        .collect()
}

fn parse_ring(values: &[Value], path: &str, space: &VectorSpace) -> Result<Vec<Micro>, CoreError> {
    if values.len() < 4 {
        return Err(fail(
            CODE_GEOMETRY_INVALID,
            path,
            "polygon rings must contain at least four positions",
        ));
    }
    let ring = values
        .iter()
        .enumerate()
        .map(|(index, value)| parse_position(value, &format!("{path}[{index}]"), space))
        .collect::<Result<Vec<_>, _>>()?;
    if ring.first() != ring.last() {
        return Err(fail(
            CODE_GEOMETRY_INVALID,
            path,
            "polygon rings must be closed",
        ));
    }
    Ok(ring)
}

fn dedup_adjacent(mut positions: Vec<Micro>) -> Vec<Micro> {
    positions.dedup();
    positions
}

fn close_ring(mut ring: Vec<Micro>) -> Vec<Micro> {
    if ring.first() != ring.last() {
        if let Some(first) = ring.first().copied() {
            ring.push(first);
        }
    }
    ring
}

fn signed_area(ring: &[Micro]) -> i128 {
    if ring.len() < 4 {
        return 0;
    }
    let mut area = 0_i128;
    for window in ring.windows(2) {
        let a = window[0];
        let b = window[1];
        area += i128::from(a.0) * i128::from(b.1) - i128::from(b.0) * i128::from(a.1);
    }
    area
}

fn orient(a: Micro, b: Micro, c: Micro) -> i128 {
    (i128::from(b.0) - i128::from(a.0)) * (i128::from(c.1) - i128::from(a.1))
        - (i128::from(b.1) - i128::from(a.1)) * (i128::from(c.0) - i128::from(a.0))
}

fn on_segment(a: Micro, b: Micro, c: Micro) -> bool {
    c.0 >= a.0.min(b.0) && c.0 <= a.0.max(b.0) && c.1 >= a.1.min(b.1) && c.1 <= a.1.max(b.1)
}

fn segments_intersect(a: Micro, b: Micro, c: Micro, d: Micro) -> bool {
    if a == c || a == d || b == c || b == d {
        return false;
    }
    let o1 = orient(a, b, c).signum();
    let o2 = orient(a, b, d).signum();
    let o3 = orient(c, d, a).signum();
    let o4 = orient(c, d, b).signum();
    if o1 != o2 && o3 != o4 {
        return true;
    }
    (o1 == 0 && on_segment(a, b, c))
        || (o2 == 0 && on_segment(a, b, d))
        || (o3 == 0 && on_segment(c, d, a))
        || (o4 == 0 && on_segment(c, d, b))
}

fn crosses_antimeridian(a: Micro, b: Micro) -> bool {
    (a.0 - b.0).abs() > ANTIMERIDIAN
}

fn validate_line(line: &[Micro], path: &str, space: &VectorSpace) -> Result<(), CoreError> {
    if line.len() < 2 {
        return Err(fail(
            CODE_GEOMETRY_INVALID,
            path,
            "line requires at least two distinct positions",
        ));
    }
    if space.is_geographic() {
        for pair in line.windows(2) {
            if crosses_antimeridian(pair[0], pair[1]) {
                return Err(fail(
                    CODE_ANTIMERIDIAN,
                    path,
                    "segment crosses the antimeridian",
                ));
            }
        }
    }
    Ok(())
}

fn canonical_ring(
    mut ring: Vec<Micro>,
    path: &str,
    hole: bool,
    space: &VectorSpace,
) -> Result<Vec<Micro>, CoreError> {
    ring = close_ring(dedup_adjacent(ring));
    if ring.len() < 4 {
        return Err(fail(
            CODE_GEOMETRY_INVALID,
            path,
            "ring requires at least three distinct positions",
        ));
    }
    let mut open = ring[..ring.len() - 1].to_vec();
    if open.len() < 3 {
        return Err(fail(
            CODE_GEOMETRY_INVALID,
            path,
            "ring requires at least three distinct positions",
        ));
    }
    if space.is_geographic() {
        for pair in ring.windows(2) {
            if crosses_antimeridian(pair[0], pair[1]) {
                return Err(fail(
                    CODE_ANTIMERIDIAN,
                    path,
                    "segment crosses the antimeridian",
                ));
            }
        }
        let min_lon = open.iter().map(|coord| coord.0).min().unwrap();
        let max_lon = open.iter().map(|coord| coord.0).max().unwrap();
        if max_lon - min_lon > ANTIMERIDIAN {
            return Err(fail(
                CODE_ANTIMERIDIAN,
                path,
                "ring longitude span exceeds 180 degrees",
            ));
        }
    }
    let n = open.len();
    for i in 0..n {
        let a = open[i];
        let b = open[(i + 1) % n];
        for j in (i + 1)..n {
            if j == i || (j + 1) % n == i || (i + 1) % n == j {
                continue;
            }
            let c = open[j];
            let d = open[(j + 1) % n];
            if segments_intersect(a, b, c, d) {
                return Err(fail(
                    CODE_GEOMETRY_INVALID,
                    path,
                    "ring is self-intersecting",
                ));
            }
        }
    }
    let area = signed_area(&close_ring(open.clone()));
    if area == 0 {
        return Err(fail(
            CODE_GEOMETRY_INVALID,
            path,
            "ring has zero signed area",
        ));
    }
    let clockwise = area < 0;
    if hole != clockwise {
        open.reverse();
    }
    let mut best_index = 0usize;
    let mut best_seq = cyclic_key(&open, 0);
    for index in 1..open.len() {
        let seq = cyclic_key(&open, index);
        if seq < best_seq {
            best_seq = seq;
            best_index = index;
        }
    }
    open.rotate_left(best_index);
    Ok(close_ring(open))
}

fn cyclic_key(open: &[Micro], start: usize) -> Vec<Micro> {
    let mut seq = Vec::with_capacity(open.len());
    for offset in 0..open.len() {
        seq.push(open[(start + offset) % open.len()]);
    }
    seq
}

fn canonical_polygon(
    rings: Vec<Vec<Micro>>,
    path: &str,
    space: &VectorSpace,
) -> Result<(Vec<Micro>, Vec<Vec<Micro>>), CoreError> {
    if rings.is_empty() {
        return Err(fail(
            CODE_GEOMETRY_INVALID,
            path,
            "polygon requires an exterior ring",
        ));
    }
    if rings.len() > VECTOR_MAX_RINGS {
        return Err(fail(
            CODE_LIMIT,
            path,
            format!("polygon exceeds {VECTOR_MAX_RINGS} rings"),
        ));
    }
    let exterior = canonical_ring(rings[0].clone(), &format!("{path}[0]"), false, space)?;
    let mut holes = Vec::new();
    for (index, hole) in rings.into_iter().skip(1).enumerate() {
        holes.push(canonical_ring(
            hole,
            &format!("{path}[{}]", index + 1),
            true,
            space,
        )?);
    }
    holes.sort();
    Ok((exterior, holes))
}

fn polygon_abs_area(exterior: &[Micro]) -> i128 {
    signed_area(exterior).abs()
}

fn count_positions(geometry: &Geometry) -> usize {
    match geometry {
        Geometry::Point(_) => 1,
        Geometry::MultiPoint(points) => points.len(),
        Geometry::LineString(line) => line.len(),
        Geometry::MultiLineString(lines) => lines.iter().map(Vec::len).sum(),
        Geometry::Polygon { exterior, holes } => {
            exterior.len() + holes.iter().map(Vec::len).sum::<usize>()
        }
        Geometry::MultiPolygon(members) => members
            .iter()
            .map(|(exterior, holes)| exterior.len() + holes.iter().map(Vec::len).sum::<usize>())
            .sum(),
    }
}

fn canonical_uuid(value: &str, path: &str) -> Result<String, CoreError> {
    let uuid = Uuid::parse_str(value)
        .map_err(|_| fail(CODE_SOURCE_INVALID, path, "feature id must be a UUID"))?;
    let text = uuid.to_string();
    if text != value {
        return Err(fail(
            CODE_SOURCE_INVALID,
            path,
            "feature id must be lowercase hyphenated UUID text",
        ));
    }
    Ok(text)
}

fn parse_semantic_type(value: Option<&Value>, path: &str) -> Result<String, CoreError> {
    let kind = value
        .and_then(Value::as_str)
        .ok_or_else(|| fail(CODE_SOURCE_INVALID, path, "semanticType is required"))?;
    if matches!(
        kind,
        "land" | "lake" | "region" | "route" | "marker" | "custom"
    ) {
        Ok(kind.to_owned())
    } else {
        Err(fail(
            CODE_SOURCE_INVALID,
            path,
            "semanticType is not supported",
        ))
    }
}

fn parse_name(value: Option<&Value>, path: &str) -> Result<Option<String>, CoreError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(name)) => {
            if name.is_empty() || name.chars().count() > 256 {
                return Err(fail(
                    CODE_SOURCE_INVALID,
                    path,
                    "name must be null or 1..=256 Unicode scalars",
                ));
            }
            Ok(Some(name.clone()))
        }
        Some(_) => Err(fail(
            CODE_SOURCE_INVALID,
            path,
            "name must be a string or null",
        )),
    }
}

fn parse_geometry(value: &Value, path: &str, space: &VectorSpace) -> Result<Geometry, CoreError> {
    let object = object_keys(value, path, &["type", "coordinates"])?;
    let kind = object
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| fail(CODE_SOURCE_INVALID, &format!("{path}.type"), "is required"))?;
    let coordinates = object.get("coordinates").ok_or_else(|| {
        fail(
            CODE_SOURCE_INVALID,
            &format!("{path}.coordinates"),
            "is required",
        )
    })?;
    match kind {
        "Point" => Ok(Geometry::Point(parse_position(
            coordinates,
            &format!("{path}.coordinates"),
            space,
        )?)),
        "MultiPoint" => {
            let values = coordinates.as_array().ok_or_else(|| {
                fail(
                    CODE_GEOMETRY_INVALID,
                    &format!("{path}.coordinates"),
                    "must be an array",
                )
            })?;
            if values.is_empty() {
                return Err(fail(
                    CODE_GEOMETRY_INVALID,
                    &format!("{path}.coordinates"),
                    "MultiPoint requires at least one position",
                ));
            }
            let mut points = values
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    parse_position(value, &format!("{path}.coordinates[{index}]"), space)
                })
                .collect::<Result<Vec<_>, _>>()?;
            points.sort();
            Ok(Geometry::MultiPoint(points))
        }
        "LineString" => {
            let values = coordinates.as_array().ok_or_else(|| {
                fail(
                    CODE_GEOMETRY_INVALID,
                    &format!("{path}.coordinates"),
                    "must be an array",
                )
            })?;
            let line = dedup_adjacent(parse_line(values, &format!("{path}.coordinates"), space)?);
            validate_line(&line, &format!("{path}.coordinates"), space)?;
            Ok(Geometry::LineString(line))
        }
        "MultiLineString" => {
            let values = coordinates.as_array().ok_or_else(|| {
                fail(
                    CODE_GEOMETRY_INVALID,
                    &format!("{path}.coordinates"),
                    "must be an array",
                )
            })?;
            if values.is_empty() {
                return Err(fail(
                    CODE_GEOMETRY_INVALID,
                    &format!("{path}.coordinates"),
                    "MultiLineString requires at least one line",
                ));
            }
            let mut lines = Vec::with_capacity(values.len());
            for (index, line) in values.iter().enumerate() {
                let positions = line.as_array().ok_or_else(|| {
                    fail(
                        CODE_GEOMETRY_INVALID,
                        &format!("{path}.coordinates[{index}]"),
                        "must be an array",
                    )
                })?;
                let line = dedup_adjacent(parse_line(
                    positions,
                    &format!("{path}.coordinates[{index}]"),
                    space,
                )?);
                validate_line(&line, &format!("{path}.coordinates[{index}]"), space)?;
                lines.push(line);
            }
            lines.sort();
            Ok(Geometry::MultiLineString(lines))
        }
        "Polygon" => {
            let values = coordinates.as_array().ok_or_else(|| {
                fail(
                    CODE_GEOMETRY_INVALID,
                    &format!("{path}.coordinates"),
                    "must be an array",
                )
            })?;
            let rings = values
                .iter()
                .enumerate()
                .map(|(index, ring)| {
                    parse_ring(
                        ring.as_array().ok_or_else(|| {
                            fail(
                                CODE_GEOMETRY_INVALID,
                                &format!("{path}.coordinates[{index}]"),
                                "ring must be an array",
                            )
                        })?,
                        &format!("{path}.coordinates[{index}]"),
                        space,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            let (exterior, holes) =
                canonical_polygon(rings, &format!("{path}.coordinates"), space)?;
            Ok(Geometry::Polygon { exterior, holes })
        }
        "MultiPolygon" => {
            let values = coordinates.as_array().ok_or_else(|| {
                fail(
                    CODE_GEOMETRY_INVALID,
                    &format!("{path}.coordinates"),
                    "must be an array",
                )
            })?;
            let mut members = Vec::new();
            for (index, polygon) in values.iter().enumerate() {
                let rings = polygon
                    .as_array()
                    .ok_or_else(|| {
                        fail(
                            CODE_GEOMETRY_INVALID,
                            &format!("{path}.coordinates[{index}]"),
                            "must be an array",
                        )
                    })?
                    .iter()
                    .enumerate()
                    .map(|(ring_index, ring)| {
                        parse_ring(
                            ring.as_array().ok_or_else(|| {
                                fail(
                                    CODE_GEOMETRY_INVALID,
                                    &format!("{path}.coordinates[{index}][{ring_index}]"),
                                    "ring must be an array",
                                )
                            })?,
                            &format!("{path}.coordinates[{index}][{ring_index}]"),
                            space,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                members.push(canonical_polygon(
                    rings,
                    &format!("{path}.coordinates[{index}]"),
                    space,
                )?);
            }
            members.sort_by(|left, right| {
                polygon_abs_area(&right.0)
                    .cmp(&polygon_abs_area(&left.0))
                    .then_with(|| left.0.cmp(&right.0))
                    .then_with(|| left.1.cmp(&right.1))
            });
            Ok(Geometry::MultiPolygon(members))
        }
        "GeometryCollection" => Err(fail(
            CODE_SOURCE_INVALID,
            path,
            "GeometryCollection is not allowed",
        )),
        _ => Err(fail(
            CODE_SOURCE_INVALID,
            &format!("{path}.type"),
            "unsupported geometry type",
        )),
    }
}

fn geometry_matches_kind(kind: &str, geometry: &Geometry) -> bool {
    matches!(
        (kind, geometry),
        (
            "land" | "lake" | "region",
            Geometry::Polygon { .. } | Geometry::MultiPolygon(_)
        ) | (
            "route",
            Geometry::LineString(_) | Geometry::MultiLineString(_)
        ) | ("marker", Geometry::Point(_) | Geometry::MultiPoint(_))
            | ("custom", _)
    )
}

fn parse_custom_properties(
    value: Option<&Value>,
    path: &str,
) -> Result<serde_json::Map<String, Value>, CoreError> {
    match value {
        None => Ok(serde_json::Map::new()),
        Some(Value::Object(object)) => {
            if object.len() > VECTOR_MAX_CUSTOM_KEYS {
                return Err(fail(
                    CODE_LIMIT,
                    path,
                    format!("custom exceeds {VECTOR_MAX_CUSTOM_KEYS} keys"),
                ));
            }
            let mut out = serde_json::Map::new();
            for (key, entry) in object {
                if key.is_empty() || key.len() > 64 {
                    return Err(fail(
                        CODE_SOURCE_INVALID,
                        path,
                        "custom keys must be 1..=64 bytes",
                    ));
                }
                match entry {
                    Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
                        if let Value::String(text) = entry {
                            if text.len() > VECTOR_MAX_LABEL_TEXT {
                                return Err(fail(
                                    CODE_LIMIT,
                                    &format!("{path}.{key}"),
                                    format!("string values exceed {VECTOR_MAX_LABEL_TEXT} bytes"),
                                ));
                            }
                        }
                        out.insert(key.clone(), entry.clone());
                    }
                    _ => {
                        return Err(fail(
                            CODE_SOURCE_INVALID,
                            &format!("{path}.{key}"),
                            "custom values must be string, number, boolean, or null",
                        ));
                    }
                }
            }
            Ok(out)
        }
        Some(_) => Err(fail(CODE_SOURCE_INVALID, path, "custom must be an object")),
    }
}

fn parse_optional_style(value: Option<&Value>, path: &str) -> Result<Option<Value>, CoreError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(style) => {
            validate_partial_vector_style(style, path)?;
            Ok(Some(style.clone()))
        }
    }
}

fn parse_optional_label(value: Option<&Value>, path: &str) -> Result<Option<Value>, CoreError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(label) => {
            validate_label(label, path)?;
            Ok(Some(label.clone()))
        }
    }
}

fn parse_feature(
    value: &Value,
    path: &str,
    profile: SourceProfile,
    known_layers: &BTreeSet<String>,
    space: &VectorSpace,
) -> Result<Feature, CoreError> {
    let allowed = match profile {
        SourceProfile::Candidate => ["type", "geometry", "properties"].as_slice(),
        SourceProfile::Committed => ["type", "id", "properties", "geometry"].as_slice(),
    };
    let object = object_keys(value, path, allowed)?;
    require_type(object, path, "Feature")?;
    if profile == SourceProfile::Candidate && object.contains_key("id") {
        return Err(fail(
            CODE_SOURCE_INVALID,
            &format!("{path}.id"),
            "candidate features must not include ids",
        ));
    }
    let geometry = parse_geometry(
        object.get("geometry").ok_or_else(|| {
            fail(
                CODE_SOURCE_INVALID,
                &format!("{path}.geometry"),
                "is required",
            )
        })?,
        &format!("{path}.geometry"),
        space,
    )?;
    if count_positions(&geometry) > VECTOR_MAX_FEATURE_POSITIONS {
        return Err(fail(
            CODE_LIMIT,
            path,
            format!("feature exceeds {VECTOR_MAX_FEATURE_POSITIONS} positions"),
        ));
    }
    let properties = object.get("properties").unwrap_or(&Value::Null);
    match profile {
        SourceProfile::Candidate => {
            if properties
                .as_object()
                .is_none_or(|object| !object.is_empty())
                && !properties.is_null()
            {
                let object = object_keys(properties, &format!("{path}.properties"), &[])?;
                if !object.is_empty() {
                    return Err(fail(
                        CODE_SOURCE_INVALID,
                        &format!("{path}.properties"),
                        "candidate properties must be empty",
                    ));
                }
            }
            if !matches!(
                geometry,
                Geometry::Polygon { .. } | Geometry::MultiPolygon(_)
            ) {
                return Err(fail(
                    CODE_SOURCE_INVALID,
                    &format!("{path}.geometry"),
                    "candidates must be polygonal",
                ));
            }
            Ok(Feature {
                id: Uuid::new_v4().to_string(),
                layer_id: "base".into(),
                semantic_type: "land".into(),
                name: None,
                style: None,
                label: None,
                custom: serde_json::Map::new(),
                geometry,
            })
        }
        SourceProfile::Committed => {
            let id = canonical_uuid(
                object.get("id").and_then(Value::as_str).ok_or_else(|| {
                    fail(CODE_SOURCE_INVALID, &format!("{path}.id"), "is required")
                })?,
                &format!("{path}.id"),
            )?;
            if properties.as_object().is_some_and(|object| {
                object.contains_key("daenaLayerId")
                    || object.contains_key("kind")
                    || (object.contains_key("name") && !object.contains_key("daena"))
            }) {
                return Err(fail(
                    CODE_UNSUPPORTED_VERSION,
                    &format!("{path}.properties"),
                    "flat feature properties are unsupported; use properties.daena",
                ));
            }
            let properties = object_keys(properties, &format!("{path}.properties"), &["daena"])?;
            let daena = properties.get("daena").ok_or_else(|| {
                fail(
                    CODE_SOURCE_INVALID,
                    &format!("{path}.properties.daena"),
                    "is required",
                )
            })?;
            let daena = object_keys(
                daena,
                &format!("{path}.properties.daena"),
                &[
                    "layerId",
                    "semanticType",
                    "name",
                    "style",
                    "label",
                    "custom",
                ],
            )?;
            let layer_id = daena
                .get("layerId")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    fail(
                        CODE_SOURCE_INVALID,
                        &format!("{path}.properties.daena.layerId"),
                        "is required",
                    )
                })?
                .to_owned();
            let semantic_type = parse_semantic_type(
                daena.get("semanticType"),
                &format!("{path}.properties.daena.semanticType"),
            )?;
            let name = parse_name(daena.get("name"), &format!("{path}.properties.daena.name"))?;
            let style = parse_optional_style(
                daena.get("style"),
                &format!("{path}.properties.daena.style"),
            )?;
            let label = parse_optional_label(
                daena.get("label"),
                &format!("{path}.properties.daena.label"),
            )?;
            let custom = parse_custom_properties(
                daena.get("custom"),
                &format!("{path}.properties.daena.custom"),
            )?;
            if layer_id == "base" {
                if !matches!(semantic_type.as_str(), "land" | "lake")
                    || !matches!(
                        geometry,
                        Geometry::Polygon { .. } | Geometry::MultiPolygon(_)
                    )
                {
                    return Err(fail(
                        CODE_SOURCE_INVALID,
                        path,
                        "base features must be land or lake polygons",
                    ));
                }
            } else {
                canonical_uuid(&layer_id, &format!("{path}.properties.daena.layerId"))?;
                if !known_layers.is_empty() && !known_layers.contains(&layer_id) {
                    return Err(fail(
                        CODE_LAYER_MISSING,
                        &format!("{path}.properties.daena.layerId"),
                        "layer does not exist",
                    ));
                }
                if !geometry_matches_kind(&semantic_type, &geometry) {
                    return Err(fail(
                        CODE_GEOMETRY_INVALID,
                        path,
                        "geometry does not match semanticType",
                    ));
                }
            }
            let encoded = encode_daena_properties(
                &layer_id,
                &semantic_type,
                &name,
                style.as_ref(),
                label.as_ref(),
                &custom,
            );
            if encoded.len() > VECTOR_MAX_PROPERTY_BYTES {
                return Err(fail(
                    CODE_LIMIT,
                    &format!("{path}.properties"),
                    "feature properties exceed 2 KiB",
                ));
            }
            Ok(Feature {
                id,
                layer_id,
                semantic_type,
                name,
                style,
                label,
                custom,
                geometry,
            })
        }
    }
}

fn parse_collection(
    value: &Value,
    profile: SourceProfile,
    known_layers: &BTreeSet<String>,
    space: &VectorSpace,
) -> Result<Vec<Feature>, CoreError> {
    let object = object_keys(value, "$", &["type", "features"])?;
    require_type(object, "$", "FeatureCollection")?;
    let features = object
        .get("features")
        .and_then(Value::as_array)
        .ok_or_else(|| fail(CODE_SOURCE_INVALID, "$.features", "must be an array"))?;
    if features.len() > VECTOR_MAX_FEATURES {
        return Err(fail(
            CODE_LIMIT,
            "$.features",
            format!("exceeds {VECTOR_MAX_FEATURES} features"),
        ));
    }
    let mut parsed = Vec::with_capacity(features.len());
    let mut ids = BTreeSet::new();
    let mut positions = 0usize;
    for (index, feature) in features.iter().enumerate() {
        let feature = parse_feature(
            feature,
            &format!("$.features[{index}]"),
            profile,
            known_layers,
            space,
        )?;
        positions += count_positions(&feature.geometry);
        if positions > VECTOR_MAX_POSITIONS {
            return Err(fail(
                CODE_LIMIT,
                "$.features",
                format!("exceeds {VECTOR_MAX_POSITIONS} positions"),
            ));
        }
        if profile == SourceProfile::Committed && !ids.insert(feature.id.clone()) {
            return Err(fail(
                CODE_SOURCE_INVALID,
                &format!("$.features[{index}].id"),
                "feature ids must be unique",
            ));
        }
        parsed.push(feature);
    }
    parsed.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(parsed)
}

fn write_positions(out: &mut String, positions: &[Micro]) {
    out.push('[');
    for (index, coord) in positions.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('[');
        out.push_str(&format_micro(coord.0));
        out.push(',');
        out.push_str(&format_micro(coord.1));
        out.push(']');
    }
    out.push(']');
}

fn write_polygon(out: &mut String, exterior: &[Micro], holes: &[Vec<Micro>]) {
    out.push('[');
    write_positions(out, exterior);
    for hole in holes {
        out.push(',');
        write_positions(out, hole);
    }
    out.push(']');
}

fn write_geometry(out: &mut String, geometry: &Geometry) {
    match geometry {
        Geometry::Point(coord) => {
            out.push_str("{\"type\":\"Point\",\"coordinates\":[");
            out.push_str(&format_micro(coord.0));
            out.push(',');
            out.push_str(&format_micro(coord.1));
            out.push_str("]}");
        }
        Geometry::MultiPoint(points) => {
            out.push_str("{\"type\":\"MultiPoint\",\"coordinates\":");
            write_positions(out, points);
            out.push('}');
        }
        Geometry::LineString(line) => {
            out.push_str("{\"type\":\"LineString\",\"coordinates\":");
            write_positions(out, line);
            out.push('}');
        }
        Geometry::MultiLineString(lines) => {
            out.push_str("{\"type\":\"MultiLineString\",\"coordinates\":[");
            for (index, line) in lines.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_positions(out, line);
            }
            out.push_str("]}");
        }
        Geometry::Polygon { exterior, holes } => {
            out.push_str("{\"type\":\"Polygon\",\"coordinates\":");
            write_polygon(out, exterior, holes);
            out.push('}');
        }
        Geometry::MultiPolygon(members) => {
            out.push_str("{\"type\":\"MultiPolygon\",\"coordinates\":[");
            for (index, (exterior, holes)) in members.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_polygon(out, exterior, holes);
            }
            out.push_str("]}");
        }
    }
}

fn encode_daena_properties(
    layer_id: &str,
    semantic_type: &str,
    name: &Option<String>,
    style: Option<&Value>,
    label: Option<&Value>,
    custom: &serde_json::Map<String, Value>,
) -> String {
    let mut out = String::from("{\"daena\":{\"layerId\":");
    out.push_str(&serde_json::to_string(layer_id).unwrap());
    out.push_str(",\"semanticType\":");
    out.push_str(&serde_json::to_string(semantic_type).unwrap());
    out.push_str(",\"name\":");
    out.push_str(&serde_json::to_string(name).unwrap());
    out.push_str(",\"style\":");
    match style {
        Some(value) => out.push_str(&serde_json::to_string(value).unwrap()),
        None => out.push_str("null"),
    }
    out.push_str(",\"label\":");
    match label {
        Some(value) => out.push_str(&serde_json::to_string(value).unwrap()),
        None => out.push_str("null"),
    }
    out.push_str(",\"custom\":{");
    let mut keys = custom.keys().cloned().collect::<Vec<_>>();
    keys.sort();
    for (index, key) in keys.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&serde_json::to_string(key).unwrap());
        out.push(':');
        out.push_str(&serde_json::to_string(&custom[key]).unwrap());
    }
    out.push_str("}}}");
    out
}

#[must_use]
pub fn empty_canonical_bytes() -> Vec<u8> {
    serialize_features(&[])
}

fn serialize_features(features: &[Feature]) -> Vec<u8> {
    let mut out = String::from("{\"type\":\"FeatureCollection\",\"features\":[");
    for (index, feature) in features.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str("{\"type\":\"Feature\",\"id\":");
        out.push_str(&serde_json::to_string(&feature.id).unwrap());
        out.push_str(",\"properties\":");
        out.push_str(&encode_daena_properties(
            &feature.layer_id,
            &feature.semantic_type,
            &feature.name,
            feature.style.as_ref(),
            feature.label.as_ref(),
            &feature.custom,
        ));
        out.push_str(",\"geometry\":");
        write_geometry(&mut out, &feature.geometry);
        out.push('}');
    }
    out.push_str("]}\n");
    out.into_bytes()
}

pub fn canonicalize_committed(
    bytes: &[u8],
    known_layers: &BTreeSet<String>,
) -> Result<Vec<u8>, CoreError> {
    canonicalize_committed_with_space(bytes, known_layers, &VectorSpace::Geographic)
}

pub fn canonicalize_committed_with_space(
    bytes: &[u8],
    known_layers: &BTreeSet<String>,
    space: &VectorSpace,
) -> Result<Vec<u8>, CoreError> {
    let value = parse_strict_json(bytes)?;
    let features = parse_collection(&value, SourceProfile::Committed, known_layers, space)?;
    Ok(serialize_features(&features))
}

/// Compare feature sets for layers that were locked in the previous revision and remain locked.
/// Newly locked, unlocked, or removed layers are not checked here.
pub fn assert_locked_layer_features_unchanged(
    previous_bytes: &[u8],
    next_bytes: &[u8],
    known_previous: &BTreeSet<String>,
    known_next: &BTreeSet<String>,
    locked_layer_ids: &BTreeSet<String>,
) -> Result<(), CoreError> {
    assert_locked_layer_features_unchanged_with_space(
        previous_bytes,
        next_bytes,
        known_previous,
        known_next,
        locked_layer_ids,
        &VectorSpace::Geographic,
    )
}

pub fn assert_locked_layer_features_unchanged_with_space(
    previous_bytes: &[u8],
    next_bytes: &[u8],
    known_previous: &BTreeSet<String>,
    known_next: &BTreeSet<String>,
    locked_layer_ids: &BTreeSet<String>,
    space: &VectorSpace,
) -> Result<(), CoreError> {
    if locked_layer_ids.is_empty() {
        return Ok(());
    }
    let previous = parse_collection(
        &parse_strict_json(previous_bytes)?,
        SourceProfile::Committed,
        known_previous,
        space,
    )?;
    let next = parse_collection(
        &parse_strict_json(next_bytes)?,
        SourceProfile::Committed,
        known_next,
        space,
    )?;
    for layer_id in locked_layer_ids {
        let mut before: Vec<&Feature> = previous
            .iter()
            .filter(|feature| feature.layer_id == *layer_id)
            .collect();
        let mut after: Vec<&Feature> = next
            .iter()
            .filter(|feature| feature.layer_id == *layer_id)
            .collect();
        before.sort_by(|left, right| left.id.cmp(&right.id));
        after.sort_by(|left, right| left.id.cmp(&right.id));
        if before != after {
            return Err(fail(
                CODE_SOURCE_INVALID,
                "$",
                format!("locked layer {layer_id} features cannot be changed"),
            ));
        }
    }
    Ok(())
}

pub fn canonicalize_candidate(bytes: &[u8]) -> Result<Vec<u8>, CoreError> {
    canonicalize_candidate_with_space(bytes, &VectorSpace::Geographic)
}

pub fn canonicalize_candidate_with_space(
    bytes: &[u8],
    space: &VectorSpace,
) -> Result<Vec<u8>, CoreError> {
    let value = parse_strict_json(bytes)?;
    let features = parse_collection(&value, SourceProfile::Candidate, &BTreeSet::new(), space)?;
    Ok(serialize_features(&features))
}

/// Import polygonal GeoJSON as read-only base land. Feature ids and properties are ignored;
/// only Point/Line geometries are rejected. Editing base land is deferred.
pub fn canonicalize_imported_base(bytes: &[u8]) -> Result<Vec<u8>, CoreError> {
    canonicalize_imported_base_with_space(bytes, &VectorSpace::Geographic)
}

pub fn canonicalize_imported_base_with_space(
    bytes: &[u8],
    space: &VectorSpace,
) -> Result<Vec<u8>, CoreError> {
    let value = parse_strict_json(bytes)?;
    let object = object_keys(&value, "$", &["type", "features"])?;
    require_type(object, "$", "FeatureCollection")?;
    let features = object
        .get("features")
        .and_then(Value::as_array)
        .ok_or_else(|| fail(CODE_SOURCE_INVALID, "$.features", "must be an array"))?;
    if features.len() > VECTOR_MAX_FEATURES {
        return Err(fail(
            CODE_LIMIT,
            "$.features",
            format!("exceeds {VECTOR_MAX_FEATURES} features"),
        ));
    }
    let mut parsed = Vec::with_capacity(features.len());
    let mut positions = 0usize;
    for (index, feature) in features.iter().enumerate() {
        let path = format!("$.features[{index}]");
        let object = object_keys(
            feature,
            &path,
            &["type", "id", "geometry", "properties", "bbox"],
        )?;
        require_type(object, &path, "Feature")?;
        let geometry = parse_geometry(
            object.get("geometry").ok_or_else(|| {
                fail(
                    CODE_SOURCE_INVALID,
                    &format!("{path}.geometry"),
                    "is required",
                )
            })?,
            &format!("{path}.geometry"),
            space,
        )?;
        if !matches!(
            geometry,
            Geometry::Polygon { .. } | Geometry::MultiPolygon(_)
        ) {
            return Err(fail(
                CODE_SOURCE_INVALID,
                &format!("{path}.geometry"),
                "imported base features must be polygonal",
            ));
        }
        let count = count_positions(&geometry);
        if count > VECTOR_MAX_FEATURE_POSITIONS {
            return Err(fail(
                CODE_LIMIT,
                &path,
                format!("feature exceeds {VECTOR_MAX_FEATURE_POSITIONS} positions"),
            ));
        }
        positions += count;
        if positions > VECTOR_MAX_POSITIONS {
            return Err(fail(
                CODE_LIMIT,
                "$.features",
                format!("exceeds {VECTOR_MAX_POSITIONS} positions"),
            ));
        }
        parsed.push(Feature {
            id: Uuid::new_v4().to_string(),
            layer_id: "base".into(),
            semantic_type: "land".into(),
            name: None,
            style: None,
            label: None,
            custom: serde_json::Map::new(),
            geometry,
        });
    }
    parsed.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(serialize_features(&parsed))
}

pub fn require_canonical_bytes(
    fs_path: &Path,
    bytes: &[u8],
    known_layers: &BTreeSet<String>,
) -> Result<Vec<u8>, CoreError> {
    require_canonical_bytes_with_space(fs_path, bytes, known_layers, &VectorSpace::Geographic)
}

pub fn require_canonical_bytes_with_space(
    fs_path: &Path,
    bytes: &[u8],
    known_layers: &BTreeSet<String>,
    space: &VectorSpace,
) -> Result<Vec<u8>, CoreError> {
    let canonical = canonicalize_committed_with_space(bytes, known_layers, space).map_err(
        |error| match error {
            CoreError::Validation(detail) => path_fail(fs_path, CODE_SOURCE_INVALID, "$", detail),
            other => other,
        },
    )?;
    if canonical.as_slice() != bytes {
        return Err(path_fail(
            fs_path,
            CODE_SOURCE_INVALID,
            "$",
            "GeoJSON source is not byte-canonical for adapter version 2",
        ));
    }
    Ok(canonical)
}

pub fn layer_ids_from_layers_field(value: &Value) -> BTreeSet<String> {
    value
        .get("layers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|layer| layer.get("kind").and_then(Value::as_str) == Some("vector"))
        .filter_map(|layer| layer.get("id").and_then(Value::as_str).map(str::to_owned))
        .collect()
}

pub fn remove_layer_features(
    bytes: &[u8],
    layer_id: &str,
    known_layers: &BTreeSet<String>,
) -> Result<(Vec<u8>, usize), CoreError> {
    remove_layer_features_with_space(bytes, layer_id, known_layers, &VectorSpace::Geographic)
}

pub fn remove_layer_features_with_space(
    bytes: &[u8],
    layer_id: &str,
    known_layers: &BTreeSet<String>,
    space: &VectorSpace,
) -> Result<(Vec<u8>, usize), CoreError> {
    let value = parse_strict_json(bytes)?;
    let mut features = parse_collection(&value, SourceProfile::Committed, known_layers, space)?;
    let before = features.len();
    features.retain(|feature| feature.layer_id != layer_id);
    let deleted = before - features.len();
    Ok((serialize_features(&features), deleted))
}

pub fn feature_bounds(
    bytes: &[u8],
    known_layers: &BTreeSet<String>,
) -> Result<Vec<FeatureBounds>, CoreError> {
    feature_bounds_with_space(bytes, known_layers, &VectorSpace::Geographic)
}

pub fn feature_bounds_with_space(
    bytes: &[u8],
    known_layers: &BTreeSet<String>,
    space: &VectorSpace,
) -> Result<Vec<FeatureBounds>, CoreError> {
    let value = parse_strict_json(bytes)?;
    let features = parse_collection(&value, SourceProfile::Committed, known_layers, space)?;
    Ok(features
        .into_iter()
        .map(|feature| {
            let mut min_x_micro = i64::MAX;
            let mut min_y_micro = i64::MAX;
            let mut max_x_micro = i64::MIN;
            let mut max_y_micro = i64::MIN;
            let mut visit = |coord: Micro| {
                min_x_micro = min_x_micro.min(coord.0);
                max_x_micro = max_x_micro.max(coord.0);
                min_y_micro = min_y_micro.min(coord.1);
                max_y_micro = max_y_micro.max(coord.1);
            };
            match &feature.geometry {
                Geometry::Point(coord) => visit(*coord),
                Geometry::MultiPoint(points) => points.iter().copied().for_each(&mut visit),
                Geometry::LineString(line) => line.iter().copied().for_each(&mut visit),
                Geometry::MultiLineString(lines) => {
                    lines.iter().flatten().copied().for_each(&mut visit);
                }
                Geometry::Polygon { exterior, holes } => {
                    exterior.iter().copied().for_each(&mut visit);
                    holes.iter().flatten().copied().for_each(&mut visit);
                }
                Geometry::MultiPolygon(members) => {
                    for (exterior, holes) in members {
                        exterior.iter().copied().for_each(&mut visit);
                        holes.iter().flatten().copied().for_each(&mut visit);
                    }
                }
            }
            let (min_x, max_y) = match space {
                VectorSpace::Geographic => lon_lat_to_normalized(
                    (min_x_micro as f64) / 1_000_000.0,
                    (max_y_micro as f64) / 1_000_000.0,
                ),
                VectorSpace::Planar { extent, is_image } => planar_to_normalized(
                    (min_x_micro as f64) / 1_000_000.0,
                    (max_y_micro as f64) / 1_000_000.0,
                    extent,
                    *is_image,
                ),
            };
            let (max_x, min_y) = match space {
                VectorSpace::Geographic => lon_lat_to_normalized(
                    (max_x_micro as f64) / 1_000_000.0,
                    (min_y_micro as f64) / 1_000_000.0,
                ),
                VectorSpace::Planar { extent, is_image } => planar_to_normalized(
                    (max_x_micro as f64) / 1_000_000.0,
                    (min_y_micro as f64) / 1_000_000.0,
                    extent,
                    *is_image,
                ),
            };
            (
                feature.id,
                feature.layer_id,
                feature.semantic_type,
                min_x,
                min_y,
                max_x,
                max_y,
            )
        })
        .collect())
}

#[must_use]
pub fn contains_feature_id(bytes: &[u8], feature_id: &str) -> bool {
    parse_strict_json(bytes)
        .ok()
        .and_then(|value| value.get("features").and_then(Value::as_array).cloned())
        .is_some_and(|features| {
            features
                .iter()
                .any(|feature| feature.get("id").and_then(Value::as_str) == Some(feature_id))
        })
}

pub fn validate_generation(value: &Value) -> Result<(), CoreError> {
    let object = value
        .as_object()
        .ok_or_else(|| fail(CODE_GENERATOR, "generation", "must be an object"))?;
    if object.get("id").and_then(Value::as_str) != Some("daena-landmass") {
        return Err(fail(
            CODE_GENERATOR,
            "generation.id",
            "must be daena-landmass",
        ));
    }
    if object.get("version").and_then(Value::as_u64) != Some(1) {
        return Err(fail(
            CODE_UNSUPPORTED_VERSION,
            "generation.version",
            "must be 1",
        ));
    }
    let seed = object
        .get("seed")
        .and_then(Value::as_u64)
        .ok_or_else(|| fail(CODE_GENERATOR, "generation.seed", "must be a uint32"))?;
    if seed > u64::from(u32::MAX) {
        return Err(fail(CODE_GENERATOR, "generation.seed", "must be a uint32"));
    }
    let settings = object
        .get("settings")
        .and_then(Value::as_object)
        .ok_or_else(|| fail(CODE_GENERATOR, "generation.settings", "must be an object"))?;
    let land = settings.get("landPercent").and_then(Value::as_u64);
    let continents = settings.get("continentCount").and_then(Value::as_u64);
    let roughness = settings.get("coastlineRoughness").and_then(Value::as_str);
    let islands = settings.get("islandFrequency").and_then(Value::as_str);
    if !matches!(land, Some(15..=70)) {
        return Err(fail(
            CODE_GENERATOR,
            "generation.settings.landPercent",
            "must be an integer 15..=70",
        ));
    }
    if !matches!(continents, Some(1..=8)) {
        return Err(fail(
            CODE_GENERATOR,
            "generation.settings.continentCount",
            "must be an integer 1..=8",
        ));
    }
    if !matches!(roughness, Some("low" | "medium" | "high")) {
        return Err(fail(
            CODE_GENERATOR,
            "generation.settings.coastlineRoughness",
            "must be low, medium, or high",
        ));
    }
    if !matches!(islands, Some("none" | "low" | "medium" | "high")) {
        return Err(fail(
            CODE_GENERATOR,
            "generation.settings.islandFrequency",
            "must be none, low, medium, or high",
        ));
    }
    let allowed = BTreeSet::from(["id", "version", "seed", "settings"]);
    if object.keys().any(|key| !allowed.contains(key.as_str())) {
        return Err(fail(
            CODE_GENERATOR,
            "generation",
            "contains unknown members",
        ));
    }
    let setting_keys = BTreeSet::from([
        "landPercent",
        "continentCount",
        "coastlineRoughness",
        "islandFrequency",
    ]);
    if settings
        .keys()
        .any(|key| !setting_keys.contains(key.as_str()))
    {
        return Err(fail(
            CODE_GENERATOR,
            "generation.settings",
            "contains unknown members",
        ));
    }
    Ok(())
}

fn validate_hex_color(value: &Value, path: &str) -> Result<(), CoreError> {
    let color = value
        .as_str()
        .ok_or_else(|| fail(CODE_SOURCE_INVALID, path, "must be a hex color"))?;
    if !color.starts_with('#')
        || color.len() != 7
        || !color[1..].chars().all(|ch| ch.is_ascii_hexdigit())
    {
        return Err(fail(CODE_SOURCE_INVALID, path, "must match #RRGGBB"));
    }
    Ok(())
}

fn validate_opacity(value: &Value, path: &str) -> Result<(), CoreError> {
    let opacity = value
        .as_f64()
        .ok_or_else(|| fail(CODE_SOURCE_INVALID, path, "must be a finite number"))?;
    if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
        return Err(fail(CODE_SOURCE_INVALID, path, "must be finite in [0, 1]"));
    }
    Ok(())
}

fn validate_label(label: &Value, path: &str) -> Result<(), CoreError> {
    let label = label
        .as_object()
        .ok_or_else(|| fail(CODE_SOURCE_INVALID, path, "must be an object"))?;
    let allowed = BTreeSet::from([
        "source",
        "text",
        "size",
        "color",
        "haloColor",
        "haloWidth",
        "placement",
        "offset",
        "rotation",
        "minZoom",
        "maxZoom",
    ]);
    if label.keys().any(|key| !allowed.contains(key.as_str()))
        || !matches!(
            label.get("source").and_then(Value::as_str),
            Some("name" | "explicit")
        )
        || !matches!(
            label.get("placement").and_then(Value::as_str),
            Some("point" | "line" | "interior")
        )
    {
        return Err(fail(CODE_SOURCE_INVALID, path, "contains invalid fields"));
    }
    if label.get("text").is_some_and(|value| {
        !value.is_null()
            && value
                .as_str()
                .is_none_or(|text| text.len() > VECTOR_MAX_LABEL_TEXT)
    }) {
        return Err(fail(
            CODE_SOURCE_INVALID,
            &format!("{path}.text"),
            format!("exceeds {VECTOR_MAX_LABEL_TEXT} bytes"),
        ));
    }
    for (key, min, max) in [
        ("size", 6.0, 96.0),
        ("haloWidth", 0.0, 16.0),
        ("rotation", -360.0, 360.0),
    ] {
        let value = label.get(key).and_then(Value::as_f64).ok_or_else(|| {
            fail(
                CODE_SOURCE_INVALID,
                &format!("{path}.{key}"),
                "must be a finite number",
            )
        })?;
        if !value.is_finite() || !(min..=max).contains(&value) {
            return Err(fail(
                CODE_SOURCE_INVALID,
                &format!("{path}.{key}"),
                "is outside the supported range",
            ));
        }
    }
    for key in ["color", "haloColor"] {
        validate_hex_color(
            label.get(key).ok_or_else(|| {
                fail(CODE_SOURCE_INVALID, &format!("{path}.{key}"), "is required")
            })?,
            &format!("{path}.{key}"),
        )?;
    }
    let offset = label
        .get("offset")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            fail(
                CODE_SOURCE_INVALID,
                &format!("{path}.offset"),
                "must contain two numbers",
            )
        })?;
    if offset.len() != 2
        || offset.iter().any(|value| {
            value
                .as_f64()
                .is_none_or(|value| !value.is_finite() || value.abs() > 512.0)
        })
    {
        return Err(fail(
            CODE_SOURCE_INVALID,
            &format!("{path}.offset"),
            "must contain two finite values in [-512, 512]",
        ));
    }
    for key in ["minZoom", "maxZoom"] {
        match label.get(key) {
            None | Some(Value::Null) => {}
            Some(value) => {
                let zoom = value.as_f64().ok_or_else(|| {
                    fail(
                        CODE_SOURCE_INVALID,
                        &format!("{path}.{key}"),
                        "must be a finite number or null",
                    )
                })?;
                if !zoom.is_finite() || !(0.0..=24.0).contains(&zoom) {
                    return Err(fail(
                        CODE_SOURCE_INVALID,
                        &format!("{path}.{key}"),
                        "must be finite in [0, 24]",
                    ));
                }
            }
        }
    }
    Ok(())
}

fn validate_partial_vector_style(style: &Value, path: &str) -> Result<(), CoreError> {
    let object = style
        .as_object()
        .ok_or_else(|| fail(CODE_SOURCE_INVALID, path, "must be an object"))?;
    let allowed = BTreeSet::from([
        "fill",
        "fillOpacity",
        "stroke",
        "strokeOpacity",
        "strokeWidth",
        "strokeDash",
        "pointRadius",
        "icon",
        "iconSize",
        "label",
    ]);
    if object.keys().any(|key| !allowed.contains(key.as_str())) {
        return Err(fail(
            CODE_SOURCE_INVALID,
            path,
            "contains unsupported style fields",
        ));
    }
    if let Some(fill) = object.get("fill") {
        validate_hex_color(fill, &format!("{path}.fill"))?;
    }
    if let Some(stroke) = object.get("stroke") {
        validate_hex_color(stroke, &format!("{path}.stroke"))?;
    }
    if let Some(fill_opacity) = object.get("fillOpacity") {
        validate_opacity(fill_opacity, &format!("{path}.fillOpacity"))?;
    }
    if let Some(stroke_opacity) = object.get("strokeOpacity") {
        validate_opacity(stroke_opacity, &format!("{path}.strokeOpacity"))?;
    }
    if let Some(stroke_width) = object.get("strokeWidth") {
        let stroke_width = stroke_width.as_f64().ok_or_else(|| {
            fail(
                CODE_SOURCE_INVALID,
                &format!("{path}.strokeWidth"),
                "must be a finite number",
            )
        })?;
        if !stroke_width.is_finite() || !(0.0..=32.0).contains(&stroke_width) {
            return Err(fail(
                CODE_SOURCE_INVALID,
                &format!("{path}.strokeWidth"),
                "must be finite in [0, 32]",
            ));
        }
    }
    if let Some(point_radius) = object.get("pointRadius") {
        let point_radius = point_radius.as_f64().ok_or_else(|| {
            fail(
                CODE_SOURCE_INVALID,
                &format!("{path}.pointRadius"),
                "must be a finite number",
            )
        })?;
        if !point_radius.is_finite() || !(1.0..=64.0).contains(&point_radius) {
            return Err(fail(
                CODE_SOURCE_INVALID,
                &format!("{path}.pointRadius"),
                "must be finite in [1, 64]",
            ));
        }
    }
    if let Some(dash) = object.get("strokeDash") {
        let dash = dash.as_array().ok_or_else(|| {
            fail(
                CODE_SOURCE_INVALID,
                &format!("{path}.strokeDash"),
                "must be an array",
            )
        })?;
        if dash.len() > 16
            || dash.iter().any(|value| {
                value
                    .as_f64()
                    .is_none_or(|value| !value.is_finite() || value < 0.0 || value > 128.0)
            })
        {
            return Err(fail(
                CODE_SOURCE_INVALID,
                &format!("{path}.strokeDash"),
                "must contain at most 16 finite values in [0, 128]",
            ));
        }
    }
    if let Some(icon) = object.get("icon") {
        if !icon.is_null()
            && !matches!(
                icon.as_str(),
                Some("square" | "diamond" | "triangle" | "star")
            )
        {
            return Err(fail(
                CODE_SOURCE_INVALID,
                &format!("{path}.icon"),
                "must be null or a supported Daena marker icon",
            ));
        }
    }
    if let Some(icon_size) = object.get("iconSize") {
        let icon_size = icon_size.as_f64().ok_or_else(|| {
            fail(
                CODE_SOURCE_INVALID,
                &format!("{path}.iconSize"),
                "must be a finite number",
            )
        })?;
        if !icon_size.is_finite() || !(4.0..=256.0).contains(&icon_size) {
            return Err(fail(
                CODE_SOURCE_INVALID,
                &format!("{path}.iconSize"),
                "must be finite in [4, 256]",
            ));
        }
    }
    if let Some(label) = object.get("label") {
        validate_label(label, &format!("{path}.label"))?;
    }
    Ok(())
}

pub fn validate_vector_style(style: &Value) -> Result<(), CoreError> {
    let object = style.as_object().ok_or_else(|| {
        fail(
            CODE_SOURCE_INVALID,
            "style",
            "vector style must be an object",
        )
    })?;
    let required = [
        "fill",
        "fillOpacity",
        "stroke",
        "strokeWidth",
        "pointRadius",
    ];
    if required.iter().any(|key| !object.contains_key(*key)) {
        return Err(fail(
            CODE_SOURCE_INVALID,
            "style",
            "vector style must contain fill, fillOpacity, stroke, strokeWidth, and pointRadius",
        ));
    }
    validate_partial_vector_style(style, "style")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Value {
        serde_json::json!({
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "id": "018f89ec-25fc-7816-8b47-6f80905f2801",
                "properties": {
                    "daena": {
                        "layerId": "base",
                        "semanticType": "land",
                        "name": null,
                        "style": null,
                        "label": null,
                        "custom": {}
                    }
                },
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [[[-10.0, -10.0], [10.0, -10.0], [10.0, 10.0], [-10.0, 10.0], [-10.0, -10.0]]]
                }
            }]
        })
    }

    #[test]
    fn committed_source_is_byte_stable() {
        let input = serde_json::to_vec(&square()).unwrap();
        let first = canonicalize_committed(&input, &BTreeSet::new()).unwrap();
        let second = canonicalize_committed(&first, &BTreeSet::new()).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.last().copied(), Some(b'\n'));
        assert!(std::str::from_utf8(&first)
            .unwrap()
            .contains("\"type\":\"FeatureCollection\""));
        let expected = concat!(
            "{\"type\":\"FeatureCollection\",\"features\":[{\"type\":\"Feature\",\"id\":\"018f89ec-25fc-7816-8b47-6f80905f2801\",",
            "\"properties\":{\"daena\":{\"layerId\":\"base\",\"semanticType\":\"land\",\"name\":null,\"style\":null,\"label\":null,\"custom\":{}}},",
            "\"geometry\":{\"type\":\"Polygon\",\"coordinates\":[[[-10,-10],[10,-10],[10,10],[-10,10],[-10,-10]]]}}]}\n"
        );
        assert_eq!(std::str::from_utf8(&first).unwrap(), expected);
    }

    #[test]
    fn rejects_duplicate_keys() {
        let duplicate = br#"{"type":"FeatureCollection","type":"FeatureCollection","features":[]}"#;
        assert!(parse_strict_json(duplicate)
            .unwrap_err()
            .to_string()
            .contains(CODE_SOURCE_INVALID));
    }

    #[test]
    fn rejects_flat_v1_feature_properties() {
        let input = serde_json::json!({
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "id": "018f89ec-25fc-7816-8b47-6f80905f2801",
                "properties": {"daenaLayerId": "base", "kind": "land", "name": null},
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [[[-10.0, -10.0], [10.0, -10.0], [10.0, 10.0], [-10.0, 10.0], [-10.0, -10.0]]]
                }
            }]
        });
        assert!(
            canonicalize_committed(&serde_json::to_vec(&input).unwrap(), &BTreeSet::new())
                .unwrap_err()
                .to_string()
                .contains(CODE_UNSUPPORTED_VERSION)
        );
    }

    #[test]
    fn multipoint_and_multilinestring_canonicalize() {
        let layer = "018f89ec-25fc-7816-8b47-6f80905f2868";
        let known = BTreeSet::from([layer.to_owned()]);
        let input = serde_json::json!({
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "id": "018f89ec-25fc-7816-8b47-6f80905f2802",
                    "properties": {
                        "daena": {
                            "layerId": layer,
                            "semanticType": "marker",
                            "name": null,
                            "style": null,
                            "label": null,
                            "custom": {}
                        }
                    },
                    "geometry": {"type": "MultiPoint", "coordinates": [[2.0, 1.0], [1.0, 2.0]]}
                },
                {
                    "type": "Feature",
                    "id": "018f89ec-25fc-7816-8b47-6f80905f2803",
                    "properties": {
                        "daena": {
                            "layerId": layer,
                            "semanticType": "route",
                            "name": "Trail",
                            "style": null,
                            "label": null,
                            "custom": {"difficulty": 2}
                        }
                    },
                    "geometry": {
                        "type": "MultiLineString",
                        "coordinates": [[[4.0, 4.0], [5.0, 5.0]], [[0.0, 0.0], [1.0, 1.0]]]
                    }
                }
            ]
        });
        let first = canonicalize_committed(&serde_json::to_vec(&input).unwrap(), &known).unwrap();
        let second = canonicalize_committed(&first, &known).unwrap();
        assert_eq!(first, second);
        let value: Value = serde_json::from_slice(&first).unwrap();
        assert_eq!(
            value["features"][0]["geometry"]["coordinates"],
            serde_json::json!([[1, 2], [2, 1]])
        );
        assert_eq!(
            value["features"][1]["geometry"]["coordinates"],
            serde_json::json!([[[0, 0], [1, 1]], [[4, 4], [5, 5]]])
        );
        assert_eq!(
            value["features"][1]["properties"]["daena"]["custom"]["difficulty"],
            2
        );
    }

    #[test]
    fn candidate_assigns_base_land_and_ids() {
        let candidate = serde_json::json!({
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "properties": {},
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [[[0.0,0.0],[2.0,0.0],[2.0,2.0],[0.0,2.0],[0.0,0.0]]]
                }
            }]
        });
        let bytes = canonicalize_candidate(&serde_json::to_vec(&candidate).unwrap()).unwrap();
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        let feature = &value["features"][0];
        assert_eq!(feature["properties"]["daena"]["layerId"], "base");
        assert_eq!(feature["properties"]["daena"]["semanticType"], "land");
        assert!(Uuid::parse_str(feature["id"].as_str().unwrap()).is_ok());
    }

    #[test]
    fn imported_base_strips_ids_and_properties() {
        let input = serde_json::json!({
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "id": "018f89ec-25fc-7816-8b47-6f80905f2869",
                "properties": {"name": "Continent"},
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [[[0.0,0.0],[2.0,0.0],[2.0,2.0],[0.0,2.0],[0.0,0.0]]]
                }
            }]
        });
        let bytes = canonicalize_imported_base(&serde_json::to_vec(&input).unwrap()).unwrap();
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        let feature = &value["features"][0];
        assert_eq!(feature["properties"]["daena"]["layerId"], "base");
        assert_eq!(feature["properties"]["daena"]["semanticType"], "land");
        assert_eq!(feature["properties"]["daena"]["name"], Value::Null);
        assert_ne!(feature["id"], "018f89ec-25fc-7816-8b47-6f80905f2869");
        assert!(canonicalize_imported_base(
            &serde_json::to_vec(&serde_json::json!({
                "type": "FeatureCollection",
                "features": [{
                    "type": "Feature",
                    "properties": {},
                    "geometry": {"type": "Point", "coordinates": [1.0, 2.0]}
                }]
            }))
            .unwrap()
        )
        .unwrap_err()
        .to_string()
        .contains("polygonal"));
    }

    #[test]
    fn rejects_antimeridian_crossing_line_and_ring() {
        let known = BTreeSet::new();
        let line = serde_json::json!({
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "id": "018f89ec-25fc-7816-8b47-6f80905f2802",
                "properties": {
                    "daena": {
                        "layerId": "base",
                        "semanticType": "route",
                        "name": null,
                        "style": null,
                        "label": null,
                        "custom": {}
                    }
                },
                "geometry": {
                    "type": "LineString",
                    "coordinates": [[170.0, 0.0], [-170.0, 0.0]]
                }
            }]
        });
        assert!(
            canonicalize_committed(&serde_json::to_vec(&line).unwrap(), &known)
                .unwrap_err()
                .to_string()
                .contains(CODE_ANTIMERIDIAN)
        );

        let polygon = serde_json::json!({
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "id": "018f89ec-25fc-7816-8b47-6f80905f2803",
                "properties": {
                    "daena": {
                        "layerId": "base",
                        "semanticType": "region",
                        "name": null,
                        "style": null,
                        "label": null,
                        "custom": {}
                    }
                },
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [[[170.0, -10.0], [-170.0, -10.0], [-170.0, 10.0], [170.0, 10.0], [170.0, -10.0]]]
                }
            }]
        });
        assert!(
            canonicalize_committed(&serde_json::to_vec(&polygon).unwrap(), &known)
                .unwrap_err()
                .to_string()
                .contains(CODE_ANTIMERIDIAN)
        );
    }

    #[test]
    fn rebuild_rejects_noncanonical_bytes() {
        let canonical =
            canonicalize_committed(&serde_json::to_vec(&square()).unwrap(), &BTreeSet::new())
                .unwrap();
        let mut dirty = canonical.clone();
        dirty.pop();
        dirty.extend_from_slice(b" \n");
        let error = require_canonical_bytes(
            Path::new("assets/maps/map.geojson"),
            &dirty,
            &BTreeSet::new(),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("assets/maps/map.geojson"));
        assert!(error.contains(CODE_SOURCE_INVALID));
    }
}
