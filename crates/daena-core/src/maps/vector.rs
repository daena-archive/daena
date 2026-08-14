use crate::error::CoreError;
use serde::de::{self, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fmt;
use std::path::Path;
use uuid::Uuid;

pub const VECTOR_PROVIDER: &str = "daena-vector";
pub const VECTOR_SOURCE_FORMAT: &str = "geojson";
pub const VECTOR_MIME: &str = "application/geo+json";
pub const VECTOR_FILENAME: &str = "map.geojson";
pub const VECTOR_MAX_BYTES: usize = 16 * 1024 * 1024;
pub const VECTOR_MAX_FEATURES: usize = 20_000;
pub const VECTOR_MAX_POSITIONS: usize = 200_000;
pub const VECTOR_MAX_FEATURE_POSITIONS: usize = 20_000;
pub const VECTOR_MAX_RINGS: usize = 256;
pub const VECTOR_MAX_LAYERS: usize = 64;
pub const VECTOR_MAX_PROPERTY_BYTES: usize = 2 * 1024;
pub const WEB_MERCATOR_MAX_LAT: f64 = 85.05112878;
pub const VECTOR_CENTER_Y_MIN: f64 = 0.027493729;
pub const VECTOR_CENTER_Y_MAX: f64 = 0.972506271;
const SCALE: i32 = 1_000_000;
const LAT_LIMIT: i32 = 85_051_129;
const LON_LIMIT: i32 = 180_000_000;
const ANTIMERIDIAN: i32 = 180_000_000;

pub const CODE_SOURCE_INVALID: &str = "vector.source.invalid";
pub const CODE_UNSUPPORTED_VERSION: &str = "vector.source.unsupported-version";
pub const CODE_GEOMETRY_INVALID: &str = "vector.geometry.invalid";
pub const CODE_ANTIMERIDIAN: &str = "vector.geometry.antimeridian";
pub const CODE_LIMIT: &str = "vector.limit.exceeded";
pub const CODE_LAYER_MISSING: &str = "vector.layer.missing";
pub const CODE_GENERATOR: &str = "vector.generator.invalid-settings";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Micro(pub i32, pub i32);

#[derive(Debug, Clone, PartialEq, Eq)]
enum Geometry {
    Point(Micro),
    LineString(Vec<Micro>),
    Polygon { exterior: Vec<Micro>, holes: Vec<Vec<Micro>> },
    MultiPolygon(Vec<(Vec<Micro>, Vec<Vec<Micro>>)>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Feature {
    id: String,
    layer_id: String,
    kind: String,
    name: Option<String>,
    geometry: Geometry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceProfile {
    Candidate,
    Committed,
}

pub fn lon_lat_to_normalized(longitude: f64, latitude: f64) -> (f64, f64) {
    ((longitude + 180.0) / 360.0, (90.0 - latitude) / 180.0)
}

pub fn fail(code: &str, path: &str, detail: impl fmt::Display) -> CoreError {
    if path.is_empty() {
        CoreError::Validation(format!("{code}: {detail}"))
    } else {
        CoreError::Validation(format!("{code}: {path}: {detail}"))
    }
}

pub fn path_fail(fs_path: &Path, code: &str, json_path: &str, detail: impl fmt::Display) -> CoreError {
    CoreError::Validation(format!(
        "{} [{code}] {json_path}: {detail}",
        fs_path.display()
    ))
}

fn to_micro(value: f64) -> i32 {
    let scaled = value * f64::from(SCALE);
    let rounded = scaled.round();
    if rounded == 0.0 {
        0
    } else {
        rounded as i32
    }
}

fn format_micro(value: i32) -> String {
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
            serde_json::Number::from_f64(value).ok_or_else(|| de::Error::custom("non-finite numbers are not allowed"))?,
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

fn object_keys<'a>(value: &'a Value, path: &str, allowed: &[&str]) -> Result<&'a serde_json::Map<String, Value>, CoreError> {
    let object = value
        .as_object()
        .ok_or_else(|| fail(CODE_SOURCE_INVALID, path, "expected an object"))?;
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(fail(CODE_SOURCE_INVALID, path, format!("unknown member `{key}`")));
        }
    }
    Ok(object)
}

fn require_type(object: &serde_json::Map<String, Value>, path: &str, expected: &str) -> Result<(), CoreError> {
    match object.get("type").and_then(Value::as_str) {
        Some(found) if found == expected => Ok(()),
        Some(_) => Err(fail(CODE_SOURCE_INVALID, &format!("{path}.type"), format!("must be {expected}"))),
        None => Err(fail(CODE_SOURCE_INVALID, &format!("{path}.type"), "is required")),
    }
}

fn as_f64(value: &Value, path: &str) -> Result<f64, CoreError> {
    value
        .as_f64()
        .ok_or_else(|| fail(CODE_SOURCE_INVALID, path, "coordinates must be finite numbers"))
}

fn parse_position(value: &Value, path: &str) -> Result<Micro, CoreError> {
    let pair = value
        .as_array()
        .ok_or_else(|| fail(CODE_GEOMETRY_INVALID, path, "position must be an array"))?;
    if pair.len() != 2 {
        return Err(fail(CODE_GEOMETRY_INVALID, path, "position must be [longitude, latitude]"));
    }
    let longitude = as_f64(&pair[0], &format!("{path}[0]"))?;
    let latitude = as_f64(&pair[1], &format!("{path}[1]"))?;
    let coord = Micro(to_micro(longitude), to_micro(latitude));
    if coord.0.abs() > LON_LIMIT {
        return Err(fail(CODE_GEOMETRY_INVALID, path, "longitude must be in [-180, 180]"));
    }
    if coord.1.abs() > LAT_LIMIT {
        return Err(fail(CODE_GEOMETRY_INVALID, path, "latitude exceeds the Web Mercator limit"));
    }
    Ok(coord)
}

fn parse_line(values: &[Value], path: &str) -> Result<Vec<Micro>, CoreError> {
    if values.len() < 2 {
        return Err(fail(CODE_GEOMETRY_INVALID, path, "LineString requires at least two positions"));
    }
    values
        .iter()
        .enumerate()
        .map(|(index, value)| parse_position(value, &format!("{path}[{index}]")))
        .collect()
}

fn parse_ring(values: &[Value], path: &str) -> Result<Vec<Micro>, CoreError> {
    if values.len() < 4 {
        return Err(fail(CODE_GEOMETRY_INVALID, path, "polygon rings must contain at least four positions"));
    }
    let ring = values
        .iter()
        .enumerate()
        .map(|(index, value)| parse_position(value, &format!("{path}[{index}]")))
        .collect::<Result<Vec<_>, _>>()?;
    if ring.first() != ring.last() {
        return Err(fail(CODE_GEOMETRY_INVALID, path, "polygon rings must be closed"));
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
    (i64::from(a.0) - i64::from(b.0)).abs() > i64::from(ANTIMERIDIAN)
}

fn validate_line(line: &[Micro], path: &str) -> Result<(), CoreError> {
    if line.len() < 2 {
        return Err(fail(CODE_GEOMETRY_INVALID, path, "line requires at least two distinct positions"));
    }
    for pair in line.windows(2) {
        if crosses_antimeridian(pair[0], pair[1]) {
            return Err(fail(CODE_ANTIMERIDIAN, path, "segment crosses the antimeridian"));
        }
    }
    Ok(())
}

fn canonical_ring(mut ring: Vec<Micro>, path: &str, hole: bool) -> Result<Vec<Micro>, CoreError> {
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
        return Err(fail(CODE_GEOMETRY_INVALID, path, "ring requires at least three distinct positions"));
    }
    for pair in ring.windows(2) {
        if crosses_antimeridian(pair[0], pair[1]) {
            return Err(fail(CODE_ANTIMERIDIAN, path, "segment crosses the antimeridian"));
        }
    }
    let min_lon = open.iter().map(|coord| coord.0).min().unwrap();
    let max_lon = open.iter().map(|coord| coord.0).max().unwrap();
    if i64::from(max_lon) - i64::from(min_lon) > i64::from(ANTIMERIDIAN) {
        return Err(fail(CODE_ANTIMERIDIAN, path, "ring longitude span exceeds 180 degrees"));
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
                return Err(fail(CODE_GEOMETRY_INVALID, path, "ring is self-intersecting"));
            }
        }
    }
    let area = signed_area(&close_ring(open.clone()));
    if area == 0 {
        return Err(fail(CODE_GEOMETRY_INVALID, path, "ring has zero signed area"));
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
) -> Result<(Vec<Micro>, Vec<Vec<Micro>>), CoreError> {
    if rings.is_empty() {
        return Err(fail(CODE_GEOMETRY_INVALID, path, "polygon requires an exterior ring"));
    }
    if rings.len() > VECTOR_MAX_RINGS {
        return Err(fail(CODE_LIMIT, path, format!("polygon exceeds {VECTOR_MAX_RINGS} rings")));
    }
    let exterior = canonical_ring(rings[0].clone(), &format!("{path}[0]"), false)?;
    let mut holes = Vec::new();
    for (index, hole) in rings.into_iter().skip(1).enumerate() {
        holes.push(canonical_ring(hole, &format!("{path}[{}]", index + 1), true)?);
    }
    holes.sort_by(|left, right| left.cmp(right));
    Ok((exterior, holes))
}

fn polygon_abs_area(exterior: &[Micro]) -> i128 {
    signed_area(exterior).abs()
}

fn count_positions(geometry: &Geometry) -> usize {
    match geometry {
        Geometry::Point(_) => 1,
        Geometry::LineString(line) => line.len(),
        Geometry::Polygon { exterior, holes } => exterior.len() + holes.iter().map(Vec::len).sum::<usize>(),
        Geometry::MultiPolygon(members) => members
            .iter()
            .map(|(exterior, holes)| exterior.len() + holes.iter().map(Vec::len).sum::<usize>())
            .sum(),
    }
}

fn canonical_uuid(value: &str, path: &str) -> Result<String, CoreError> {
    let uuid = Uuid::parse_str(value).map_err(|_| fail(CODE_SOURCE_INVALID, path, "feature id must be a UUID"))?;
    let text = uuid.to_string();
    if text != value {
        return Err(fail(CODE_SOURCE_INVALID, path, "feature id must be lowercase hyphenated UUID text"));
    }
    Ok(text)
}

fn parse_kind(value: Option<&Value>, path: &str) -> Result<String, CoreError> {
    let kind = value
        .and_then(Value::as_str)
        .ok_or_else(|| fail(CODE_SOURCE_INVALID, path, "kind is required"))?;
    if matches!(kind, "land" | "lake" | "region" | "route" | "marker" | "custom") {
        Ok(kind.to_owned())
    } else {
        Err(fail(CODE_SOURCE_INVALID, path, "kind is not supported"))
    }
}

fn parse_name(value: Option<&Value>, path: &str) -> Result<Option<String>, CoreError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(name)) => {
            if name.is_empty() || name.chars().count() > 256 {
                return Err(fail(CODE_SOURCE_INVALID, path, "name must be null or 1..=256 Unicode scalars"));
            }
            Ok(Some(name.clone()))
        }
        Some(_) => Err(fail(CODE_SOURCE_INVALID, path, "name must be a string or null")),
    }
}

fn parse_geometry(value: &Value, path: &str) -> Result<Geometry, CoreError> {
    let object = object_keys(value, path, &["type", "coordinates"])?;
    let kind = object
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| fail(CODE_SOURCE_INVALID, &format!("{path}.type"), "is required"))?;
    let coordinates = object
        .get("coordinates")
        .ok_or_else(|| fail(CODE_SOURCE_INVALID, &format!("{path}.coordinates"), "is required"))?;
    match kind {
        "Point" => Ok(Geometry::Point(parse_position(coordinates, &format!("{path}.coordinates"))?)),
        "LineString" => {
            let values = coordinates
                .as_array()
                .ok_or_else(|| fail(CODE_GEOMETRY_INVALID, &format!("{path}.coordinates"), "must be an array"))?;
            let line = dedup_adjacent(parse_line(values, &format!("{path}.coordinates"))?);
            validate_line(&line, &format!("{path}.coordinates"))?;
            Ok(Geometry::LineString(line))
        }
        "Polygon" => {
            let values = coordinates
                .as_array()
                .ok_or_else(|| fail(CODE_GEOMETRY_INVALID, &format!("{path}.coordinates"), "must be an array"))?;
            let rings = values
                .iter()
                .enumerate()
                .map(|(index, ring)| parse_ring(ring.as_array().ok_or_else(|| fail(CODE_GEOMETRY_INVALID, &format!("{path}.coordinates[{index}]"), "ring must be an array"))?, &format!("{path}.coordinates[{index}]")))
                .collect::<Result<Vec<_>, _>>()?;
            let (exterior, holes) = canonical_polygon(rings, &format!("{path}.coordinates"))?;
            Ok(Geometry::Polygon { exterior, holes })
        }
        "MultiPolygon" => {
            let values = coordinates
                .as_array()
                .ok_or_else(|| fail(CODE_GEOMETRY_INVALID, &format!("{path}.coordinates"), "must be an array"))?;
            let mut members = Vec::new();
            for (index, polygon) in values.iter().enumerate() {
                let rings = polygon
                    .as_array()
                    .ok_or_else(|| fail(CODE_GEOMETRY_INVALID, &format!("{path}.coordinates[{index}]"), "must be an array"))?
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
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                members.push(canonical_polygon(rings, &format!("{path}.coordinates[{index}]"))?);
            }
            members.sort_by(|left, right| {
                polygon_abs_area(&right.0)
                    .cmp(&polygon_abs_area(&left.0))
                    .then_with(|| left.0.cmp(&right.0))
                    .then_with(|| left.1.cmp(&right.1))
            });
            Ok(Geometry::MultiPolygon(members))
        }
        "GeometryCollection" => Err(fail(CODE_SOURCE_INVALID, path, "GeometryCollection is not allowed")),
        _ => Err(fail(CODE_SOURCE_INVALID, &format!("{path}.type"), "unsupported geometry type")),
    }
}

fn geometry_matches_kind(kind: &str, geometry: &Geometry) -> bool {
    match (kind, geometry) {
        ("land" | "lake" | "region", Geometry::Polygon { .. } | Geometry::MultiPolygon(_)) => true,
        ("route", Geometry::LineString(_)) => true,
        ("marker", Geometry::Point(_)) => true,
        ("custom", _) => true,
        _ => false,
    }
}

fn parse_feature(value: &Value, path: &str, profile: SourceProfile, known_layers: &BTreeSet<String>) -> Result<Feature, CoreError> {
    let allowed = match profile {
        SourceProfile::Candidate => ["type", "geometry", "properties"].as_slice(),
        SourceProfile::Committed => ["type", "id", "properties", "geometry"].as_slice(),
    };
    let object = object_keys(value, path, allowed)?;
    require_type(object, path, "Feature")?;
    if profile == SourceProfile::Candidate && object.contains_key("id") {
        return Err(fail(CODE_SOURCE_INVALID, &format!("{path}.id"), "candidate features must not include ids"));
    }
    let geometry = parse_geometry(
        object.get("geometry").ok_or_else(|| fail(CODE_SOURCE_INVALID, &format!("{path}.geometry"), "is required"))?,
        &format!("{path}.geometry"),
    )?;
    if count_positions(&geometry) > VECTOR_MAX_FEATURE_POSITIONS {
        return Err(fail(CODE_LIMIT, path, format!("feature exceeds {VECTOR_MAX_FEATURE_POSITIONS} positions")));
    }
    let properties = object.get("properties").unwrap_or(&Value::Null);
    match profile {
        SourceProfile::Candidate => {
            if properties.as_object().is_none_or(|object| !object.is_empty()) && !properties.is_null() {
                let object = object_keys(properties, &format!("{path}.properties"), &[])?;
                if !object.is_empty() {
                    return Err(fail(CODE_SOURCE_INVALID, &format!("{path}.properties"), "candidate properties must be empty"));
                }
            }
            if !matches!(geometry, Geometry::Polygon { .. } | Geometry::MultiPolygon(_)) {
                return Err(fail(CODE_SOURCE_INVALID, &format!("{path}.geometry"), "candidates must be polygonal"));
            }
            Ok(Feature {
                id: Uuid::new_v4().to_string(),
                layer_id: "base".into(),
                kind: "land".into(),
                name: None,
                geometry,
            })
        }
        SourceProfile::Committed => {
            let id = canonical_uuid(
                object.get("id").and_then(Value::as_str).ok_or_else(|| fail(CODE_SOURCE_INVALID, &format!("{path}.id"), "is required"))?,
                &format!("{path}.id"),
            )?;
            let properties = object_keys(
                properties,
                &format!("{path}.properties"),
                &["daenaLayerId", "kind", "name"],
            )?;
            let layer_id = properties
                .get("daenaLayerId")
                .and_then(Value::as_str)
                .ok_or_else(|| fail(CODE_SOURCE_INVALID, &format!("{path}.properties.daenaLayerId"), "is required"))?
                .to_owned();
            let kind = parse_kind(properties.get("kind"), &format!("{path}.properties.kind"))?;
            let name = parse_name(properties.get("name"), &format!("{path}.properties.name"))?;
            if layer_id == "base" {
                if !matches!(kind.as_str(), "land" | "lake") || !matches!(geometry, Geometry::Polygon { .. } | Geometry::MultiPolygon(_)) {
                    return Err(fail(CODE_SOURCE_INVALID, path, "base features must be land or lake polygons"));
                }
            } else {
                canonical_uuid(&layer_id, &format!("{path}.properties.daenaLayerId"))?;
                if !known_layers.is_empty() && !known_layers.contains(&layer_id) {
                    return Err(fail(CODE_LAYER_MISSING, &format!("{path}.properties.daenaLayerId"), "layer does not exist"));
                }
                if !geometry_matches_kind(&kind, &geometry) {
                    return Err(fail(CODE_GEOMETRY_INVALID, path, "geometry does not match kind"));
                }
            }
            let encoded = format!(
                "{{\"daenaLayerId\":{},\"kind\":{},\"name\":{}}}",
                serde_json::to_string(&layer_id).unwrap(),
                serde_json::to_string(&kind).unwrap(),
                serde_json::to_string(&name).unwrap()
            );
            if encoded.len() > VECTOR_MAX_PROPERTY_BYTES {
                return Err(fail(CODE_LIMIT, &format!("{path}.properties"), "feature properties exceed 2 KiB"));
            }
            Ok(Feature {
                id,
                layer_id,
                kind,
                name,
                geometry,
            })
        }
    }
}

fn parse_collection(value: &Value, profile: SourceProfile, known_layers: &BTreeSet<String>) -> Result<Vec<Feature>, CoreError> {
    let object = object_keys(value, "$", &["type", "features"])?;
    require_type(object, "$", "FeatureCollection")?;
    let features = object
        .get("features")
        .and_then(Value::as_array)
        .ok_or_else(|| fail(CODE_SOURCE_INVALID, "$.features", "must be an array"))?;
    if features.len() > VECTOR_MAX_FEATURES {
        return Err(fail(CODE_LIMIT, "$.features", format!("exceeds {VECTOR_MAX_FEATURES} features")));
    }
    let mut parsed = Vec::with_capacity(features.len());
    let mut ids = BTreeSet::new();
    let mut positions = 0usize;
    for (index, feature) in features.iter().enumerate() {
        let feature = parse_feature(feature, &format!("$.features[{index}]"), profile, known_layers)?;
        positions += count_positions(&feature.geometry);
        if positions > VECTOR_MAX_POSITIONS {
            return Err(fail(CODE_LIMIT, "$.features", format!("exceeds {VECTOR_MAX_POSITIONS} positions")));
        }
        if profile == SourceProfile::Committed && !ids.insert(feature.id.clone()) {
            return Err(fail(CODE_SOURCE_INVALID, &format!("$.features[{index}].id"), "feature ids must be unique"));
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
        Geometry::LineString(line) => {
            out.push_str("{\"type\":\"LineString\",\"coordinates\":");
            write_positions(out, line);
            out.push('}');
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
        out.push_str(",\"properties\":{\"daenaLayerId\":");
        out.push_str(&serde_json::to_string(&feature.layer_id).unwrap());
        out.push_str(",\"kind\":");
        out.push_str(&serde_json::to_string(&feature.kind).unwrap());
        out.push_str(",\"name\":");
        out.push_str(&serde_json::to_string(&feature.name).unwrap());
        out.push_str("},\"geometry\":");
        write_geometry(&mut out, &feature.geometry);
        out.push('}');
    }
    out.push_str("]}\n");
    out.into_bytes()
}

pub fn canonicalize_committed(bytes: &[u8], known_layers: &BTreeSet<String>) -> Result<Vec<u8>, CoreError> {
    let value = parse_strict_json(bytes)?;
    let features = parse_collection(&value, SourceProfile::Committed, known_layers)?;
    Ok(serialize_features(&features))
}

pub fn canonicalize_candidate(bytes: &[u8]) -> Result<Vec<u8>, CoreError> {
    let value = parse_strict_json(bytes)?;
    let features = parse_collection(&value, SourceProfile::Candidate, &BTreeSet::new())?;
    Ok(serialize_features(&features))
}

pub fn require_canonical_bytes(fs_path: &Path, bytes: &[u8], known_layers: &BTreeSet<String>) -> Result<Vec<u8>, CoreError> {
    let canonical = canonicalize_committed(bytes, known_layers).map_err(|error| match error {
        CoreError::Validation(detail) => path_fail(fs_path, CODE_SOURCE_INVALID, "$", detail),
        other => other,
    })?;
    if canonical.as_slice() != bytes {
        return Err(path_fail(
            fs_path,
            CODE_SOURCE_INVALID,
            "$",
            "GeoJSON source is not byte-canonical for adapter version 1",
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

pub fn remove_layer_features(bytes: &[u8], layer_id: &str, known_layers: &BTreeSet<String>) -> Result<(Vec<u8>, usize), CoreError> {
    let value = parse_strict_json(bytes)?;
    let mut features = parse_collection(&value, SourceProfile::Committed, known_layers)?;
    let before = features.len();
    features.retain(|feature| feature.layer_id != layer_id);
    let deleted = before - features.len();
    Ok((serialize_features(&features), deleted))
}

pub fn feature_bounds(bytes: &[u8], known_layers: &BTreeSet<String>) -> Result<Vec<(String, String, String, f64, f64, f64, f64)>, CoreError> {
    let value = parse_strict_json(bytes)?;
    let features = parse_collection(&value, SourceProfile::Committed, known_layers)?;
    Ok(features
        .into_iter()
        .map(|feature| {
            let mut min_lon = i32::MAX;
            let mut min_lat = i32::MAX;
            let mut max_lon = i32::MIN;
            let mut max_lat = i32::MIN;
            let mut visit = |coord: Micro| {
                min_lon = min_lon.min(coord.0);
                max_lon = max_lon.max(coord.0);
                min_lat = min_lat.min(coord.1);
                max_lat = max_lat.max(coord.1);
            };
            match &feature.geometry {
                Geometry::Point(coord) => visit(*coord),
                Geometry::LineString(line) => line.iter().copied().for_each(&mut visit),
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
            let (min_x, max_y) = lon_lat_to_normalized(f64::from(min_lon) / 1_000_000.0, f64::from(max_lat) / 1_000_000.0);
            let (max_x, min_y) = lon_lat_to_normalized(f64::from(max_lon) / 1_000_000.0, f64::from(min_lat) / 1_000_000.0);
            (feature.id, feature.layer_id, feature.kind, min_x, min_y, max_x, max_y)
        })
        .collect())
}

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
        return Err(fail(CODE_GENERATOR, "generation.id", "must be daena-landmass"));
    }
    if !matches!(
        object.get("version").and_then(Value::as_u64),
        Some(2 | 3)
    ) {
        return Err(fail(
            CODE_UNSUPPORTED_VERSION,
            "generation.version",
            "must be 2 or 3",
        ));
    }
    let seed = object.get("seed").and_then(Value::as_u64).ok_or_else(|| fail(CODE_GENERATOR, "generation.seed", "must be a uint32"))?;
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
        return Err(fail(CODE_GENERATOR, "generation.settings.landPercent", "must be an integer 15..=70"));
    }
    if !matches!(continents, Some(1..=8)) {
        return Err(fail(CODE_GENERATOR, "generation.settings.continentCount", "must be an integer 1..=8"));
    }
    if !matches!(roughness, Some("low" | "medium" | "high")) {
        return Err(fail(CODE_GENERATOR, "generation.settings.coastlineRoughness", "must be low, medium, or high"));
    }
    if !matches!(islands, Some("none" | "low" | "medium" | "high")) {
        return Err(fail(CODE_GENERATOR, "generation.settings.islandFrequency", "must be none, low, medium, or high"));
    }
    let allowed = BTreeSet::from(["id", "version", "seed", "settings"]);
    if object.keys().any(|key| !allowed.contains(key.as_str())) {
        return Err(fail(CODE_GENERATOR, "generation", "contains unknown members"));
    }
    let setting_keys = BTreeSet::from(["landPercent", "continentCount", "coastlineRoughness", "islandFrequency"]);
    if settings.keys().any(|key| !setting_keys.contains(key.as_str())) {
        return Err(fail(CODE_GENERATOR, "generation.settings", "contains unknown members"));
    }
    Ok(())
}

pub fn validate_vector_style(style: &Value) -> Result<(), CoreError> {
    let object = style
        .as_object()
        .ok_or_else(|| fail(CODE_SOURCE_INVALID, "style", "vector style must be an object"))?;
    let required = ["fill", "fillOpacity", "stroke", "strokeWidth", "pointRadius"];
    if object.len() != required.len() || required.iter().any(|key| !object.contains_key(*key)) {
        return Err(fail(CODE_SOURCE_INVALID, "style", "vector style must contain fill, fillOpacity, stroke, strokeWidth, and pointRadius"));
    }
    for key in ["fill", "stroke"] {
        let color = object.get(key).and_then(Value::as_str).ok_or_else(|| fail(CODE_SOURCE_INVALID, key, "must be a hex color"))?;
        if !color.starts_with('#') || color.len() != 7 || !color[1..].chars().all(|ch| ch.is_ascii_hexdigit()) {
            return Err(fail(CODE_SOURCE_INVALID, key, "must match #RRGGBB"));
        }
    }
    let fill_opacity = object.get("fillOpacity").and_then(Value::as_f64).ok_or_else(|| fail(CODE_SOURCE_INVALID, "fillOpacity", "must be a finite number"))?;
    if !fill_opacity.is_finite() || !(0.0..=1.0).contains(&fill_opacity) {
        return Err(fail(CODE_SOURCE_INVALID, "fillOpacity", "must be finite in [0, 1]"));
    }
    let stroke_width = object.get("strokeWidth").and_then(Value::as_f64).ok_or_else(|| fail(CODE_SOURCE_INVALID, "strokeWidth", "must be a finite number"))?;
    if !stroke_width.is_finite() || !(0.0..=32.0).contains(&stroke_width) {
        return Err(fail(CODE_SOURCE_INVALID, "strokeWidth", "must be finite in [0, 32]"));
    }
    let point_radius = object.get("pointRadius").and_then(Value::as_f64).ok_or_else(|| fail(CODE_SOURCE_INVALID, "pointRadius", "must be a finite number"))?;
    if !point_radius.is_finite() || !(1.0..=64.0).contains(&point_radius) {
        return Err(fail(CODE_SOURCE_INVALID, "pointRadius", "must be finite in [1, 64]"));
    }
    Ok(())
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
                "properties": {"daenaLayerId": "base", "kind": "land", "name": null},
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
        assert!(std::str::from_utf8(&first).unwrap().contains("\"type\":\"FeatureCollection\""));
        let expected = concat!(
            "{\"type\":\"FeatureCollection\",\"features\":[{\"type\":\"Feature\",\"id\":\"018f89ec-25fc-7816-8b47-6f80905f2801\",",
            "\"properties\":{\"daenaLayerId\":\"base\",\"kind\":\"land\",\"name\":null},",
            "\"geometry\":{\"type\":\"Polygon\",\"coordinates\":[[[-10,-10],[10,-10],[10,10],[-10,10],[-10,-10]]]}}]}\n"
        );
        assert_eq!(std::str::from_utf8(&first).unwrap(), expected);
    }

    #[test]
    fn rejects_duplicate_keys_and_antimeridian() {
        let duplicate = br#"{"type":"FeatureCollection","type":"FeatureCollection","features":[]}"#;
        assert!(parse_strict_json(duplicate).unwrap_err().to_string().contains(CODE_SOURCE_INVALID));
        let crossing = serde_json::json!({
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "id": "018f89ec-25fc-7816-8b47-6f80905f2801",
                "properties": {"daenaLayerId": "base", "kind": "land", "name": null},
                "geometry": {"type":"Polygon","coordinates":[[[170.0,-1.0],[-170.0,-1.0],[-170.0,1.0],[170.0,1.0],[170.0,-1.0]]]}
            }]
        });
        let error = canonicalize_committed(&serde_json::to_vec(&crossing).unwrap(), &BTreeSet::new())
            .unwrap_err()
            .to_string();
        assert!(error.contains(CODE_ANTIMERIDIAN), "{error}");
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
        assert_eq!(feature["properties"]["daenaLayerId"], "base");
        assert_eq!(feature["properties"]["kind"], "land");
        assert!(Uuid::parse_str(feature["id"].as_str().unwrap()).is_ok());
    }

    #[test]
    fn rebuild_rejects_noncanonical_bytes() {
        let canonical = canonicalize_committed(&serde_json::to_vec(&square()).unwrap(), &BTreeSet::new()).unwrap();
        let mut dirty = canonical.clone();
        dirty.pop();
        dirty.extend_from_slice(b" \n");
        let error = require_canonical_bytes(Path::new("assets/maps/map.geojson"), &dirty, &BTreeSet::new())
            .unwrap_err()
            .to_string();
        assert!(error.contains("assets/maps/map.geojson"));
        assert!(error.contains(CODE_SOURCE_INVALID));
    }
}
