//! Atlas capability reporting and immutable snapshot capture.

use daena_atlas::request::{
    physical_layer_role, AtlasRenderRequest, ResourceEstimate, ATLAS_LAYER_ROLES,
};
use daena_atlas::style::bundled_style_ids;
use daena_physical::history::{
    ForcingComponent, HistoricalForcingParameters, FORCING_COMPONENT_COUNT,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    HistoricalForcingSettings, MapDescriptor, MapGenerationSettings, PHYSICAL_ADAPTER_VERSION,
    PHYSICAL_PROVIDER, PHYSICAL_SOURCE_FORMAT,
};
use crate::error::CoreError;
use crate::project::ProjectStore;

pub const CODE_PROVIDER_UNSUPPORTED: &str = daena_atlas::CODE_PROVIDER_UNSUPPORTED;
pub const CODE_REQUEST_INVALID: &str = daena_atlas::CODE_REQUEST_INVALID;
pub const CODE_SNAPSHOT_CHANGED: &str = "atlas.snapshot.changed";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AtlasRenderCapabilities {
    pub supported: bool,
    pub time_modes: Vec<String>,
    pub projections: Vec<String>,
    pub formats: Vec<String>,
    pub styles: Vec<String>,
    pub layers: Vec<AtlasLayerChoice>,
    pub max_width_px: u32,
    pub max_height_px: u32,
    pub max_pixel_count: u64,
    pub supports_authored_layers: bool,
    pub supports_semantic_layers: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AtlasLayerChoice {
    pub id: String,
    pub name: String,
    pub role: String,
    pub default_visible: bool,
}

#[derive(Debug, Clone)]
pub struct AtlasRenderSnapshot {
    pub request: AtlasRenderRequest,
    pub estimate: ResourceEstimate,
    pub identity: String,
    pub source_bytes: Vec<u8>,
    pub source_sha256: String,
    pub forcing: HistoricalForcingParameters,
    pub content_generation: i64,
    pub database_epoch: String,
    pub map_entity_id: String,
}

pub fn capabilities_for_descriptor(descriptor: &Value, layers: &Value) -> AtlasRenderCapabilities {
    let supported = descriptor
        .get("provider")
        .and_then(|provider| provider.get("id"))
        .and_then(Value::as_str)
        == Some(PHYSICAL_PROVIDER)
        && descriptor
            .get("provider")
            .and_then(|provider| provider.get("sourceFormat"))
            .and_then(Value::as_str)
            == Some(PHYSICAL_SOURCE_FORMAT)
        && descriptor
            .get("provider")
            .and_then(|provider| provider.get("adapterVersion"))
            .and_then(Value::as_u64)
            == Some(u64::from(PHYSICAL_ADAPTER_VERSION));
    let mut choices = ATLAS_LAYER_ROLES
        .iter()
        .map(|role| AtlasLayerChoice {
            id: (*role).to_string(),
            name: role_name(role),
            role: (*role).to_string(),
            default_visible: matches!(
                *role,
                "ocean"
                    | "relief"
                    | "ice"
                    | "lakes"
                    | "rivers"
                    | "coastlines"
                    | "graticule"
                    | "frame"
            ),
        })
        .collect::<Vec<_>>();
    if let Some(stored) = layers.get("layers").and_then(Value::as_array) {
        for layer in stored {
            let Some(id) = layer.get("id").and_then(Value::as_str) else {
                continue;
            };
            if let Some(role) = physical_layer_role(id) {
                if let Some(choice) = choices.iter_mut().find(|choice| choice.role == role) {
                    if let Some(visible) = layer.get("defaultVisible").and_then(Value::as_bool) {
                        choice.default_visible = visible;
                    }
                    if let Some(name) = layer.get("name").and_then(Value::as_str) {
                        if role == id {
                            choice.name = name.to_string();
                        }
                    }
                }
            }
        }
    }
    AtlasRenderCapabilities {
        supported,
        time_modes: if supported {
            vec!["physical-offset-year".into()]
        } else {
            Vec::new()
        },
        projections: if supported {
            vec!["equirectangular".into()]
        } else {
            Vec::new()
        },
        formats: if supported {
            vec!["png".into()]
        } else {
            Vec::new()
        },
        styles: if supported {
            bundled_style_ids()
                .iter()
                .map(|id| (*id).to_string())
                .collect()
        } else {
            Vec::new()
        },
        layers: if supported { choices } else { Vec::new() },
        max_width_px: daena_atlas::request::PREVIEW_MAX_WIDTH * 4,
        max_height_px: daena_atlas::request::PREVIEW_MAX_HEIGHT * 4,
        max_pixel_count: daena_atlas::request::MAX_PIXEL_COUNT,
        supports_authored_layers: false,
        supports_semantic_layers: false,
    }
}

fn role_name(role: &str) -> String {
    match role {
        "ocean" => "Ocean".into(),
        "relief" => "Relief".into(),
        "ice" => "Land ice".into(),
        "lakes" => "Lakes".into(),
        "rivers" => "Rivers".into(),
        "coastlines" => "Coastlines".into(),
        "contours" => "Contours".into(),
        "graticule" => "Graticule".into(),
        "frame" => "Frame".into(),
        other => other.to_string(),
    }
}

pub fn capabilities_for_map(
    project: &ProjectStore,
    map_entity_id: &str,
) -> Result<AtlasRenderCapabilities, CoreError> {
    let (descriptor, layers) = map_fields(project, map_entity_id)?;
    Ok(capabilities_for_descriptor(&descriptor, &layers))
}

pub fn capture_snapshot(
    project: &ProjectStore,
    map_entity_id: &str,
    request: AtlasRenderRequest,
) -> Result<AtlasRenderSnapshot, CoreError> {
    let request = request
        .normalize()
        .map_err(|error| CoreError::Validation(format!("{}: {}", error.code, error.message)))?;
    let (descriptor_value, _layers) = map_fields(project, map_entity_id)?;
    let capabilities = capabilities_for_descriptor(&descriptor_value, &_layers);
    if !capabilities.supported {
        return Err(CoreError::Validation(format!(
            "{CODE_PROVIDER_UNSUPPORTED}: map provider does not support atlas rendering"
        )));
    }
    let descriptor: MapDescriptor = serde_json::from_value(descriptor_value.clone())?;
    let source_id = descriptor
        .source_asset_id
        .ok_or_else(|| CoreError::Validation("maps: sourceAssetId is missing".into()))?;
    let generation = descriptor_value
        .get("generation")
        .cloned()
        .ok_or_else(|| CoreError::Validation("maps: physical generation is missing".into()))?;
    let source_bytes = project.asset_bytes(source_id)?;
    let validated = super::physical::validate_source(&source_bytes, &generation)?;
    let forcing = forcing_from_generation(&generation)?;
    let source_sha256 = {
        use sha2::{Digest, Sha256};
        format!("sha256:{:x}", Sha256::digest(&source_bytes))
    };
    Ok(AtlasRenderSnapshot {
        estimate: request.estimate(),
        request,
        identity: validated.identity,
        source_bytes,
        source_sha256,
        forcing,
        content_generation: project.content_generation()?,
        database_epoch: project.database_epoch().to_string(),
        map_entity_id: map_entity_id.to_string(),
    })
}

fn map_fields(project: &ProjectStore, map_entity_id: &str) -> Result<(Value, Value), CoreError> {
    let fields = project.list_fields(map_entity_id.to_string())?;
    let descriptor = fields
        .iter()
        .find(|field| field.namespace == super::MAP_NAMESPACE && field.key == "map")
        .ok_or_else(|| CoreError::Validation("maps:map descriptor is missing".into()))?
        .value
        .clone();
    let layers = fields
        .iter()
        .find(|field| field.namespace == super::MAP_NAMESPACE && field.key == "layers")
        .map(|field| field.value.clone())
        .unwrap_or_else(|| serde_json::json!({"schemaVersion": 1, "layers": []}));
    Ok((descriptor, layers))
}

fn forcing_from_generation(generation: &Value) -> Result<HistoricalForcingParameters, CoreError> {
    let settings = generation
        .get("settings")
        .cloned()
        .ok_or_else(|| CoreError::Validation("physical generation settings are missing".into()))?;
    match serde_json::from_value::<MapGenerationSettings>(settings)? {
        MapGenerationSettings::Physical(physical) => forcing_params(&physical.historical_forcing),
        MapGenerationSettings::Vector(_) => Err(CoreError::Validation(format!(
            "{CODE_PROVIDER_UNSUPPORTED}: map provider does not support atlas rendering"
        ))),
    }
}

fn forcing_params(
    settings: &HistoricalForcingSettings,
) -> Result<HistoricalForcingParameters, CoreError> {
    if settings.components.len() != FORCING_COMPONENT_COUNT {
        return Err(CoreError::Validation(
            "historicalForcing.components must contain three independent terms".into(),
        ));
    }
    Ok(HistoricalForcingParameters {
        version: settings.version,
        components: [
            component(&settings.components[0]),
            component(&settings.components[1]),
            component(&settings.components[2]),
        ],
        sensitivity_ppm: settings.sensitivity_ppm,
        land_ice_amplitude_ppm: settings.land_ice_amplitude_ppm,
        ice_response_years: settings.ice_response_years,
        ice_midpoint_centi_c: settings.ice_midpoint_centi_c,
        ice_transition_width_centi_c: settings.ice_transition_width_centi_c,
        thermal_expansion_ppm_per_degree_c: settings.thermal_expansion_ppm_per_degree_c,
    })
}

fn component(value: &super::HistoricalForcingComponentSettings) -> ForcingComponent {
    ForcingComponent {
        amplitude_centi_c: value.amplitude_centi_c,
        period_years: value.period_years,
        phase_offset_years: value.phase_offset_years,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_providers_are_reported_not_inferred_from_names() {
        let descriptor = serde_json::json!({
            "schemaVersion": 1,
            "provider": {"id": "daena-vector", "adapterVersion": 1, "sourceFormat": "geojson"}
        });
        let capabilities =
            capabilities_for_descriptor(&descriptor, &serde_json::json!({"layers": []}));
        assert!(!capabilities.supported);
        assert!(capabilities.formats.is_empty());
        let physical = serde_json::json!({
            "schemaVersion": 1,
            "provider": {
                "id": "daena-physical",
                "adapterVersion": 2,
                "sourceFormat": "physical-world-v2"
            }
        });
        let supported = capabilities_for_descriptor(&physical, &serde_json::json!({"layers": []}));
        assert!(supported.supported);
        assert_eq!(supported.formats, vec!["png"]);
        assert!(supported.styles.contains(&"daena-atlas-relief".into()));
        assert!(supported.styles.contains(&"daena-atlas-antique".into()));
    }

    #[test]
    fn snapshot_captures_identity_forcing_and_generation_without_mutating() {
        let settings = daena_physical::GenerationSettings {
            width: 16,
            height: 8,
            radius_metres: daena_physical::DEFAULT_RADIUS_METRES,
            target_land_fraction_ppm: 300_000,
        };
        let world =
            daena_physical::generate_world(settings, 831_429, 0, &mut daena_physical::NoopProgress)
                .unwrap();
        let generation = serde_json::json!({
            "id": crate::maps::PHYSICAL_GENERATOR_ID,
            "version": crate::maps::PHYSICAL_GENERATOR_VERSION,
            "seed": 831429,
            "retryIndex": 0,
            "settings": {
                "width": settings.width,
                "height": settings.height,
                "radiusMetres": settings.radius_metres,
                "targetLandFractionPpm": settings.target_land_fraction_ppm,
                "referenceWaterInventoryM3": world.report.reference_water_inventory_m3,
                "plateCount": world.tectonics.settings.plate_count,
                "continentalPlateCount": world.tectonics.settings.continental_plate_count,
                "tectonicActivityPpm": world.tectonics.settings.tectonic_activity_ppm,
                "islandActivityPpm": world.tectonics.settings.island_activity_ppm,
                "evolutionPreset": "mature",
                "hazardDerivationVersion": daena_physical::hazards::HAZARD_DERIVATION_VERSION,
                "historicalForcing": {
                    "version": 2,
                    "components": [
                        { "amplitudeCentiC": 180, "periodYears": 12000, "phaseOffsetYears": 0 },
                        { "amplitudeCentiC": 90, "periodYears": 4100, "phaseOffsetYears": 200 },
                        { "amplitudeCentiC": 40, "periodYears": 2300, "phaseOffsetYears": 800 }
                    ],
                    "sensitivityPpm": 1000000,
                    "landIceAmplitudePpm": 24000,
                    "iceResponseYears": 800,
                    "iceMidpointCentiC": 0,
                    "iceTransitionWidthCentiC": 400,
                    "thermalExpansionPpmPerDegreeC": 210
                }
            }
        });
        let root = std::env::temp_dir().join(format!("daena-atlas-snap-{}", uuid::Uuid::new_v4()));
        let store = ProjectStore::open_directory(&root).unwrap();
        let accepted = store
            .accept_physical_map(
                "Atlas snapshot".into(),
                world.source.clone(),
                generation,
                Some("00000000-0000-4000-8000-0000000000aa"),
            )
            .unwrap();
        let before = store.content_generation().unwrap();
        let snapshot = capture_snapshot(
            &store,
            &accepted.entity.id,
            AtlasRenderRequest::spike_png(64, 32).unwrap(),
        )
        .unwrap();
        assert_eq!(store.content_generation().unwrap(), before);
        assert_eq!(snapshot.content_generation, before);
        assert_eq!(snapshot.identity, accepted.physical_identity);
        assert_eq!(snapshot.forcing.version, 2);
        assert_eq!(snapshot.forcing.land_ice_amplitude_ppm, 24_000);
        store
            .update_entity(accepted.entity.id.clone(), Some("Renamed".into()), None)
            .unwrap();
        let after = store.content_generation().unwrap();
        assert!(after > snapshot.content_generation);
        let _ = std::fs::remove_dir_all(root);
    }
}
