use super::{MapGeneration, MapGenerationSettings, PhysicalMapGenerationSettings};
use crate::error::CoreError;
use daena_physical::{
    engine, history::HistoricalForcingParameters, tectonics::TectonicWorld, ValidationReport,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub const CODE_INVALID_GENERATION: &str = "physical.generator.invalid-settings";
pub const CODE_UNSUPPORTED_GENERATOR_VERSION: &str = "physical.generator.unsupported-version";
pub const CODE_INVALID_SOURCE: &str = "physical.source.invalid";
pub const CODE_UNSUPPORTED_SOURCE_VERSION: &str = "physical.source.unsupported-version";
pub const PHYSICAL_DERIVED_CACHE_RELATIVE: &str = ".daena/cache/physical-derived";

/// The validated `DAENAPW2` descriptor/source pair exposed to the host.
///
/// The identity is deliberately opaque to callers. Only this module constructs
/// it, so TypeScript and Tauri cannot accidentally implement a second
/// normalization or hashing scheme.
#[derive(Debug)]
pub struct ValidatedPhysicalSource {
    pub source: engine::DecodedSource,
    pub report: ValidationReport,
    pub identity: String,
    /// Transitional slot for consumers that still read the retired `DAENAPW1`
    /// tectonic type. It is always `None` for a `DAENAPW2` source; those
    /// consumers fail honestly until their engine-backed replacement lands.
    pub legacy_world: Option<TectonicWorld>,
}

impl ValidatedPhysicalSource {
    pub fn legacy_world(&self) -> Result<&TectonicWorld, CoreError> {
        self.legacy_world.as_ref().ok_or_else(|| {
            invalid(
                CODE_UNSUPPORTED_SOURCE_VERSION,
                "this consumer reads the retired DAENAPW1 tectonic type and moves to the engine in a later phase",
            )
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalIdentityManifestV2 {
    pub provider_id: String,
    pub source_format: String,
    pub adapter_version: u32,
    pub generator_id: String,
    pub generator_version: u32,
    pub source_version: u16,
    pub engine_version: u32,
    pub resolution: u32,
    pub source_sha256: String,
    pub elevation_hash: u64,
    pub tectonics_hash: u64,
    pub reference_water_inventory_m3: u64,
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

/// Encode `PhysicalIdentityManifestV2` in its one locked byte representation.
#[must_use]
pub fn encode_identity_manifest(manifest: &PhysicalIdentityManifestV2) -> Vec<u8> {
    let mut bytes = Vec::new();
    push_manifest_string(&mut bytes, &manifest.provider_id);
    push_manifest_string(&mut bytes, &manifest.source_format);
    push_manifest_u32(&mut bytes, manifest.adapter_version);
    push_manifest_string(&mut bytes, &manifest.generator_id);
    push_manifest_u32(&mut bytes, manifest.generator_version);
    push_manifest_u16(&mut bytes, manifest.source_version);
    push_manifest_u32(&mut bytes, manifest.engine_version);
    push_manifest_u32(&mut bytes, manifest.resolution);
    push_manifest_string(&mut bytes, &manifest.source_sha256);
    push_manifest_u64(&mut bytes, manifest.elevation_hash);
    push_manifest_u64(&mut bytes, manifest.tectonics_hash);
    push_manifest_u64(&mut bytes, manifest.reference_water_inventory_m3);
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

fn physical_identity(manifest: &PhysicalIdentityManifestV2) -> String {
    let manifest_bytes = encode_identity_manifest(manifest);
    let mut input = Vec::with_capacity(32 + manifest_bytes.len());
    input.extend_from_slice(b"daena-physical-identity-v2\0");
    push_manifest_u32(&mut input, manifest_bytes.len() as u32);
    input.extend_from_slice(&manifest_bytes);
    format!("sha256:{:x}", Sha256::digest(input))
}

/// Directory for disposable static derived physics keyed by physical identity
/// and planetary configuration. Terrain identity does not hash planetary
/// settings, so the cache directory includes a planetary token.
pub fn physical_derived_cache_dir(
    project_root: &Path,
    identity: &str,
    planetary: daena_physical::planetary::PlanetaryConfiguration,
    view_width: u32,
    view_height: u32,
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
        .join(
            daena_physical::derived_cache::StaticDerivedPhysics::version_dir(
                planetary,
                view_width,
                view_height,
            ),
        ))
}

#[must_use]
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
            | "winds"
            | "currents"
    )
}

/// Locked physical renderer layers are persisted in the shared map layer
/// field so authored vector layers can be added above them without making
/// the physical source editable.
#[must_use]
pub fn initial_layers_value() -> Value {
    serde_json::json!({
        "schemaVersion": 1,
        "layers": [
            {"id":"base","name":"Physical base","order":0,"defaultVisible":false,"locked":true,"selector":{},"style":{"fill":"#c9a96e","fillOpacity":0.92,"stroke":"#8a7048","strokeWidth":1.25,"pointRadius":2},"kind":"vector"},
            {"id":"ocean","name":"Ocean","order":1,"defaultVisible":false,"locked":true,"selector":{},"style":{"fill":"#245c80","fillOpacity":0.58,"stroke":"#397da5","strokeWidth":0.3,"pointRadius":2},"kind":"vector"},
            {"id":"land","name":"Exposed land","order":2,"defaultVisible":false,"locked":true,"selector":{},"style":{"fill":"#b99b62","fillOpacity":0.55,"stroke":"#d8bd83","strokeWidth":0.45,"pointRadius":2},"kind":"vector"},
            {"id":"shelves","name":"Continental shelves","order":3,"defaultVisible":false,"locked":true,"selector":{},"style":{"fill":"#4f87a2","fillOpacity":0.25,"stroke":"#8db4c3","strokeWidth":0.35,"pointRadius":2},"kind":"vector"},
            {"id":"bathymetric-contours","name":"Bathymetric contours","order":4,"defaultVisible":false,"locked":true,"selector":{},"style":{"fill":"#78b3ca","fillOpacity":0,"stroke":"#78b3ca","strokeWidth":0.6,"pointRadius":2},"kind":"vector"},
            {"id":"tectonic-plates","name":"Tectonic plates","order":5,"defaultVisible":false,"locked":true,"selector":{},"style":{"fill":"#6c8ebf","fillOpacity":0.12,"stroke":"#5c7aa5","strokeWidth":0.5,"pointRadius":2},"kind":"vector"},
            {"id":"tectonic-boundaries","name":"Plate boundaries","order":6,"defaultVisible":false,"locked":true,"selector":{},"style":{"fill":"#d46a5e","fillOpacity":0,"stroke":"#d46a5e","strokeWidth":2,"pointRadius":2},"kind":"vector"},
            {"id":"bathymetry","name":"Bathymetry","order":7,"defaultVisible":false,"locked":true,"selector":{},"style":{"fill":"#4e89b5","fillOpacity":0.12,"stroke":"#386b91","strokeWidth":0.35,"pointRadius":2},"kind":"vector"},
            {"id":"volcanic-centers","name":"Volcanic centers","order":8,"defaultVisible":false,"locked":true,"selector":{},"style":{"fill":"#ef9b4a","fillOpacity":0.9,"stroke":"#8f4c25","strokeWidth":1,"pointRadius":5},"kind":"vector"},
            {"id":"lakes","name":"Lakes","order":9,"defaultVisible":false,"locked":true,"selector":{},"style":{"fill":"#4d9ac2","fillOpacity":0.72,"stroke":"#b8e4f5","strokeWidth":1,"pointRadius":2},"kind":"vector"},
            {"id":"rivers","name":"Rivers","order":10,"defaultVisible":false,"locked":true,"selector":{},"style":{"fill":"#71c7e8","fillOpacity":0,"stroke":"#71c7e8","strokeWidth":1.5,"pointRadius":2},"kind":"vector"},
            {"id":"watersheds","name":"Watersheds","order":11,"defaultVisible":false,"locked":true,"selector":{},"style":{"fill":"#9c80d1","fillOpacity":0.08,"stroke":"#bba7e5","strokeWidth":0.45,"pointRadius":2},"kind":"vector"},
            {"id":"islands","name":"Islands","order":12,"defaultVisible":false,"locked":true,"selector":{},"style":{"fill":"#e0bb78","fillOpacity":0.18,"stroke":"#f0d39b","strokeWidth":0.7,"pointRadius":2},"kind":"vector"},
            {"id":"ice","name":"Ice","order":13,"defaultVisible":false,"locked":true,"selector":{},"style":{"fill":"#e8f2f8","fillOpacity":0.82,"stroke":"#c5d8e6","strokeWidth":0.4,"pointRadius":2},"kind":"vector"},
            {"id":"winds","name":"Winds","order":14,"defaultVisible":false,"locked":true,"selector":{},"style":{"fill":"#d69434","fillOpacity":0.18,"stroke":"#f2d9a8","strokeWidth":0.7,"pointRadius":2},"kind":"vector"},
            {"id":"currents","name":"Currents","order":15,"defaultVisible":false,"locked":true,"selector":{},"style":{"fill":"#3ab8c4","fillOpacity":0.18,"stroke":"#9ee7ee","strokeWidth":0.7,"pointRadius":2},"kind":"vector"}
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
    for field in ["id", "version", "seed", "settings"] {
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
        "resolution",
        "radiusMetres",
        "targetLandFractionPpm",
        "referenceWaterInventoryM3",
        "preset",
        "engineVersion",
        "sourceVersion",
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

fn generation_settings(
    generation: &MapGeneration,
) -> Result<PhysicalMapGenerationSettings, CoreError> {
    let MapGenerationSettings::Physical(settings) = &generation.settings else {
        return Err(invalid(
            CODE_INVALID_GENERATION,
            "physical generation settings are required",
        ));
    };
    Ok(settings.clone())
}

fn engine_settings(generation: &MapGeneration) -> Result<engine::Settings, CoreError> {
    let settings = generation_settings(generation)?;
    let preset = engine::WorldPreset::parse(&settings.preset).ok_or_else(|| {
        invalid(
            CODE_INVALID_GENERATION,
            format!("unknown world preset {}", settings.preset),
        )
    })?;
    Ok(engine::Settings::from_parts(
        generation.seed,
        preset,
        settings.resolution,
        settings.radius_metres,
        settings.target_land_fraction_ppm,
        settings.planetary,
    ))
}

pub fn validate_generation(value: &Value) -> Result<PhysicalMapGenerationSettings, CoreError> {
    let generation = parse_generation(value)?;
    if generation.id != super::PHYSICAL_GENERATOR_ID
        || generation.version != super::PHYSICAL_GENERATOR_VERSION
    {
        return Err(invalid(
            CODE_UNSUPPORTED_GENERATOR_VERSION,
            "generator id and version are unsupported",
        ));
    }
    let settings = generation_settings(&generation)?;
    if settings.engine_version != engine::WORLD_ENGINE_VERSION
        || settings.source_version != engine::SOURCE_VERSION
    {
        return Err(invalid(
            CODE_UNSUPPORTED_GENERATOR_VERSION,
            "engine or source version is unsupported",
        ));
    }
    if settings.reference_water_inventory_m3 == 0 {
        return Err(invalid(
            CODE_INVALID_GENERATION,
            "referenceWaterInventoryM3 must be positive",
        ));
    }
    if settings.target_land_fraction_ppm == 0 || settings.target_land_fraction_ppm >= 1_000_000 {
        return Err(invalid(
            CODE_INVALID_GENERATION,
            "targetLandFractionPpm must be the resolved fraction",
        ));
    }
    settings
        .planetary
        .validate()
        .map_err(|error| invalid(CODE_INVALID_GENERATION, error.to_string()))?;
    if settings.planetary.radius_metres != settings.radius_metres {
        return Err(invalid(
            CODE_INVALID_GENERATION,
            "planetary.radiusMetres must match generation.settings.radiusMetres",
        ));
    }
    let request = engine_settings(&generation)?;
    request
        .snapshot()
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
                amplitude_centi_c: components.first().map_or(0, |c| c.amplitude_centi_c),
                period_years: components.first().map_or(0, |c| c.period_years),
                phase_offset_years: components.first().map_or(0, |c| c.phase_offset_years),
            },
            daena_physical::history::ForcingComponent {
                amplitude_centi_c: components.get(1).map_or(0, |c| c.amplitude_centi_c),
                period_years: components.get(1).map_or(0, |c| c.period_years),
                phase_offset_years: components.get(1).map_or(0, |c| c.phase_offset_years),
            },
            daena_physical::history::ForcingComponent {
                amplitude_centi_c: components.get(2).map_or(0, |c| c.amplitude_centi_c),
                period_years: components.get(2).map_or(0, |c| c.period_years),
                phase_offset_years: components.get(2).map_or(0, |c| c.phase_offset_years),
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

fn decode_error(error: daena_physical::PhysicalError) -> CoreError {
    let code = match error.code() {
        daena_physical::CODE_SOURCE_UNSUPPORTED_VERSION => CODE_UNSUPPORTED_SOURCE_VERSION,
        _ => CODE_INVALID_SOURCE,
    };
    invalid(code, error.to_string())
}

pub fn validate_source(
    bytes: &[u8],
    generation: &Value,
) -> Result<ValidatedPhysicalSource, CoreError> {
    // Decode the source first. This makes malformed or unsupported source data
    // authoritative over descriptor parsing and prevents descriptor defaults
    // from masking a corrupt canonical asset.
    let decoded = engine::decode_source(bytes).map_err(decode_error)?;
    let settings = validate_generation(generation)?;
    let parsed_generation = parse_generation(generation)?;
    if decoded.settings_snapshot().seed != parsed_generation.seed {
        return Err(source_mismatch("seed"));
    }
    if decoded.engine_version() != settings.engine_version {
        return Err(source_mismatch("engineVersion"));
    }
    if decoded.source_version() != settings.source_version {
        return Err(source_mismatch("sourceVersion"));
    }
    let expected = engine_settings(&parsed_generation).and_then(|request| {
        request
            .snapshot()
            .map_err(|error| invalid(CODE_INVALID_GENERATION, error.to_string()))
    })?;
    let actual = decoded.settings_snapshot();
    if let Some(field) = expected.mismatch(&actual) {
        return Err(source_mismatch(field));
    }
    if decoded.reference_water_inventory_m3() != settings.reference_water_inventory_m3 {
        return Err(source_mismatch("referenceWaterInventoryM3"));
    }
    let report = decoded
        .validation_report()
        .map_err(|error| invalid(CODE_INVALID_SOURCE, error.to_string()))?;
    let source_sha256 = format!("sha256:{:x}", Sha256::digest(bytes));
    let manifest = PhysicalIdentityManifestV2 {
        provider_id: super::PHYSICAL_PROVIDER.into(),
        source_format: super::PHYSICAL_SOURCE_FORMAT.into(),
        adapter_version: super::PHYSICAL_ADAPTER_VERSION,
        generator_id: parsed_generation.id,
        generator_version: parsed_generation.version,
        source_version: decoded.source_version(),
        engine_version: decoded.engine_version(),
        resolution: actual.resolution,
        source_sha256,
        elevation_hash: decoded.hashes().elevation_mm,
        tectonics_hash: decoded.hashes().tectonics,
        reference_water_inventory_m3: decoded.reference_water_inventory_m3(),
        historical_forcing: historical_forcing(&settings),
    };
    let identity = physical_identity(&manifest);
    Ok(ValidatedPhysicalSource {
        source: decoded,
        report,
        identity,
        legacy_world: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use daena_physical::engine::{Settings, WorldPreset};
    use daena_physical::planetary::PlanetaryConfiguration;
    use daena_physical::NoopProgress;

    fn products(resolution: u32) -> daena_physical::engine::Products {
        let settings = Settings::from_parts(
            831_429,
            WorldPreset::Earthlike,
            resolution,
            6_371_000,
            290_000,
            PlanetaryConfiguration::earth_like(),
        );
        engine::generate(settings, &mut NoopProgress).unwrap()
    }

    fn generation(products: &daena_physical::engine::Products) -> Value {
        products.generation().unwrap().clone()
    }

    #[test]
    fn engine_generation_validates_and_has_a_stable_identity() {
        let world = products(32);
        let generation = generation(&world);
        let first = validate_source(world.source(), &generation).unwrap();
        let second = validate_source(world.source(), &generation).unwrap();
        assert_eq!(first.identity, second.identity);
        assert_eq!(
            first.report.reference_water_inventory_m3,
            world.reference_water_inventory_m3()
        );
        assert!(first.identity.starts_with("sha256:"));
    }

    #[test]
    fn old_source_magic_is_an_unsupported_version() {
        let world = products(32);
        let mut old = world.source().to_vec();
        old[0..8].copy_from_slice(b"DAENAPW1");
        let generation = generation(&world);
        let error = validate_source(&old, &generation).unwrap_err();
        assert!(error.to_string().contains(CODE_UNSUPPORTED_SOURCE_VERSION));
    }

    #[test]
    fn source_and_generation_mismatches_are_invalid() {
        let world = products(32);
        let mut mismatched = generation(&world);
        mismatched["settings"]["targetLandFractionPpm"] = serde_json::json!(300_001);
        let error = validate_source(world.source(), &mismatched).unwrap_err();
        assert!(error.to_string().contains("source provenance mismatch"));
        assert!(error.to_string().contains(CODE_INVALID_SOURCE));

        let mut incomplete = generation(&world);
        incomplete["settings"]
            .as_object_mut()
            .unwrap()
            .remove("referenceWaterInventoryM3");
        let error = validate_source(world.source(), &incomplete).unwrap_err();
        assert!(error.to_string().contains(CODE_INVALID_GENERATION));
    }

    #[test]
    fn changed_source_bytes_fail_elevation_validation() {
        let world = products(32);
        let mut source = world.source().to_vec();
        let last = source.len() - 1;
        source[last] ^= 1;
        let error = validate_source(&source, &generation(&world)).unwrap_err();
        assert!(error.to_string().contains(CODE_INVALID_SOURCE));
    }

    #[test]
    fn identity_manifest_is_deterministic() {
        let world = products(32);
        let validated = validate_source(world.source(), &generation(&world)).unwrap();
        let manifest = PhysicalIdentityManifestV2 {
            provider_id: super::super::PHYSICAL_PROVIDER.into(),
            source_format: super::super::PHYSICAL_SOURCE_FORMAT.into(),
            adapter_version: super::super::PHYSICAL_ADAPTER_VERSION,
            generator_id: super::super::PHYSICAL_GENERATOR_ID.into(),
            generator_version: super::super::PHYSICAL_GENERATOR_VERSION,
            source_version: engine::SOURCE_VERSION,
            engine_version: engine::WORLD_ENGINE_VERSION,
            resolution: 32,
            source_sha256: format!("sha256:{:x}", Sha256::digest(world.source())),
            elevation_hash: validated.source.hashes().elevation_mm,
            tectonics_hash: validated.source.hashes().tectonics,
            reference_water_inventory_m3: validated.report.reference_water_inventory_m3,
            historical_forcing: HistoricalForcingParameters::default_for(831_429, 0),
        };
        let first = encode_identity_manifest(&manifest);
        let second = encode_identity_manifest(&manifest);
        assert_eq!(first, second);
        assert!(!first.is_empty());
    }
}
