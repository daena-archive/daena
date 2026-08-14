use super::{MapGeneration, MapGenerationSettings, PhysicalMapGenerationSettings};
use crate::error::CoreError;
use daena_physical_spike::{decode_source, validate_field_report, PhysicalField};
use serde_json::Value;

pub const CODE_INVALID_GENERATION: &str = "maps.physical.invalid-generation";
pub const CODE_INVALID_SOURCE: &str = "maps.physical.invalid-source";

fn invalid(code: &str, message: impl Into<String>) -> CoreError {
    CoreError::Validation(format!("{code}: {}", message.into()))
}

pub fn validate_generation(value: &Value) -> Result<PhysicalMapGenerationSettings, CoreError> {
    let generation: MapGeneration = serde_json::from_value(value.clone())
        .map_err(|error| invalid(CODE_INVALID_GENERATION, error.to_string()))?;
    if generation.id != super::PHYSICAL_GENERATOR_ID
        || generation.version != super::PHYSICAL_GENERATOR_VERSION
        || generation.retry_index.is_none()
    {
        return Err(invalid(
            CODE_INVALID_GENERATION,
            "generator id, version, and retryIndex are unsupported",
        ));
    }
    let MapGenerationSettings::Physical(settings) = generation.settings else {
        return Err(invalid(
            CODE_INVALID_GENERATION,
            "physical generation settings are required",
        ));
    };
    daena_physical_spike::Grid::new(settings.width, settings.height, settings.radius_metres)
        .map_err(|error| invalid(CODE_INVALID_GENERATION, error))?;
    if settings.reference_water_inventory_m3 == 0 {
        return Err(invalid(
            CODE_INVALID_GENERATION,
            "referenceWaterInventoryM3 must be positive",
        ));
    }
    Ok(settings)
}

pub fn validate_source(
    bytes: &[u8],
    generation: &Value,
) -> Result<(PhysicalField, daena_physical_spike::ValidationReport), CoreError> {
    let settings = validate_generation(generation)?;
    let parsed = decode_source(bytes).map_err(|error| invalid(CODE_INVALID_SOURCE, error))?;
    let parsed_generation: MapGeneration = serde_json::from_value(generation.clone())
        .map_err(|error| invalid(CODE_INVALID_GENERATION, error.to_string()))?;
    let retry_index = parsed_generation.retry_index.ok_or_else(|| {
        invalid(
            CODE_INVALID_GENERATION,
            "retryIndex is required for physical sources",
        )
    })?;
    if parsed.seed != parsed_generation.seed
        || parsed.retry_index != retry_index
        || parsed.grid.width != settings.width
        || parsed.grid.height != settings.height
        || parsed.grid.radius_metres != settings.radius_metres
        || parsed.target_land_fraction_ppm != settings.target_land_fraction_ppm
    {
        return Err(invalid(
            CODE_INVALID_SOURCE,
            "source provenance does not match the physical generation descriptor",
        ));
    }
    let report = validate_field_report(&parsed)
        .map_err(|error| invalid(CODE_INVALID_SOURCE, error.to_string()))?;
    if report.reference_water_inventory_m3 != settings.reference_water_inventory_m3 {
        return Err(invalid(
            CODE_INVALID_SOURCE,
            "reference water inventory does not match the source field",
        ));
    }
    Ok((parsed, report))
}
