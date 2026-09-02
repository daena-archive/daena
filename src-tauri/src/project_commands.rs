// Project, Git, and map-location Tauri commands.
use super::*;

#[tauri::command]
pub(super) async fn project_info(
    state: tauri::State<'_, SharedCore>,
) -> Result<Option<ProjectInfo>, String> {
    with_read_project(state, |project| Ok(project.info())).await
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
