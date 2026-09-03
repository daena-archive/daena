// AI project index management.
use super::*;

pub fn attach_project_index(runtime: &SharedAiRuntime, project_root: &str) {
    let index = if project_root.trim().is_empty() {
        None
    } else {
        AiIndex::open(std::path::Path::new(project_root).join(".daena/ai/index.sqlite")).ok()
    };
    if let Ok(mut runtime) = runtime.lock() {
        runtime.index = index;
        runtime.index_cancel = None;
        runtime.index_state = if runtime.index.is_some() {
            None
        } else {
            Some(IndexState::Failed)
        };
    }
}

pub fn detach_project_index(runtime: &SharedAiRuntime) {
    if let Ok(mut runtime) = runtime.lock() {
        if let Some(cancel) = runtime.index_cancel.as_ref() {
            cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        runtime.index = None;
        runtime.index_cancel = None;
        runtime.index_state = None;
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiIndexStatus {
    pub available: bool,
    pub state: Option<IndexState>,
    pub provider: Option<String>,
    pub embedding_available: bool,
    pub message: Option<String>,
}

pub fn index_status(runtime: &SharedAiRuntime) -> AiIndexStatus {
    let Ok(runtime) = runtime.lock() else {
        return AiIndexStatus {
            available: false,
            state: None,
            provider: None,
            embedding_available: false,
            message: Some("AI index runtime is unavailable".into()),
        };
    };
    if runtime.index.is_none() && runtime.index_cancel.is_some() {
        return AiIndexStatus {
            available: false,
            state: Some(IndexState::Indexing),
            provider: None,
            embedding_available: false,
            message: Some("AI index rebuild is in progress".into()),
        };
    }
    let Some(index) = runtime.index.as_ref() else {
        return AiIndexStatus {
            available: false,
            state: runtime.index_state,
            provider: None,
            embedding_available: false,
            message: Some("No AI index is attached to the open project".into()),
        };
    };
    match index.state() {
        Ok(state) => AiIndexStatus {
            available: true,
            state: Some(state),
            provider: None,
            embedding_available: true,
            message: None,
        },
        Err(_) => AiIndexStatus {
            available: false,
            state: Some(IndexState::Failed),
            provider: None,
            embedding_available: false,
            message: Some("AI index status could not be read".into()),
        },
    }
}

#[tauri::command]
pub fn ai_index_cancel(runtime: State<'_, SharedAiRuntime>) -> Result<(), String> {
    let runtime = runtime
        .lock()
        .map_err(|_| "AI runtime lock poisoned".to_string())?;
    if let Some(cancel) = runtime.index_cancel.as_ref() {
        cancel.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    Ok(())
}

#[tauri::command]
pub fn ai_index_status(
    core: State<'_, crate::SharedCore>,
    runtime: State<'_, SharedAiRuntime>,
    settings: State<'_, Arc<Mutex<SettingsStore>>>,
) -> Result<AiIndexStatus, String> {
    let project_id = crate::current_info(core.inner())?.map(|info| info.root);
    if let Some(project_id) = project_id.as_deref() {
        crate::ensure_project_ai_enabled(project_id)?;
    }
    let mut status = index_status(runtime.inner());
    let configured = settings.lock().ok().and_then(|store| store.load().ok());
    let Some(configured) = configured else {
        status.message = Some("AI provider settings are unavailable".into());
        return Ok(status);
    };
    match resolve_ai_provider_with_credential(&configured, project_id.as_deref(), false, false) {
        Ok(provider) => {
            status.provider = Some(provider.provider_id.clone());
            status.embedding_available = provider.embedding_available;
            if !provider.embedding_available {
                status.message = Some(format!(
                    "Semantic indexing is unavailable for active provider '{}': embedding capability is not configured",
                    provider.provider_id
                ));
            }
        }
        Err(error) => status.message = Some(error),
    }
    Ok(status)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiIndexRebuildResult {
    pub chunk_count: usize,
    pub embedded_count: usize,
    pub reused_count: usize,
    pub state: IndexState,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiHybridMatch {
    pub chunk_id: String,
    pub source_id: String,
    pub source_kind: String,
    pub score: f32,
}

#[tauri::command]
pub async fn ai_index_search(
    core: State<'_, crate::SharedCore>,
    runtime: State<'_, SharedAiRuntime>,
    settings: State<'_, Arc<Mutex<SettingsStore>>>,
    query: String,
    limit: usize,
) -> Result<Vec<AiHybridMatch>, String> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    let limit = limit.clamp(1, 32);
    let configured = settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .load()?;
    let project_id = crate::current_info(core.inner())?
        .map(|info| info.root)
        .ok_or_else(|| "No project is open".to_string())?;
    crate::ensure_project_ai_enabled(&project_id)?;
    let provider = resolve_ai_provider(&configured, Some(&project_id), true)?;
    if !provider.embedding_available {
        return Err(format!(
            "Semantic indexing is unavailable for active provider '{}': embedding capability is not configured",
            provider.provider_id
        ));
    }
    let runtime = runtime.inner().clone();
    let embedding_model = provider.embedding_model_or_model();
    let endpoint = provider.endpoint;
    let remote = provider.remote;
    let api_key = provider.api_key;
    tauri::async_runtime::spawn_blocking(move || {
        let runtime = runtime
            .lock()
            .map_err(|_| "AI runtime lock poisoned".to_string())?;
        let index = runtime
            .index
            .as_ref()
            .ok_or_else(|| "No directory-backed project AI index is attached".to_string())?;
        let provider = LmStudioEmbeddingProvider {
            endpoint,
            model: embedding_model,
            remote,
            api_key,
        };
        let query_vector = provider
            .embed(std::slice::from_ref(&query))
            .map_err(|error| error.to_string())?
            .pop()
            .ok_or_else(|| "embedding provider returned no query vector".to_string())?;
        let records = index.records().map_err(|error| error.to_string())?;
        let semantic = daena_ai::index::exact_cosine_search(&records, &query_vector, limit);
        let terms = query
            .split_whitespace()
            .map(str::to_lowercase)
            .collect::<Vec<_>>();
        let lexical = records
            .iter()
            .filter(|record| {
                let text = record.chunk.text.to_lowercase();
                terms.iter().all(|term| text.contains(term))
            })
            .enumerate()
            .map(|(rank, record)| (record.chunk.id.clone(), rank))
            .collect::<Vec<_>>();
        let fused = daena_ai::index::reciprocal_rank_fusion(&lexical, &semantic, limit);
        Ok(fused
            .into_iter()
            .filter_map(|(chunk_id, score)| {
                records
                    .iter()
                    .find(|record| record.chunk.id == chunk_id)
                    .map(|record| AiHybridMatch {
                        chunk_id,
                        source_id: record.chunk.source.source_id.clone(),
                        source_kind: record.chunk.source.source_kind.clone(),
                        score,
                    })
            })
            .collect())
    })
    .await
    .map_err(|error| error.to_string())?
}

pub(super) fn display_metadata_value(value: &serde_json::Value) -> String {
    let rendered = match value {
        serde_json::Value::String(value) => value.clone(),
        serde_json::Value::Bool(value) => value.to_string(),
        serde_json::Value::Number(value) => value.to_string(),
        _ => value.to_string(),
    };
    let normalized = rendered.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = normalized.chars();
    let display = chars.by_ref().take(160).collect::<String>();
    if chars.next().is_some() {
        format!("{display}...")
    } else {
        display
    }
}

pub(super) fn relationship_summary(
    project: &ProjectStore,
    source_name: &str,
    target_name: &str,
    relationship_type: &str,
    raw_metadata: &str,
) -> String {
    let mut summary = format!("{source_name} --{relationship_type}--> {target_name}");
    let Ok(serde_json::Value::Object(metadata)) = serde_json::from_str(raw_metadata) else {
        return summary;
    };
    let labels = project
        .relationship_metadata_fields_for_type(relationship_type)
        .map(|fields| {
            fields
                .iter()
                .map(|field| (field.key.as_str(), field.label.as_str()))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    let metadata_summary = metadata
        .iter()
        .filter(|(_, value)| !value.is_null() && value.as_str() != Some(""))
        .take(8)
        .map(|(key, value)| {
            format!(
                "{}: {}",
                labels.get(key.as_str()).copied().unwrap_or(key),
                display_metadata_value(value)
            )
        })
        .collect::<Vec<_>>();
    if !metadata_summary.is_empty() {
        summary.push_str(" · ");
        summary.push_str(&metadata_summary.join(" · "));
    }
    summary
}

pub(super) fn project_chunks(project: &ProjectStore) -> Result<Vec<TextChunk>, String> {
    let mut chunks = Vec::new();
    for entity in project.list_entities().map_err(|error| error.to_string())? {
        for document in project
            .list_documents(entity.id.clone())
            .map_err(|error| error.to_string())?
        {
            if document.format != "markdown" && document.format != "plain-text" {
                continue;
            }
            chunks.extend(daena_ai::index::chunk_markdown(
                ChunkSource {
                    source_id: document.id,
                    source_kind: "document".into(),
                    revision: document.revision,
                    source_hash: daena_ai::index::hash_text(&document.body),
                },
                &document.body,
                16 * 1024,
            ));
        }
        for field in project
            .list_fields(entity.id.clone())
            .map_err(|error| error.to_string())?
        {
            let value = serde_json::json!({
                "entityId": field.entity_id,
                "namespace": field.namespace,
                "key": field.key,
                "value": field.value,
            });
            let text = serde_json::to_string(&value).map_err(|error| error.to_string())?;
            chunks.extend(daena_ai::index::chunk_structured(
                ChunkSource {
                    source_id: format!("field:{}:{}:{}", entity.id, field.namespace, field.key),
                    source_kind: "field".into(),
                    revision: field.revision,
                    source_hash: daena_ai::index::hash_text(&text),
                },
                &value,
                16 * 1024,
            ));
        }
        for relationship in project
            .list_relationships(entity.id.clone())
            .map_err(|error| error.to_string())?
            .into_iter()
            .filter(|relationship| relationship.source_id == entity.id)
        {
            let value = serde_json::json!({
                "id": relationship.id,
                "sourceId": relationship.source_id,
                "targetId": relationship.target_id,
                "type": relationship.relationship_type,
                "metadata": relationship.metadata,
            });
            let text = serde_json::to_string(&value).map_err(|error| error.to_string())?;
            chunks.extend(daena_ai::index::chunk_structured(
                ChunkSource {
                    source_id: format!("relationship:{}", relationship.id),
                    source_kind: "relationship".into(),
                    revision: relationship.revision,
                    source_hash: daena_ai::index::hash_text(&text),
                },
                &value,
                16 * 1024,
            ));
        }
    }
    Ok(chunks)
}

#[tauri::command]
pub async fn ai_index_rebuild(
    core: State<'_, crate::SharedCore>,
    runtime: State<'_, SharedAiRuntime>,
    settings: State<'_, Arc<Mutex<SettingsStore>>>,
) -> Result<AiIndexRebuildResult, String> {
    let configured = settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .load()?;
    let project_id = crate::current_info(core.inner())?
        .map(|info| info.root)
        .ok_or_else(|| "No project is open".to_string())?;
    crate::ensure_project_ai_enabled(&project_id)?;
    let provider = resolve_ai_provider(&configured, Some(&project_id), true)?;
    if !provider.embedding_available {
        return Err(format!(
            "Semantic indexing is unavailable for active provider '{}': embedding capability is not configured",
            provider.provider_id
        ));
    }
    let endpoint = provider.endpoint.clone();
    let model = provider.embedding_model_or_model();
    let provider_id = provider.provider_id.clone();
    let remote = provider.remote;
    let api_key = provider.api_key.clone();
    let capability_identity = provider.capability_identity.clone();
    let chunks = crate::with_read_project(core, |project| {
        project_chunks(project).map_err(CoreError::Conflict)
    })
    .await?;
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let (index, cancel) = {
            let mut runtime = runtime
                .lock()
                .map_err(|_| "AI runtime lock poisoned".to_string())?;
            let index = runtime
                .index
                .take()
                .ok_or_else(|| "No directory-backed project AI index is attached".to_string())?;
            let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
            runtime.index_cancel = Some(cancel.clone());
            (index, cancel)
        };
        let outcome = (|| {
            let provider = LmStudioEmbeddingProvider {
                endpoint,
                model: model.clone(),
                remote,
                api_key,
            };
            let probe_dimension = provider
                .embed(&["Daena embedding dimension probe".into()])
                .map_err(|error| error.to_string())?
                .pop()
                .map(|vector| vector.len())
                .ok_or_else(|| "embedding provider returned no probe vector".to_string())?;
            let mut metadata = EmbeddingMetadata {
                provider_id: provider_id.clone(),
                model_id: model,
                dimension: probe_dimension,
                normalized: true,
                capability_identity,
                serializer_version: format!(
                    "{}:{}",
                    daena_ai::index::EMBEDDING_SERIALIZER_VERSION,
                    AI_CHUNKER_VERSION
                ),
            };
            let mut sources = BTreeMap::<String, Vec<TextChunk>>::new();
            for chunk in chunks {
                sources
                    .entry(chunk.source.source_id.clone())
                    .or_default()
                    .push(chunk);
            }
            let mut chunk_count = 0;
            let mut embedded_count = 0;
            let mut reused_count = 0;
            for source_chunks in sources.values() {
                if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                    return Err("AI index rebuild cancelled".to_string());
                }
                let report = index
                    .index_source(source_chunks, &metadata, &provider, || {
                        cancel.load(std::sync::atomic::Ordering::Relaxed)
                    })
                    .map_err(|error| error.to_string())?;
                chunk_count += report.chunk_count;
                embedded_count += report.embedded_count;
                reused_count += report.reused_count;
                metadata = index
                    .embedding_metadata()
                    .map_err(|error| error.to_string())?
                    .unwrap_or(metadata);
            }
            Ok(AiIndexRebuildResult {
                chunk_count,
                embedded_count,
                reused_count,
                state: index.state().map_err(|error| error.to_string())?,
            })
        })();
        if let Ok(mut runtime) = runtime.lock() {
            let owns_rebuild = runtime
                .index_cancel
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(current, &cancel));
            if owns_rebuild {
                runtime.index = Some(index);
                runtime.index_cancel = None;
                runtime.index_state = None;
            }
        }
        outcome
    })
    .await
    .map_err(|error| error.to_string())?
}
