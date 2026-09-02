// Runtime plugin management commands.
use super::*;

#[tauri::command]
pub(super) fn plugin_install_package(
    app: tauri::AppHandle,
    plugins: tauri::State<'_, SharedPluginHost>,
    archive: String,
    allow_unsigned: bool,
) -> Result<serde_json::Value, String> {
    let install_root = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("plugins");
    let policy = if allow_unsigned {
        VerificationPolicy::with_unsigned_consent()
    } else {
        VerificationPolicy::default()
    };
    let package = plugins
        .lock()
        .map_err(|_| "plugin host lock poisoned".to_string())?
        .install_package(archive, install_root, ArchiveLimits::default(), policy)
        .map_err(|error| error.to_string())?;
    serde_json::to_value(serde_json::json!({
        "id": package.manifest.id,
        "version": package.manifest.version,
        "publisher": package.manifest.publisher,
        "digest": package.digest,
        "signed": package.signed,
    }))
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub(super) async fn plugin_upgrade(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    plugin_id: String,
    version: String,
    consent: bool,
) -> Result<(), String> {
    let plugins = plugins.inner().clone();
    with_core(state, move |core| {
        let project = core.project_mut(trusted_shell())?;
        let project_id = project
            .info()
            .map(|info| info.root)
            .ok_or(CoreError::ProjectNotOpen)?;
        let (old_version, old_grants, target_manifest, target_digest, plan) = {
            let host = plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
            let old_version = host.selected_project_version(&project_id, &plugin_id);
            let old_grants = host.grants.get(&project_id, &plugin_id);
            let target = host.packages.get(&plugin_id, &version).ok_or_else(|| {
                CoreError::Validation("target plugin version is not installed".into())
            })?;
            let target_manifest = daena_plugin_api::parse_manifest(
                &std::fs::read_to_string(target.root.join("manifest.json"))
                    .map_err(|error| CoreError::Validation(error.to_string()))?,
            )
            .map_err(|error| CoreError::Validation(error.to_string()))?;
            let plan = host
                .plan_upgrade(
                    &plugin_id,
                    &version,
                    &project_id,
                    project.get_module_version(&plugin_id)? as u32,
                )
                .map_err(|error| CoreError::Validation(error.to_string()))?;
            (
                old_version,
                old_grants,
                target_manifest,
                target.digest.clone(),
                plan,
            )
        };
        if plan.consent.requires_renewal && !consent {
            return Err(CoreError::Unauthorized {
                operation: "consent to plugin capability changes",
            });
        }
        let old_version =
            old_version.ok_or(CoreError::Validation("plugin has no active version".into()))?;
        let current = project.get_module_version(&plugin_id)? as u32;
        let backup = project.create_plugin_backup(
            &plugin_id,
            Some(&old_version),
            Some(&version),
            i64::from(current),
        )?;
        let requested = target_manifest.capabilities.clone();
        let mut next_grants = old_grants
            .iter()
            .filter(|grant| requested.contains(grant))
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        if consent {
            next_grants.extend(plan.consent.added.iter().cloned());
        }
        {
            let mut host = plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
            host.deactivate_bundled(&project_id, &plugin_id);
            host.select_project_version(&project_id, &plugin_id, &version)
                .map_err(|error| CoreError::Validation(error.to_string()))?;
            host.grants
                .set(&project_id, &plugin_id, &requested, next_grants)
                .map_err(|error| CoreError::Validation(error.to_string()))?;
            host.persist_project_grants(&project_id)
                .map_err(|error| CoreError::Conflict(error.to_string()))?;
        }
        let migrations = core_migrations(&target_manifest, &target_digest)
            .map_err(|error| CoreError::Validation(error.clone()))?
            .into_iter()
            .filter(|migration| migration.from >= i64::from(current))
            .collect::<Vec<_>>();
        if let Err(error) = project.apply_migrations(&migrations) {
            let _ = project.restore_plugin_backup(&backup);
            let mut host = plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
            host.select_project_version(&project_id, &plugin_id, &old_version)
                .map_err(|restore| CoreError::Validation(restore.to_string()))?;
            host.grants
                .set(
                    &project_id,
                    &plugin_id,
                    &old_grants.iter().cloned().collect::<Vec<_>>(),
                    old_grants.clone(),
                )
                .map_err(|restore| CoreError::Validation(restore.to_string()))?;
            host.persist_project_grants(&project_id)
                .map_err(|restore| CoreError::Conflict(restore.to_string()))?;
            let _ = host.activate_bundled(&project_id, &plugin_id);
            return Err(error);
        }
        let activation = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?
            .activate_bundled(&project_id, &plugin_id);
        if let Err(error) = activation {
            let _ = project.restore_plugin_backup(&backup);
            let mut host = plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
            host.select_project_version(&project_id, &plugin_id, &old_version)
                .map_err(|restore| CoreError::Validation(restore.to_string()))?;
            host.grants
                .set(
                    &project_id,
                    &plugin_id,
                    &old_grants.iter().cloned().collect::<Vec<_>>(),
                    old_grants,
                )
                .map_err(|restore| CoreError::Validation(restore.to_string()))?;
            host.persist_project_grants(&project_id)
                .map_err(|restore| CoreError::Conflict(restore.to_string()))?;
            let _ = host.activate_bundled(&project_id, &plugin_id);
            return Err(CoreError::Validation(error.to_string()));
        }
        project.set_module_package_version(&plugin_id, Some(&version))?;
        project.set_module_enabled(plugin_id, true)?;
        let host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        apply_relationship_runtime_schemas(project, &host)?;
        Ok(())
    })
    .await
}

#[tauri::command]
pub(super) async fn plugin_rollback(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    plugin_id: String,
    version: String,
) -> Result<(), String> {
    let plugins = plugins.inner().clone();
    with_core(state, move |core| {
        let project = core.project_mut(trusted_shell())?;
        let project_id = project
            .info()
            .map(|info| info.root)
            .ok_or(CoreError::ProjectNotOpen)?;
        let current = project.get_module_version(&plugin_id)? as u32;
        let current_version = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?
            .selected_project_version(&project_id, &plugin_id)
            .ok_or_else(|| CoreError::Validation("plugin has no selected version".into()))?;
        if current_version == version {
            return Err(CoreError::Validation(
                "rollback target is already selected".into(),
            ));
        }
        let backup = project
            .latest_plugin_backup(&plugin_id, Some(&version), Some(&current_version))?
            .ok_or_else(|| {
                CoreError::Validation("no pre-upgrade backup exists for rollback".into())
            })?;
        let safety_backup = project.create_plugin_backup(
            &plugin_id,
            Some(&current_version),
            Some(&version),
            i64::from(current),
        )?;
        let result = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?
            .rollback_plugin(
                &project_id,
                &plugin_id,
                &version,
                backup.data_version as u32,
            );
        if let Err(error) = result {
            let _ = project.restore_plugin_backup(&safety_backup);
            return Err(CoreError::Validation(error.to_string()));
        }
        if let Err(error) = project.restore_plugin_backup(&backup) {
            let _ = project.restore_plugin_backup(&safety_backup);
            let mut host = plugins.lock().map_err(|lock_error| {
                CoreError::Conflict(format!("plugin host lock poisoned: {lock_error}"))
            })?;
            host.select_project_version(&project_id, &plugin_id, &current_version)
                .map_err(|restore| CoreError::Validation(restore.to_string()))?;
            let _ = host.activate_bundled(&project_id, &plugin_id);
            return Err(error);
        }
        project.set_module_package_version(&plugin_id, Some(&version))?;
        project.set_module_enabled(plugin_id, true)?;
        let host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        apply_relationship_runtime_schemas(project, &host)?;
        Ok(())
    })
    .await
}

#[tauri::command]
pub(super) async fn plugin_uninstall_code(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    plugin_id: String,
    version: String,
) -> Result<(), String> {
    let plugins = plugins.inner().clone();
    with_core(state, move |core| {
        let mut detached_project: Option<(String, String)> = None;
        if core.info().is_some() {
            let project = core.project(trusted_shell())?;
            if project.module_package_version(&plugin_id)?.as_deref() == Some(version.as_str()) {
                if project.is_module_enabled(&plugin_id)? {
                    return Err(CoreError::Conflict(
                        "disable the plugin before uninstalling its selected code".into(),
                    ));
                }
                let project_id = project
                    .info()
                    .map(|info| info.root)
                    .ok_or(CoreError::ProjectNotOpen)?;
                project.set_module_package_version(&plugin_id, None)?;
                let clear_result = plugins
                    .lock()
                    .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?
                    .clear_project_usage(&project_id, &plugin_id);
                if let Err(error) = clear_result {
                    let _ = project.set_module_package_version(&plugin_id, Some(&version));
                    return Err(CoreError::Conflict(error.to_string()));
                }
                detached_project = Some((project_id, version.clone()));
            }
        }
        let uninstall_result = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?
            .uninstall_code(&plugin_id, &version)
            .map_err(|error| CoreError::Validation(error.to_string()));
        if let Err(error) = uninstall_result {
            if let Some((project_id, selected_version)) = detached_project {
                let project = core.project(trusted_shell())?;
                let _ = project.set_module_package_version(&plugin_id, Some(&selected_version));
                let restore_result = plugins
                    .lock()
                    .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?
                    .select_project_version(&project_id, &plugin_id, &selected_version);
                if let Err(restore_error) = restore_result {
                    return Err(CoreError::Conflict(format!(
                        "{error}; failed to restore project plugin selection: {restore_error}"
                    )));
                }
            }
            return Err(error);
        }
        Ok(())
    })
    .await
}

#[tauri::command]
pub(super) async fn plugin_delete_data(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    plugin_id: String,
    confirmation: String,
) -> Result<String, String> {
    let plugins = plugins.inner().clone();
    with_core(state, move |core| {
        if plugin_id.trim().is_empty() || confirmation != plugin_id {
            return Err(CoreError::Unauthorized {
                operation: "confirm plugin data deletion",
            });
        }
        let project = core.project_mut(trusted_shell())?;
        let project_id = project
            .info()
            .map(|info| info.root)
            .ok_or(CoreError::ProjectNotOpen)?;
        {
            let mut host = plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
            host.deactivate_bundled(&project_id, &plugin_id);
        }
        project.set_module_enabled(plugin_id.clone(), false)?;
        let backup = project.delete_plugin_data(&plugin_id, &confirmation)?;
        plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?
            .clear_project_usage(&project_id, &plugin_id)
            .map_err(|error| CoreError::Conflict(error.to_string()))?;
        Ok(backup)
    })
    .await
}

#[tauri::command]
pub(super) async fn plugin_admin_view(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
) -> Result<serde_json::Value, String> {
    let plugins = plugins.inner().clone();
    with_core(state, move |core| {
        let context = trusted_shell();
        let project = core.project(context).ok();
        let project_id = project.and_then(|project| project.info().map(|info| info.root));
        let host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        let mut plugin_ids = host
            .catalog
            .list()
            .map(|entry| entry.manifest.id.clone())
            .collect::<Vec<_>>();
        for id in host.packages.plugin_ids() {
            if !plugin_ids.contains(id) {
                plugin_ids.push(id.clone());
            }
        }
        let mut plugins_view = Vec::with_capacity(plugin_ids.len());
        for id in plugin_ids {
            let entry = project_id
                .as_deref()
                .and_then(|project_id| host.runtime_entry(project_id, &id))
                .or_else(|| host.catalog.get(&id).cloned())
                .ok_or_else(|| CoreError::Validation("plugin catalog entry disappeared".into()))?;
            let manifest = &entry.manifest;
            let selected_version = project_id.as_deref().and_then(|project_id| {
                project
                    .and_then(|project| project.module_package_version(&id).ok().flatten())
                    .or_else(|| host.selected_project_version(project_id, &id))
            });
            let empty_project = "";
            let project_for_scope = project_id.as_deref().unwrap_or(empty_project);
            let lifecycle = host.lifecycle.state(project_for_scope, &id);
            let runtime_running = host.wasm.is_running(project_for_scope, &id);
            let granted_capabilities = if project_id.is_some() {
                host.grants.get(project_for_scope, &id).into_iter().collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            let mut versions = Vec::new();
            let active_candidate = host
                .packages
                .active_candidate(&id)
                .map(|version| version.version.clone());
            for installed in host.packages.list(&id) {
                versions.push(serde_json::json!({
                    "version": installed.version,
                    "publisher": installed.publisher,
                    "digest": installed.digest,
                    "signed": installed.signed,
                    "unsignedConsent": installed.unsigned_consent,
                    "installedAt": installed.installed_at,
                    "isSelected": selected_version.as_deref() == Some(installed.version.as_str()),
                    "isActiveCandidate": active_candidate.as_deref() == Some(installed.version.as_str()),
                    "bundled": false,
                    "rollbackAvailable": false,
                }));
            }
            if versions.is_empty() {
                versions.push(serde_json::json!({
                    "version": manifest.version,
                    "publisher": manifest.publisher,
                    "digest": "",
                    "signed": false,
                    "unsignedConsent": false,
                    "installedAt": 0,
                    "isSelected": true,
                    "isActiveCandidate": true,
                    "bundled": true,
                    "rollbackAvailable": false,
                }));
            } else if let (Some(project), Some(selected)) = (project, selected_version.as_deref()) {
                for version in &mut versions {
                    let candidate = version
                        .get("version")
                        .and_then(|value| value.as_str())
                        .unwrap_or_default();
                    if candidate != selected {
                        let backup =
                            project.latest_plugin_backup(&id, Some(candidate), Some(selected))?;
                        version["rollbackAvailable"] =
                            serde_json::Value::Bool(backup.is_some());
                    }
                }
            }
            let dependency_state = match DependencyResolver::resolve(&host.catalog, &id) {
                Ok(plan) => serde_json::json!({
                    "resolved": true,
                    "order": plan.order,
                    "error": null,
                }),
                Err(error) => serde_json::json!({
                    "resolved": false,
                    "order": [],
                    "error": error.to_string(),
                }),
            };
            let mut view = serde_json::to_value(manifest)
                .map_err(|error| CoreError::Validation(error.to_string()))?;
            let object = view
                .as_object_mut()
                .expect("manifest is a JSON object");
            object.insert(
                "enabled".into(),
                serde_json::Value::Bool(
                    project.is_some_and(|project| project.is_module_enabled(&id).unwrap_or(false)),
                ),
            );
            object.insert(
                "selectedVersion".into(),
                selected_version.map_or(serde_json::Value::Null, |value| {
                    serde_json::Value::String(value)
                }),
            );
            object.insert(
                "dataVersion".into(),
                serde_json::Value::Number(
                    project
                        .and_then(|project| project.get_module_version(&id).ok())
                        .unwrap_or(0)
                        .into(),
                ),
            );
            object.insert(
                "lifecycle".into(),
                serde_json::json!({
                    "state": lifecycle.state,
                    "failures": lifecycle.failures,
                    "lastError": lifecycle.last_error,
                }),
            );
            object.insert("runtimeRunning".into(), serde_json::Value::Bool(runtime_running));
            object.insert(
                "grantedCapabilities".into(),
                serde_json::Value::Array(
                    granted_capabilities
                        .into_iter()
                        .map(serde_json::Value::String)
                        .collect(),
                ),
            );
            object.insert("installedVersions".into(), serde_json::Value::Array(versions));
            object.insert("dependencyState".into(), dependency_state);
            let bundled = entry.package_root.as_os_str().is_empty();
            object.insert(
                "distribution".into(),
                serde_json::json!({
                    "origin": if bundled { "bundled" } else { "installed" },
                    "management": if bundled { "app" } else { "user" },
                    "canUninstall": !bundled,
                }),
            );
            plugins_view.push(view);
        }
        Ok(serde_json::json!({ "plugins": plugins_view }))
    })
    .await
}

#[tauri::command]
pub(super) async fn plugin_upgrade_plan(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    plugin_id: String,
    version: String,
) -> Result<serde_json::Value, String> {
    let plugins = plugins.inner().clone();
    with_core(state, move |core| {
        let project = core.project(trusted_shell())?;
        let project_id = project
            .info()
            .map(|info| info.root)
            .ok_or(CoreError::ProjectNotOpen)?;
        let current = project.get_module_version(&plugin_id)? as u32;
        let host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        let plan = host
            .plan_upgrade(&plugin_id, &version, &project_id, current)
            .map_err(|error| CoreError::Validation(error.to_string()))?;
        let target = host.packages.get(&plugin_id, &version).ok_or_else(|| {
            CoreError::Validation("target plugin version is not installed".into())
        })?;
        Ok(serde_json::json!({
            "pluginId": plan.plugin_id,
            "fromVersion": plan.from_version,
            "toVersion": plan.to_version,
            "consent": {
                "added": plan.consent.added,
                "removed": plan.consent.removed,
                "requiresRenewal": plan.consent.requires_renewal,
            },
            "migrations": {
                "from": plan.migrations.from,
                "to": plan.migrations.to,
                "migrationIds": plan.migrations.migration_ids,
                "requiresBackup": plan.migrations.requires_backup,
            },
            "target": {
                "signed": target.signed,
                "publisher": target.publisher,
            },
        }))
    })
    .await
}

#[tauri::command]
pub(super) async fn plugin_retry(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    plugin_id: String,
) -> Result<(), String> {
    let plugins = plugins.inner().clone();
    with_core(state, move |core| {
        let project = core.project(trusted_shell())?;
        let project_id = project
            .info()
            .map(|info| info.root)
            .ok_or(CoreError::ProjectNotOpen)?;
        let enabled = project.is_module_enabled(&plugin_id)?;
        let mut host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        host.retry_plugin(&project_id, &plugin_id);
        if enabled {
            host.activate_bundled(&project_id, &plugin_id)
                .map_err(|error| CoreError::Validation(error.to_string()))?;
        }
        Ok(())
    })
    .await
}
