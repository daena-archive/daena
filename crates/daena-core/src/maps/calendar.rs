//! Shared atlas calendar math. Does not use civil time or JavaScript dates.

use crate::error::CoreError;
use serde::{Deserialize, Serialize};

pub const PHYSICAL_CALENDAR_BINDING_KEY: &str = "physicalCalendarBinding";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PhysicalCalendarBinding {
    pub schema_version: u32,
    pub calendar_id: String,
    pub calendar_reference_year: i64,
    pub physical_offset_at_reference: i64,
    pub has_year_zero: bool,
}

impl PhysicalCalendarBinding {
    pub fn validate(&self) -> Result<(), CoreError> {
        if self.schema_version != 1 {
            return Err(invalid("unsupported physical calendar binding schema"));
        }
        if self.calendar_id.is_empty() || self.calendar_id.len() > 128 {
            return Err(invalid("calendarId must be 1..=128 characters"));
        }
        year_index(self.calendar_reference_year, self.has_year_zero)?;
        if !(-daena_physical::history::MAX_HISTORICAL_OFFSET_YEARS
            ..=daena_physical::history::MAX_HISTORICAL_OFFSET_YEARS)
            .contains(&self.physical_offset_at_reference)
        {
            return Err(invalid("physicalOffsetAtReference is out of range"));
        }
        Ok(())
    }
}

pub fn year_index(year: i64, has_year_zero: bool) -> Result<i64, CoreError> {
    if !has_year_zero && year == 0 {
        return Err(invalid(
            "this calendar has no year zero; use -1 for 1 BCE and 1 for 1 CE",
        ));
    }
    if has_year_zero || year > 0 {
        Ok(year)
    } else {
        year.checked_add(1)
            .ok_or_else(|| invalid("authored year overflowed"))
    }
}

pub fn physical_offset_for_authored_year(
    authored_year: i64,
    binding: &PhysicalCalendarBinding,
) -> Result<i64, CoreError> {
    binding.validate()?;
    let selected = year_index(authored_year, binding.has_year_zero)?;
    let reference = year_index(binding.calendar_reference_year, binding.has_year_zero)?;
    selected
        .checked_sub(reference)
        .and_then(|delta| delta.checked_add(binding.physical_offset_at_reference))
        .ok_or_else(|| invalid("authored year overflowed the physical offset range"))
}

pub fn signed_year_from_date(value: &serde_json::Value) -> Option<i64> {
    let year = value.get("year").and_then(serde_json::Value::as_i64)?;
    match value.get("era").and_then(serde_json::Value::as_str) {
        Some("BCE") => Some(-year),
        Some("CE") | None => Some(year),
        _ => None,
    }
}

pub fn year_in_interval(
    authored_year: i64,
    from: Option<&serde_json::Value>,
    to: Option<&serde_json::Value>,
    has_year_zero: bool,
) -> bool {
    let Ok(selected) = year_index(authored_year, has_year_zero) else {
        return false;
    };
    if let Some(from) = from.and_then(signed_year_from_date) {
        if let Ok(start) = year_index(from, has_year_zero) {
            if selected < start {
                return false;
            }
        }
    }
    if let Some(to) = to.and_then(signed_year_from_date) {
        if let Ok(end) = year_index(to, has_year_zero) {
            if selected > end {
                return false;
            }
        }
    }
    true
}

fn invalid(message: &str) -> CoreError {
    CoreError::Validation(format!("atlas.calendar.invalid: {message}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binding(reference: i64, has_year_zero: bool) -> PhysicalCalendarBinding {
        PhysicalCalendarBinding {
            schema_version: 1,
            calendar_id: "project-calendar".into(),
            calendar_reference_year: reference,
            physical_offset_at_reference: 0,
            has_year_zero,
        }
    }

    #[test]
    fn authored_years_map_without_javascript_dates() {
        let none = binding(1, false);
        assert_eq!(physical_offset_for_authored_year(1, &none).unwrap(), 0);
        assert_eq!(physical_offset_for_authored_year(42, &none).unwrap(), 41);
        assert_eq!(physical_offset_for_authored_year(-1, &none).unwrap(), -1);
        assert!(physical_offset_for_authored_year(0, &none).is_err());
        let zero = binding(0, true);
        assert_eq!(physical_offset_for_authored_year(0, &zero).unwrap(), 0);
        assert_eq!(physical_offset_for_authored_year(-5, &zero).unwrap(), -5);
    }
}
