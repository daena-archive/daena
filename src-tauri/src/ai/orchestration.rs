// AI request orchestration.
use super::*;

struct RequestCleanup {
    runtime: SharedAiRuntime,
    request_id: String,
}

impl Drop for RequestCleanup {
    fn drop(&mut self) {
        if let Ok(mut runtime) = self.runtime.lock() {
            runtime.cancellations.remove(&self.request_id);
        }
    }
}

struct ProviderFinished(Arc<std::sync::atomic::AtomicBool>);

impl Drop for ProviderFinished {
    fn drop(&mut self) {
        self.0.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderStatus {
    pub endpoint: String,
    pub model: String,
    pub available: bool,
    pub model_available: bool,
    pub embedding_available: bool,
    pub credential_available: bool,
    pub error: Option<String>,
}

pub(super) fn record_event(runtime: &SharedAiRuntime, event: &mut AiStreamEvent) -> bool {
    if let Ok(mut runtime) = runtime.lock() {
        let queue = runtime.events.entry(event.request_id.clone()).or_default();
        if queue
            .iter()
            .any(|recorded| is_terminal_phase(&recorded.phase))
        {
            return false;
        }
        event.sequence = queue.back().map_or(0, |last| last.sequence + 1);
        if queue.len() >= MAX_BUFFERED_EVENTS {
            queue.pop_front();
        }
        queue.push_back(event.clone());
        return true;
    }
    false
}

pub(super) fn emit_ai_event(
    app: Option<&AppHandle>,
    event_name: &str,
    runtime: &SharedAiRuntime,
    mut event: AiStreamEvent,
) {
    if !record_event(runtime, &mut event) {
        return;
    }
    if let Some(app) = app {
        let _ = app.emit(event_name, event);
    }
}

pub(super) fn register_request(
    runtime: &SharedAiRuntime,
    request_id: &str,
    cancellation: Arc<std::sync::atomic::AtomicBool>,
) -> Result<(), AiError> {
    let mut runtime = runtime.lock().map_err(|_| AiError::QueueFull)?;
    if runtime.cancellations.len() >= DEFAULT_LIMITS.max_concurrent_requests {
        return Err(AiError::QueueFull);
    }
    if runtime.events.len() >= MAX_BUFFERED_REQUESTS {
        if let Some(oldest) = runtime.request_order.pop_front() {
            runtime.events.remove(&oldest);
            runtime.citations.remove(&oldest);
        }
    }
    runtime.request_order.push_back(request_id.to_string());
    runtime
        .events
        .insert(request_id.to_string(), VecDeque::new());
    runtime
        .cancellations
        .insert(request_id.to_string(), cancellation);
    Ok(())
}

pub(super) async fn provider_status(provider: ResolvedAiProvider) -> AiProviderStatus {
    let endpoint = provider.endpoint.clone();
    let model = provider.model.clone();
    let mut status = AiProviderStatus {
        endpoint: endpoint.clone(),
        model: model.clone(),
        available: false,
        model_available: false,
        embedding_available: provider.embedding_available,
        credential_available: !provider.remote || provider.api_key.is_some(),
        error: None,
    };
    let result = tauri::async_runtime::spawn_blocking(move || {
        if provider.remote {
            remote_http_request(
                &endpoint,
                provider.api_key.as_deref().unwrap_or_default(),
                "GET",
                "models",
                None,
                DEFAULT_LIMITS.default_deadline,
            )
        } else {
            let stream = connect_request(
                &endpoint,
                "GET",
                "models",
                None,
                DEFAULT_LIMITS.default_deadline,
            )?;
            read_response(stream)
        }
    })
    .await;
    let response = match result {
        Ok(Ok(response)) => response,
        Ok(Err(_error)) => {
            status.error = Some(AiError::ProviderUnavailable.to_string());
            return status;
        }
        Err(_error) => {
            status.error = Some(AiError::ProviderUnavailable.to_string());
            return status;
        }
    };
    if response.0 / 100 != 2 {
        if provider.remote && matches!(response.0, 401 | 403) {
            status.available = true;
            status.credential_available = false;
        }
        status.error = Some(normalized_http_error(response.0).to_string());
        return status;
    }
    status.available = true;
    status.credential_available = true;
    match serde_json::from_slice::<OpenAiModels>(&response.1) {
        Ok(models) => status.model_available = models.data.iter().any(|item| item.id == model),
        Err(_error) => status.error = Some(AiError::InvalidProviderResponse.to_string()),
    }
    status
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderConnectResult {
    pub status: AiProviderStatus,
    pub models: Vec<String>,
}

#[tauri::command]
pub async fn ai_provider_status(
    settings: State<'_, Arc<Mutex<SettingsStore>>>,
    project_id: String,
) -> Result<AiProviderStatus, String> {
    let configured = settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .load()?;
    let provider =
        resolve_ai_provider_with_credential(&configured, Some(&project_id), false, false)?;
    Ok(provider_status(provider).await)
}

#[tauri::command]
pub async fn ai_provider_models(
    settings: State<'_, Arc<Mutex<SettingsStore>>>,
    project_id: String,
) -> Result<Vec<String>, String> {
    let configured = settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .load()?;
    let provider =
        resolve_ai_provider_with_credential(&configured, Some(&project_id), false, false)?;
    let result = tauri::async_runtime::spawn_blocking(move || {
        if provider.remote {
            remote_http_request(
                &provider.endpoint,
                provider.api_key.as_deref().unwrap_or_default(),
                "GET",
                "models",
                None,
                DEFAULT_LIMITS.default_deadline,
            )
        } else {
            let stream = connect_request(
                &provider.endpoint,
                "GET",
                "models",
                None,
                DEFAULT_LIMITS.default_deadline,
            )?;
            read_response(stream)
        }
    })
    .await
    .map_err(|error| error.to_string())??;
    if result.0 / 100 != 2 {
        return Err(normalized_http_error(result.0).to_string());
    }
    let mut models = serde_json::from_slice::<OpenAiModels>(&result.1)
        .map_err(|_| AiError::InvalidProviderResponse.to_string())?
        .data
        .into_iter()
        .map(|model| model.id)
        .collect::<Vec<_>>();
    models.sort();
    models.dedup();
    Ok(models)
}

#[tauri::command]
pub async fn ai_provider_connect(
    settings: State<'_, Arc<Mutex<SettingsStore>>>,
    project_id: String,
) -> Result<AiProviderConnectResult, String> {
    let models = ai_provider_models(settings.clone(), project_id.clone()).await;
    let status = ai_provider_status(settings, project_id).await?;
    Ok(AiProviderConnectResult {
        status,
        models: models.unwrap_or_default(),
    })
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn ai_generate_text(
    app: AppHandle,
    core: State<'_, crate::SharedCore>,
    runtime: State<'_, SharedAiRuntime>,
    settings: State<'_, Arc<Mutex<SettingsStore>>>,
    project_id: String,
    instruction: String,
    selection: String,
    entity_id: Option<String>,
    retrieval_query: Option<String>,
    retrieval_depth: Option<u8>,
    include_retrieval: bool,
) -> Result<String, String> {
    ensure_active_project(core.inner(), &project_id)?;
    let configured = settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .load()?;
    crate::ensure_project_ai_enabled(&project_id)?;
    let provider = resolve_ai_provider(&configured, Some(&project_id), include_retrieval)?;
    let (retrieved_context, citations) = if include_retrieval {
        direct_retrieval_context(
            core,
            runtime.inner().clone(),
            provider.clone(),
            entity_id,
            retrieval_query,
            retrieval_depth,
        )
        .await?
    } else {
        (String::new(), Vec::new())
    };
    start_ai_request_mode(
        Some(app),
        runtime.inner().clone(),
        daena_ai::AiCaller::trusted_shell("trusted-shell", "pending"),
        provider.endpoint,
        provider.model,
        instruction,
        append_retrieved_context(selection, retrieved_context),
        None,
        DEFAULT_GENERATION_DEADLINE,
        citations,
        provider.remote,
        provider.api_key,
    )
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn ai_generate_structured(
    app: AppHandle,
    core: State<'_, crate::SharedCore>,
    runtime: State<'_, SharedAiRuntime>,
    settings: State<'_, Arc<Mutex<SettingsStore>>>,
    project_id: String,
    instruction: String,
    context: String,
    output_contract: serde_json::Value,
    entity_id: Option<String>,
    retrieval_query: Option<String>,
    retrieval_depth: Option<u8>,
    include_retrieval: bool,
) -> Result<String, String> {
    ensure_active_project(core.inner(), &project_id)?;
    let configured = settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .load()?;
    crate::ensure_project_ai_enabled(&project_id)?;
    let provider = resolve_ai_provider(&configured, Some(&project_id), include_retrieval)?;
    let (retrieved_context, citations) = if include_retrieval {
        direct_retrieval_context(
            core,
            runtime.inner().clone(),
            provider.clone(),
            entity_id,
            retrieval_query,
            retrieval_depth,
        )
        .await?
    } else {
        (String::new(), Vec::new())
    };
    validate_structured_schema(&output_contract)?;
    start_ai_request_mode(
        Some(app),
        runtime.inner().clone(),
        daena_ai::AiCaller::trusted_shell("trusted-shell", "pending"),
        provider.endpoint,
        provider.model,
        instruction,
        append_retrieved_context(context, retrieved_context),
        Some(output_contract),
        DEFAULT_GENERATION_DEADLINE,
        citations,
        provider.remote,
        provider.api_key,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn start_ai_request_mode(
    app: Option<AppHandle>,
    runtime: SharedAiRuntime,
    mut caller: daena_ai::AiCaller,
    endpoint: String,
    model: String,
    instruction: String,
    selection: String,
    output_contract: Option<serde_json::Value>,
    deadline: Duration,
    citations: Vec<SourceRef>,
    remote: bool,
    api_key: Option<String>,
) -> Result<String, String> {
    if instruction.trim().is_empty() {
        return Err("An AI instruction is required".to_string());
    }
    let provider = runtime
        .lock()
        .ok()
        .and_then(|runtime| runtime.provider.clone());
    if model.trim().is_empty() {
        if remote {
            return Err("A remote AI model ID is required".to_string());
        }
        if provider.is_none() {
            return Err("A loaded local AI provider model is required".to_string());
        }
    }
    if instruction.len() + selection.len() > DEFAULT_LIMITS.max_input_bytes {
        return Err(AiError::ContextTooLarge.to_string());
    }
    if provider.is_none() && !remote {
        parse_loopback_endpoint(&endpoint)?;
    }
    if remote {
        validate_remote_endpoint(&endpoint)?;
        if api_key.as_deref().is_none_or(str::is_empty) {
            return Err(AiError::AuthenticationFailed.to_string());
        }
    }
    let request_id = Uuid::new_v4().to_string();
    caller.request_id.clone_from(&request_id);
    let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
    register_request(&runtime, &request_id, cancelled.clone())
        .map_err(|error| error.to_string())?;
    runtime
        .lock()
        .map_err(|_| "AI runtime lock poisoned".to_string())?
        .citations
        .insert(request_id.clone(), citations);
    let event_name = format!("ai-stream:{request_id}");
    let request_id_for_task = request_id.clone();
    if remote {
        let runtime_for_task = runtime.clone();
        let cancelled_for_task = cancelled.clone();
        let api_key = api_key.unwrap_or_default();
        tauri::async_runtime::spawn(async move {
            let _cleanup = RequestCleanup {
                runtime: runtime_for_task,
                request_id: request_id_for_task.clone(),
            };
            emit_ai_event(
                app.as_ref(),
                &event_name,
                &_cleanup.runtime,
                ai_event(&request_id_for_task, "started", None),
            );
            generate_remote_stream(
                app.as_ref(),
                &_cleanup.runtime,
                &event_name,
                endpoint,
                api_key,
                model,
                instruction,
                selection,
                output_contract,
                &request_id_for_task,
                &cancelled_for_task,
                deadline,
            )
            .await;
        });
        return Ok(request_id);
    }
    tauri::async_runtime::spawn_blocking(move || {
        let _cleanup = RequestCleanup {
            runtime,
            request_id: request_id_for_task.clone(),
        };
        let event_runtime = _cleanup.runtime.clone();
        let emit =
            |event: AiStreamEvent| emit_ai_event(app.as_ref(), &event_name, &event_runtime, event);
        emit(AiStreamEvent {
            sequence: 0,
            request_id: request_id_for_task.clone(),
            phase: "started".into(),
            delta: None,
            output: None,
            error: None,
        });
        let request_started = Instant::now();
        let deadline_exceeded = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let provider_finished = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let _provider_finished = ProviderFinished(provider_finished.clone());
        let deadline_flag = deadline_exceeded.clone();
        let finished_flag = provider_finished.clone();
        let deadline_cancel = cancelled.clone();
        std::thread::spawn(move || {
            std::thread::sleep(deadline);
            if !finished_flag.load(std::sync::atomic::Ordering::Relaxed) {
                deadline_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                deadline_cancel.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        });
        if let Some(provider) = provider {
            let events = provider.generate(
                ProviderRequest {
                    request_id: request_id_for_task.clone(),
                    caller,
                    model,
                    instruction,
                    context: selection,
                    output_contract,
                    deadline,
                },
                cancelled.clone(),
            );
            provider_finished.store(true, std::sync::atomic::Ordering::Relaxed);
            if deadline_exceeded.load(std::sync::atomic::Ordering::Relaxed)
                || request_started.elapsed() >= deadline
            {
                emit(remote_terminal_event(&request_id_for_task, true));
                return;
            }
            if cancelled.load(std::sync::atomic::Ordering::Relaxed) {
                emit(remote_terminal_event(&request_id_for_task, false));
                return;
            }
            let mut streamed_bytes = 0usize;
            let mut terminal_seen = false;
            for event in events {
                if event.phase == "started" {
                    continue;
                }
                if let Some(delta) = event.delta.as_deref() {
                    streamed_bytes = streamed_bytes.saturating_add(delta.len());
                }
                if let Some(output) = event.output.as_deref() {
                    streamed_bytes = streamed_bytes.max(output.len());
                }
                if streamed_bytes > DEFAULT_LIMITS.max_output_bytes {
                    emit(ai_event(
                        &request_id_for_task,
                        "failed",
                        Some(AiError::OutputValidationFailed),
                    ));
                    return;
                }
                terminal_seen = is_terminal_phase(&event.phase);
                emit(event);
                if terminal_seen {
                    break;
                }
            }
            if !terminal_seen {
                emit(ai_event(
                    &request_id_for_task,
                    "failed",
                    Some(AiError::InvalidProviderResponse),
                ));
            }
            return;
        }
        let (system_prompt, user_prompt) =
            build_generation_prompt(&instruction, &selection, output_contract.as_ref());
        let body = serde_json::json!({
            "model": model,
            "messages": [
                { "role": "system", "content": system_prompt },
                { "role": "user", "content": user_prompt }
            ],
            "stream": true
        })
        .to_string();
        let mut stream =
            match connect_request(&endpoint, "POST", "chat/completions", Some(&body), deadline) {
                Ok(stream) => stream,
                Err(error) => {
                    emit(AiStreamEvent {
                        sequence: 0,
                        request_id: request_id_for_task.clone(),
                        phase: "failed".into(),
                        delta: None,
                        output: None,
                        error: Some(if error.contains("invalid") {
                            AiError::InvalidProviderResponse.to_string()
                        } else {
                            AiError::ProviderUnavailable.to_string()
                        }),
                    });
                    return;
                }
            };
        if stream
            .set_read_timeout(Some(AI_CANCEL_POLL_INTERVAL.min(deadline)))
            .is_err()
        {
            emit(ai_event(
                &request_id_for_task,
                "failed",
                Some(AiError::ProviderUnavailable),
            ));
            return;
        }
        let (status, mut bytes) = match read_http_headers(
            &mut stream,
            &cancelled,
            &deadline_exceeded,
            request_started,
            deadline,
        ) {
            Ok(response) => response,
            Err(AiError::DeadlineExceeded) => {
                emit(remote_terminal_event(&request_id_for_task, true));
                return;
            }
            Err(AiError::Cancelled) => {
                emit(remote_terminal_event(&request_id_for_task, false));
                return;
            }
            Err(error) => {
                emit(ai_event(&request_id_for_task, "failed", Some(error)));
                return;
            }
        };
        if status / 100 != 2 {
            let error = normalized_http_error(status);
            if error == AiError::DeadlineExceeded {
                emit(remote_terminal_event(&request_id_for_task, true));
            } else {
                emit(ai_event(&request_id_for_task, "failed", Some(error)));
            }
            return;
        }
        let mut output = String::new();
        let mut terminal_emitted = false;
        let mut finish_reason_seen = false;
        let mut reasoning_seen = false;
        loop {
            let deadline_hit = deadline_exceeded.load(std::sync::atomic::Ordering::Relaxed)
                || request_started.elapsed() >= deadline;
            if deadline_hit || cancelled.load(std::sync::atomic::Ordering::Relaxed) {
                emit(terminal_event_with_partial_output(
                    &request_id_for_task,
                    deadline_hit,
                    &output,
                ));
                terminal_emitted = true;
                break;
            }
            let mut chunk = [0; 8192];
            let read = match stream.read(&mut chunk) {
                Ok(0) => {
                    if !bytes.is_empty() {
                        bytes.push(b'\n');
                        match drain_remote_sse_lines(
                            &mut bytes,
                            &request_id_for_task,
                            &mut output,
                            &mut finish_reason_seen,
                            &mut reasoning_seen,
                        ) {
                            Ok((events, done)) => {
                                for event in events {
                                    emit(event);
                                }
                                if done {
                                    emit(completed_event(&request_id_for_task, output));
                                    return;
                                }
                            }
                            Err(error) => {
                                emit(ai_event(&request_id_for_task, "failed", Some(error)));
                                return;
                            }
                        }
                    }
                    break;
                }
                Ok(read) => read,
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                    ) =>
                {
                    continue;
                }
                Err(_) => {
                    emit(AiStreamEvent {
                        sequence: 0,
                        request_id: request_id_for_task.clone(),
                        phase: "failed".into(),
                        delta: None,
                        output: None,
                        error: Some(AiError::ProviderUnavailable.to_string()),
                    });
                    terminal_emitted = true;
                    break;
                }
            };
            bytes.extend_from_slice(&chunk[..read]);
            if bytes.len() > MAX_PROVIDER_FRAME_BYTES {
                emit(ai_event(
                    &request_id_for_task,
                    "failed",
                    Some(AiError::InvalidProviderResponse),
                ));
                return;
            }
            while let Some(newline) = bytes.iter().position(|byte| *byte == b'\n') {
                let line = bytes.drain(..=newline).collect::<Vec<_>>();
                let line = String::from_utf8_lossy(&line);
                let line = line.trim();
                if line.is_empty() || line.starts_with("HTTP/") {
                    continue;
                }
                let Some(line) = line.strip_prefix("data:").map(str::trim) else {
                    continue;
                };
                if line == "[DONE]" {
                    emit(AiStreamEvent {
                        sequence: 0,
                        request_id: request_id_for_task.clone(),
                        phase: "completed".into(),
                        delta: None,
                        output: Some(output.clone()),
                        error: None,
                    });
                    return;
                }
                let parsed = match serde_json::from_str::<OpenAiStreamChunk>(line) {
                    Ok(parsed) => parsed,
                    Err(error) => {
                        let _ = error;
                        emit(AiStreamEvent {
                            sequence: 0,
                            request_id: request_id_for_task.clone(),
                            phase: "failed".into(),
                            delta: None,
                            output: None,
                            error: Some(AiError::InvalidProviderResponse.to_string()),
                        });
                        return;
                    }
                };
                if parsed.error.is_some() {
                    emit(AiStreamEvent {
                        sequence: 0,
                        request_id: request_id_for_task.clone(),
                        phase: "failed".into(),
                        delta: None,
                        output: None,
                        error: Some(AiError::InvalidProviderResponse.to_string()),
                    });
                    return;
                }
                if let Some(usage) = parsed.usage {
                    emit(usage_event(&request_id_for_task, usage));
                }
                for choice in parsed.choices {
                    let mut events = Vec::new();
                    if let Err(error) = append_openai_choice_events(
                        choice,
                        &request_id_for_task,
                        &mut output,
                        &mut finish_reason_seen,
                        &mut reasoning_seen,
                        &mut events,
                    ) {
                        emit(ai_event(&request_id_for_task, "failed", Some(error)));
                        return;
                    }
                    for event in events {
                        emit(event);
                    }
                }
            }
        }
        if !terminal_emitted {
            if finish_reason_seen {
                emit(completed_event(&request_id_for_task, output));
            } else {
                emit(ai_event(
                    &request_id_for_task,
                    "failed",
                    Some(AiError::InvalidProviderResponse),
                ));
            }
        }
    });
    Ok(request_id)
}

#[tauri::command]
pub fn ai_cancel_text(
    runtime: State<'_, SharedAiRuntime>,
    request_id: String,
) -> Result<(), String> {
    let runtime = runtime
        .lock()
        .map_err(|_| "AI runtime lock poisoned".to_string())?;
    if let Some(cancelled) = runtime.cancellations.get(&request_id) {
        cancelled.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    Ok(())
}

#[tauri::command]
pub fn ai_poll_text(
    runtime: State<'_, SharedAiRuntime>,
    request_id: String,
) -> Result<Vec<AiStreamEvent>, String> {
    poll_ai_events(runtime.inner(), &request_id)
}

pub fn poll_ai_events(
    runtime: &SharedAiRuntime,
    request_id: &str,
) -> Result<Vec<AiStreamEvent>, String> {
    let runtime = runtime
        .lock()
        .map_err(|_| "AI runtime lock poisoned".to_string())?;
    Ok(runtime
        .events
        .get(request_id)
        .cloned()
        .map(|events| events.into_iter().collect())
        .unwrap_or_default())
}

pub fn cancel_ai_request(runtime: &SharedAiRuntime, request_id: &str) -> Result<(), String> {
    let runtime = runtime
        .lock()
        .map_err(|_| "AI runtime lock poisoned".to_string())?;
    if let Some(cancelled) = runtime.cancellations.get(request_id) {
        cancelled.store(true, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    } else {
        Err("AI request does not exist".into())
    }
}

pub fn ai_request_result(runtime: &SharedAiRuntime, request_id: &str) -> Result<String, String> {
    let events = poll_ai_events(runtime, request_id)?;
    let terminal = events.iter().rev().find(|event| {
        matches!(
            event.phase.as_str(),
            "completed" | "cancelled" | "failed" | "deadline_exceeded"
        )
    });
    match terminal {
        Some(event) if event.phase == "completed" => event
            .output
            .clone()
            .ok_or_else(|| "AI result is empty".into()),
        Some(event) if event.phase == "cancelled" => Err("AI request was cancelled".into()),
        Some(event) => Err(event
            .error
            .clone()
            .unwrap_or_else(|| "AI request failed".into())),
        None => Err("AI request is still running".into()),
    }
}

pub fn ai_request_citations(
    runtime: &SharedAiRuntime,
    request_id: &str,
) -> Result<Vec<SourceRef>, String> {
    let runtime = runtime
        .lock()
        .map_err(|_| "AI runtime lock poisoned".to_string())?;
    Ok(runtime
        .citations
        .get(request_id)
        .cloned()
        .unwrap_or_default())
}

pub fn remove_ai_citations(runtime: &SharedAiRuntime, request_id: &str) -> Result<(), String> {
    runtime
        .lock()
        .map_err(|_| "AI runtime lock poisoned".to_string())?
        .citations
        .remove(request_id);
    Ok(())
}

pub fn validate_structured_output(
    schema: &serde_json::Value,
    value: &serde_json::Value,
) -> Result<(), String> {
    validate_schema_node(schema, value, 0)
}

pub fn validate_structured_schema(schema: &serde_json::Value) -> Result<(), String> {
    if serde_json::to_vec(schema)
        .map_err(|_| "structured output contract is invalid".to_string())?
        .len()
        > 16 * 1024
    {
        return Err("structured output contract is too large".into());
    }
    validate_schema_shape(schema, 0)
}

pub(super) fn validate_schema_shape(
    schema: &serde_json::Value,
    depth: usize,
) -> Result<(), String> {
    if depth > 8 {
        return Err("structured schema exceeds maximum depth".into());
    }
    let kind = schema
        .get("type")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "structured schema requires type".to_string())?;
    if !matches!(
        kind,
        "object" | "array" | "string" | "boolean" | "number" | "integer"
    ) {
        return Err("structured schema uses an unsupported type".into());
    }
    if let Some(max) = schema.get("maxLength").and_then(serde_json::Value::as_u64) {
        if max > 4000 {
            return Err("structured schema string limit is too large".into());
        }
    }
    if let Some(max) = schema.get("maxItems").and_then(serde_json::Value::as_u64) {
        if max > 64 {
            return Err("structured schema array limit is too large".into());
        }
    }
    if kind == "object" {
        let properties = schema
            .get("properties")
            .and_then(serde_json::Value::as_object)
            .ok_or_else(|| "structured object schema requires properties".to_string())?;
        if schema
            .get("additionalProperties")
            .and_then(serde_json::Value::as_bool)
            != Some(false)
        {
            return Err("structured object schema must reject unknown fields".into());
        }
        if let Some(required) = schema.get("required").and_then(serde_json::Value::as_array) {
            if required.iter().any(|item| {
                item.as_str()
                    .is_none_or(|name| !properties.contains_key(name))
            }) {
                return Err("structured schema required field is not declared".into());
            }
        }
        for child in properties.values() {
            validate_schema_shape(child, depth + 1)?;
        }
    }
    if kind == "array" {
        validate_schema_shape(
            schema
                .get("items")
                .ok_or_else(|| "structured array schema requires items".to_string())?,
            depth + 1,
        )?;
    }
    Ok(())
}

pub(super) fn validate_schema_node(
    schema: &serde_json::Value,
    value: &serde_json::Value,
    depth: usize,
) -> Result<(), String> {
    if depth > 8 {
        return Err("structured schema exceeds maximum depth".into());
    }
    let kind = schema
        .get("type")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "structured schema requires type".to_string())?;
    let valid = match kind {
        "object" => {
            let object = value
                .as_object()
                .ok_or_else(|| "structured output must be an object".to_string())?;
            let properties = schema
                .get("properties")
                .and_then(serde_json::Value::as_object)
                .cloned()
                .unwrap_or_default();
            for required in schema
                .get("required")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
            {
                if !object.contains_key(required) {
                    return Err(format!("structured output is missing {required}"));
                }
            }
            if schema
                .get("additionalProperties")
                .and_then(serde_json::Value::as_bool)
                == Some(false)
            {
                if let Some(key) = object.keys().find(|key| !properties.contains_key(*key)) {
                    return Err(format!("structured output contains unknown field {key}"));
                }
            }
            for (key, child) in properties {
                if let Some(actual) = object.get(&key) {
                    validate_schema_node(&child, actual, depth + 1)?;
                }
            }
            true
        }
        "array" => {
            let array = value
                .as_array()
                .ok_or_else(|| "structured output must be an array".to_string())?;
            if let Some(max) = schema.get("maxItems").and_then(serde_json::Value::as_u64) {
                if array.len() as u64 > max {
                    return Err("structured output array is too large".into());
                }
            }
            if let Some(items) = schema.get("items") {
                for item in array {
                    validate_schema_node(items, item, depth + 1)?;
                }
            }
            true
        }
        "string" => value.is_string(),
        "boolean" => value.is_boolean(),
        "number" | "integer" => value.is_number(),
        _ => return Err("structured schema uses an unsupported type".into()),
    };
    if !valid {
        return Err(format!("structured output does not match type {kind}"));
    }
    if kind == "string" {
        if let Some(max) = schema.get("maxLength").and_then(serde_json::Value::as_u64) {
            if value.as_str().is_some_and(|text| text.len() as u64 > max) {
                return Err("structured output string is too long".into());
            }
        }
    }
    Ok(())
}
