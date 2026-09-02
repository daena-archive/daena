// Bundled module commands.
use super::*;

pub(super) fn bundled_plugin_host(core: SharedCore) -> Result<PluginHost, String> {
    let mut host = PluginHost::new();
    for (manifest, wasm) in [
        (
            include_str!("../../packages/modules/lore/manifest.json"),
            None,
        ),
        (
            include_str!("../../packages/modules/timeline/manifest.json"),
            Some(BUNDLED_TIMELINE_SERVICE_WASM),
        ),
        (
            include_str!("../../packages/modules/writing/manifest.json"),
            None,
        ),
        (
            include_str!("../../packages/modules/maps/manifest.json"),
            None,
        ),
        (
            include_str!("../../packages/modules/language/manifest.json"),
            None,
        ),
        (
            include_str!("../../packages/modules/houses/manifest.json"),
            None,
        ),
    ] {
        host.register_bundled_json_with_wasm(manifest, wasm)
            .map_err(|error| error.to_string())?;
    }
    // Native provider for the public navigation service. Registered before
    // project activation so the manifest-declared WASM stub is skipped and
    // Lore/Timeline consumers reach the same resolution as the shell command.
    let navigation = maps_navigation_service_handler(core);
    host.register_declared_service_provider("daena.maps", "daena.maps/navigation", 1, navigation)
        .map_err(|error| error.to_string())?;
    Ok(host)
}

pub(super) fn core_migrations(
    manifest: &PluginManifest,
    package_digest: &str,
) -> Result<Vec<Migration>, String> {
    manifest
        .migrations
        .iter()
        .map(|migration| {
            let operations = migration
                .operations
                .iter()
                .map(|operation| match operation {
                    MigrationOperation::CreateNamespace { namespace } => {
                        Operation::CreateNamespace {
                            namespace: namespace.clone(),
                        }
                    }
                    MigrationOperation::AddField {
                        namespace, field, ..
                    } => Operation::AddField {
                        namespace: namespace.clone(),
                        field: daena_core::FieldDefinition {
                            key: field.key.clone(),
                            field_type: field.field_type.clone(),
                            required: field.required.unwrap_or(false),
                        },
                    },
                    MigrationOperation::RenameField {
                        namespace,
                        from,
                        to,
                    } => Operation::RenameField {
                        namespace: namespace.clone(),
                        from: from.clone(),
                        to: to.clone(),
                    },
                    MigrationOperation::DropField { namespace, key } => Operation::DropField {
                        namespace: namespace.clone(),
                        key: key.clone(),
                    },
                })
                .collect();
            Ok(Migration {
                id: migration.id.clone(),
                module_id: manifest.id.clone(),
                from: i64::from(migration.from),
                to: i64::from(migration.to),
                operations,
                recovery: migration.recovery.clone(),
                package_digest: package_digest.into(),
            })
        })
        .collect()
}

pub(crate) fn effective_module_manifests(
    project: &ProjectStore,
    host: &PluginHost,
) -> Result<Vec<(PluginManifest, bool)>, CoreError> {
    let project_id = project.info().map(|info| info.root);
    let plugin_ids = host
        .catalog
        .list()
        .map(|entry| entry.manifest.id.clone())
        .collect::<Vec<_>>();
    let mut manifests = Vec::with_capacity(plugin_ids.len());
    for id in plugin_ids {
        let entry = project_id
            .as_deref()
            .and_then(|project_id| host.runtime_entry(project_id, &id))
            .or_else(|| host.catalog.get(&id).cloned())
            .ok_or_else(|| CoreError::Validation("plugin catalog entry disappeared".into()))?;
        let mut manifest = entry.manifest.clone();
        if supports_schema_overlay(&entry.manifest) {
            let overlay_value = project
                .module_schema_overlay(&id)?
                .unwrap_or_else(|| serde_json::json!({}));
            let overlay = parse_module_overlay(&overlay_value).map_err(CoreError::Validation)?;
            manifest =
                merge_module_manifest(&entry.manifest, &overlay).map_err(CoreError::Validation)?;
        }
        manifests.push((manifest, project.is_module_enabled(&id)?));
    }
    Ok(manifests)
}

#[tauri::command]
pub(super) async fn module_list_manifests(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
) -> Result<Vec<serde_json::Value>, String> {
    let plugins = plugins.inner().clone();
    with_read_project(state, move |project| {
        let host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        effective_module_manifests(project, &host)?
            .into_iter()
            .map(|(manifest_struct, enabled)| {
                let mut manifest = serde_json::to_value(&manifest_struct)
                    .map_err(|error| CoreError::Validation(error.to_string()))?;
                manifest
                    .as_object_mut()
                    .expect("manifest is an object")
                    .insert("enabled".into(), serde_json::Value::Bool(enabled));
                Ok(manifest)
            })
            .collect()
    })
    .await
}

#[tauri::command]
pub(super) async fn module_schema_overlay_get(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    module_id: String,
) -> Result<ModuleSchemaOverlay, String> {
    let plugins = plugins.inner().clone();
    with_read_project(state, move |project| {
        let project_id = project
            .info()
            .map(|info| info.root)
            .ok_or(CoreError::ProjectNotOpen)?;
        let package = {
            let host = plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
            host.runtime_entry(&project_id, &module_id)
                .or_else(|| host.catalog.get(&module_id).cloned())
                .map(|entry| entry.manifest.clone())
                .ok_or_else(|| {
                    CoreError::Validation(format!("plugin is unavailable: {module_id}"))
                })?
        };
        if !supports_schema_overlay(&package) {
            return Err(CoreError::Validation(format!(
                "schema overlays are not supported for {module_id}"
            )));
        }
        let value = project
            .module_schema_overlay(&module_id)?
            .unwrap_or_else(|| serde_json::json!({}));
        parse_module_overlay(&value).map_err(CoreError::Validation)
    })
    .await
}

#[tauri::command]
pub(super) async fn module_schema_editor_load(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    module_id: String,
) -> Result<ModuleSchemaEditorState, String> {
    let plugins = plugins.inner().clone();
    with_read_project(state, move |project| {
        let project_id = project
            .info()
            .map(|info| info.root)
            .ok_or(CoreError::ProjectNotOpen)?;
        let package = {
            let host = plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
            host.runtime_entry(&project_id, &module_id)
                .or_else(|| host.catalog.get(&module_id).cloned())
                .map(|entry| entry.manifest.clone())
                .ok_or_else(|| {
                    CoreError::Validation(format!("plugin is unavailable: {module_id}"))
                })?
        };
        if !supports_schema_overlay(&package) {
            return Err(CoreError::Validation(format!(
                "schema overlays are not supported for {module_id}"
            )));
        }
        if !project.is_module_enabled(&module_id)? {
            return Err(CoreError::Validation(format!(
                "plugin is disabled: {module_id}"
            )));
        }
        let value = project
            .module_schema_overlay(&module_id)?
            .unwrap_or_else(|| serde_json::json!({}));
        let overlay = parse_module_overlay(&value).map_err(CoreError::Validation)?;
        let revision = project.revision_for_module_schema_overlay(&module_id)?;
        Ok(ModuleSchemaEditorState {
            id: package.id.clone(),
            name: package.name.clone(),
            schemas: package.schemas.clone(),
            templates: package.templates.clone(),
            overlay,
            revision,
        })
    })
    .await
}

#[tauri::command]
pub(super) async fn module_schema_overlay_preview(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    module_id: String,
    overlay: ModuleSchemaOverlay,
) -> Result<SchemaOverlayPreviewResult, String> {
    let plugins = plugins.inner().clone();
    with_read_project(state, move |project| {
        let project_id = project
            .info()
            .map(|info| info.root)
            .ok_or(CoreError::ProjectNotOpen)?;
        let package = {
            let host = plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
            host.runtime_entry(&project_id, &module_id)
                .or_else(|| host.catalog.get(&module_id).cloned())
                .map(|entry| entry.manifest.clone())
                .ok_or_else(|| {
                    CoreError::Validation(format!("plugin is unavailable: {module_id}"))
                })?
        };
        if !supports_schema_overlay(&package) {
            return Err(CoreError::Validation(format!(
                "schema overlays are not supported for {module_id}"
            )));
        }
        project.preview_module_schema_overlay(&module_id, &package, &overlay)
    })
    .await
}

#[tauri::command]
pub(super) async fn module_schema_overlay_set(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    module_id: String,
    overlay: ModuleSchemaOverlay,
    expected_revision: Option<String>,
    request_id: Option<String>,
    acknowledge_impact: Option<bool>,
) -> Result<ModuleSchemaOverlayMutationResult, String> {
    let plugins = plugins.inner().clone();
    with_core(state, move |core| {
        let context = trusted_shell();
        let project = core.project_mut(context)?;
        let project_id = project
            .info()
            .map(|info| info.root)
            .ok_or(CoreError::ProjectNotOpen)?;
        let package = {
            let host = plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
            host.runtime_entry(&project_id, &module_id)
                .or_else(|| host.catalog.get(&module_id).cloned())
                .map(|entry| entry.manifest.clone())
                .ok_or_else(|| {
                    CoreError::Validation(format!("plugin is unavailable: {module_id}"))
                })?
        };
        if !supports_schema_overlay(&package) {
            return Err(CoreError::Validation(format!(
                "schema overlays are not supported for {module_id}"
            )));
        }
        let mut normalized = overlay;
        if normalized.version == 0 {
            normalized.version = SCHEMA_OVERLAY_VERSION;
        }
        daena_plugin_api::qualify_module_overlay(&package, &mut normalized)
            .map_err(CoreError::Validation)?;
        daena_plugin_api::validate_module_overlay(&package, &normalized)
            .map_err(CoreError::Validation)?;
        let preview = project.preview_module_schema_overlay(&module_id, &package, &normalized)?;
        if !preview.ok {
            let message = preview
                .errors
                .first()
                .map(|issue| issue.message.clone())
                .unwrap_or_else(|| "schema overlay preview failed".into());
            return Err(CoreError::Validation(message));
        }
        if preview.requires_acknowledgement && !acknowledge_impact.unwrap_or(false) {
            return Err(CoreError::Validation(
                "schema overlay has live-data impact; preview and acknowledge before saving".into(),
            ));
        }
        let value = if normalized.is_empty() {
            None
        } else {
            Some(
                serde_json::to_value(&normalized)
                    .map_err(|error| CoreError::Validation(error.to_string()))?,
            )
        };
        let revision = project.set_module_schema_overlay_with_request(
            module_id,
            value,
            expected_revision.as_deref(),
            request_id.as_deref(),
        )?;
        let host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        apply_relationship_runtime_schemas(project, &host)?;
        Ok(ModuleSchemaOverlayMutationResult {
            overlay: normalized,
            revision,
        })
    })
    .await
}

#[tauri::command]
pub(super) async fn module_enable(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    id: String,
    granted_capabilities: Option<Vec<String>>,
) -> Result<(), String> {
    let known = plugins
        .lock()
        .map_err(|_| "plugin host lock poisoned".to_string())?
        .catalog
        .get(&id)
        .is_some();
    if !known {
        return Err(format!("unknown plugin: {id}"));
    }
    let plugins = plugins.inner().clone();
    with_core(state, move |core| {
        let context = trusted_shell();
        let project = core.project_mut(context)?;
        let project_id = project
            .info()
            .map(|info| info.root)
            .ok_or(CoreError::ProjectNotOpen)?;
        if project.is_module_enabled(&id)? {
            let granted_capabilities = granted_capabilities.ok_or(CoreError::Unauthorized {
                operation: "explicit plugin capability review",
            })?;
            let mut host = plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
            host.grant_capabilities(&project_id, &id, granted_capabilities.into_iter().collect())
                .map_err(|error| CoreError::Validation(error.to_string()))?;
            return Ok(());
        }
        let (manifest, package_digest) = {
            let mut host = plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
            if let Some(version) = project.module_package_version(&id)? {
                host.select_project_version(&project_id, &id, &version)
                    .map_err(|error| CoreError::Validation(error.to_string()))?;
            }
            let entry = host
                .runtime_entry(&project_id, &id)
                .ok_or_else(|| CoreError::Validation("plugin runtime version is missing".into()))?;
            (entry.manifest, entry.digest)
        };
        {
            let mut host = plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
            if let Some(granted_capabilities) = granted_capabilities {
                host.grant_capabilities(
                    &project_id,
                    &manifest.id,
                    granted_capabilities.into_iter().collect(),
                )
                .map_err(|error| CoreError::Validation(error.to_string()))?;
            } else if !manifest.capabilities.is_empty()
                && host.grants.is_empty(&project_id, &manifest.id)
            {
                return Err(CoreError::Unauthorized {
                    operation: "consent to plugin capabilities",
                });
            }
        }
        let migrations = core_migrations(&manifest, &package_digest)
            .map_err(|error| CoreError::Validation(error.clone()))?;
        let current = project.get_module_version(&id)?;
        let pending = migrations
            .iter()
            .filter(|migration| migration.from >= current)
            .cloned()
            .collect::<Vec<_>>();
        let backup = if pending.is_empty() {
            None
        } else {
            Some(project.create_plugin_backup(
                &manifest.id,
                project.module_package_version(&id)?.as_deref(),
                Some(&manifest.version),
                current,
            )?)
        };
        if let Some(first) = pending.first() {
            if current != first.from {
                return Err(CoreError::Validation(format!(
                    "unsupported stored version {current} for plugin {}",
                    manifest.id
                )));
            }
            if let Err(error) = project.apply_migrations(&pending) {
                if let Some(backup) = &backup {
                    let _ = project.restore_plugin_backup(backup);
                }
                return Err(error);
            }
        }
        let mut host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        if let Err(error) = host.activate_bundled(&project_id, &manifest.id) {
            if let Some(backup) = &backup {
                let _ = project.restore_plugin_backup(backup);
            }
            return Err(CoreError::Validation(error.to_string()));
        }
        host.record_project_usage(&project_id, &id, &manifest.version)
            .map_err(|error| CoreError::Conflict(error.to_string()))?;
        project.set_module_enabled(id, true)?;
        apply_relationship_runtime_schemas(project, &host)?;
        Ok(())
    })
    .await
}

#[tauri::command]
pub(super) async fn module_disable(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    ai_runtime: tauri::State<'_, ai::SharedAiRuntime>,
    id: String,
) -> Result<(), String> {
    let known = plugins
        .lock()
        .map_err(|_| "plugin host lock poisoned".to_string())?
        .catalog
        .get(&id)
        .is_some();
    if !known {
        return Err(format!("unknown bundled plugin: {id}"));
    }
    let plugins = plugins.inner().clone();
    let ai_runtime = ai_runtime.inner().clone();
    with_core(state, move |core| {
        let project = core.project_mut(trusted_shell())?;
        let project_id = project
            .info()
            .map(|info| info.root)
            .ok_or(CoreError::ProjectNotOpen)?;
        project.set_module_enabled(id.clone(), false)?;
        let mut host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        for request_id in host.ai_request_ids_for(&project_id, Some(&id)) {
            let _ = ai::cancel_ai_request(&ai_runtime, &request_id);
            let _ = ai::remove_ai_citations(&ai_runtime, &request_id);
            host.remove_ai_request(&request_id);
        }
        if let Err(error) = host.clear_project_usage(&project_id, &id) {
            project.set_module_enabled(id, true)?;
            return Err(CoreError::Conflict(error.to_string()));
        }
        host.deactivate_bundled(&project_id, &id);
        close_plugin_webview(&app, &id);
        apply_relationship_runtime_schemas(project, &host)?;
        Ok(())
    })
    .await
}
