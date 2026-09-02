// AI retrieval context builders.
use super::*;

pub(super) fn has_capability(caller: &AiCaller, capability: &str) -> bool {
    matches!(caller.kind, daena_ai::CallerKind::TrustedShell)
        || caller
            .capabilities
            .iter()
            .any(|granted| granted == capability)
}

pub(super) fn has_project_scope(caller: &AiCaller) -> bool {
    matches!(caller.kind, daena_ai::CallerKind::TrustedShell)
        || caller
            .resource_scopes
            .iter()
            .any(|scope| scope == &format!("project:{}", caller.project_id) || scope == "project:*")
}

pub(super) fn source_allowed(policy: &RetrievalPolicy, source_kind: &str) -> bool {
    policy.allowed_source_kinds.is_empty()
        || policy
            .allowed_source_kinds
            .iter()
            .any(|kind| kind == source_kind)
}

pub(super) fn hash_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

pub(super) fn retrieval_entity_ids(
    project: &ProjectStore,
    mode: RetrievalMode,
    seed_ids: &[String],
    relationship_depth: u8,
) -> Result<std::collections::BTreeSet<String>, String> {
    let mut entity_ids = seed_ids
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    if matches!(mode, RetrievalMode::Project) {
        entity_ids.extend(
            project
                .list_entities()
                .map_err(|error| error.to_string())?
                .into_iter()
                .map(|entity| entity.id),
        );
    } else if matches!(mode, RetrievalMode::Related) {
        for _ in 0..relationship_depth {
            let current = entity_ids.iter().cloned().collect::<Vec<_>>();
            for entity_id in current {
                for relationship in project
                    .list_relationships(entity_id)
                    .map_err(|error| error.to_string())?
                {
                    entity_ids.insert(relationship.source_id);
                    entity_ids.insert(relationship.target_id);
                }
            }
        }
    }
    Ok(entity_ids)
}

pub fn build_retrieval_context(
    project: &ProjectStore,
    caller: &AiCaller,
    payload: &AiRetrievalPolicyPayload,
) -> Result<(String, Vec<SourceRef>), String> {
    build_retrieval_context_with_semantic(project, caller, payload, &[])
}

pub(super) fn passage_key(passage: &RetrievedPassage) -> String {
    format!(
        "{}:{}:{}",
        passage.source.source_kind,
        passage.source.document_id.as_deref().unwrap_or_default(),
        passage.source.canonical_path.as_deref().unwrap_or_default()
    )
}

pub(super) fn merge_hybrid_passages(
    mut lexical: Vec<RetrievedPassage>,
    semantic: &[RetrievedPassage],
) -> Vec<RetrievedPassage> {
    if semantic.is_empty() {
        return lexical;
    }
    let lexical_ranks =
        lexical
            .iter()
            .enumerate()
            .fold(HashMap::new(), |mut ranks, (rank, passage)| {
                ranks.entry(passage_key(passage)).or_insert(rank);
                ranks
            });
    let semantic_ranks =
        semantic
            .iter()
            .enumerate()
            .fold(HashMap::new(), |mut ranks, (rank, passage)| {
                ranks.entry(passage_key(passage)).or_insert(rank);
                ranks
            });
    lexical.extend(semantic.iter().cloned());
    lexical.sort_by(|left, right| {
        let score = |passage: &RetrievedPassage| {
            let key = passage_key(passage);
            let lexical_score = lexical_ranks
                .get(&key)
                .map(|rank| 1.0 / (60.0 + *rank as f64 + 1.0))
                .unwrap_or_default();
            let semantic_score = semantic_ranks
                .get(&key)
                .map(|rank| 1.0 / (60.0 + *rank as f64 + 1.0))
                .unwrap_or_default();
            lexical_score + semantic_score
        };
        score(right)
            .partial_cmp(&score(left))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for (rank, passage) in lexical.iter_mut().enumerate() {
        passage.lexical_rank = rank as u32;
    }
    lexical
}

pub(super) fn build_retrieval_context_with_semantic(
    project: &ProjectStore,
    caller: &AiCaller,
    payload: &AiRetrievalPolicyPayload,
    semantic_passages: &[RetrievedPassage],
) -> Result<(String, Vec<SourceRef>), String> {
    let mode = match payload.mode {
        AiRetrievalMode::None => RetrievalMode::None,
        AiRetrievalMode::ExplicitOnly => RetrievalMode::ExplicitOnly,
        AiRetrievalMode::Related => RetrievalMode::Related,
        AiRetrievalMode::Project => RetrievalMode::Project,
    };
    if payload.relationship_depth > 2 || payload.passage_count > 32 {
        return Err("retrieval policy exceeds the host bounds".into());
    }
    if !matches!(mode, RetrievalMode::None) && payload.passage_count == 0 {
        return Err("retrieval policy requires passageCount".into());
    }
    if matches!(mode, RetrievalMode::Project) && !has_capability(caller, "search.query") {
        return Err(AiError::RemoteContextDenied.to_string());
    }
    let policy = RetrievalPolicy {
        mode: mode.clone(),
        seed_ids: payload.seed_ids.clone(),
        allowed_source_kinds: payload.allowed_source_kinds.clone(),
        relationship_depth: payload.relationship_depth,
        passage_count: payload.passage_count,
        include_shared_fields: payload.include_shared_fields,
    };
    if matches!(mode, RetrievalMode::None) {
        return Ok((String::new(), Vec::new()));
    }
    if policy.seed_ids.is_empty() && !matches!(mode, RetrievalMode::Project) {
        return Err("retrieval policy requires seedIds".into());
    }
    if !has_capability(caller, "document.read") {
        return Err(AiError::RemoteContextDenied.to_string());
    }
    if !has_project_scope(caller) {
        return Err(AiError::RemoteContextDenied.to_string());
    }
    if matches!(mode, RetrievalMode::Related)
        && policy.relationship_depth > 0
        && !has_capability(caller, "relationship.read")
    {
        return Err(AiError::RemoteContextDenied.to_string());
    }
    let entity_ids = retrieval_entity_ids(
        project,
        mode.clone(),
        &policy.seed_ids,
        policy.relationship_depth,
    )?;
    let mut passages = Vec::new();
    if matches!(mode, RetrievalMode::Related)
        && policy.relationship_depth > 0
        && source_allowed(&policy, "relationship")
    {
        let entity_names = project
            .list_entities()
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|entity| (entity.id, entity.name))
            .collect::<std::collections::BTreeMap<_, _>>();
        let mut relationship_ids = std::collections::BTreeSet::new();
        for entity_id in &entity_ids {
            for relationship in project
                .list_relationships(entity_id.clone())
                .map_err(|error| error.to_string())?
            {
                if !entity_ids.contains(&relationship.source_id)
                    || !entity_ids.contains(&relationship.target_id)
                    || !relationship_ids.insert(relationship.id.clone())
                {
                    continue;
                }
                let source_name = entity_names
                    .get(&relationship.source_id)
                    .map(String::as_str)
                    .unwrap_or(&relationship.source_id);
                let target_name = entity_names
                    .get(&relationship.target_id)
                    .map(String::as_str)
                    .unwrap_or(&relationship.target_id);
                let metadata = if relationship.metadata.trim().is_empty() {
                    String::new()
                } else {
                    format!(" Metadata: {}", relationship.metadata)
                };
                let text = format!(
                    "{source_name} --{}--> {target_name}.{metadata}",
                    relationship.relationship_type
                );
                let summary = relationship_summary(
                    project,
                    source_name,
                    target_name,
                    &relationship.relationship_type,
                    &relationship.metadata,
                );
                let hash = hash_text(&text);
                passages.push(RetrievedPassage {
                    source: SourceRef {
                        source_kind: "relationship".into(),
                        summary: Some(summary),
                        entity_id: Some(relationship.source_id),
                        document_id: None,
                        canonical_path: Some(format!("relationships/{}.json", relationship.id)),
                        revision: relationship.revision,
                        content_hash: hash.clone(),
                        byte_start: Some(0),
                        byte_end: Some(text.len() as u64),
                        excerpt_hash: hash,
                    },
                    text,
                    lexical_rank: 0,
                });
            }
        }
    }
    let mut retrieved_document_ids = HashSet::new();
    if matches!(mode, RetrievalMode::Project | RetrievalMode::Related) {
        if let Some(query) = payload
            .query
            .as_deref()
            .filter(|query| !query.trim().is_empty())
        {
            for passage in project
                .search_passages(query.to_string(), policy.passage_count as usize)
                .map_err(|error| error.to_string())?
            {
                if !source_allowed(&policy, &passage.source_kind)
                    || passage.source_kind != "document"
                    || (matches!(mode, RetrievalMode::Related)
                        && !entity_ids.contains(&passage.entity_id))
                {
                    continue;
                }
                let document = project
                    .list_documents(passage.entity_id.clone())
                    .map_err(|error| error.to_string())?
                    .into_iter()
                    .find(|document| document.body == passage.content);
                let Some(document) = document else {
                    continue;
                };
                let hash = hash_text(&passage.content);
                retrieved_document_ids.insert(document.id.clone());
                passages.push(RetrievedPassage {
                    source: SourceRef {
                        source_kind: "document".into(),
                        summary: None,
                        entity_id: Some(passage.entity_id),
                        document_id: Some(document.id),
                        canonical_path: Some(passage.source_path),
                        revision: document.revision,
                        content_hash: if passage.source_hash.is_empty() {
                            hash.clone()
                        } else {
                            passage.source_hash
                        },
                        byte_start: Some(0),
                        byte_end: Some(passage.content.len() as u64),
                        excerpt_hash: hash,
                    },
                    text: passage.content,
                    lexical_rank: passage.lexical_rank as u32,
                });
            }
        }
    }
    let use_entity_fallback = !matches!(mode, RetrievalMode::Project)
        || payload
            .query
            .as_deref()
            .is_none_or(|query| query.trim().is_empty());
    let fallback_rank_start = passages.len() as u32;
    if use_entity_fallback {
        for (rank, entity_id) in entity_ids.into_iter().enumerate() {
            let documents = project
                .list_documents(entity_id.clone())
                .map_err(|error| error.to_string())?;
            if source_allowed(&policy, "document") {
                for document in documents {
                    if retrieved_document_ids.contains(&document.id) {
                        continue;
                    }
                    let source = SourceRef {
                        source_kind: "document".into(),
                        summary: None,
                        entity_id: Some(entity_id.clone()),
                        document_id: Some(document.id.clone()),
                        canonical_path: Some(format!("entities/{entity_id}/document.md")),
                        revision: document.revision,
                        content_hash: hash_text(&document.body),
                        byte_start: Some(0),
                        byte_end: Some(document.body.len() as u64),
                        excerpt_hash: hash_text(&document.body),
                    };
                    passages.push(RetrievedPassage {
                        source,
                        text: document.body,
                        lexical_rank: fallback_rank_start + rank as u32,
                    });
                }
            }
            if policy.include_shared_fields && source_allowed(&policy, "field") {
                if !has_capability(caller, "field.read:self") {
                    return Err(AiError::RemoteContextDenied.to_string());
                }
                for field in project
                    .list_fields(entity_id.clone())
                    .map_err(|error| error.to_string())?
                {
                    let text = format!("{} {} {}", field.namespace, field.key, field.value);
                    let hash = hash_text(&text);
                    passages.push(RetrievedPassage {
                        source: SourceRef {
                            source_kind: "field".into(),
                            summary: None,
                            entity_id: Some(entity_id.clone()),
                            document_id: None,
                            canonical_path: Some(format!(
                                "entities/{}/fields/{}-{}.json",
                                entity_id, field.namespace, field.key
                            )),
                            revision: field.revision,
                            content_hash: hash.clone(),
                            byte_start: Some(0),
                            byte_end: Some(text.len() as u64),
                            excerpt_hash: hash,
                        },
                        text,
                        lexical_rank: rank as u32 + 1,
                    });
                }
            }
        }
    }
    let passages = merge_hybrid_passages(passages, semantic_passages);
    let built = daena_ai::build_context(
        &passages,
        ContextBudget {
            max_bytes: DEFAULT_LIMITS.max_input_bytes,
            max_passages: policy.passage_count as usize,
        },
    );
    Ok((
        daena_ai::render_context_blocks(&built),
        built.blocks.into_iter().map(|block| block.source).collect(),
    ))
}

pub(super) fn retrieval_source_ids(
    project: &ProjectStore,
    payload: &AiRetrievalPolicyPayload,
) -> Result<RetrievalSourceIds, String> {
    let mode = match payload.mode {
        AiRetrievalMode::Related => RetrievalMode::Related,
        AiRetrievalMode::Project => RetrievalMode::Project,
        AiRetrievalMode::ExplicitOnly => RetrievalMode::ExplicitOnly,
        AiRetrievalMode::None => RetrievalMode::None,
    };
    let entity_ids =
        retrieval_entity_ids(project, mode, &payload.seed_ids, payload.relationship_depth)?;
    let entity_names = project
        .list_entities()
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|entity| (entity.id, entity.name))
        .collect::<BTreeMap<_, _>>();
    let mut source_ids = RetrievalSourceIds::new();
    for entity_id in &entity_ids {
        for document in project
            .list_documents(entity_id.clone())
            .map_err(|error| error.to_string())?
        {
            source_ids.insert(
                document.id,
                RetrievalSource {
                    entity_id: Some(entity_id.clone()),
                    canonical_path: Some(format!("entities/{entity_id}/document.md")),
                    summary: None,
                },
            );
        }
        for field in project
            .list_fields(entity_id.clone())
            .map_err(|error| error.to_string())?
        {
            source_ids.insert(
                format!("field:{}:{}:{}", entity_id, field.namespace, field.key),
                RetrievalSource {
                    entity_id: Some(entity_id.clone()),
                    canonical_path: Some(format!(
                        "entities/{}/fields/{}-{}.json",
                        entity_id, field.namespace, field.key
                    )),
                    summary: None,
                },
            );
        }
        for relationship in project
            .list_relationships(entity_id.clone())
            .map_err(|error| error.to_string())?
        {
            if !entity_ids.contains(&relationship.source_id)
                || !entity_ids.contains(&relationship.target_id)
            {
                continue;
            }
            let source_name = entity_names
                .get(&relationship.source_id)
                .map(String::as_str)
                .unwrap_or(&relationship.source_id);
            let target_name = entity_names
                .get(&relationship.target_id)
                .map(String::as_str)
                .unwrap_or(&relationship.target_id);
            source_ids.insert(
                format!("relationship:{}", relationship.id),
                RetrievalSource {
                    entity_id: Some(relationship.source_id.clone()),
                    canonical_path: Some(format!("relationships/{}.json", relationship.id)),
                    summary: Some(relationship_summary(
                        project,
                        source_name,
                        target_name,
                        &relationship.relationship_type,
                        &relationship.metadata,
                    )),
                },
            );
        }
    }
    Ok(source_ids)
}

pub(super) async fn semantic_retrieval_passages(
    runtime: SharedAiRuntime,
    provider: ResolvedAiProvider,
    query: String,
    allowed_source_ids: RetrievalSourceIds,
    allowed_source_kinds: Vec<String>,
    limit: usize,
) -> Result<Vec<RetrievedPassage>, String> {
    if query.trim().is_empty() || !provider.embedding_available || allowed_source_ids.is_empty() {
        return Ok(Vec::new());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let runtime = runtime
            .lock()
            .map_err(|_| "AI runtime lock poisoned".to_string())?;
        let Some(index) = runtime.index.as_ref() else {
            return Ok(Vec::new());
        };
        let embedding_model = provider.embedding_model_or_model();
        let embedding_provider = LmStudioEmbeddingProvider {
            endpoint: provider.endpoint,
            model: embedding_model,
            remote: provider.remote,
            api_key: provider.api_key,
        };
        let query_vector = embedding_provider
            .embed(&[query])
            .map_err(|error| error.to_string())?
            .pop()
            .ok_or_else(|| "embedding provider returned no query vector".to_string())?;
        let records = index.records().map_err(|error| error.to_string())?;
        let semantic = daena_ai::index::exact_cosine_search(&records, &query_vector, limit);
        let allowed_kind = |kind: &str| {
            allowed_source_kinds.is_empty()
                || allowed_source_kinds.iter().any(|allowed| allowed == kind)
        };
        Ok(semantic
            .into_iter()
            .enumerate()
            .filter_map(|(rank, matched)| {
                let record = records
                    .iter()
                    .find(|record| record.chunk.id == matched.chunk_id)?;
                let source_metadata = allowed_source_ids.get(&record.chunk.source.source_id)?;
                if !allowed_kind(&record.chunk.source.source_kind) {
                    return None;
                }
                let source = SourceRef {
                    source_kind: record.chunk.source.source_kind.clone(),
                    summary: source_metadata.summary.clone(),
                    entity_id: source_metadata.entity_id.clone(),
                    document_id: (record.chunk.source.source_kind == "document")
                        .then(|| record.chunk.source.source_id.clone()),
                    canonical_path: source_metadata.canonical_path.clone(),
                    revision: record.chunk.source.revision.clone(),
                    content_hash: record.chunk.source.source_hash.clone(),
                    byte_start: Some(record.chunk.byte_start),
                    byte_end: Some(record.chunk.byte_end),
                    excerpt_hash: record.chunk.text_hash.clone(),
                };
                Some(RetrievedPassage {
                    source,
                    text: record.chunk.text.clone(),
                    lexical_rank: rank as u32,
                })
            })
            .collect())
    })
    .await
    .map_err(|error| error.to_string())?
}

pub(super) async fn direct_retrieval_context(
    core: State<'_, crate::SharedCore>,
    runtime: SharedAiRuntime,
    provider: ResolvedAiProvider,
    entity_id: Option<String>,
    query: Option<String>,
    relationship_depth: Option<u8>,
) -> Result<(String, Vec<SourceRef>), String> {
    let entity_id = entity_id.filter(|id| !id.trim().is_empty());
    let has_query = query
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty());
    if entity_id.is_none() && !has_query {
        return Ok((String::new(), Vec::new()));
    }
    let related = entity_id.is_some();
    let caller = AiCaller::trusted_shell("trusted-shell", "pending");
    let policy = AiRetrievalPolicyPayload {
        mode: if related {
            AiRetrievalMode::Related
        } else {
            AiRetrievalMode::Project
        },
        query,
        seed_ids: entity_id.into_iter().collect(),
        allowed_source_kinds: Vec::new(),
        relationship_depth: if related {
            relationship_depth.unwrap_or(1).min(2)
        } else {
            0
        },
        passage_count: 8,
        include_shared_fields: true,
    };
    let query_for_semantic = policy.query.clone();
    let policy_for_sources = policy.clone();
    let allowed_source_ids = crate::with_read_project(core.clone(), move |project| {
        retrieval_source_ids(project, &policy_for_sources).map_err(CoreError::Conflict)
    })
    .await?;
    let semantic = if let Some(query) = query_for_semantic {
        semantic_retrieval_passages(runtime, provider, query, allowed_source_ids, Vec::new(), 4)
            .await
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    crate::with_read_project(core, move |project| {
        build_retrieval_context_with_semantic(project, &caller, &policy, &semantic)
            .map_err(CoreError::Conflict)
    })
    .await
}

pub(super) fn append_retrieved_context(selection: String, retrieved_context: String) -> String {
    if retrieved_context.is_empty() {
        selection
    } else {
        format!("{selection}\n\n[RETRIEVED_CONTEXT]\n{retrieved_context}\n[/RETRIEVED_CONTEXT]")
    }
}

pub(crate) fn ensure_active_project(
    core: &crate::SharedCore,
    project_id: &str,
) -> Result<(), String> {
    let active = crate::current_info(core)?.ok_or_else(|| "No project is open".to_string())?;
    if active.root != project_id {
        return Err("AI request project does not match the open project".to_string());
    }
    Ok(())
}
