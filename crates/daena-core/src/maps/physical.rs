use super::{MapGeneration, MapGenerationSettings, PhysicalMapGenerationSettings};
use crate::error::CoreError;
use daena_physical::{
    decode_source,
    history::HistoricalForcingParameters,
    tectonics::{TectonicSettings, TectonicWorld},
    validate_field_report, PRODUCTION_MAX_HEIGHT, PRODUCTION_MAX_WIDTH,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub const CODE_INVALID_GENERATION: &str = "physical.generator.invalid-settings";
pub const CODE_UNSUPPORTED_GENERATOR_VERSION: &str = "physical.generator.unsupported-version";
pub const CODE_INVALID_SOURCE: &str = "physical.source.invalid";
pub const CODE_UNSUPPORTED_SOURCE_VERSION: &str = "physical.source.unsupported-version";
pub const PHYSICAL_DERIVED_CACHE_RELATIVE: &str = ".daena/cache/physical-derived";

/// The validated physical descriptor/source pair exposed to the host.
///
/// The identity is deliberately opaque to callers. Only this module constructs
/// it, so TypeScript and Tauri cannot accidentally implement a second
/// normalization or hashing scheme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedPhysicalSource {
    pub world: TectonicWorld,
    pub report: daena_physical::ValidationReport,
    pub identity: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalIdentityManifestV1 {
    pub provider_id: String,
    pub source_format: String,
    pub adapter_version: u32,
    pub generator_id: String,
    pub generator_version: u32,
    pub source_version: u16,
    pub width: u32,
    pub height: u32,
    pub radius_metres: u64,
    pub seed: u32,
    pub retry_index: u32,
    pub target_land_fraction_ppm: u32,
    pub sea_level_mm: i32,
    pub sample_count: u32,
    pub plate_count: u16,
    pub continental_plate_count: u16,
    pub tectonic_activity_ppm: u32,
    pub island_activity_ppm: u32,
    pub boundary_count: u32,
    pub volcanic_center_count: u32,
    pub reference_water_inventory_m3: u64,
    pub evolution_preset: String,
    pub historical_forcing: HistoricalForcingParameters,
}

fn push_manifest_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_manifest_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_manifest_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_manifest_i32(bytes: &mut Vec<u8>, value: i32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_manifest_string(bytes: &mut Vec<u8>, value: &str) {
    let value = value.as_bytes();
    push_manifest_u32(bytes, value.len() as u32);
    bytes.extend_from_slice(value);
}

/// Encode `PhysicalIdentityManifestV1` in its one locked byte representation.
pub fn encode_identity_manifest(manifest: &PhysicalIdentityManifestV1) -> Vec<u8> {
    let mut bytes = Vec::new();
    push_manifest_string(&mut bytes, &manifest.provider_id);
    push_manifest_string(&mut bytes, &manifest.source_format);
    push_manifest_u32(&mut bytes, manifest.adapter_version);
    push_manifest_string(&mut bytes, &manifest.generator_id);
    push_manifest_u32(&mut bytes, manifest.generator_version);
    push_manifest_u16(&mut bytes, manifest.source_version);
    push_manifest_u32(&mut bytes, manifest.width);
    push_manifest_u32(&mut bytes, manifest.height);
    push_manifest_u64(&mut bytes, manifest.radius_metres);
    push_manifest_u32(&mut bytes, manifest.seed);
    push_manifest_u32(&mut bytes, manifest.retry_index);
    push_manifest_u32(&mut bytes, manifest.target_land_fraction_ppm);
    push_manifest_i32(&mut bytes, manifest.sea_level_mm);
    push_manifest_u32(&mut bytes, manifest.sample_count);
    push_manifest_u16(&mut bytes, manifest.plate_count);
    push_manifest_u16(&mut bytes, manifest.continental_plate_count);
    push_manifest_u32(&mut bytes, manifest.tectonic_activity_ppm);
    push_manifest_u32(&mut bytes, manifest.island_activity_ppm);
    push_manifest_u32(&mut bytes, manifest.boundary_count);
    push_manifest_u32(&mut bytes, manifest.volcanic_center_count);
    push_manifest_u64(&mut bytes, manifest.reference_water_inventory_m3);
    push_manifest_string(&mut bytes, &manifest.evolution_preset);
    push_manifest_u16(&mut bytes, manifest.historical_forcing.version);
    for component in manifest.historical_forcing.components {
        push_manifest_i32(&mut bytes, component.amplitude_centi_c);
        push_manifest_u64(&mut bytes, component.period_years as u64);
        push_manifest_u64(&mut bytes, component.phase_offset_years as u64);
    }
    push_manifest_u32(&mut bytes, manifest.historical_forcing.sensitivity_ppm);
    push_manifest_u32(
        &mut bytes,
        manifest.historical_forcing.land_ice_amplitude_ppm,
    );
    push_manifest_u64(
        &mut bytes,
        manifest.historical_forcing.ice_response_years as u64,
    );
    push_manifest_i32(&mut bytes, manifest.historical_forcing.ice_midpoint_centi_c);
    push_manifest_i32(
        &mut bytes,
        manifest.historical_forcing.ice_transition_width_centi_c,
    );
    push_manifest_u32(
        &mut bytes,
        manifest
            .historical_forcing
            .thermal_expansion_ppm_per_degree_c,
    );
    bytes
}

fn physical_identity(manifest: &PhysicalIdentityManifestV1, source_bytes: &[u8]) -> String {
    let manifest_bytes = encode_identity_manifest(manifest);
    let mut input = Vec::with_capacity(32 + manifest_bytes.len() + source_bytes.len());
    input.extend_from_slice(b"daena-physical-identity-v1\0");
    push_manifest_u32(&mut input, manifest_bytes.len() as u32);
    input.extend_from_slice(&manifest_bytes);
    push_manifest_u64(&mut input, source_bytes.len() as u64);
    input.extend_from_slice(source_bytes);
    format!("sha256:{:x}", Sha256::digest(input))
}

pub fn physical_derived_cache_dir(
    project_root: &Path,
    identity: &str,
) -> Result<PathBuf, CoreError> {
    let Some(hex) = identity.strip_prefix("sha256:") else {
        return Err(CoreError::Validation(
            "physical identity is not a sha256 digest".into(),
        ));
    };
    if hex.len() != 64
        || !hex
            .chars()
            .all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase())
    {
        return Err(CoreError::Validation(
            "physical identity is not a lowercase sha256 digest".into(),
        ));
    }
    Ok(project_root
        .join(PHYSICAL_DERIVED_CACHE_RELATIVE)
        .join(format!("sha256-{hex}"))
        .join(daena_physical::derived_cache::StaticDerivedPhysics::version_dir()))
}

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
            | "ice"
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
            {"id":"islands","name":"Islands","order":12,"defaultVisible":false,"locked":true,"selector":{},"style":{"fill":"#e0bb78","fillOpacity":0.18,"stroke":"#f0d39b","strokeWidth":0.7,"pointRadius":2},"kind":"vector"},
            {"id":"ice","name":"Ice","order":13,"defaultVisible":false,"locked":true,"selector":{},"style":{"fill":"#e8f2f8","fillOpacity":0.82,"stroke":"#c5d8e6","strokeWidth":0.4,"pointRadius":2},"kind":"vector"}
        ]
    })
}

fn invalid(code: &str, message: impl Into<String>) -> CoreError {
    CoreError::Validation(format!("{code}: {}", message.into()))
}

fn parse_generation(value: &Value) -> Result<MapGeneration, CoreError> {
    let object = value
        .as_object()
        .ok_or_else(|| invalid(CODE_INVALID_GENERATION, "generation must be an object"))?;
    for field in ["id", "version", "seed", "retryIndex", "settings"] {
        if !object.contains_key(field) {
            return Err(invalid(
                CODE_INVALID_GENERATION,
                format!("generation.{field} is required"),
            ));
        }
    }
    let settings = object["settings"].as_object().ok_or_else(|| {
        invalid(
            CODE_INVALID_GENERATION,
            "generation.settings must be an object",
        )
    })?;
    for field in [
        "width",
        "height",
        "radiusMetres",
        "targetLandFractionPpm",
        "referenceWaterInventoryM3",
        "plateCount",
        "continentalPlateCount",
        "tectonicActivityPpm",
        "islandActivityPpm",
        "evolutionPreset",
        "historicalForcing",
    ] {
        if !settings.contains_key(field) {
            return Err(invalid(
                CODE_INVALID_GENERATION,
                format!("generation.settings.{field} is required"),
            ));
        }
    }
    serde_json::from_value(value.clone())
        .map_err(|error| invalid(CODE_INVALID_GENERATION, format!("generation: {error}")))
}

pub fn validate_generation(value: &Value) -> Result<PhysicalMapGenerationSettings, CoreError> {
    let generation = parse_generation(value)?;
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
    daena_physical::Grid::new(settings.width, settings.height, settings.radius_metres)
        .map_err(|error| invalid(CODE_INVALID_GENERATION, error))?;
    if settings.width > PRODUCTION_MAX_WIDTH || settings.height > PRODUCTION_MAX_HEIGHT {
        return Err(invalid(
            CODE_INVALID_GENERATION,
            format!(
                "production grid is limited to {}x{}; larger grids are preview-only",
                PRODUCTION_MAX_WIDTH, PRODUCTION_MAX_HEIGHT
            ),
        ));
    }
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
    daena_physical::evolution::EvolutionPreset::parse(&settings.evolution_preset)
        .map_err(|error| invalid(CODE_INVALID_GENERATION, error.to_string()))?;
    if let Some(version) = settings.hazard_derivation_version {
        if version != daena_physical::hazards::HAZARD_DERIVATION_VERSION {
            return Err(invalid(
                CODE_INVALID_GENERATION,
                format!("unsupported hazardDerivationVersion {version}"),
            ));
        }
    }
    if settings.historical_forcing.components.len()
        != daena_physical::history::FORCING_COMPONENT_COUNT
    {
        return Err(invalid(
            CODE_INVALID_GENERATION,
            "historicalForcing.components must contain three independent terms",
        ));
    }
    historical_forcing(&settings)
        .validate()
        .map_err(|error| invalid(CODE_INVALID_GENERATION, error.to_string()))?;
    Ok(settings)
}

fn historical_forcing(settings: &PhysicalMapGenerationSettings) -> HistoricalForcingParameters {
    let components = &settings.historical_forcing.components;
    HistoricalForcingParameters {
        version: settings.historical_forcing.version,
        components: [
            daena_physical::history::ForcingComponent {
                amplitude_centi_c: components.first().map(|c| c.amplitude_centi_c).unwrap_or(0),
                period_years: components.first().map(|c| c.period_years).unwrap_or(0),
                phase_offset_years: components
                    .first()
                    .map(|c| c.phase_offset_years)
                    .unwrap_or(0),
            },
            daena_physical::history::ForcingComponent {
                amplitude_centi_c: components.get(1).map(|c| c.amplitude_centi_c).unwrap_or(0),
                period_years: components.get(1).map(|c| c.period_years).unwrap_or(0),
                phase_offset_years: components.get(1).map(|c| c.phase_offset_years).unwrap_or(0),
            },
            daena_physical::history::ForcingComponent {
                amplitude_centi_c: components.get(2).map(|c| c.amplitude_centi_c).unwrap_or(0),
                period_years: components.get(2).map(|c| c.period_years).unwrap_or(0),
                phase_offset_years: components.get(2).map(|c| c.phase_offset_years).unwrap_or(0),
            },
        ],
        sensitivity_ppm: settings.historical_forcing.sensitivity_ppm,
        land_ice_amplitude_ppm: settings.historical_forcing.land_ice_amplitude_ppm,
        ice_response_years: settings.historical_forcing.ice_response_years,
        ice_midpoint_centi_c: settings.historical_forcing.ice_midpoint_centi_c,
        ice_transition_width_centi_c: settings.historical_forcing.ice_transition_width_centi_c,
        thermal_expansion_ppm_per_degree_c: settings
            .historical_forcing
            .thermal_expansion_ppm_per_degree_c,
    }
}

fn source_mismatch(field: &str) -> CoreError {
    invalid(
        CODE_INVALID_SOURCE,
        format!("source provenance mismatch: {field}"),
    )
}

pub fn validate_source(
    bytes: &[u8],
    generation: &Value,
) -> Result<ValidatedPhysicalSource, CoreError> {
    // Decode the source first. This makes malformed or unsupported source data
    // authoritative over descriptor parsing and prevents descriptor defaults
    // from masking a corrupt canonical asset.
    let parsed = decode_source(bytes).map_err(|error| {
        let code = if error.contains("version is unsupported") {
            CODE_UNSUPPORTED_SOURCE_VERSION
        } else {
            CODE_INVALID_SOURCE
        };
        invalid(code, error)
    })?;
    let settings = validate_generation(generation)?;
    let field = parsed.physical_field();
    let parsed_generation = parse_generation(generation)?;
    let retry_index = parsed_generation
        .retry_index
        .ok_or_else(|| invalid(CODE_INVALID_GENERATION, "generation.retryIndex is required"))?;
    if parsed.seed != parsed_generation.seed {
        return Err(source_mismatch("seed"));
    }
    if parsed.retry_index != retry_index {
        return Err(source_mismatch("retryIndex"));
    }
    if parsed.grid.width != settings.width {
        return Err(source_mismatch("width"));
    }
    if parsed.grid.height != settings.height {
        return Err(source_mismatch("height"));
    }
    if parsed.grid.radius_metres != settings.radius_metres {
        return Err(source_mismatch("radiusMetres"));
    }
    if parsed.target_land_fraction_ppm != settings.target_land_fraction_ppm {
        return Err(source_mismatch("targetLandFractionPpm"));
    }
    if parsed.settings.plate_count != settings.plate_count {
        return Err(source_mismatch("plateCount"));
    }
    if parsed.settings.continental_plate_count != settings.continental_plate_count {
        return Err(source_mismatch("continentalPlateCount"));
    }
    if parsed.settings.tectonic_activity_ppm != settings.tectonic_activity_ppm {
        return Err(source_mismatch("tectonicActivityPpm"));
    }
    if parsed.settings.island_activity_ppm != settings.island_activity_ppm {
        return Err(source_mismatch("islandActivityPpm"));
    }
    let report = validate_field_report(&field)
        .map_err(|error| invalid(CODE_INVALID_SOURCE, error.to_string()))?;
    if report.reference_water_inventory_m3 != settings.reference_water_inventory_m3 {
        return Err(source_mismatch("referenceWaterInventoryM3"));
    }
    let preset = daena_physical::evolution::EvolutionPreset::parse(&settings.evolution_preset)
        .map_err(|error| invalid(CODE_INVALID_GENERATION, error.to_string()))?;
    let manifest = PhysicalIdentityManifestV1 {
        provider_id: super::PHYSICAL_PROVIDER.into(),
        source_format: super::PHYSICAL_SOURCE_FORMAT.into(),
        adapter_version: super::PHYSICAL_ADAPTER_VERSION,
        generator_id: parsed_generation.id,
        generator_version: parsed_generation.version,
        source_version: daena_physical::SOURCE_VERSION,
        width: parsed.grid.width,
        height: parsed.grid.height,
        radius_metres: parsed.grid.radius_metres,
        seed: parsed.seed,
        retry_index: parsed.retry_index,
        target_land_fraction_ppm: parsed.target_land_fraction_ppm,
        sea_level_mm: parsed.sea_level_mm,
        sample_count: parsed.grid.sample_count() as u32,
        plate_count: parsed.settings.plate_count,
        continental_plate_count: parsed.settings.continental_plate_count,
        tectonic_activity_ppm: parsed.settings.tectonic_activity_ppm,
        island_activity_ppm: parsed.settings.island_activity_ppm,
        boundary_count: parsed.boundaries.len() as u32,
        volcanic_center_count: parsed.volcanic_centers.len() as u32,
        reference_water_inventory_m3: settings.reference_water_inventory_m3,
        evolution_preset: preset.as_str().into(),
        historical_forcing: historical_forcing(&settings),
    };
    let identity = physical_identity(&manifest, bytes);
    Ok(ValidatedPhysicalSource {
        world: parsed,
        report,
        identity,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn forcing_json(parameters: HistoricalForcingParameters) -> Value {
        serde_json::json!({
            "version": parameters.version,
            "components": parameters.components.iter().map(|component| serde_json::json!({
                "amplitudeCentiC": component.amplitude_centi_c,
                "periodYears": component.period_years,
                "phaseOffsetYears": component.phase_offset_years,
            })).collect::<Vec<_>>(),
            "sensitivityPpm": parameters.sensitivity_ppm,
            "landIceAmplitudePpm": parameters.land_ice_amplitude_ppm,
            "iceResponseYears": parameters.ice_response_years,
            "iceMidpointCentiC": parameters.ice_midpoint_centi_c,
            "iceTransitionWidthCentiC": parameters.ice_transition_width_centi_c,
            "thermalExpansionPpmPerDegreeC": parameters.thermal_expansion_ppm_per_degree_c,
        })
    }

    fn generation(forcing_value: Option<Value>) -> Value {
        let mut settings = serde_json::json!({
            "width": 8,
            "height": 4,
            "radiusMetres": daena_physical::DEFAULT_RADIUS_METRES,
            "targetLandFractionPpm": 300_000,
            "referenceWaterInventoryM3": 1_000_000,
            "plateCount": 8,
            "continentalPlateCount": 4,
            "tectonicActivityPpm": 600_000,
            "islandActivityPpm": 300_000,
            "evolutionPreset": "mature",
            "hazardDerivationVersion": daena_physical::hazards::HAZARD_DERIVATION_VERSION,
        });
        settings.as_object_mut().unwrap().insert(
            "historicalForcing".into(),
            forcing_value.unwrap_or_else(forcing),
        );
        serde_json::json!({
            "id": super::super::PHYSICAL_GENERATOR_ID,
            "version": super::super::PHYSICAL_GENERATOR_VERSION,
            "seed": 831_429,
            "retryIndex": 0,
            "settings": settings,
        })
    }

    fn forcing() -> Value {
        forcing_json(HistoricalForcingParameters::default_for(831_429, 0))
    }

    fn generated_pair() -> (Vec<u8>, Value) {
        let settings = daena_physical::GenerationSettings {
            width: 8,
            height: 4,
            radius_metres: daena_physical::DEFAULT_RADIUS_METRES,
            target_land_fraction_ppm: 300_000,
        };
        let mut progress = daena_physical::NoopProgress;
        let world = daena_physical::generate_world(settings, 831_429, 0, &mut progress).unwrap();
        let parameters = HistoricalForcingParameters::default_for(831_429, 0);
        let generation = serde_json::json!({
            "id": super::super::PHYSICAL_GENERATOR_ID,
            "version": super::super::PHYSICAL_GENERATOR_VERSION,
            "seed": world.field.seed,
            "retryIndex": world.field.retry_index,
            "settings": {
                "width": world.field.grid.width,
                "height": world.field.grid.height,
                "radiusMetres": world.field.grid.radius_metres,
                "targetLandFractionPpm": world.field.target_land_fraction_ppm,
                "referenceWaterInventoryM3": world.report.reference_water_inventory_m3,
                "plateCount": world.tectonics.settings.plate_count,
                "continentalPlateCount": world.tectonics.settings.continental_plate_count,
                "tectonicActivityPpm": world.tectonics.settings.tectonic_activity_ppm,
                "islandActivityPpm": world.tectonics.settings.island_activity_ppm,
                "evolutionPreset": "mature",
                "historicalForcing": forcing_json(parameters),
            },
        });
        (world.source, generation)
    }

    fn manifest_fixture() -> PhysicalIdentityManifestV1 {
        PhysicalIdentityManifestV1 {
            provider_id: super::super::PHYSICAL_PROVIDER.into(),
            source_format: super::super::PHYSICAL_SOURCE_FORMAT.into(),
            adapter_version: super::super::PHYSICAL_ADAPTER_VERSION,
            generator_id: super::super::PHYSICAL_GENERATOR_ID.into(),
            generator_version: super::super::PHYSICAL_GENERATOR_VERSION,
            source_version: daena_physical::SOURCE_VERSION,
            width: 8,
            height: 4,
            radius_metres: daena_physical::DEFAULT_RADIUS_METRES,
            seed: 831_429,
            retry_index: 0,
            target_land_fraction_ppm: 300_000,
            sea_level_mm: 0,
            sample_count: 32,
            plate_count: 8,
            continental_plate_count: 4,
            tectonic_activity_ppm: 600_000,
            island_activity_ppm: 300_000,
            boundary_count: 16,
            volcanic_center_count: 4,
            reference_water_inventory_m3: 1_000_000,
            evolution_preset: "mature".into(),
            historical_forcing: HistoricalForcingParameters::default_for(831_429, 0),
        }
    }

    #[test]
    fn historical_forcing_is_mandatory_validated_metadata() {
        let validated = validate_generation(&generation(Some(forcing()))).unwrap();
        assert_eq!(
            validated.historical_forcing.version,
            HistoricalForcingParameters::default_for(831_429, 0).version
        );
        let mut missing = generation(Some(forcing()));
        missing["settings"]
            .as_object_mut()
            .unwrap()
            .remove("historicalForcing");
        assert!(validate_generation(&missing).is_err());

        let mut invalid = forcing();
        invalid["version"] = serde_json::json!(99);
        assert!(validate_generation(&generation(Some(invalid))).is_err());

        let mut invalid_hazard_version = generation(None);
        invalid_hazard_version["settings"]["hazardDerivationVersion"] = serde_json::json!(99);
        assert!(validate_generation(&invalid_hazard_version).is_err());
    }

    #[test]
    fn identity_is_stable_for_semantic_json_and_changes_for_every_manifest_field() {
        let (source, generation) = generated_pair();
        let validated = validate_source(&source, &generation).unwrap();
        assert!(validated.identity.starts_with("sha256:"));
        assert!(validated.identity[7..]
            .chars()
            .all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase()));
        assert_eq!(
            physical_derived_cache_dir(
                std::path::Path::new("/tmp/example-project"),
                &validated.identity
            )
            .unwrap(),
            std::path::PathBuf::from(format!(
                "/tmp/example-project/.daena/cache/physical-derived/sha256-{}/{}",
                &validated.identity[7..],
                daena_physical::derived_cache::StaticDerivedPhysics::version_dir()
            ))
        );
        assert!(PHYSICAL_DERIVED_CACHE_RELATIVE.starts_with(".daena/cache/"));

        let reordered = serde_json::json!({
            "settings": generation["settings"].clone(),
            "retryIndex": generation["retryIndex"],
            "seed": generation["seed"],
            "version": generation["version"],
            "id": generation["id"],
        });
        assert_eq!(
            validated.identity,
            validate_source(&source, &reordered).unwrap().identity
        );

        let base = manifest_fixture();
        let base_identity = physical_identity(&base, &source);
        let mut changed = base.clone();
        changed.provider_id.push_str("-changed");
        assert_ne!(base_identity, physical_identity(&changed, &source));
        changed = base.clone();
        changed.source_format.push_str("-changed");
        assert_ne!(base_identity, physical_identity(&changed, &source));
        changed = base.clone();
        changed.adapter_version += 1;
        assert_ne!(base_identity, physical_identity(&changed, &source));
        changed = base.clone();
        changed.generator_id.push_str("-changed");
        assert_ne!(base_identity, physical_identity(&changed, &source));
        changed = base.clone();
        changed.generator_version += 1;
        assert_ne!(base_identity, physical_identity(&changed, &source));
        changed = base.clone();
        changed.source_version += 1;
        assert_ne!(base_identity, physical_identity(&changed, &source));
        changed = base.clone();
        changed.width += 1;
        assert_ne!(base_identity, physical_identity(&changed, &source));
        changed = base.clone();
        changed.height += 1;
        assert_ne!(base_identity, physical_identity(&changed, &source));
        changed = base.clone();
        changed.radius_metres += 1;
        assert_ne!(base_identity, physical_identity(&changed, &source));
        changed = base.clone();
        changed.seed += 1;
        assert_ne!(base_identity, physical_identity(&changed, &source));
        changed = base.clone();
        changed.retry_index += 1;
        assert_ne!(base_identity, physical_identity(&changed, &source));
        changed = base.clone();
        changed.target_land_fraction_ppm += 1;
        assert_ne!(base_identity, physical_identity(&changed, &source));
        changed = base.clone();
        changed.sea_level_mm += 1;
        assert_ne!(base_identity, physical_identity(&changed, &source));
        changed = base.clone();
        changed.sample_count += 1;
        assert_ne!(base_identity, physical_identity(&changed, &source));
        changed = base.clone();
        changed.plate_count += 1;
        assert_ne!(base_identity, physical_identity(&changed, &source));
        changed = base.clone();
        changed.continental_plate_count += 1;
        assert_ne!(base_identity, physical_identity(&changed, &source));
        changed = base.clone();
        changed.tectonic_activity_ppm += 1;
        assert_ne!(base_identity, physical_identity(&changed, &source));
        changed = base.clone();
        changed.island_activity_ppm += 1;
        assert_ne!(base_identity, physical_identity(&changed, &source));
        changed = base.clone();
        changed.boundary_count += 1;
        assert_ne!(base_identity, physical_identity(&changed, &source));
        changed = base.clone();
        changed.volcanic_center_count += 1;
        assert_ne!(base_identity, physical_identity(&changed, &source));
        changed = base.clone();
        changed.reference_water_inventory_m3 += 1;
        assert_ne!(base_identity, physical_identity(&changed, &source));
        changed = base.clone();
        changed.evolution_preset = "young".into();
        assert_ne!(base_identity, physical_identity(&changed, &source));
        changed = base.clone();
        changed.historical_forcing.components[0].amplitude_centi_c += 1;
        assert_ne!(base_identity, physical_identity(&changed, &source));
        changed = base.clone();
        changed.historical_forcing.components[1].period_years += 1;
        assert_ne!(base_identity, physical_identity(&changed, &source));
        changed = base.clone();
        changed.historical_forcing.sensitivity_ppm -= 1_000;
        assert_ne!(base_identity, physical_identity(&changed, &source));

        let mut changed_source = source.clone();
        changed_source.push(0);
        assert_ne!(base_identity, physical_identity(&base, &changed_source));
    }

    #[test]
    fn pair_validation_reports_source_first_and_stable_duplicate_fields() {
        let (source, generation) = generated_pair();
        let mut malformed = source.clone();
        malformed.truncate(malformed.len() - 1);
        let error = validate_source(&malformed, &Value::Null)
            .unwrap_err()
            .to_string();
        assert!(error.starts_with(CODE_INVALID_SOURCE));

        let mut unsupported_v1 = source.clone();
        unsupported_v1[8..10].copy_from_slice(&1u16.to_le_bytes());
        let error = validate_source(&unsupported_v1, &Value::Null)
            .unwrap_err()
            .to_string();
        assert!(error.starts_with(CODE_UNSUPPORTED_SOURCE_VERSION));

        let mut mismatch = generation.clone();
        mismatch["settings"]["targetLandFractionPpm"] = serde_json::json!(300_001);
        let error = validate_source(&source, &mismatch).unwrap_err().to_string();
        assert!(error.contains("source provenance mismatch: targetLandFractionPpm"));

        let mut missing_water = generation.clone();
        missing_water["settings"]
            .as_object_mut()
            .unwrap()
            .remove("referenceWaterInventoryM3");
        let error = validate_source(&source, &missing_water)
            .unwrap_err()
            .to_string();
        assert!(error.contains("referenceWaterInventoryM3"));

        let mut missing_forcing = generation;
        missing_forcing["settings"]
            .as_object_mut()
            .unwrap()
            .remove("historicalForcing");
        let error = validate_source(&source, &missing_forcing)
            .unwrap_err()
            .to_string();
        assert!(error.contains("historicalForcing"));
    }
}
