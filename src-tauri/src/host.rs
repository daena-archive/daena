// Plugin host bootstrap and dispatch.
use super::*;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PluginBootstrap {
    pub(super) rpc_version: u32,
    pub(super) session_id: String,
    pub(super) plugin_id: String,
    pub(super) project_id: String,
    pub(super) version: String,
    pub(super) host_api: String,
    pub(super) granted_capabilities: Vec<String>,
    pub(super) optional_features: Vec<String>,
    pub(super) package_digest: String,
    pub(super) manifest: PluginManifest,
}

/// Bootstrap and RPC use a host-created `plugin:<id>` webview label as the
/// origin binding. The trusted main window cannot call this surface, and the
/// plugin cannot submit an arbitrary origin in its request payload.
#[tauri::command]
pub(super) fn plugin_bootstrap(
    window: tauri::WebviewWindow,
    core: tauri::State<'_, SharedCore>,
    state: tauri::State<'_, SharedPluginHost>,
    plugin_id: String,
    project_id: String,
) -> Result<PluginBootstrap, String> {
    let origin = plugin_webview_identity(&window, Some(&plugin_id))?;
    let current_project = current_info(core.inner())?
        .map(|info| info.root)
        .ok_or_else(|| "project is not open".to_string())?;
    if current_project != project_id {
        return Err("plugin bootstrap project mismatch".into());
    }
    let mut host = state
        .lock()
        .map_err(|_| "plugin host lock poisoned".to_string())?;
    let session = host
        .bootstrap(&plugin_id, &project_id, &origin)
        .map_err(|error| error.to_string())?;
    let entry = host
        .runtime_entry(&project_id, &plugin_id)
        .ok_or_else(|| "plugin was removed during bootstrap".to_string())?;
    Ok(PluginBootstrap {
        rpc_version: daena_plugin_api::RPC_VERSION,
        session_id: session.id.clone(),
        plugin_id,
        project_id,
        version: entry.manifest.version.clone(),
        host_api: entry.manifest.host_api.clone(),
        granted_capabilities: session.grants.iter().cloned().collect(),
        optional_features: Vec::new(),
        package_digest: entry.digest,
        manifest: entry.manifest,
    })
}

#[tauri::command]
pub(super) async fn plugin_rpc(
    window: tauri::WebviewWindow,
    core: tauri::State<'_, SharedCore>,
    state: tauri::State<'_, SharedPluginHost>,
    settings: tauri::State<'_, SharedSettings>,
    ai_runtime: tauri::State<'_, ai::SharedAiRuntime>,
    request: RpcRequest,
) -> Result<RpcResponse, String> {
    let mut request = request;
    if matches!(
        request.method.as_str(),
        "relationship.delete" | "relationship.update"
    ) {
        let id = payload_string(&request.payload, "id").map_err(|error| error.to_string())?;
        let stored = with_core(core.clone(), move |core| {
            core.project(trusted_shell())?.relationship(id)
        })
        .await?;
        let object = request
            .payload
            .as_object_mut()
            .ok_or_else(|| "relationship delete payload must be an object".to_string())?;
        object.insert(
            "__stored_relationship_type".into(),
            serde_json::Value::String(stored.relationship_type),
        );
    }
    let origin = plugin_webview_identity(&window, None)?;
    let session = {
        let host = state
            .lock()
            .map_err(|_| "plugin host lock poisoned".to_string())?;
        host.authorize_rpc(&origin, &request).map_err(|error| {
            serde_json::to_string(&error).unwrap_or_else(|_| error.message.clone())
        })?
    };
    let request_id = request.request_id.clone();
    let plugin_id = session.plugin_id.clone();
    let method = request.method;
    let event_method = method.clone();
    let mut payload = request.payload;
    if matches!(
        method.as_str(),
        "relationship.delete" | "relationship.update"
    ) {
        payload
            .as_object_mut()
            .ok_or_else(|| "relationship mutation payload must be an object".to_string())?
            .remove("__stored_relationship_type");
    }
    let shared_field_keys = {
        let host = state
            .lock()
            .map_err(|_| "plugin host lock poisoned".to_string())?;
        shared_field_keys_for_request(&host, &plugin_id, &method, &payload)?
    };
    let current_project = current_info(&core)?.map(|info| info.root);
    let event_project_id = session.project_id.clone();
    let result = if method == "app.version" {
        Ok(serde_json::json!({ "version": crate::version::current() }))
    } else if current_project.as_deref() != Some(session.project_id.as_str()) {
        Err("plugin session is not bound to the open project".to_string())
    } else if matches!(
        method.as_str(),
        "event.subscribe"
            | "event.poll"
            | "event.publish"
            | "service.call"
            | "ai.request.start"
            | "ai.request.poll"
            | "ai.request.cancel"
            | "ai.request.result"
            | "ai.request.citations"
    ) {
        dispatch_host_rpc(
            &state,
            &session.plugin_id,
            &session.project_id,
            &method,
            payload,
            AiBrokerContext {
                app: Some(window.app_handle().clone()),
                core: Some(core.inner().clone()),
                settings: Some(settings.inner().clone()),
                ai_runtime: ai_runtime.inner().clone(),
                session_id: session.id.clone(),
                caller: daena_ai::AiCaller::authorized_plugin(
                    session.plugin_id.clone(),
                    session.project_id.clone(),
                    session.grants.iter().cloned().collect(),
                    vec![format!("project:{}", session.project_id)],
                    session.generation,
                    "pending",
                ),
            },
        )
    } else {
        let project_id = session.project_id;
        let request_id_for_dispatch = sanitize_mutation_request_id(&request_id).map(str::to_owned);
        let record_owner_entity_types = method
            .starts_with("record.")
            .then(|| {
                let collection = payload
                    .get("collection")
                    .and_then(serde_json::Value::as_str)?;
                state.lock().ok()?.record_owner_entity_types(
                    &project_id,
                    &session.plugin_id,
                    collection,
                )
            })
            .flatten();
        with_core(core, move |core| {
            let current_project = core
                .info()
                .map(|info| info.root)
                .ok_or(CoreError::ProjectNotOpen)?;
            if current_project != project_id {
                return Err(CoreError::Unauthorized {
                    operation: "access another project",
                });
            }
            dispatch_module_rpc(
                core,
                Some(&session.plugin_id),
                shared_field_keys,
                record_owner_entity_types,
                &method,
                payload,
                request_id_for_dispatch.as_deref(),
            )
        })
        .await
    };
    let result = result.and_then(|result| {
        publish_core_mutation_event(state.inner(), &event_project_id, &event_method, &result)?;
        Ok(result)
    });
    match result {
        Ok(result) => Ok(RpcResponse {
            rpc_version: daena_plugin_api::RPC_VERSION,
            request_id,
            ok: true,
            result: Some(result),
            error: None,
        }),
        Err(error) => Ok(RpcResponse {
            rpc_version: daena_plugin_api::RPC_VERSION,
            request_id,
            ok: false,
            result: None,
            error: Some(daena_plugin_api::RpcError {
                code: "core.error".into(),
                message: format!("plugin {plugin_id}: {error}"),
                retryable: false,
                details: None,
            }),
        }),
    }
}

pub(super) struct AiBrokerContext {
    pub(super) app: Option<tauri::AppHandle>,
    pub(super) core: Option<SharedCore>,
    pub(super) settings: Option<SharedSettings>,
    pub(super) ai_runtime: ai::SharedAiRuntime,
    pub(super) session_id: String,
    pub(super) caller: daena_ai::AiCaller,
}

pub(super) fn dispatch_host_rpc(
    plugins: &SharedPluginHost,
    plugin_id: &str,
    project_id: &str,
    method: &str,
    payload: serde_json::Value,
    context: AiBrokerContext,
) -> Result<serde_json::Value, String> {
    let mut host = plugins
        .lock()
        .map_err(|_| "plugin host lock poisoned".to_string())?;
    if method == "service.call" {
        let name = payload
            .get("name")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "service RPC payload requires name".to_string())?;
        let major = payload
            .get("major")
            .and_then(serde_json::Value::as_u64)
            .and_then(|major| u32::try_from(major).ok())
            .ok_or_else(|| "service RPC payload requires a valid major".to_string())?;
        let deadline_ms = payload
            .get("deadlineMs")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(5_000)
            .clamp(1, 30_000);
        return host
            .call_service_authorized(
                plugin_id,
                project_id,
                name,
                major,
                payload
                    .get("payload")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
                std::time::Duration::from_millis(deadline_ms),
            )
            .map_err(|error| error.to_string());
    }

    if method.starts_with("ai.request.") {
        drop(host);
        let request_id_value = payload.get("requestId").and_then(serde_json::Value::as_str);
        if method == "ai.request.start" {
            if let Some(core) = context.core.as_ref() {
                if current_info(core)?.is_some() {
                    ai::ensure_active_project(core, &context.caller.project_id)?;
                }
            }
            crate::ensure_project_ai_enabled(&context.caller.project_id)?;
            let operation = payload
                .get("operation")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "AI operation is required".to_string())?;
            let instruction = payload
                .get("userInstruction")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "AI instruction is required".to_string())?;
            let immediate_context = payload
                .get("immediateContext")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let deadline_ms = payload
                .get("deadlineMs")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(daena_ai::DEFAULT_GENERATION_DEADLINE.as_millis() as u64)
                .clamp(1, daena_ai::MAX_GENERATION_DEADLINE.as_millis() as u64);
            let selection = immediate_context
                .get("selection")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            let retrieval_policy = payload
                .get("retrievalPolicy")
                .cloned()
                .map(serde_json::from_value::<daena_plugin_api::AiRetrievalPolicyPayload>)
                .transpose()
                .map_err(|error| format!("invalid retrieval policy: {error}"))?;
            let (retrieved_context, citations) = if let Some(policy) = retrieval_policy {
                let core = context
                    .core
                    .as_ref()
                    .ok_or_else(|| "AI retrieval requires an open project".to_string())?;
                let session = current_session(core)?;
                let core = session
                    .core
                    .lock()
                    .map_err(|_| "core lock poisoned".to_string())?;
                let project = core
                    .project(trusted_shell())
                    .map_err(|error| error.to_string())?;
                ai::build_retrieval_context(project, &context.caller, &policy)?
            } else {
                (String::new(), Vec::new())
            };
            let selection = if retrieved_context.is_empty() {
                selection
            } else {
                format!(
                    "{selection}\n\n[RETRIEVED_CONTEXT]\n{retrieved_context}\n[/RETRIEVED_CONTEXT]"
                )
            };
            let settings = context
                .settings
                .map(|settings| {
                    settings
                        .lock()
                        .map_err(|_| "settings lock poisoned".to_string())
                        .and_then(|store| store.load())
                })
                .transpose()?
                .unwrap_or_default();
            let provider =
                ai::resolve_ai_provider(&settings, Some(&context.caller.project_id), true)?;
            let started = match operation {
                "generate_text" => ai::start_ai_request_mode(
                    context.app.clone(),
                    context.ai_runtime.clone(),
                    context.caller.clone(),
                    provider.endpoint.clone(),
                    provider.model.clone(),
                    instruction.to_string(),
                    selection,
                    None,
                    std::time::Duration::from_millis(deadline_ms),
                    citations.clone(),
                    provider.remote,
                    provider.api_key.clone(),
                )?,
                "generate_structured" => {
                    let contract = payload.get("outputContract").cloned().ok_or_else(|| {
                        "structured AI requests require outputContract".to_string()
                    })?;
                    ai::validate_structured_schema(&contract)?;
                    ai::start_ai_request_mode(
                        context.app.clone(),
                        context.ai_runtime.clone(),
                        context.caller.clone(),
                        provider.endpoint,
                        provider.model,
                        instruction.to_string(),
                        if retrieved_context.is_empty() {
                            immediate_context.to_string()
                        } else {
                            selection.clone()
                        },
                        Some(contract),
                        std::time::Duration::from_millis(deadline_ms),
                        citations,
                        provider.remote,
                        provider.api_key,
                    )?
                }
                _ => return Err("unsupported AI operation".into()),
            };
            let mut host = plugins
                .lock()
                .map_err(|_| "plugin host lock poisoned".to_string())?;
            host.register_ai_request(
                &started,
                project_id,
                plugin_id,
                &context.session_id,
                operation,
                payload.get("outputContract").cloned(),
            );
            return Ok(serde_json::json!({ "requestId": started }));
        }
        let target = request_id_value.ok_or_else(|| "AI requestId is required".to_string())?;
        let mut host = plugins
            .lock()
            .map_err(|_| "plugin host lock poisoned".to_string())?;
        let operation = host
            .authorize_ai_request(target, project_id, plugin_id, &context.session_id)
            .map_err(|error| error.to_string())?;
        let contract = host.ai_contract(target);
        match method {
            "ai.request.poll" => {
                return serde_json::to_value(
                    ai::poll_ai_events(&context.ai_runtime, target)
                        .map_err(|error| error.clone())?,
                )
                .map_err(|error| error.to_string())
            }
            "ai.request.cancel" => {
                ai::cancel_ai_request(&context.ai_runtime, target)?;
                return Ok(serde_json::Value::Null);
            }
            "ai.request.citations" => {
                let citations = ai::ai_request_citations(&context.ai_runtime, target)?;
                host.remove_ai_request(target);
                ai::remove_ai_citations(&context.ai_runtime, target)?;
                return serde_json::to_value(citations).map_err(|error| error.to_string());
            }
            "ai.request.result" => {
                let output = ai::ai_request_result(&context.ai_runtime, target)?;
                let has_citations =
                    !ai::ai_request_citations(&context.ai_runtime, target)?.is_empty();
                if operation == "generate_structured" {
                    let value: serde_json::Value = serde_json::from_str(&output)
                        .map_err(|_| "structured AI output is not valid JSON".to_string())?;
                    if let Some(contract) = contract.as_ref() {
                        ai::validate_structured_output(contract, &value)?;
                    }
                    if !has_citations {
                        host.remove_ai_request(target);
                        ai::remove_ai_citations(&context.ai_runtime, target)?;
                    }
                    return Ok(value);
                }
                if !has_citations {
                    host.remove_ai_request(target);
                    ai::remove_ai_citations(&context.ai_runtime, target)?;
                }
                return Ok(serde_json::json!({ "output": output }));
            }
            _ => return Err("unknown AI request method".into()),
        }
    }

    let event_type = payload
        .get("type")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "event RPC payload requires type".to_string())?;
    let (name, version) = event_type
        .rsplit_once('@')
        .ok_or_else(|| "event type must include @version".to_string())?;
    let version = version
        .parse::<u32>()
        .map_err(|_| "event version is invalid".to_string())?;
    match method {
        "event.subscribe" => {
            host.subscribe_event_authorized(plugin_id, project_id, name, version)
                .map_err(|error| error.to_string())?;
            Ok(serde_json::Value::Null)
        }
        "event.poll" => serde_json::to_value(
            host.poll_events_authorized(plugin_id, project_id, name, version)
                .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string()),
        "event.publish" => serde_json::to_value(
            host.publish_event_authorized(
                plugin_id,
                project_id,
                name,
                version,
                payload
                    .get("payload")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
            )
            .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string()),
        _ => Err(format!("unknown plugin host RPC method: {method}")),
    }
}

pub(super) fn plugin_webview_identity(
    window: &tauri::WebviewWindow,
    expected: Option<&str>,
) -> Result<String, String> {
    let label = window.label();
    let _plugin_id = label
        .strip_prefix("plugin:")
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "plugin RPC requires a plugin webview".to_string())?;
    if expected.is_some_and(|expected| plugin_window_label(expected) != label) {
        return Err("plugin webview identity mismatch".into());
    }
    Ok(label.to_string())
}

pub(super) fn sync_project_usage(
    project: &ProjectStore,
    host: &mut PluginHost,
) -> Result<(), CoreError> {
    let project_id = project
        .info()
        .map(|info| info.root)
        .ok_or(CoreError::ProjectNotOpen)?;
    host.bind_project_grants(Path::new(&project_id), &project_id)
        .map_err(|error| CoreError::Conflict(error.to_string()))?;
    let states = project
        .module_states()?
        .into_iter()
        .map(|module| (module.module_id, (module.enabled, module.package_version)))
        .collect::<BTreeMap<_, _>>();
    let mut module_ids = states.keys().cloned().collect::<BTreeSet<_>>();
    module_ids.extend(host.catalog.list().map(|entry| entry.manifest.id.clone()));

    for module_id in module_ids {
        let (enabled, package_version) = if let Some(state) = states.get(&module_id).cloned() {
            state
        } else {
            let enabled = host
                .catalog
                .get(&module_id)
                .and_then(|entry| entry.manifest.enabled_by_default)
                .unwrap_or(true);
            if !enabled {
                project.set_module_enabled(module_id.clone(), false)?;
            }
            (enabled, None)
        };
        if enabled {
            if let Some(version) = package_version {
                host.record_project_usage(&project_id, &module_id, &version)
                    .map_err(|error| CoreError::Conflict(error.to_string()))?;
            }
            host.ensure_first_party_bundled_grants(&project_id, &module_id)
                .map_err(|error| CoreError::Validation(error.to_string()))?;
            host.activate_bundled(&project_id, &module_id)
                .map_err(|error| CoreError::Validation(error.to_string()))?;
        } else {
            host.deactivate_bundled(&project_id, &module_id);
            host.clear_project_usage(&project_id, &module_id)
                .map_err(|error| CoreError::Conflict(error.to_string()))?;
        }
    }
    Ok(())
}

pub(super) fn relationship_metadata_schemas_for_project(
    project: &ProjectStore,
    host: &PluginHost,
) -> Result<BTreeMap<String, Vec<MetadataFieldDefinition>>, CoreError> {
    let project_id = project.info().map(|info| info.root);
    let mut entries = host
        .catalog
        .list()
        .map(|entry| entry.manifest.id.clone())
        .collect::<Vec<_>>();
    entries.sort();
    let mut merged: BTreeMap<String, BTreeMap<String, MetadataFieldDefinition>> = BTreeMap::new();
    for module_id in entries {
        let enabled = if project_id.is_some() {
            project.is_module_enabled(&module_id)?
        } else {
            host.catalog
                .get(&module_id)
                .and_then(|entry| entry.manifest.enabled_by_default)
                .unwrap_or(true)
        };
        if !enabled {
            continue;
        }
        let entry = host
            .runtime_entry(project_id.as_deref().unwrap_or_default(), &module_id)
            .or_else(|| host.catalog.get(&module_id).cloned())
            .ok_or_else(|| CoreError::Validation("plugin catalog entry disappeared".into()))?;
        let mut manifest = entry.manifest.clone();
        if supports_schema_overlay(&manifest) {
            let overlay_value = if project_id.is_some() {
                project
                    .module_schema_overlay(&module_id)?
                    .unwrap_or_else(|| serde_json::json!({}))
            } else {
                serde_json::json!({})
            };
            let overlay = parse_module_overlay(&overlay_value).map_err(CoreError::Validation)?;
            manifest = merge_module_manifest(&manifest, &overlay).map_err(CoreError::Validation)?;
        }
        for schema in &manifest.schemas {
            for field in &schema.fields {
                let Some(metadata_fields) = field.metadata_fields.as_deref() else {
                    continue;
                };
                let fields = merged
                    .entry(field.relationship_type.clone().ok_or_else(|| {
                        CoreError::Validation(format!(
                            "relationship metadata field {} is missing relationshipType",
                            field.key
                        ))
                    })?)
                    .or_default();
                for metadata_field in metadata_fields {
                    if let Some(existing) = fields.get(&metadata_field.key) {
                        if existing.field_type != metadata_field.field_type {
                            return Err(CoreError::Validation(format!(
                                "conflicting relationship metadata field type for {}",
                                metadata_field.key
                            )));
                        }
                    }
                    fields.insert(metadata_field.key.clone(), metadata_field.clone());
                }
            }
        }
    }
    Ok(merged
        .into_iter()
        .map(|(relationship_type, fields)| {
            (relationship_type, fields.into_values().collect::<Vec<_>>())
        })
        .collect())
}

pub(super) fn relationship_constraints_for_project(
    project: &ProjectStore,
    host: &PluginHost,
) -> Result<BTreeMap<String, daena_plugin_api::RelationshipConstraints>, CoreError> {
    let project_id = project.info().map(|info| info.root);
    let mut entries = host
        .catalog
        .list()
        .map(|entry| entry.manifest.id.clone())
        .collect::<Vec<_>>();
    entries.sort();
    let mut merged: BTreeMap<String, daena_plugin_api::RelationshipConstraints> = BTreeMap::new();
    for module_id in entries {
        let enabled = if project_id.is_some() {
            project.is_module_enabled(&module_id)?
        } else {
            host.catalog
                .get(&module_id)
                .and_then(|entry| entry.manifest.enabled_by_default)
                .unwrap_or(true)
        };
        if !enabled {
            continue;
        }
        let entry = host
            .runtime_entry(project_id.as_deref().unwrap_or_default(), &module_id)
            .or_else(|| host.catalog.get(&module_id).cloned())
            .ok_or_else(|| CoreError::Validation("plugin catalog entry disappeared".into()))?;
        let mut manifest = entry.manifest.clone();
        if supports_schema_overlay(&manifest) {
            let overlay_value = if project_id.is_some() {
                project
                    .module_schema_overlay(&module_id)?
                    .unwrap_or_else(|| serde_json::json!({}))
            } else {
                serde_json::json!({})
            };
            let overlay = parse_module_overlay(&overlay_value).map_err(CoreError::Validation)?;
            manifest = merge_module_manifest(&manifest, &overlay).map_err(CoreError::Validation)?;
        }
        for schema in &manifest.schemas {
            for field in &schema.fields {
                let Some(relationship_type) = field.relationship_type.as_deref() else {
                    continue;
                };
                let constraints = daena_plugin_api::RelationshipConstraints::from_field(field);
                if let Some(existing) = merged.get(relationship_type) {
                    if existing != &constraints {
                        return Err(CoreError::Validation(format!(
                            "conflicting relationship constraints for {relationship_type}"
                        )));
                    }
                } else {
                    merged.insert(relationship_type.to_owned(), constraints);
                }
            }
        }
    }
    Ok(merged)
}

pub(super) fn apply_relationship_runtime_schemas(
    project: &mut ProjectStore,
    host: &PluginHost,
) -> Result<(), CoreError> {
    project.set_relationship_metadata_schemas(relationship_metadata_schemas_for_project(
        project, host,
    )?)?;
    project.set_relationship_constraints(relationship_constraints_for_project(project, host)?)
}

pub(super) fn refresh_relationship_metadata_schemas(
    core: &SharedCore,
    plugins: &SharedPluginHost,
) -> Result<(), String> {
    let session = current_session(core)?;
    let mut service = session
        .core
        .lock()
        .map_err(|_| "core lock poisoned".to_string())?;
    let project = service
        .project_mut(trusted_shell())
        .map_err(|error| error.to_string())?;
    let host = plugins
        .lock()
        .map_err(|_| "plugin host lock poisoned".to_string())?;
    apply_relationship_runtime_schemas(project, &host).map_err(|error| error.to_string())
}

pub(super) async fn sync_project_usage_and_wait(
    core: SharedCore,
    plugins: SharedPluginHost,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let project_id = current_info(&core)?
            .ok_or_else(|| "project is not open".to_string())?
            .root;
        let bundled = {
            let host = plugins
                .lock()
                .map_err(|_| "plugin host lock poisoned".to_string())?;
            host.catalog
                .list()
                .map(|entry| (entry.manifest.clone(), entry.digest.clone()))
                .collect::<Vec<_>>()
        };
        let project_root = Path::new(&project_id);
        let project =
            ProjectStore::open_read_only(project_root).map_err(|error| error.to_string())?;
        let states = project
            .module_states()
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|module| (module.module_id, (module.enabled, module.version)))
            .collect::<BTreeMap<_, _>>();
        let missing_disabled = bundled
            .iter()
            .filter(|(manifest, _)| {
                !manifest.enabled_by_default.unwrap_or(true) && !states.contains_key(&manifest.id)
            })
            .map(|(manifest, _)| manifest.id.clone())
            .collect::<Vec<_>>();
        let pending_migrations = bundled
            .iter()
            .filter(|(manifest, _)| {
                states.get(&manifest.id).map_or_else(
                    || manifest.enabled_by_default.unwrap_or(true),
                    |(enabled, _)| *enabled,
                )
            })
            .map(|(manifest, digest)| {
                let current = states
                    .get(&manifest.id)
                    .map(|(_, version)| *version)
                    .unwrap_or_default();
                core_migrations(manifest, digest).map(|migrations| {
                    migrations
                        .into_iter()
                        .filter(|migration| migration.from >= current)
                        .collect::<Vec<_>>()
                })
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        if !missing_disabled.is_empty() || !pending_migrations.is_empty() {
            let session = current_session(&core)?;
            let mut service = session
                .core
                .lock()
                .map_err(|_| "core lock poisoned".to_string())?;
            let current_root = service
                .info()
                .ok_or_else(|| "project is not open".to_string())?
                .root;
            if current_root != project_id {
                return Err("project changed while synchronizing module state".into());
            }
            let project = service
                .project_mut(trusted_shell())
                .map_err(|error| error.to_string())?;
            for module_id in missing_disabled {
                project
                    .set_module_enabled(module_id, false)
                    .map_err(|error| error.to_string())?;
            }
            if !pending_migrations.is_empty() {
                project
                    .apply_migrations(&pending_migrations)
                    .map_err(|error| error.to_string())?;
            }
        }

        let project =
            ProjectStore::open_read_only(project_root).map_err(|error| error.to_string())?;
        {
            let mut host = plugins
                .lock()
                .map_err(|_| "plugin host lock poisoned".to_string())?;
            sync_project_usage(&project, &mut host).map_err(|error| error.to_string())?;
        }
        refresh_relationship_metadata_schemas(&core, &plugins)
    })
    .await
    .map_err(|error| format!("module synchronization worker failed: {error}"))?
}
