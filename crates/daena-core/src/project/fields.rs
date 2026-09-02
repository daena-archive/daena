// Field, document, calendar, and relationship-metadata helpers.
use super::fs_util::digest_bytes;
use super::EXTERNAL_IMPORT_SOURCE_KEY_PREFIX;
use crate::error::CoreError;
use daena_plugin_api::MetadataFieldDefinition;
use std::collections::BTreeSet;

pub(super) fn decode_field_value(value: String) -> serde_json::Value {
    serde_json::from_str(&value).unwrap_or(serde_json::Value::String(value))
}

pub(super) fn encode_field_value(value: &serde_json::Value) -> Result<String, CoreError> {
    serde_json::to_string(value).map_err(|error| CoreError::NotFound(error.to_string()))
}

pub(super) fn encode_asset_provenance(
    provenance: &Option<serde_json::Value>,
) -> Result<Option<String>, CoreError> {
    const MAX_ASSET_PROVENANCE_BYTES: usize = 256 * 1024;
    let Some(value) = provenance else {
        return Ok(None);
    };
    if !value.is_object() {
        return Err(CoreError::Validation(
            "asset provenance must be a JSON object".into(),
        ));
    }
    let encoded = serde_json::to_string(value)
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
    if encoded.len() > MAX_ASSET_PROVENANCE_BYTES {
        return Err(CoreError::Validation(
            "asset provenance exceeds 256 KiB".into(),
        ));
    }
    Ok(Some(encoded))
}

pub(super) fn decode_asset_provenance(
    row: &rusqlite::Row<'_>,
    index: usize,
) -> rusqlite::Result<Option<serde_json::Value>> {
    let encoded = row.get::<_, Option<String>>(index)?;
    encoded
        .map(|value| {
            serde_json::from_str(&value).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    index,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })
        })
        .transpose()
}

pub(super) fn external_import_source_key(importer_id: &str, source_id: &str) -> String {
    let fingerprint = digest_bytes(format!("{importer_id}\0{source_id}").as_bytes());
    format!("{EXTERNAL_IMPORT_SOURCE_KEY_PREFIX}{}", &fingerprint[..24])
}

pub(super) fn validate_document_format(
    format: Option<&str>,
    directory_backed: bool,
) -> Result<(), CoreError> {
    let format = format.unwrap_or("markdown");
    if directory_backed && format != "markdown" {
        return Err(CoreError::Validation(
            "directory-backed projects require Markdown documents".into(),
        ));
    }
    if format != "markdown" && format != "plain-text" && format != "rich-text" {
        return Err(CoreError::NotFound("unsupported document format".into()));
    }
    Ok(())
}

pub(super) fn is_leap_year(year: u32) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

pub(super) fn valid_calendar_date(year: u32, month: u32, day: u32) -> bool {
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days).contains(&day)
}

pub(super) fn parse_fixed_decimal(value: &[u8], start: usize, end: usize) -> Option<u32> {
    if end > value.len() || end <= start || !value[start..end].iter().all(u8::is_ascii_digit) {
        return None;
    }
    std::str::from_utf8(&value[start..end]).ok()?.parse().ok()
}

pub(super) fn is_rfc3339_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() < 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes[..4].iter().all(u8::is_ascii_digit)
        || !bytes[5..7].iter().all(u8::is_ascii_digit)
        || !bytes[8..10].iter().all(u8::is_ascii_digit)
    {
        return false;
    }
    let year = parse_fixed_decimal(bytes, 0, 4).unwrap_or_default();
    let month = parse_fixed_decimal(bytes, 5, 7).unwrap_or_default();
    let day = parse_fixed_decimal(bytes, 8, 10).unwrap_or_default();
    if !valid_calendar_date(year, month, day) {
        return false;
    }
    if bytes.len() == 10 {
        return true;
    }
    if bytes.get(10) != Some(&b'T') {
        return false;
    }
    if bytes.len() < 20 || bytes[13] != b':' || bytes[16] != b':' {
        return false;
    }
    let hour = parse_fixed_decimal(bytes, 11, 13).unwrap_or_default();
    let minute = parse_fixed_decimal(bytes, 14, 16).unwrap_or_default();
    let second = parse_fixed_decimal(bytes, 17, 19).unwrap_or_default();
    if hour > 23 || minute > 59 || second > 59 {
        return false;
    }
    let mut index = 19;
    if bytes.get(index) == Some(&b'.') {
        index += 1;
        let fraction_start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        if fraction_start == index {
            return false;
        }
    }
    match bytes.get(index) {
        Some(b'Z') => index + 1 == bytes.len(),
        Some(b'+' | b'-') => {
            index + 6 == bytes.len()
                && bytes[index + 3] == b':'
                && parse_fixed_decimal(bytes, index + 1, index + 3).is_some_and(|hour| hour <= 23)
                && parse_fixed_decimal(bytes, index + 4, index + 6)
                    .is_some_and(|minute| minute <= 59)
        }
        _ => false,
    }
}

pub(super) fn is_gregorian_date_string(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.contains('T') {
        // With time, require full date and use strict RFC3339 validation
        return is_rfc3339_date(trimmed);
    }
    let parts: Vec<&str> = trimmed.split('-').collect();
    match parts.len() {
        1 => parts[0].parse::<u32>().is_ok(),
        2 => {
            let Ok(_year) = parts[0].parse::<u32>() else {
                return false;
            };
            let Ok(month) = parts[1].parse::<u32>() else {
                return false;
            };
            (1..=12).contains(&month)
        }
        3 => {
            let Ok(year) = parts[0].parse::<u32>() else {
                return false;
            };
            let Ok(month) = parts[1].parse::<u32>() else {
                return false;
            };
            let Ok(day) = parts[2].parse::<u32>() else {
                return false;
            };
            valid_calendar_date(year, month, day)
        }
        _ => false,
    }
}

pub(super) fn is_calendar_date_object(value: &serde_json::Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    let Some(year) = object.get("year").and_then(|candidate| {
        candidate
            .as_i64()
            .or(candidate.as_u64().map(|value| value as i64))
    }) else {
        return false;
    };
    let _ = year;
    if let Some(calendar) = object.get("calendar") {
        if !calendar.is_string() {
            return false;
        }
    }
    for key in ["month", "day", "hour", "minute", "second"] {
        if let Some(candidate) = object.get(key) {
            if candidate.as_u64().is_none() && candidate.as_i64().is_none() {
                return false;
            }
        }
    }
    if let Some(precision) = object.get("precision").and_then(|v| v.as_str()) {
        let has_month = object.contains_key("month");
        let has_day = object.contains_key("day");
        let has_hour = object.contains_key("hour");
        match precision {
            "year" => {
                if has_month || has_day || has_hour {
                    return false;
                }
            }
            "month" => {
                if !has_month || has_day {
                    return false;
                }
            }
            "day" => {
                if !has_month || !has_day {
                    return false;
                }
            }
            "hour" | "minute" | "second" => {
                if !has_month || !has_day || !has_hour {
                    return false;
                }
            }
            _ => return false,
        }
    }
    true
}

pub(super) fn validate_relationship_metadata(
    relationship_type: &str,
    metadata: &serde_json::Value,
    declared: Option<&[MetadataFieldDefinition]>,
) -> Result<serde_json::Value, CoreError> {
    let object = metadata.as_object().ok_or_else(|| {
        CoreError::Validation(format!(
            "relationship metadata for {relationship_type} must be a JSON object"
        ))
    })?;
    let Some(declared) = declared.filter(|fields| !fields.is_empty()) else {
        return Ok(metadata.clone());
    };
    let declared_keys = declared
        .iter()
        .map(|field| field.key.as_str())
        .collect::<BTreeSet<_>>();
    let mut cleaned = object
        .iter()
        .filter(|(key, _)| !declared_keys.contains(key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<serde_json::Map<_, _>>();
    for field in declared {
        let Some(value) = object.get(&field.key) else {
            if field.required.unwrap_or(false) {
                return Err(CoreError::Validation(format!(
                    "relationship {relationship_type} is missing required metadata field: {}",
                    field.key
                )));
            }
            continue;
        };
        if value.is_null() {
            if field.required.unwrap_or(false) {
                return Err(CoreError::Validation(format!(
                    "relationship {relationship_type} is missing required metadata field: {}",
                    field.key
                )));
            }
            continue;
        }
        let valid = match field.field_type.as_str() {
            "text" => value.as_str().is_some(),
            "number" => value.is_number(),
            "boolean" => value.is_boolean(),
            "date" => {
                value.as_str().is_some_and(is_gregorian_date_string)
                    || is_calendar_date_object(value)
            }
            "enum" => value.as_str().is_some_and(|candidate| {
                field
                    .options
                    .as_ref()
                    .is_some_and(|options| options.iter().any(|option| option == candidate))
            }),
            "oneof" => {
                let mut all_options = field.options.clone().unwrap_or_default();
                if let Some(one_of) = &field.one_of {
                    for variant in one_of {
                        if let Some(opts) = &variant.options {
                            all_options.extend(opts.clone());
                        }
                    }
                }
                value
                    .as_str()
                    .is_some_and(|candidate| all_options.iter().any(|opt| opt == candidate))
            }
            _ => false,
        };
        if !valid {
            return Err(CoreError::Validation(format!(
                "relationship {relationship_type} metadata field {} has the wrong type or value",
                field.key
            )));
        }
        if field.required.unwrap_or(false)
            && field.field_type == "text"
            && value.as_str().is_some_and(str::is_empty)
        {
            return Err(CoreError::Validation(format!(
                "relationship {relationship_type} metadata field {} is required",
                field.key
            )));
        }
        cleaned.insert(field.key.clone(), value.clone());
    }
    Ok(serde_json::Value::Object(cleaned))
}
