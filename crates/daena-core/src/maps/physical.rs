use super::{MapGeneration, MapGenerationSettings, PhysicalMapGenerationSettings};
use crate::error::CoreError;
use daena_physical_spike::{
    decode_source,
    history::HistoricalForcingParameters,
    tectonics::{TectonicSettings, TectonicWorld},
    validate_field_report,
};
use serde_json::Value;

pub const CODE_INVALID_GENERATION: &str = "physical.generator.invalid-settings";
pub const CODE_UNSUPPORTED_GENERATOR_VERSION: &str = "physical.generator.unsupported-version";
pub const CODE_INVALID_SOURCE: &str = "physical.source.invalid";
pub const CODE_UNSUPPORTED_SOURCE_VERSION: &str = "physical.source.unsupported-version";

pub fn is_reserved_layer_id(id: &str) -> bool {
    matches!(
        id,
        "base"
            | "ocean"
            | "land"
            | "shelves"
            | "bathymetric-contours"
            | "tectonic-plates"
            | "tectonic-boundaries"
            | "bathymetry"
            | "volcanic-centers"
            | "lakes"
            | "rivers"
            | "watersheds"
            | "islands"
    )
}

/// Locked physical renderer layers are persisted in the shared map layer
/// field so authored vector layers can be added above them without making the
/// physical source editable.
pub fn initial_layers_value() -> Value {
    serde_json::json!({
        "schemaVersion": 1,
        "layers": [
            {"id":"base","name":"Physical base","order":0,"defaultVisible":true,"locked":true,"selector":{},"style":{"fill":"#c9a96e","fillOpacity":0.92,"stroke":"#8a7048","strokeWidth":1.25,"pointRadius":2},"kind":"vector"},
            {"id":"ocean","name":"Ocean","order":1,"defaultVisible":true,"locked":true,"selector":{},"style":{"fill":"#245c80","fillOpacity":0.58,"stroke":"#397da5","strokeWidth":0.3,"pointRadius":2},"kind":"vector"},
            {"id":"land","name":"Exposed land","order":2,"defaultVisible":true,"locked":true,"selector":{},"style":{"fill":"#b99b62","fillOpacity":0.55,"stroke":"#d8bd83","strokeWidth":0.45,"pointRadius":2},"kind":"vector"},
            {"id":"shelves","name":"Continental shelves","order":3,"defaultVisible":false,"locked":true,"selector":{},"style":{"fill":"#4f87a2","fillOpacity":0.25,"stroke":"#8db4c3","strokeWidth":0.35,"pointRadius":2},"kind":"vector"},
            {"id":"bathymetric-contours","name":"Bathymetric contours","order":4,"defaultVisible":false,"locked":true,"selector":{},"style":{"fill":"#78b3ca","fillOpacity":0,"stroke":"#78b3ca","strokeWidth":0.6,"pointRadius":2},"kind":"vector"},
            {"id":"tectonic-plates","name":"Tectonic plates","order":5,"defaultVisible":false,"locked":true,"selector":{},"style":{"fill":"#6c8ebf","fillOpacity":0.12,"stroke":"#5c7aa5","strokeWidth":0.5,"pointRadius":2},"kind":"vector"},
            {"id":"tectonic-boundaries","name":"Plate boundaries","order":6,"defaultVisible":false,"locked":true,"selector":{},"style":{"fill":"#d46a5e","fillOpacity":0,"stroke":"#d46a5e","strokeWidth":2,"pointRadius":2},"kind":"vector"},
            {"id":"bathymetry","name":"Bathymetry","order":7,"defaultVisible":false,"locked":true,"selector":{},"style":{"fill":"#4e89b5","fillOpacity":0.12,"stroke":"#386b91","strokeWidth":0.35,"pointRadius":2},"kind":"vector"},
            {"id":"volcanic-centers","name":"Volcanic centers","order":8,"defaultVisible":true,"locked":true,"selector":{},"style":{"fill":"#ef9b4a","fillOpacity":0.9,"stroke":"#8f4c25","strokeWidth":1,"pointRadius":5},"kind":"vector"},
            {"id":"lakes","name":"Lakes","order":9,"defaultVisible":true,"locked":true,"selector":{},"style":{"fill":"#4d9ac2","fillOpacity":0.72,"stroke":"#b8e4f5","strokeWidth":1,"pointRadius":2},"kind":"vector"},
            {"id":"rivers","name":"Rivers","order":10,"defaultVisible":true,"locked":true,"selector":{},"style":{"fill":"#71c7e8","fillOpacity":0,"stroke":"#71c7e8","strokeWidth":1.5,"pointRadius":2},"kind":"vector"},
            {"id":"watersheds","name":"Watersheds","order":11,"defaultVisible":false,"locked":true,"selector":{},"style":{"fill":"#9c80d1","fillOpacity":0.08,"stroke":"#bba7e5","strokeWidth":0.45,"pointRadius":2},"kind":"vector"},
            {"id":"islands","name":"Islands","order":12,"defaultVisible":false,"locked":true,"selector":{},"style":{"fill":"#e0bb78","fillOpacity":0.18,"stroke":"#f0d39b","strokeWidth":0.7,"pointRadius":2},"kind":"vector"}
        ]
    })
}

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
            CODE_UNSUPPORTED_GENERATOR_VERSION,
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
    TectonicSettings {
        plate_count: settings.plate_count,
        continental_plate_count: settings.continental_plate_count,
        tectonic_activity_ppm: settings.tectonic_activity_ppm,
        island_activity_ppm: settings.island_activity_ppm,
    }
    .validate()
    .map_err(|error| invalid(CODE_INVALID_GENERATION, error.to_string()))?;
    daena_physical_spike::evolution::EvolutionPreset::parse(&settings.evolution_preset)
        .map_err(|error| invalid(CODE_INVALID_GENERATION, error.to_string()))?;
    if let Some(version) = settings.hazard_derivation_version {
        if version != daena_physical_spike::hazards::HAZARD_DERIVATION_VERSION {
            return Err(invalid(
                CODE_INVALID_GENERATION,
                format!("unsupported hazardDerivationVersion {version}"),
            ));
        }
    }
    if let Some(forcing) = &settings.historical_forcing {
        HistoricalForcingParameters {
            version: forcing.version,
            temperature_amplitude_centi_c: forcing.temperature_amplitude_centi_c,
            period_years: forcing.period_years,
            phase_offset_years: forcing.phase_offset_years,
            land_ice_amplitude_ppm: forcing.land_ice_amplitude_ppm,
            ice_response_years: forcing.ice_response_years,
            thermal_expansion_ppm_per_degree_c: forcing.thermal_expansion_ppm_per_degree_c,
        }
        .validate()
        .map_err(|error| invalid(CODE_INVALID_GENERATION, error.to_string()))?;
    }
    Ok(settings)
}

pub fn validate_source(
    bytes: &[u8],
    generation: &Value,
) -> Result<(TectonicWorld, daena_physical_spike::ValidationReport), CoreError> {
    let settings = validate_generation(generation)?;
    let parsed = decode_source(bytes).map_err(|error| {
        let code = if error.contains("version is unsupported") {
            CODE_UNSUPPORTED_SOURCE_VERSION
        } else {
            CODE_INVALID_SOURCE
        };
        invalid(code, error)
    })?;
    let field = parsed.physical_field();
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
        || parsed.settings.plate_count != settings.plate_count
        || parsed.settings.continental_plate_count != settings.continental_plate_count
        || parsed.settings.tectonic_activity_ppm != settings.tectonic_activity_ppm
        || parsed.settings.island_activity_ppm != settings.island_activity_ppm
    {
        return Err(invalid(
            CODE_INVALID_SOURCE,
            "source provenance does not match the physical generation descriptor",
        ));
    }
    let report = validate_field_report(&field)
        .map_err(|error| invalid(CODE_INVALID_SOURCE, error.to_string()))?;
    if report.reference_water_inventory_m3 != settings.reference_water_inventory_m3 {
        return Err(invalid(
            CODE_INVALID_SOURCE,
            "reference water inventory does not match the source field",
        ));
    }
    Ok((parsed, report))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generation(forcing: Option<Value>) -> Value {
        let mut settings = serde_json::json!({
            "width": 8,
            "height": 4,
            "radiusMetres": daena_physical_spike::DEFAULT_RADIUS_METRES,
            "targetLandFractionPpm": 300_000,
            "referenceWaterInventoryM3": 1_000_000,
            "plateCount": 8,
            "continentalPlateCount": 4,
            "tectonicActivityPpm": 600_000,
            "islandActivityPpm": 300_000,
            "evolutionPreset": "mature",
            "hazardDerivationVersion": daena_physical_spike::hazards::HAZARD_DERIVATION_VERSION,
        });
        if let Some(forcing) = forcing {
            settings
                .as_object_mut()
                .unwrap()
                .insert("historicalForcing".into(), forcing);
        }
        serde_json::json!({
            "id": super::super::PHYSICAL_GENERATOR_ID,
            "version": super::super::PHYSICAL_GENERATOR_VERSION,
            "seed": 831_429,
            "retryIndex": 0,
            "settings": settings,
        })
    }

    fn forcing() -> Value {
        let parameters = HistoricalForcingParameters::default_for(831_429, 0);
        serde_json::json!({
            "version": parameters.version,
            "temperatureAmplitudeCentiC": parameters.temperature_amplitude_centi_c,
            "periodYears": parameters.period_years,
            "phaseOffsetYears": parameters.phase_offset_years,
            "landIceAmplitudePpm": parameters.land_ice_amplitude_ppm,
            "iceResponseYears": parameters.ice_response_years,
            "thermalExpansionPpmPerDegreeC": parameters.thermal_expansion_ppm_per_degree_c,
        })
    }

    #[test]
    fn historical_forcing_is_persisted_as_validated_optional_metadata() {
        let validated = validate_generation(&generation(Some(forcing()))).unwrap();
        assert!(validated.historical_forcing.is_some());
        assert!(validate_generation(&generation(None)).is_ok());

        let mut invalid = forcing();
        invalid["version"] = serde_json::json!(99);
        assert!(validate_generation(&generation(Some(invalid))).is_err());

        let mut invalid_hazard_version = generation(None);
        invalid_hazard_version["settings"]["hazardDerivationVersion"] = serde_json::json!(99);
        assert!(validate_generation(&invalid_hazard_version).is_err());
    }
}
