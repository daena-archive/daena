// Project, Git, and map-location Tauri commands.
use super::*;

#[tauri::command]
pub(super) async fn project_info(
    state: tauri::State<'_, SharedCore>,
) -> Result<Option<ProjectInfo>, String> {
    with_read_project(state, |project| Ok(project.info())).await
}

#[tauri::command]
pub(super) async fn project_ai_prompts_get(
    state: tauri::State<'_, SharedCore>,
) -> Result<serde_json::Value, String> {
    with_read_project(state, |project| project.ai_prompt_overlay()).await
}

#[tauri::command]
pub(super) async fn project_ai_prompts_set(
    state: tauri::State<'_, SharedCore>,
    overlay: serde_json::Value,
) -> Result<serde_json::Value, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?
            .set_ai_prompt_overlay(overlay)
    })
    .await
}

#[tauri::command]
pub(super) async fn project_set_ai_enabled(
    state: tauri::State<'_, SharedCore>,
    enabled: bool,
) -> Result<ProjectInfo, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.set_ai_enabled(enabled)
    })
    .await
}

#[tauri::command]
pub(super) async fn project_import_checkpoint(
    state: tauri::State<'_, SharedCore>,
) -> Result<ExternalChangeReport, String> {
    with_core(state, |core| {
        core.project_mut(trusted_shell())?.import_checkpoint()
    })
    .await
}

#[tauri::command]
pub(super) async fn project_save_recovery_copy(
    state: tauri::State<'_, SharedCore>,
    entity_id: String,
    body: String,
) -> Result<String, String> {
    with_read_project(state, move |project| {
        project.save_recovery_copy(&entity_id, &body)
    })
    .await
}

#[tauri::command]
pub(super) async fn git_tool_info() -> Result<GitToolInfo, String> {
    tauri::async_runtime::spawn_blocking(ProjectStore::git_tool_info)
        .await
        .map_err(|error| format!("git tool info worker failed: {error}"))
}

#[tauri::command]
pub(super) async fn project_git_status(
    state: tauri::State<'_, SharedCore>,
    reprobe: Option<bool>,
) -> Result<GitStatus, String> {
    let force = reprobe.unwrap_or(false);
    with_read_project(state, move |project| {
        if force {
            project.git_status_reprobe()
        } else {
            project.git_status()
        }
    })
    .await
}

#[tauri::command]
pub(super) async fn project_git_preflight(
    state: tauri::State<'_, SharedCore>,
) -> Result<GitPreflight, String> {
    flush_project_checkpoint(state.clone(), "git preflight").await?;
    with_read_project(
        state,
        daena_core::ProjectStore::git_preflight_after_checkpoint,
    )
    .await
}

#[tauri::command]
pub(super) async fn project_git_staging_preview(
    state: tauri::State<'_, SharedCore>,
) -> Result<GitPreflight, String> {
    flush_project_checkpoint(state.clone(), "git staging preview").await?;
    with_read_project(
        state,
        daena_core::ProjectStore::git_preflight_after_checkpoint,
    )
    .await
}

#[tauri::command]
pub(super) async fn project_git_init(
    state: tauri::State<'_, SharedCore>,
) -> Result<GitStatus, String> {
    with_core(state, |core| core.project(trusted_shell())?.git_init()).await
}

#[tauri::command]
pub(super) async fn project_git_log(
    state: tauri::State<'_, SharedCore>,
) -> Result<Vec<GitLogEntry>, String> {
    with_read_project(state, daena_core::ProjectStore::git_log).await
}

#[tauri::command]
pub(super) async fn project_git_commit(
    state: tauri::State<'_, SharedCore>,
    message: String,
    paths: Option<Vec<String>>,
) -> Result<GitStatus, String> {
    flush_project_checkpoint(state.clone(), "git commit").await?;
    with_read_project(state, move |project| {
        project.git_commit_after_checkpoint(message, paths)
    })
    .await
}

#[tauri::command]
pub(super) async fn project_git_super_squash(
    state: tauri::State<'_, SharedCore>,
    message: String,
) -> Result<GitResetResult, String> {
    flush_project_checkpoint(state.clone(), "git super squash").await?;
    with_read_project(state, move |project| {
        project.git_super_squash_after_checkpoint(&message)
    })
    .await
}

#[tauri::command]
pub(super) async fn project_git_show_tree(
    state: tauri::State<'_, SharedCore>,
    hash: String,
) -> Result<Vec<String>, String> {
    with_read_project(state, move |project| project.git_show_tree(&hash)).await
}

#[tauri::command]
pub(super) async fn project_git_show_message(
    state: tauri::State<'_, SharedCore>,
    hash: String,
) -> Result<String, String> {
    with_read_project(state, move |project| project.git_show_message(&hash)).await
}

#[tauri::command]
pub(super) async fn project_git_show_changes(
    state: tauri::State<'_, SharedCore>,
    hash: String,
) -> Result<Vec<daena_core::GitChange>, String> {
    with_read_project(state, move |project| project.git_show_changes(&hash)).await
}

#[tauri::command]
pub(super) async fn project_git_show_diff(
    state: tauri::State<'_, SharedCore>,
    hash: String,
    path: String,
) -> Result<String, String> {
    with_read_project(state, move |project| project.git_show_diff(&hash, &path)).await
}

#[tauri::command]
pub(super) async fn project_git_worktree_diff(
    state: tauri::State<'_, SharedCore>,
    paths: Vec<String>,
) -> Result<String, String> {
    with_read_project(state, move |project| project.git_worktree_diff(&paths)).await
}

#[tauri::command]
pub(super) async fn project_git_show_file(
    state: tauri::State<'_, SharedCore>,
    hash: String,
    path: String,
) -> Result<String, String> {
    with_read_project(state, move |project| project.git_show_file(&hash, &path)).await
}

#[tauri::command]
pub(super) async fn project_git_reset_hard(
    state: tauri::State<'_, SharedCore>,
    hash: String,
) -> Result<GitResetResult, String> {
    with_core(state, move |core| {
        core.project_mut(trusted_shell())?.git_reset_hard(&hash)
    })
    .await
}

#[tauri::command]
pub(super) async fn project_git_remote_list(
    state: tauri::State<'_, SharedCore>,
) -> Result<Vec<GitRemote>, String> {
    with_read_project(state, daena_core::ProjectStore::git_remote_list).await
}

#[tauri::command]
pub(super) async fn project_git_remote_add(
    state: tauri::State<'_, SharedCore>,
    name: String,
    url: String,
) -> Result<Vec<GitRemote>, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.git_remote_add(&name, &url)
    })
    .await
}

#[tauri::command]
pub(super) async fn project_git_remote_set_url(
    state: tauri::State<'_, SharedCore>,
    name: String,
    url: String,
) -> Result<Vec<GitRemote>, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?
            .git_remote_set_url(&name, &url)
    })
    .await
}

#[tauri::command]
pub(super) async fn project_git_remote_remove(
    state: tauri::State<'_, SharedCore>,
    name: String,
) -> Result<Vec<GitRemote>, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.git_remote_remove(&name)
    })
    .await
}

#[tauri::command]
pub(super) async fn project_git_push(
    state: tauri::State<'_, SharedCore>,
    remote: String,
    branch: Option<String>,
    force_with_lease: bool,
) -> Result<GitStatus, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?
            .git_push(&remote, branch.as_deref(), force_with_lease)
    })
    .await
}

#[tauri::command]
pub(super) async fn project_git_restore_from_upstream(
    state: tauri::State<'_, SharedCore>,
) -> Result<GitResetResult, String> {
    with_core(state, |core| {
        core.project_mut(trusted_shell())?
            .git_restore_from_upstream()
    })
    .await
}

#[tauri::command]
pub(super) async fn open_external_url(url: String) -> Result<(), String> {
    if !app_update::allowed_external_url(&url) {
        return Err("external URL is not allowlisted".into());
    }
    tauri_plugin_opener::open_url(url, None::<&str>).map_err(|error| error.to_string())
}

#[tauri::command]
pub(super) async fn project_open_memory(
    state: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedPhysicalJobs>,
    plugins: tauri::State<'_, SharedPluginHost>,
    ai_runtime: tauri::State<'_, ai::SharedAiRuntime>,
    watcher: tauri::State<'_, SharedProjectWatcher>,
) -> Result<(), String> {
    cancel_physical_jobs(jobs.inner())?;
    cancel_external_import_jobs()?;
    stop_project_watcher(watcher.inner())?;
    flush_project_checkpoint(state.clone(), "project lifecycle transition").await?;
    let core = state.inner().clone();
    let plugins = plugins.inner().clone();
    let result = with_core(state, move |core| {
        core.open_memory_without_flush(trusted_shell())?;
        let project = core.project_mut(trusted_shell())?;
        let host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        apply_relationship_runtime_schemas(project, &host)
    })
    .await;
    if result.is_ok() {
        begin_current_physical_session(jobs.inner(), &core)?;
        schedule_ai_index_refresh(&core, ai_runtime.inner());
    }
    result
}

#[tauri::command]
pub(super) async fn project_open_default(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedPhysicalJobs>,
    plugins: tauri::State<'_, SharedPluginHost>,
    ai_runtime: tauri::State<'_, ai::SharedAiRuntime>,
    watcher: tauri::State<'_, SharedProjectWatcher>,
) -> Result<(), String> {
    cancel_physical_jobs(jobs.inner())?;
    cancel_external_import_jobs()?;
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    let plugins = plugins.inner().clone();
    let core = state.inner().clone();
    let ai_runtime = ai_runtime.inner().clone();
    let sync_plugins = plugins.clone();
    flush_project_checkpoint(state.clone(), "project lifecycle transition").await?;
    let result = with_core(state, move |core| {
        std::fs::create_dir_all(&directory).map_err(|error| CoreError::Io {
            operation: "create app data directory",
            source: error,
        })?;
        let project_directory = directory.join("Daena Archive");
        if let Some(previous_project) = core.info().map(|info| info.root) {
            plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?
                .deactivate_project(&previous_project);
        }
        core.open_directory_without_flush(trusted_shell(), project_directory)?;
        Ok(())
    })
    .await;
    if result.is_ok() {
        begin_current_physical_session(jobs.inner(), &core)?;
        sync_project_usage_and_wait(core.clone(), sync_plugins).await?;
        flush_project_checkpoint_for_shared_core(core.clone(), "project startup synchronization")
            .await?;
        schedule_ai_index_refresh(&core, &ai_runtime);
        start_project_watcher(&app, &core, watcher.inner())?;
    }
    result
}

#[tauri::command]
pub(super) async fn project_create_entity(
    state: tauri::State<'_, SharedCore>,
    input: CreateEntity,
    request_id: Option<String>,
) -> Result<Entity, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?
            .create_entity_with_request(input, request_id.as_deref())
    })
    .await
}

#[tauri::command]
pub(super) async fn project_list_entities(
    state: tauri::State<'_, SharedCore>,
) -> Result<Vec<Entity>, String> {
    with_read_project(state, daena_core::ProjectStore::list_entities).await
}

#[tauri::command]
pub(super) async fn project_get_entity(
    state: tauri::State<'_, SharedCore>,
    id: String,
) -> Result<Option<Entity>, String> {
    with_read_project(state, move |project| project.get_entity(&id)).await
}

#[tauri::command]
pub(super) async fn project_query_entities(
    state: tauri::State<'_, SharedCore>,
    query: EntityListQuery,
) -> Result<EntityPage, String> {
    with_read_project(state, move |project| project.query_entities(query)).await
}

#[tauri::command]
pub(super) async fn project_search(
    state: tauri::State<'_, SharedCore>,
    query: String,
) -> Result<Vec<Entity>, String> {
    with_read_project(state, move |project| project.search(query)).await
}

#[tauri::command]
pub(super) async fn project_search_map_features(
    state: tauri::State<'_, SharedCore>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<daena_core::MapFeatureSearchResult>, String> {
    with_read_project(state, move |project| {
        project.search_map_features(query, limit.unwrap_or(50))
    })
    .await
}

#[tauri::command]
pub(super) async fn project_update_entity(
    state: tauri::State<'_, SharedCore>,
    id: String,
    name: Option<String>,
    entity_type: Option<String>,
    expected_revision: Option<String>,
    request_id: Option<String>,
) -> Result<Entity, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.update_entity_with_options(
            id,
            name,
            entity_type,
            expected_revision.as_deref(),
            request_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
pub(super) async fn project_delete_entity(
    state: tauri::State<'_, SharedCore>,
    id: String,
    expected_revision: Option<String>,
    request_id: Option<String>,
) -> Result<(), String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.delete_entity_with_options(
            id,
            expected_revision.as_deref(),
            request_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
pub(super) async fn project_restore_entity(
    state: tauri::State<'_, SharedCore>,
    id: String,
    expected_revision: Option<String>,
    request_id: Option<String>,
) -> Result<(), String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.restore_entity_with_options(
            id,
            expected_revision.as_deref(),
            request_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
pub(super) async fn project_purge_entity(
    state: tauri::State<'_, SharedCore>,
    id: String,
    expected_revision: Option<String>,
    request_id: Option<String>,
) -> Result<(), String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.purge_entity_with_options(
            id,
            expected_revision.as_deref(),
            request_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
pub(super) async fn project_save_document(
    state: tauri::State<'_, SharedCore>,
    input: SaveDocument,
    expected_revision: Option<String>,
    request_id: Option<String>,
) -> Result<(), String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.save_document_with_options(
            input,
            expected_revision.as_deref(),
            request_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
pub(super) async fn project_save_entry(
    state: tauri::State<'_, SharedCore>,
    input: SaveEntry,
    expected_revision: Option<String>,
    request_id: Option<String>,
) -> Result<(), String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.save_entry_with_options(
            input,
            expected_revision.as_deref(),
            request_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
pub(super) async fn project_list_documents(
    state: tauri::State<'_, SharedCore>,
    entity_id: String,
) -> Result<Vec<daena_core::Document>, String> {
    with_read_project(state, move |project| project.list_documents(entity_id)).await
}

#[tauri::command]
pub(super) async fn project_set_field(
    state: tauri::State<'_, SharedCore>,
    field: FieldValue,
    request_id: Option<String>,
) -> Result<(), String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?
            .set_field_with_request(field, request_id.as_deref())
    })
    .await
}

#[tauri::command]
pub(super) async fn project_list_fields(
    state: tauri::State<'_, SharedCore>,
    entity_id: String,
) -> Result<Vec<FieldValue>, String> {
    with_read_project(state, move |project| project.list_fields(entity_id)).await
}

#[tauri::command]
pub(super) async fn project_create_relationship(
    state: tauri::State<'_, SharedCore>,
    input: RelationshipInput,
    expected_revision: Option<String>,
    request_id: Option<String>,
) -> Result<Relationship, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?
            .create_relationship_with_options(
                input,
                expected_revision.as_deref(),
                request_id.as_deref(),
            )
    })
    .await
}

#[tauri::command]
pub(super) async fn project_update_relationship(
    state: tauri::State<'_, SharedCore>,
    input: daena_core::RelationshipUpdate,
    expected_revision: Option<String>,
    request_id: Option<String>,
) -> Result<Relationship, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?
            .update_relationship_with_options(
                input,
                expected_revision.as_deref(),
                request_id.as_deref(),
            )
    })
    .await
}

#[tauri::command]
pub(super) async fn project_list_relationships(
    state: tauri::State<'_, SharedCore>,
    entity_id: String,
) -> Result<Vec<Relationship>, String> {
    with_read_project(state, move |project| project.list_relationships(entity_id)).await
}

#[tauri::command]
pub(super) async fn project_list_map_locations(
    state: tauri::State<'_, SharedCore>,
    entity_id: String,
) -> Result<Vec<daena_core::maps::LocationReference>, String> {
    with_read_project(state, move |project| project.map_locations(entity_id)).await
}

#[tauri::command]
pub(super) async fn project_upsert_map_location(
    state: tauri::State<'_, SharedCore>,
    entity_id: String,
    location: daena_core::maps::LocationReference,
    request_id: Option<String>,
) -> Result<(), String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.upsert_map_location(
            entity_id,
            location,
            request_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
pub(super) async fn project_unlink_map_location(
    state: tauri::State<'_, SharedCore>,
    entity_id: String,
    location_id: String,
    request_id: Option<String>,
) -> Result<(), String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.unlink_map_location(
            entity_id,
            location_id,
            request_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
pub(super) async fn maps_recovery_list(
    state: tauri::State<'_, SharedCore>,
    entity_id: String,
) -> Result<Vec<daena_core::maps::MapRecoveryCopy>, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?
            .list_map_recovery_copies(&entity_id)
    })
    .await
}

#[tauri::command]
pub(super) async fn maps_recovery_restore(
    state: tauri::State<'_, SharedCore>,
    entity_id: String,
    file_name: String,
    request_id: Option<String>,
) -> Result<daena_core::MapEditApply, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.restore_map_recovery_copy(
            &entity_id,
            &file_name,
            request_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
pub(super) async fn project_register_asset(
    state: tauri::State<'_, SharedCore>,
    input: AssetInput,
    expected_revision: Option<String>,
    request_id: Option<String>,
) -> Result<Asset, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.register_asset_with_options(
            input,
            expected_revision.as_deref(),
            request_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
pub(super) async fn project_register_asset_file(
    state: tauri::State<'_, SharedCore>,
    input: AssetFileInput,
    expected_revision: Option<String>,
    request_id: Option<String>,
) -> Result<Asset, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?
            .register_asset_file_with_options(
                input,
                expected_revision.as_deref(),
                request_id.as_deref(),
            )
    })
    .await
}

#[tauri::command]
pub(super) async fn project_list_assets(
    state: tauri::State<'_, SharedCore>,
    entity_id: String,
) -> Result<Vec<Asset>, String> {
    with_read_project(state, move |project| project.list_assets(entity_id)).await
}

#[tauri::command]
pub(super) async fn project_list_shared_assets(
    state: tauri::State<'_, SharedCore>,
) -> Result<Vec<Asset>, String> {
    with_read_project(state, move |project| project.list_shared_assets()).await
}

#[tauri::command]
pub(super) async fn project_import_image_map_file(
    state: tauri::State<'_, SharedCore>,
    source_path: String,
) -> Result<daena_core::ImportedImageMap, String> {
    let path = PathBuf::from(&source_path);
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("image filename is invalid")?
        .to_string();
    let name = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("Image map")
        .to_string();
    let mime = match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        _ => return Err("Choose a PNG, JPEG, or SVG image".into()),
    }
    .to_string();
    let bytes = std::fs::read(&path).map_err(|error| format!("read image: {error}"))?;
    with_core(state, move |core| {
        core.project(trusted_shell())?
            .import_image_map(name, bytes, mime, filename, None)
    })
    .await
}

#[tauri::command]
pub(super) async fn project_attach_map_raster_asset(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
    source_path: String,
) -> Result<daena_core::AttachedMapRaster, String> {
    let path = PathBuf::from(&source_path);
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("image filename is invalid")?
        .to_string();
    let mime = match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        _ => return Err("Choose a PNG, JPEG, or SVG image".into()),
    }
    .to_string();
    let bytes = std::fs::read(&path).map_err(|error| format!("read image: {error}"))?;
    with_core(state, move |core| {
        core.project(trusted_shell())?.attach_map_raster_asset(
            map_entity_id,
            bytes,
            mime,
            filename,
            None,
        )
    })
    .await
}

#[tauri::command]
pub(super) async fn project_duplicate_map_raster_asset(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
    asset_id: String,
) -> Result<daena_core::AttachedMapRaster, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?
            .duplicate_map_raster_asset(map_entity_id, asset_id, None)
    })
    .await
}

#[tauri::command]
pub(super) async fn project_import_vector_map_file(
    state: tauri::State<'_, SharedCore>,
    source_path: String,
) -> Result<daena_core::AcceptedVectorMap, String> {
    let path = PathBuf::from(&source_path);
    let name = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("Vector map")
        .to_string();
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("geojson" | "json") => {}
        _ => return Err("Choose a GeoJSON (.geojson or .json) file".into()),
    }
    let bytes = std::fs::read(&path).map_err(|error| format!("read GeoJSON: {error}"))?;
    with_core(state, move |core| {
        core.project(trusted_shell())?
            .import_vector_map(name, bytes, None)
    })
    .await
}

#[tauri::command]
pub(super) async fn project_accept_vector_map(
    state: tauri::State<'_, SharedCore>,
    name: String,
    candidate_json: String,
    generation: serde_json::Value,
    request_id: Option<String>,
) -> Result<daena_core::AcceptedVectorMap, String> {
    let bytes = candidate_json.into_bytes();
    with_core(state, move |core| {
        core.project(trusted_shell())?.accept_vector_map(
            name,
            bytes,
            generation,
            request_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
pub(super) async fn project_replace_vector_source(
    state: tauri::State<'_, SharedCore>,
    asset_id: String,
    bytes: Vec<u8>,
    upload_content_hash: String,
    expected_revision: String,
    request_id: Option<String>,
) -> Result<daena_core::VectorSourceReplace, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.replace_vector_source(
            asset_id,
            bytes,
            upload_content_hash,
            &expected_revision,
            request_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub(super) async fn project_apply_map_edit(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
    descriptor: serde_json::Value,
    layers: serde_json::Value,
    bytes: Vec<u8>,
    upload_content_hash: String,
    expected_map_revision: String,
    expected_layers_revision: String,
    expected_source_revision: String,
    link_mutations: Option<Vec<daena_core::MapLinkMutation>>,
    request_id: Option<String>,
) -> Result<daena_core::MapEditApply, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.apply_map_edit(
            map_entity_id,
            descriptor,
            layers,
            bytes,
            upload_content_hash,
            &expected_map_revision,
            &expected_layers_revision,
            &expected_source_revision,
            link_mutations.unwrap_or_default(),
            request_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
pub(super) async fn project_create_vector_layer(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
    name: String,
    expected_revision: String,
    style: Option<serde_json::Value>,
    request_id: Option<String>,
) -> Result<daena_core::RasterLayerChange, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.create_vector_layer(
            map_entity_id,
            name,
            &expected_revision,
            request_id.as_deref(),
            style,
        )
    })
    .await
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub(super) async fn project_delete_vector_layer(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
    layer_id: String,
    expected_revision: String,
    expected_source_revision: String,
    expected_feature_count: i64,
    request_id: Option<String>,
) -> Result<daena_core::VectorLayerDelete, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.delete_vector_layer(
            map_entity_id,
            layer_id,
            &expected_revision,
            &expected_source_revision,
            expected_feature_count,
            request_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
pub(super) async fn maps_recovery_export(
    state: tauri::State<'_, SharedCore>,
    entity_id: String,
    bytes: Vec<u8>,
) -> Result<String, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?
            .save_map_recovery_copy(&entity_id, &bytes)
    })
    .await
}

#[tauri::command]
pub(super) async fn project_read_asset_bytes(
    state: tauri::State<'_, SharedCore>,
    asset_id: String,
) -> Result<Vec<u8>, String> {
    with_read_project(state, move |project| project.asset_bytes(asset_id)).await
}

#[tauri::command]
pub(super) async fn project_read_asset_bytes_by_path(
    state: tauri::State<'_, SharedCore>,
    path: String,
) -> Result<Vec<u8>, String> {
    with_read_project(state, move |project| project.asset_bytes_by_path(path)).await
}

#[tauri::command]
pub(super) async fn project_get_asset_by_path(
    state: tauri::State<'_, SharedCore>,
    path: String,
) -> Result<Asset, String> {
    with_read_project(state, move |project| project.asset_by_path(path)).await
}

#[tauri::command]
pub(super) async fn project_create_raster_layer(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
    name: String,
    expected_revision: String,
    request_id: Option<String>,
) -> Result<daena_core::RasterLayerChange, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.create_raster_layer(
            map_entity_id,
            name,
            &expected_revision,
            request_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
pub(super) async fn project_create_semantic_layer(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
    name: String,
    expected_revision: String,
    style: Option<serde_json::Value>,
    selector: Option<serde_json::Value>,
    request_id: Option<String>,
) -> Result<daena_core::RasterLayerChange, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.create_semantic_layer(
            map_entity_id,
            name,
            &expected_revision,
            request_id.as_deref(),
            style,
            selector,
        )
    })
    .await
}

#[tauri::command]
pub(super) async fn project_delete_semantic_layer(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
    layer_id: String,
    expected_revision: String,
    request_id: Option<String>,
) -> Result<daena_core::RasterLayerChange, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.delete_semantic_layer(
            map_entity_id,
            layer_id,
            &expected_revision,
            request_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub(super) async fn project_update_map_layer(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
    layer_id: String,
    expected_revision: String,
    name: Option<String>,
    order: Option<i64>,
    default_visible: Option<bool>,
    opacity: Option<f64>,
    locked: Option<bool>,
    style: Option<serde_json::Value>,
    selector: Option<serde_json::Value>,
    request_id: Option<String>,
) -> Result<daena_core::RasterLayerChange, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.update_map_layer(
            map_entity_id,
            layer_id,
            daena_core::RasterLayerUpdate {
                name,
                order,
                default_visible,
                opacity,
                locked,
                style,
                selector,
            },
            &expected_revision,
            request_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
pub(super) async fn project_delete_raster_layer(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
    layer_id: String,
    expected_revision: String,
    request_id: Option<String>,
) -> Result<daena_core::RasterLayerChange, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.delete_raster_layer(
            map_entity_id,
            layer_id,
            &expected_revision,
            request_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
pub(super) async fn project_replace_asset_bytes(
    state: tauri::State<'_, SharedCore>,
    asset_id: String,
    bytes: Vec<u8>,
    content_hash: String,
    mime_type: String,
    expected_revision: String,
    request_id: Option<String>,
) -> Result<Asset, String> {
    let size = bytes.len() as i64;
    with_core(state, move |core| {
        core.project(trusted_shell())?
            .replace_asset_bytes_with_request(
                AssetReplaceInput {
                    asset_id,
                    content_hash,
                    size,
                    mime_type,
                },
                bytes,
                &expected_revision,
                request_id.as_deref(),
            )
    })
    .await
}

#[tauri::command]
pub(super) async fn project_replace_asset_file(
    state: tauri::State<'_, SharedCore>,
    asset_id: String,
    source_path: String,
    mime_type: String,
    expected_revision: String,
    request_id: Option<String>,
) -> Result<Asset, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?
            .replace_asset_file_with_request(
                AssetFileReplaceInput {
                    asset_id,
                    source_path,
                    mime_type,
                },
                &expected_revision,
                request_id.as_deref(),
            )
    })
    .await
}

#[tauri::command]
pub(super) async fn project_update_asset_metadata(
    state: tauri::State<'_, SharedCore>,
    asset_id: String,
    filename: Option<String>,
    role: Option<String>,
    reference_scope: Option<String>,
    expected_revision: String,
    request_id: Option<String>,
) -> Result<Asset, String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?
            .update_asset_metadata_with_request(
                AssetMetadataUpdate {
                    asset_id,
                    filename,
                    role,
                    reference_scope,
                },
                &expected_revision,
                request_id.as_deref(),
            )
    })
    .await
}

#[tauri::command]
pub(super) async fn project_delete_asset(
    state: tauri::State<'_, SharedCore>,
    asset_id: String,
    expected_revision: String,
    request_id: Option<String>,
) -> Result<(), String> {
    with_core(state, move |core| {
        core.project(trusted_shell())?.delete_asset_with_request(
            asset_id,
            &expected_revision,
            request_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
pub(super) async fn project_map_location_projection(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    with_read_project(state, move |project| {
        project.map_location_projection(map_entity_id)
    })
    .await
}

#[tauri::command]
pub(super) async fn project_query_map_locations(
    state: tauri::State<'_, SharedCore>,
    map_entity_id: String,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
) -> Result<Vec<serde_json::Value>, String> {
    with_read_project(state, move |project| {
        project.query_map_locations(map_entity_id, min_x, min_y, max_x, max_y)
    })
    .await
}

pub(super) fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|error| error.to_string())?;
    for entry in fs::read_dir(src).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub(super) async fn project_backup(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedCore>,
) -> Result<String, String> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    flush_project_checkpoint(state.clone(), "portable backup").await?;
    let backup_path = with_read_project(state, move |project| {
        project.portable_backup_after_checkpoint(directory)
    })
    .await?;
    let backup_path_buf = PathBuf::from(&backup_path);
    if let Some(folder) = app.dialog().file().blocking_pick_folder() {
        let folder_path = folder.into_path().map_err(|error| error.to_string())?;
        if let Some(file_name) = backup_path_buf.file_name() {
            let dest = folder_path.join(file_name);
            copy_dir_recursive(&backup_path_buf, &dest)?;
            return Ok(dest.to_string_lossy().into_owned());
        }
    }
    Ok(backup_path)
}

#[tauri::command]
pub(super) async fn project_export_markdown(
    state: tauri::State<'_, SharedCore>,
    destination: String,
) -> Result<String, String> {
    with_read_project(state, move |project| {
        project.export_markdown_to(destination)
    })
    .await
}

#[tauri::command]
pub(super) async fn project_export_wiki_page(
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    entity_id: String,
    destination: String,
    format: WikiPageExportFormat,
    manifest_id: String,
) -> Result<String, String> {
    let plugins = plugins.inner().clone();
    with_read_project(state, move |project| {
        let host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        let manifest = effective_module_manifests(project, &host)?
            .into_iter()
            .find_map(|(manifest, enabled)| {
                (enabled && manifest.id == manifest_id).then_some(manifest)
            })
            .ok_or_else(|| CoreError::Validation("wiki manifest is not enabled".into()))?;
        project.export_wiki_page_to(&entity_id, destination, format, &manifest)
    })
    .await
}

#[tauri::command]
pub(super) async fn project_recovery_backup(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedCore>,
) -> Result<String, String> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    let backup_path = with_core(state, move |core| {
        core.project_mut(trusted_shell())?
            .recovery_backup_to(directory)
    })
    .await?;
    let backup_path_buf = PathBuf::from(&backup_path);
    if let Some(folder) = app.dialog().file().blocking_pick_folder() {
        let folder_path = folder.into_path().map_err(|error| error.to_string())?;
        if let Some(file_name) = backup_path_buf.file_name() {
            let dest = folder_path.join(file_name);
            copy_dir_recursive(&backup_path_buf, &dest)?;
            return Ok(dest.to_string_lossy().into_owned());
        }
    }
    Ok(backup_path)
}

#[tauri::command]
pub(super) async fn project_restore_recovery_backup(
    state: tauri::State<'_, SharedCore>,
    path: String,
) -> Result<(), String> {
    with_core(state, move |core| {
        core.project_mut(trusted_shell())?
            .restore_recovery_backup(path)
    })
    .await
}

#[tauri::command]
pub(super) async fn project_restore(
    state: tauri::State<'_, SharedCore>,
    path: String,
) -> Result<(), String> {
    with_core(state, move |core| {
        core.project_mut(trusted_shell())?.restore(path)
    })
    .await
}

#[tauri::command]
pub(super) async fn project_restore_payload(
    state: tauri::State<'_, SharedCore>,
    payload: String,
    request_id: Option<String>,
) -> Result<(), String> {
    with_core(state, move |core| {
        core.project_mut(trusted_shell())?
            .restore_payload_with_request(&payload, request_id.as_deref())
    })
    .await
}

#[tauri::command]
pub(super) async fn project_rebuild_search(
    state: tauri::State<'_, SharedCore>,
) -> Result<(), String> {
    with_core(state, |core| {
        core.project(trusted_shell())?.rebuild_search()
    })
    .await
}

#[tauri::command]
pub(super) async fn project_seed_example(
    state: tauri::State<'_, SharedCore>,
) -> Result<usize, String> {
    with_core(state, |core| {
        core.project_mut(trusted_shell())?.seed_example()
    })
    .await
}

#[tauri::command]
pub(super) async fn migration_validate(
    state: tauri::State<'_, SharedCore>,
    module_id: String,
    migration: serde_json::Value,
) -> Result<(), String> {
    let migration: Migration =
        serde_json::from_value(migration).map_err(|error| error.to_string())?;
    if migration.module_id != module_id {
        return Err("migration module ID does not match command module ID".into());
    }
    with_core(state, move |core| {
        let project = core.project(trusted_shell())?;
        let current = project.get_module_version(&module_id)?;
        project.validate_migration(&migration, current)
    })
    .await
}

#[tauri::command]
pub(super) async fn migration_apply(
    state: tauri::State<'_, SharedCore>,
    module_id: String,
    migration: serde_json::Value,
) -> Result<(), String> {
    let migration: Migration =
        serde_json::from_value(migration).map_err(|error| error.to_string())?;
    if migration.module_id != module_id {
        return Err("migration module ID does not match command module ID".into());
    }
    with_core(state, move |core| {
        core.project_mut(trusted_shell())?
            .apply_migration(&migration)
    })
    .await
}

pub(super) fn close_project_for_app(
    app: &tauri::AppHandle,
    core: &SharedCore,
    jobs: &SharedPhysicalJobs,
    plugins: &SharedPluginHost,
    ai_runtime: &ai::SharedAiRuntime,
    watcher: &SharedProjectWatcher,
    image_jobs: &image_generation::SharedImageGeneration,
) -> Result<(), String> {
    cancel_physical_jobs(jobs)?;
    cancel_external_import_jobs()?;
    cancel_image_generation_jobs(image_jobs)?;
    stop_project_watcher(watcher)?;
    flush_checkpoint_for_shared_core(core, "project lifecycle transition")?;
    let session = current_session(core)?;
    let mut service = session
        .core
        .lock()
        .map_err(|_| "core lock poisoned".to_string())?;
    let Some(project_id) = service.info().map(|info| info.root) else {
        return Ok(());
    };
    service
        .close_without_flush(trusted_shell())
        .map_err(|error| error.to_string())?;
    ai::detach_project_index(ai_runtime);
    let plugin_ids = {
        let mut host = plugins
            .lock()
            .map_err(|_| "plugin host lock poisoned".to_string())?;
        for request_id in host.ai_request_ids_for(&project_id, None) {
            let _ = ai::cancel_ai_request(ai_runtime, &request_id);
            let _ = ai::remove_ai_citations(ai_runtime, &request_id);
            host.remove_ai_request(&request_id);
        }
        host.deactivate_project(&project_id);
        host.catalog
            .list()
            .map(|entry| entry.manifest.id.clone())
            .collect::<Vec<_>>()
    };
    drop(service);
    for plugin_id in plugin_ids {
        close_plugin_webview(app, &plugin_id);
    }
    Ok(())
}
