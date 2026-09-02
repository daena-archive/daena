// Maps navigation service.
use super::*;

/// Versioned host handoff for the public `daena.maps/navigation@1` service.
/// The service resolves canonical links before asking the shell to mount Maps;
/// provider availability remains a concern of the child webview.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct MapsNavigationRequest {
    pub(super) operation: String,
    #[serde(default)]
    pub(super) map_entity_id: Option<String>,
    #[serde(default)]
    pub(super) entity_id: Option<String>,
    #[serde(default)]
    pub(super) link_id: Option<String>,
    #[serde(default)]
    pub(super) date: Option<serde_json::Value>,
    #[serde(default)]
    pub(super) entity_ids: Option<Vec<String>>,
}

/// The result of resolving a navigation operation. `emit` carries the shell
/// handoff (`maps-navigation` Tauri event) when the shell must mount or
/// re-focus the map editor; `result` is the service response. An unresolved
/// link emits the handoff (so the map opens and the notice can surface) while
/// still returning a typed `link-unresolved` error.
pub(super) struct MapsNavigationOutcome {
    pub(super) emit: Option<(String, Option<serde_json::Value>)>,
    pub(super) result: Result<serde_json::Value, String>,
}

pub(super) fn resolve_maps_navigation(
    core: &mut CoreService,
    request: &MapsNavigationRequest,
) -> Result<MapsNavigationOutcome, String> {
    let project = core
        .project(trusted_shell())
        .map_err(|error| error.to_string())?;
    let outcome = match request.operation.as_str() {
        "openMap" => {
            let id = request
                .map_entity_id
                .clone()
                .ok_or_else(|| "map-unavailable: mapEntityId is required".to_string())?;
            let exists = project
                .list_entities()
                .map_err(|error| error.to_string())?
                .into_iter()
                .any(|entity| {
                    entity.id == id
                        && entity.entity_type.as_deref() == Some(daena_core::maps::MAP_ENTITY_TYPE)
                });
            if !exists {
                return Err("map-unavailable".into());
            }
            let link = request.link_id.clone().map(serde_json::Value::String);
            MapsNavigationOutcome {
                emit: Some((id.clone(), link)),
                result: Ok(serde_json::json!({
                    "mapEntityId": id,
                    "linkId": request.link_id,
                })),
            }
        }
        "focusEntity" => {
            let id = request
                .entity_id
                .clone()
                .ok_or_else(|| "not-on-map: entityId is required".to_string())?;
            let locations = project
                .map_locations_for_entity(id)
                .map_err(|error| error.to_string())?;
            let filtered: Vec<serde_json::Value> = locations
                .into_iter()
                .filter(|location| {
                    request.map_entity_id.as_deref().is_none_or(|map| {
                        map == location["mapEntityId"].as_str().unwrap_or_default()
                    })
                })
                .collect();
            if let Some(link_id) = request.link_id.as_deref() {
                let Some(location) = filtered
                    .iter()
                    .find(|location| location["id"].as_str() == Some(link_id))
                else {
                    return Err("link-unresolved: location no longer exists on the entity".into());
                };
                let map_id = location["mapEntityId"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                let unresolved = location["anchorKind"].as_str() == Some("provider-feature")
                    && location["resolution"].as_str() == Some("unresolved");
                let result = if unresolved {
                    Err("link-unresolved: the map feature was removed or renumbered".into())
                } else {
                    Ok(serde_json::json!({
                        "mapEntityId": map_id,
                        "linkId": link_id,
                    }))
                };
                MapsNavigationOutcome {
                    emit: Some((map_id, Some(serde_json::Value::String(link_id.to_string())))),
                    result,
                }
            } else if filtered.is_empty() {
                return Err("not-on-map".into());
            } else if filtered.len() > 1 {
                MapsNavigationOutcome {
                    emit: None,
                    result: Ok(serde_json::json!({
                        "status": "multiple-links",
                        "locations": filtered,
                    })),
                }
            } else {
                let location = &filtered[0];
                let map_id = location["mapEntityId"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                let link_id = location["id"].as_str().unwrap_or_default().to_string();
                let unresolved = location["anchorKind"].as_str() == Some("provider-feature")
                    && location["resolution"].as_str() == Some("unresolved");
                let result = if unresolved {
                    Err("link-unresolved: the map feature was removed or renumbered".into())
                } else {
                    Ok(serde_json::json!({
                        "mapEntityId": map_id,
                        "linkId": link_id,
                    }))
                };
                MapsNavigationOutcome {
                    emit: Some((map_id, Some(serde_json::Value::String(link_id)))),
                    result,
                }
            }
        }
        "listLocations" => {
            let id = request
                .entity_id
                .clone()
                .ok_or_else(|| "entityId is required".to_string())?;
            let locations = project
                .map_locations(id)
                .map_err(|error| error.to_string())?;
            MapsNavigationOutcome {
                emit: None,
                result: serde_json::to_value(locations)
                    .map_err(|error| format!("serialize locations: {error}")),
            }
        }
        "setDate" => {
            let date = request
                .date
                .clone()
                .ok_or_else(|| "setDate requires a date payload".to_string())?;
            let era_ok = date
                .get("era")
                .and_then(serde_json::Value::as_str)
                .is_none_or(|era| era == "BCE" || era == "CE");
            if !date.is_object() || date.get("year").is_none() || !era_ok {
                return Err("validation: invalid date payload".into());
            }
            MapsNavigationOutcome {
                emit: None,
                result: Ok(serde_json::json!({ "accepted": true, "date": date })),
            }
        }
        "showResults" => {
            let ids = request
                .entity_ids
                .clone()
                .ok_or_else(|| "showResults requires entityIds".to_string())?;
            let mut rows: Vec<serde_json::Value> = Vec::new();
            for entity_id in &ids {
                let locations = project
                    .map_locations_for_entity(entity_id.clone())
                    .map_err(|error| error.to_string())?;
                rows.extend(locations.into_iter().filter(|location| {
                    request.map_entity_id.as_deref().is_none_or(|map| {
                        map == location["mapEntityId"].as_str().unwrap_or_default()
                    })
                }));
            }
            let map_id = request
                .map_entity_id
                .clone()
                .or_else(|| {
                    rows.first()
                        .and_then(|row| row["mapEntityId"].as_str())
                        .map(String::from)
                })
                .ok_or_else(|| "not-on-map: no locations for the requested entities".to_string())?;
            rows.retain(|row| row["mapEntityId"].as_str() == Some(map_id.as_str()));
            if rows.is_empty() {
                return Err("not-on-map: no locations for the requested entities".into());
            }
            MapsNavigationOutcome {
                emit: Some((map_id.clone(), None)),
                result: Ok(serde_json::json!({ "mapEntityId": map_id, "locations": rows })),
            }
        }
        _ => {
            return Err("unsupported daena.maps/navigation@1 operation".into());
        }
    };
    Ok(outcome)
}

/// Plugin RPC envelopes carry correlation-only `requestId`s (for example
/// `maps-request-N`). Transaction receipts are UUID-keyed, so only pass a
/// request id into core mutations when it is a real UUID; the response echo
/// keeps the original envelope id.
pub(super) fn sanitize_mutation_request_id(request_id: &str) -> Option<&str> {
    request_id
        .parse::<uuid::Uuid>()
        .is_ok()
        .then_some(request_id)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub(super) async fn maps_navigation(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedCore>,
    operation: String,
    map_entity_id: Option<String>,
    entity_id: Option<String>,
    link_id: Option<String>,
    date: Option<serde_json::Value>,
    entity_ids: Option<Vec<String>>,
) -> Result<serde_json::Value, String> {
    let request = MapsNavigationRequest {
        operation,
        map_entity_id,
        entity_id,
        link_id,
        date,
        entity_ids,
    };
    let outcome = with_core(state, move |core| {
        resolve_maps_navigation(core, &request).map_err(daena_core::CoreError::Validation)
    })
    .await?;
    if let Some((map_id, link)) = &outcome.emit {
        app.emit(
            "maps-navigation",
            serde_json::json!({
                "mapEntityId": map_id,
                "linkId": link.as_ref().and_then(|value| value.as_str()),
            }),
        )
        .map_err(|error| error.to_string())?;
    }
    outcome.result
}

/// Native handler for the public `daena.maps/navigation@1` service. Registered
/// on the plugin host before project activation so the manifest-declared WASM
/// stub is skipped; consumers such as Lore and Timeline reach the same
/// resolution and shell handoff as the `maps_navigation` Tauri command.
pub(super) fn maps_navigation_service_handler(
    core: SharedCore,
) -> daena_plugin_host::ServiceHandler {
    use daena_plugin_host::{HostError, ServiceRequest};
    std::sync::Arc::new(move |request: ServiceRequest| {
        let app = APP_HANDLE
            .get()
            .ok_or_else(|| HostError("map editor is not open".into()))?;
        let request: MapsNavigationRequest = serde_json::from_value(request.payload.clone())
            .map_err(|error| {
                HostError(format!("invalid daena.maps/navigation@1 payload: {error}"))
            })?;
        let session = current_session(&core).map_err(HostError)?;
        let mut core = session
            .core
            .lock()
            .map_err(|_| HostError("core lock poisoned".into()))?;
        let outcome = resolve_maps_navigation(&mut core, &request).map_err(HostError)?;
        if let Some((map_id, link)) = &outcome.emit {
            let _ = app.emit(
                "maps-navigation",
                serde_json::json!({
                    "mapEntityId": map_id,
                    "linkId": link.as_ref().and_then(|value| value.as_str()),
                }),
            );
        }
        outcome.result.map_err(HostError)
    })
}
