use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use daena_core::{Asset, AssetFileInput, CoreError};
use reqwest::blocking::{Client, Response};
use reqwest::{StatusCode, Url};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::settings::ImageProviderSettings;
use crate::{SharedCore, SharedSettings};

const MAX_PROMPT_BYTES: usize = 16 * 1024;
const MAX_CONTEXT_ITEMS: usize = 64;
const MAX_OUTPUTS: u32 = 4;
const MAX_DIMENSION: u32 = 4096;
const MAX_PIXELS: u64 = 16_777_216;
const MAX_IMAGE_BYTES: usize = 32 * 1024 * 1024;
const MAX_TOTAL_IMAGE_BYTES: usize = 96 * 1024 * 1024;
const MAX_JSON_BYTES: usize = 4 * 1024 * 1024;
const MAX_ACTIVE_JOBS: usize = 2;
const MAX_RETAINED_JOBS: usize = 4;
const JOB_TTL: Duration = Duration::from_mins(15);
const GENERATION_DEADLINE: Duration = Duration::from_mins(15);
const POLL_INTERVAL: Duration = Duration::from_millis(500);

pub type SharedImageGeneration = Arc<Mutex<ImageGenerationManager>>;

#[derive(Default)]
pub struct ImageGenerationManager {
    jobs: BTreeMap<String, ImageGenerationJob>,
}

impl ImageGenerationManager {
    fn reap_expired(&mut self) {
        let now = Instant::now();
        self.jobs.retain(|_, job| {
            if job.expires_at <= now {
                job.cancel.store(true, Ordering::Relaxed);
                false
            } else {
                true
            }
        });
    }

    pub fn cancel_all(&mut self) {
        for job in self.jobs.values() {
            job.cancel.store(true, Ordering::Relaxed);
        }
        self.jobs.clear();
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageProviderDiscovery {
    provider_id: String,
    provider_name: String,
    endpoint: String,
    local: bool,
    capabilities: Vec<String>,
    models: Vec<String>,
    samplers: Vec<String>,
    schedulers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageProviderStatus {
    provider_id: String,
    provider_name: String,
    endpoint: String,
    model: String,
    enabled: bool,
    local: bool,
    available: bool,
    model_available: bool,
    capabilities: Vec<String>,
    error_code: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageContextItem {
    entity_id: String,
    label: String,
    source_kind: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImagePromptProvenance {
    method: String,
    llm_assisted: bool,
    edited_after_assistance: bool,
    text_provider_id: Option<String>,
    text_model: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageGenerationRequest {
    project_id: String,
    entity_id: String,
    prompt: String,
    #[serde(default)]
    negative_prompt: String,
    model: String,
    width: u32,
    height: u32,
    seed: u64,
    output_count: u32,
    steps: u32,
    guidance_scale: f64,
    sampler: String,
    scheduler: String,
    context: Vec<ImageContextItem>,
    prompt_provenance: ImagePromptProvenance,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageCandidate {
    id: String,
    filename: String,
    mime_type: String,
    size: usize,
    width: u32,
    height: u32,
    seed: u64,
    accepted_asset_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageGenerationStatus {
    job_id: String,
    state: String,
    stage: String,
    completed: u32,
    total: u32,
    queue_position: Option<u32>,
    candidates: Vec<ImageCandidate>,
    error_code: Option<String>,
    error: Option<String>,
}

struct CandidateData {
    metadata: ImageCandidate,
    bytes: Option<Vec<u8>>,
    accepted: Option<Asset>,
}

struct ImageGenerationJob {
    project_id: String,
    entity_id: String,
    expires_at: Instant,
    cancel: Arc<AtomicBool>,
    status: ImageGenerationStatus,
    candidates: BTreeMap<String, CandidateData>,
    request: ImageGenerationRequest,
    provider: ImageProviderSettings,
    created_at: String,
}

#[derive(Debug)]
struct ImageError {
    code: &'static str,
    message: String,
}

impl ImageError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    fn provider_transport(error: &reqwest::Error) -> Self {
        if error.is_timeout() {
            Self::new(
                "connection_failed",
                "ComfyUI did not respond before the timeout",
            )
        } else if error.is_connect() {
            Self::new(
                "provider_unavailable",
                "ComfyUI is not reachable at the configured local endpoint",
            )
        } else {
            Self::new("connection_failed", "The ComfyUI connection failed")
        }
    }
}

fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .to_string()
}

fn capabilities() -> Vec<String> {
    [
        "text-to-image",
        "negative-prompts",
        "seeds",
        "model-selection",
        "multiple-outputs",
        "generation-status",
        "custom-generation-parameters",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn has_disallowed_control(value: &str) -> bool {
    value
        .chars()
        .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
}

fn validate_local_endpoint(endpoint: &str) -> Result<Url, ImageError> {
    if endpoint.trim().len() > 2_048 {
        return Err(ImageError::new(
            "invalid_configuration",
            "The ComfyUI endpoint is too long",
        ));
    }
    let mut url = Url::parse(endpoint.trim()).map_err(|_| {
        ImageError::new(
            "invalid_configuration",
            "The ComfyUI endpoint is not a valid URL",
        )
    })?;
    if url.scheme() != "http"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(ImageError::new(
            "invalid_configuration",
            "ComfyUI must use a loopback HTTP endpoint without credentials, a query, or a fragment",
        ));
    }
    let host = url.host_str().ok_or_else(|| {
        ImageError::new("invalid_configuration", "The ComfyUI endpoint has no host")
    })?;
    let loopback = host.eq_ignore_ascii_case("localhost")
        || host.eq_ignore_ascii_case("localhost.localdomain")
        || host
            .trim_matches(['[', ']'])
            .parse::<IpAddr>()
            .is_ok_and(|ip| ip.is_loopback());
    if !loopback {
        return Err(ImageError::new(
            "invalid_configuration",
            "The initial ComfyUI provider only permits a loopback endpoint",
        ));
    }
    let normalized_path = url.path().trim_end_matches('/').to_string();
    url.set_path(&normalized_path);
    Ok(url)
}

fn endpoint_url(base: &Url, route: &str) -> Result<Url, ImageError> {
    let mut url = base.clone();
    let base_path = base.path().trim_end_matches('/');
    url.set_path(&format!("{base_path}/{}", route.trim_start_matches('/')));
    Ok(url)
}

fn provider_client(timeout: Duration) -> Result<Client, ImageError> {
    Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(4))
        .timeout(timeout)
        .build()
        .map_err(|_| ImageError::new("connection_failed", "Could not create the ComfyUI client"))
}

fn read_bounded_response(mut response: Response, limit: usize) -> Result<Vec<u8>, ImageError> {
    if !response.status().is_success() {
        let code = match response.status() {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => "authentication_failed",
            StatusCode::NOT_FOUND => "unsupported_capability",
            StatusCode::UNPROCESSABLE_ENTITY | StatusCode::BAD_REQUEST => "invalid_configuration",
            _ if response.status().is_server_error() => "provider_error",
            _ => "connection_failed",
        };
        return Err(ImageError::new(
            code,
            format!("ComfyUI returned HTTP {}", response.status().as_u16()),
        ));
    }
    if response
        .content_length()
        .is_some_and(|content_length| content_length > limit as u64)
    {
        return Err(ImageError::new(
            "provider_error",
            "ComfyUI returned more data than Daena permits",
        ));
    }
    let mut bytes = Vec::new();
    response
        .by_ref()
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ImageError::new("connection_failed", "Could not read the ComfyUI response"))?;
    if bytes.len() > limit {
        return Err(ImageError::new(
            "provider_error",
            "ComfyUI returned more data than Daena permits",
        ));
    }
    Ok(bytes)
}

fn get_json(client: &Client, url: Url) -> Result<Value, ImageError> {
    let response = client
        .get(url)
        .send()
        .map_err(|error| ImageError::provider_transport(&error))?;
    let bytes = read_bounded_response(response, MAX_JSON_BYTES)?;
    serde_json::from_slice(&bytes)
        .map_err(|_| ImageError::new("provider_error", "ComfyUI returned malformed JSON"))
}

fn choice_list(value: &Value, node: &str, input: &str) -> Vec<String> {
    value
        .get(node)
        .and_then(|value| value.pointer(&format!("/input/required/{input}/0")))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn discover_provider(
    provider: &ImageProviderSettings,
) -> Result<ImageProviderDiscovery, ImageError> {
    if provider.adapter != "comfyui" {
        return Err(ImageError::new(
            "unsupported_capability",
            "The configured image provider adapter is not supported",
        ));
    }
    if provider.id.trim().is_empty()
        || provider.id.len() > 256
        || has_disallowed_control(&provider.id)
        || provider.name.trim().is_empty()
        || provider.name.len() > 256
        || has_disallowed_control(&provider.name)
    {
        return Err(ImageError::new(
            "invalid_configuration",
            "The configured image provider identity is invalid",
        ));
    }
    let endpoint = validate_local_endpoint(&provider.endpoint)?;
    let client = provider_client(Duration::from_secs(12))?;
    get_json(&client, endpoint_url(&endpoint, "system_stats")?)?;

    let mut object_info = serde_json::Map::new();
    for required in [
        "CheckpointLoaderSimple",
        "CLIPTextEncode",
        "EmptyLatentImage",
        "KSampler",
        "VAEDecode",
        "SaveImage",
    ] {
        let node_info = get_json(
            &client,
            endpoint_url(&endpoint, &format!("object_info/{required}"))?,
        )?;
        let Some(node) = node_info.get(required) else {
            return Err(ImageError::new(
                "unsupported_capability",
                format!("ComfyUI is missing the required {required} node"),
            ));
        };
        object_info.insert(required.to_string(), node.clone());
    }
    let object_info = Value::Object(object_info);

    let mut models = get_json(&client, endpoint_url(&endpoint, "models/checkpoints")?)
        .ok()
        .and_then(|value| {
            value.as_array().map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
        })
        .unwrap_or_default();
    if models.is_empty() {
        models = choice_list(&object_info, "CheckpointLoaderSimple", "ckpt_name");
    }
    models
        .retain(|value| !value.is_empty() && value.len() <= 512 && !has_disallowed_control(value));
    models.sort();
    models.dedup();
    let mut samplers = choice_list(&object_info, "KSampler", "sampler_name");
    let mut schedulers = choice_list(&object_info, "KSampler", "scheduler");
    samplers
        .retain(|value| !value.is_empty() && value.len() <= 128 && !has_disallowed_control(value));
    schedulers
        .retain(|value| !value.is_empty() && value.len() <= 128 && !has_disallowed_control(value));
    samplers.sort();
    samplers.dedup();
    schedulers.sort();
    schedulers.dedup();
    Ok(ImageProviderDiscovery {
        provider_id: provider.id.clone(),
        provider_name: provider.name.clone(),
        endpoint: provider.endpoint.clone(),
        local: true,
        capabilities: capabilities(),
        models,
        samplers,
        schedulers,
    })
}

fn load_provider(settings: &SharedSettings) -> Result<ImageProviderSettings, String> {
    settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .load()
        .map(|settings| settings.ai.image_provider)
}

#[tauri::command]
pub async fn image_provider_discover(
    settings: tauri::State<'_, SharedSettings>,
) -> Result<ImageProviderDiscovery, String> {
    let provider = load_provider(settings.inner())?;
    tauri::async_runtime::spawn_blocking(move || discover_provider(&provider))
        .await
        .map_err(|error| format!("image provider worker failed: {error}"))?
        .map_err(|error| error.message)
}

#[tauri::command]
pub async fn image_provider_status(
    settings: tauri::State<'_, SharedSettings>,
) -> Result<ImageProviderStatus, String> {
    let provider = load_provider(settings.inner())?;
    let configured = provider.clone();
    let result = if provider.enabled {
        tauri::async_runtime::spawn_blocking(move || discover_provider(&provider))
            .await
            .map_err(|error| format!("image provider worker failed: {error}"))?
    } else {
        Err(ImageError::new(
            "invalid_configuration",
            "AI image generation is disabled",
        ))
    };
    Ok(match result {
        Ok(discovery) => ImageProviderStatus {
            provider_id: configured.id,
            provider_name: configured.name,
            endpoint: configured.endpoint,
            model: configured.model.clone(),
            enabled: true,
            local: true,
            available: true,
            model_available: !configured.model.is_empty()
                && discovery
                    .models
                    .iter()
                    .any(|model| model == &configured.model),
            capabilities: discovery.capabilities,
            error_code: None,
            error: None,
        },
        Err(error) => ImageProviderStatus {
            provider_id: configured.id,
            provider_name: configured.name,
            endpoint: configured.endpoint,
            model: configured.model,
            enabled: configured.enabled,
            local: true,
            available: false,
            model_available: false,
            capabilities: capabilities(),
            error_code: Some(error.code.into()),
            error: Some(error.message),
        },
    })
}

fn validate_request(
    request: &mut ImageGenerationRequest,
    provider: &ImageProviderSettings,
    discovery: &ImageProviderDiscovery,
) -> Result<(), ImageError> {
    request.prompt = request.prompt.trim().to_string();
    request.negative_prompt = request.negative_prompt.trim().to_string();
    request.model = request.model.trim().to_string();
    request.sampler = request.sampler.trim().to_string();
    request.scheduler = request.scheduler.trim().to_string();
    if request.prompt.is_empty()
        || request.prompt.len() > MAX_PROMPT_BYTES
        || has_disallowed_control(&request.prompt)
    {
        return Err(ImageError::new(
            "invalid_configuration",
            "The image prompt must contain 1 to 16 KiB of text",
        ));
    }
    if request.negative_prompt.len() > MAX_PROMPT_BYTES
        || has_disallowed_control(&request.negative_prompt)
    {
        return Err(ImageError::new(
            "invalid_configuration",
            "The negative prompt exceeds 16 KiB",
        ));
    }
    if request.model.is_empty() {
        request.model = provider.model.clone();
    }
    if request.model.is_empty() || !discovery.models.iter().any(|model| model == &request.model) {
        return Err(ImageError::new(
            "model_unavailable",
            "Select an available ComfyUI checkpoint model",
        ));
    }
    if request.width < 64
        || request.height < 64
        || request.width > MAX_DIMENSION
        || request.height > MAX_DIMENSION
        || !request.width.is_multiple_of(8)
        || !request.height.is_multiple_of(8)
        || u64::from(request.width) * u64::from(request.height) > MAX_PIXELS
    {
        return Err(ImageError::new(
            "invalid_configuration",
            "Image dimensions must be multiples of 8 between 64 and 4096 and at most 16 megapixels",
        ));
    }
    if !(1..=MAX_OUTPUTS).contains(&request.output_count) {
        return Err(ImageError::new(
            "invalid_configuration",
            "Generate between 1 and 4 outputs",
        ));
    }
    if !(1..=150).contains(&request.steps)
        || !request.guidance_scale.is_finite()
        || !(0.0..=30.0).contains(&request.guidance_scale)
    {
        return Err(ImageError::new(
            "invalid_configuration",
            "Advanced generation settings are outside the supported range",
        ));
    }
    if !discovery
        .samplers
        .iter()
        .any(|value| value == &request.sampler)
        || !discovery
            .schedulers
            .iter()
            .any(|value| value == &request.scheduler)
    {
        return Err(ImageError::new(
            "invalid_configuration",
            "The selected sampler or scheduler is not available",
        ));
    }
    if request.context.len() > MAX_CONTEXT_ITEMS {
        return Err(ImageError::new(
            "invalid_configuration",
            "Too many context items were selected",
        ));
    }
    let allowed_sources = [
        "identity",
        "field",
        "document",
        "relationship",
        "timeline",
        "location",
    ];
    for item in &mut request.context {
        item.entity_id = item.entity_id.trim().to_string();
        item.label = item.label.trim().to_string();
        if item.entity_id.is_empty()
            || item.entity_id.len() > 128
            || has_disallowed_control(&item.entity_id)
            || item.label.is_empty()
            || item.label.len() > 256
            || has_disallowed_control(&item.label)
            || !allowed_sources.contains(&item.source_kind.as_str())
        {
            return Err(ImageError::new(
                "invalid_configuration",
                "Selected image context is malformed",
            ));
        }
    }
    let allowed_methods = [
        "manual",
        "entity",
        "selected-context",
        "rewrite",
        "detailed",
        "simplified",
    ];
    if !allowed_methods.contains(&request.prompt_provenance.method.as_str()) {
        return Err(ImageError::new(
            "invalid_configuration",
            "Prompt provenance is malformed",
        ));
    }
    let text_provider_valid = request
        .prompt_provenance
        .text_provider_id
        .as_deref()
        .is_some_and(|value| {
            !value.trim().is_empty() && value.len() <= 256 && !has_disallowed_control(value)
        });
    let text_model_valid = request
        .prompt_provenance
        .text_model
        .as_deref()
        .is_some_and(|value| {
            !value.trim().is_empty() && value.len() <= 512 && !has_disallowed_control(value)
        });
    if request.prompt_provenance.llm_assisted {
        if request.prompt_provenance.method == "manual" || !text_provider_valid || !text_model_valid
        {
            return Err(ImageError::new(
                "invalid_configuration",
                "LLM-assisted prompt provenance is incomplete",
            ));
        }
    } else if request.prompt_provenance.method != "manual"
        || request.prompt_provenance.text_provider_id.is_some()
        || request.prompt_provenance.text_model.is_some()
    {
        return Err(ImageError::new(
            "invalid_configuration",
            "Manual prompt provenance is inconsistent",
        ));
    }
    Ok(())
}

fn workflow(request: &ImageGenerationRequest, job_id: &str) -> Value {
    json!({
        "1": {"class_type": "CheckpointLoaderSimple", "inputs": {"ckpt_name": request.model}},
        "2": {"class_type": "CLIPTextEncode", "inputs": {"clip": ["1", 1], "text": request.prompt}},
        "3": {"class_type": "CLIPTextEncode", "inputs": {"clip": ["1", 1], "text": request.negative_prompt}},
        "4": {"class_type": "EmptyLatentImage", "inputs": {
            "width": request.width,
            "height": request.height,
            "batch_size": request.output_count
        }},
        "5": {"class_type": "KSampler", "inputs": {
            "model": ["1", 0],
            "positive": ["2", 0],
            "negative": ["3", 0],
            "latent_image": ["4", 0],
            "seed": request.seed,
            "steps": request.steps,
            "cfg": request.guidance_scale,
            "sampler_name": request.sampler,
            "scheduler": request.scheduler,
            "denoise": 1.0
        }},
        "6": {"class_type": "VAEDecode", "inputs": {"samples": ["5", 0], "vae": ["1", 2]}},
        "7": {"class_type": "SaveImage", "inputs": {
            "filename_prefix": format!("Daena/{}", &job_id[..8]),
            "images": ["6", 0]
        }}
    })
}

fn update_job(
    manager: &SharedImageGeneration,
    job_id: &str,
    update: impl FnOnce(&mut ImageGenerationJob),
) {
    if let Ok(mut manager) = manager.lock() {
        if let Some(job) = manager.jobs.get_mut(job_id) {
            job.expires_at = Instant::now() + JOB_TTL;
            update(job);
        }
    }
}

fn fail_job(manager: &SharedImageGeneration, job_id: &str, error: ImageError) {
    update_job(manager, job_id, |job| {
        if job.status.state == "cancelled" {
            return;
        }
        job.status.state = "failed".into();
        job.status.stage = "Generation failed".into();
        job.status.error_code = Some(error.code.into());
        job.status.error = Some(error.message);
    });
}

fn post_json(client: &Client, url: Url, body: &Value) -> Result<Value, ImageError> {
    let response = client
        .post(url)
        .json(body)
        .send()
        .map_err(|error| ImageError::provider_transport(&error))?;
    let bytes = read_bounded_response(response, MAX_JSON_BYTES)?;
    serde_json::from_slice(&bytes)
        .map_err(|_| ImageError::new("provider_error", "ComfyUI returned malformed JSON"))
}

fn queue_position(queue: &Value, prompt_id: &str) -> (bool, Option<u32>) {
    let running = queue
        .get("queue_running")
        .and_then(Value::as_array)
        .is_some_and(|items| {
            items.iter().any(|item| {
                item.as_array()
                    .and_then(|parts| parts.get(1))
                    .and_then(Value::as_str)
                    == Some(prompt_id)
            })
        });
    let position = queue
        .get("queue_pending")
        .and_then(Value::as_array)
        .and_then(|items| {
            items.iter().position(|item| {
                item.as_array()
                    .and_then(|parts| parts.get(1))
                    .and_then(Value::as_str)
                    == Some(prompt_id)
            })
        })
        .map(|position| position as u32 + 1);
    (running, position)
}

fn history_entry<'a>(history: &'a Value, prompt_id: &str) -> Option<&'a Value> {
    history.get(prompt_id)
}

#[derive(Debug)]
struct ProviderImageRef {
    filename: String,
    subfolder: String,
    folder_type: String,
}

fn history_images(entry: &Value) -> Result<Vec<ProviderImageRef>, ImageError> {
    let status = entry.get("status");
    let status_text = status
        .map(Value::to_string)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if status_text.contains("error") || status_text.contains("failed") {
        let code = if status_text.contains("out of memory") || status_text.contains("oom") {
            "insufficient_resources"
        } else {
            "provider_error"
        };
        return Err(ImageError::new(
            code,
            "ComfyUI could not complete the generation",
        ));
    }
    let mut images = Vec::new();
    if let Some(outputs) = entry.get("outputs").and_then(Value::as_object) {
        for output in outputs.values() {
            if let Some(values) = output.get("images").and_then(Value::as_array) {
                for image in values {
                    let filename = image.get("filename").and_then(Value::as_str);
                    let subfolder = image.get("subfolder").and_then(Value::as_str);
                    let folder_type = image.get("type").and_then(Value::as_str);
                    if let (Some(filename), Some(subfolder), Some(folder_type)) =
                        (filename, subfolder, folder_type)
                    {
                        if !matches!(folder_type, "output" | "temp")
                            || filename.len() > 1_024
                            || subfolder.len() > 1_024
                            || filename.contains(['/', '\\', '\0'])
                            || subfolder.contains("..")
                        {
                            return Err(ImageError::new(
                                "provider_error",
                                "ComfyUI returned an unsafe output reference",
                            ));
                        }
                        images.push(ProviderImageRef {
                            filename: filename.into(),
                            subfolder: subfolder.into(),
                            folder_type: folder_type.into(),
                        });
                    }
                }
            }
        }
    }
    Ok(images)
}

fn image_mime(bytes: &[u8]) -> Result<&'static str, ImageError> {
    match infer::get(bytes).map(|kind| kind.mime_type()) {
        Some("image/png") => Ok("image/png"),
        Some("image/jpeg") => Ok("image/jpeg"),
        Some("image/webp") => Ok("image/webp"),
        _ => Err(ImageError::new(
            "provider_error",
            "ComfyUI returned an unsupported or malformed image",
        )),
    }
}

fn download_image(
    client: &Client,
    endpoint: &Url,
    image: &ProviderImageRef,
) -> Result<Vec<u8>, ImageError> {
    let mut url = endpoint_url(endpoint, "view")?;
    url.query_pairs_mut()
        .append_pair("filename", &image.filename)
        .append_pair("subfolder", &image.subfolder)
        .append_pair("type", &image.folder_type);
    let response = client
        .get(url)
        .send()
        .map_err(|error| ImageError::provider_transport(&error))?;
    read_bounded_response(response, MAX_IMAGE_BYTES)
}

fn cancel_provider_prompt(client: &Client, endpoint: &Url, prompt_id: &str) {
    let _ = post_json(
        client,
        endpoint_url(endpoint, "queue").unwrap_or_else(|_| endpoint.clone()),
        &json!({"delete": [prompt_id]}),
    );
    if let Ok(queue) = get_json(
        client,
        endpoint_url(endpoint, "queue").unwrap_or_else(|_| endpoint.clone()),
    ) {
        if queue_position(&queue, prompt_id).0 {
            let _ = client
                .post(endpoint_url(endpoint, "interrupt").unwrap_or_else(|_| endpoint.clone()))
                .json(&json!({}))
                .send();
        }
    }
}

fn prompt_submission(request: &ImageGenerationRequest, job_id: &str) -> Value {
    json!({
        "prompt": workflow(request, job_id),
        "client_id": format!("daena-{job_id}"),
    })
}

fn run_generation(
    manager: SharedImageGeneration,
    job_id: String,
    provider: ImageProviderSettings,
    request: ImageGenerationRequest,
    cancel: Arc<AtomicBool>,
) -> Result<(), ImageError> {
    let endpoint = validate_local_endpoint(&provider.endpoint)?;
    let client = provider_client(Duration::from_secs(30))?;
    let body = prompt_submission(&request, &job_id);
    let queued = post_json(&client, endpoint_url(&endpoint, "prompt")?, &body)?;
    let provider_prompt_id = queued
        .get("prompt_id")
        .and_then(Value::as_str)
        .ok_or_else(|| ImageError::new("provider_error", "ComfyUI did not return a prompt ID"))?;
    if provider_prompt_id.is_empty()
        || provider_prompt_id.len() > 128
        || !provider_prompt_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(ImageError::new(
            "provider_error",
            "ComfyUI returned an invalid prompt ID",
        ));
    }
    let provider_prompt_id = provider_prompt_id.to_string();

    let started = Instant::now();
    let image_refs = loop {
        if cancel.load(Ordering::Relaxed) {
            cancel_provider_prompt(&client, &endpoint, &provider_prompt_id);
            update_job(&manager, &job_id, |job| {
                job.status.state = "cancelled".into();
                job.status.stage = "Cancelled".into();
                job.status.error_code = None;
                job.status.error = None;
            });
            return Ok(());
        }
        if started.elapsed() >= GENERATION_DEADLINE {
            cancel_provider_prompt(&client, &endpoint, &provider_prompt_id);
            return Err(ImageError::new(
                "provider_error",
                "ComfyUI generation exceeded the 15 minute deadline",
            ));
        }

        let history = get_json(
            &client,
            endpoint_url(&endpoint, &format!("history/{provider_prompt_id}"))?,
        )?;
        if let Some(entry) = history_entry(&history, &provider_prompt_id) {
            let images = history_images(entry)?;
            if images.is_empty() {
                return Err(ImageError::new(
                    "provider_error",
                    "ComfyUI completed without returning an image",
                ));
            }
            break images;
        }

        if let Ok(queue) = get_json(&client, endpoint_url(&endpoint, "queue")?) {
            let (running, position) = queue_position(&queue, &provider_prompt_id);
            update_job(&manager, &job_id, |job| {
                job.status.state = if running { "running" } else { "queued" }.into();
                job.status.stage = if running {
                    "Generating in ComfyUI".into()
                } else if let Some(position) = position {
                    format!("Queued in ComfyUI · position {position}")
                } else {
                    "Waiting for ComfyUI".into()
                };
                job.status.queue_position = position;
            });
        }
        thread::sleep(POLL_INTERVAL);
    };

    if image_refs.len() > request.output_count as usize || image_refs.len() > MAX_OUTPUTS as usize {
        return Err(ImageError::new(
            "provider_error",
            "ComfyUI returned more outputs than requested",
        ));
    }
    update_job(&manager, &job_id, |job| {
        job.status.state = "downloading".into();
        job.status.stage = "Receiving generated images".into();
        job.status.queue_position = None;
        job.status.total = image_refs.len() as u32;
    });

    let mut downloaded = Vec::new();
    let mut total_bytes = 0usize;
    for (index, image_ref) in image_refs.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            update_job(&manager, &job_id, |job| {
                job.status.state = "cancelled".into();
                job.status.stage = "Cancelled".into();
            });
            return Ok(());
        }
        let bytes = download_image(&client, &endpoint, image_ref)?;
        total_bytes = total_bytes.saturating_add(bytes.len());
        if total_bytes > MAX_TOTAL_IMAGE_BYTES {
            return Err(ImageError::new(
                "provider_error",
                "Generated images exceed Daena's total byte limit",
            ));
        }
        let mime_type = image_mime(&bytes)?.to_string();
        let id = Uuid::new_v4().to_string();
        let metadata = ImageCandidate {
            id: id.clone(),
            filename: image_ref.filename.clone(),
            mime_type,
            size: bytes.len(),
            width: request.width,
            height: request.height,
            seed: request.seed,
            accepted_asset_id: None,
        };
        downloaded.push((
            id,
            CandidateData {
                metadata,
                bytes: Some(bytes),
                accepted: None,
            },
        ));
        update_job(&manager, &job_id, |job| {
            job.status.completed = index as u32 + 1;
        });
    }

    update_job(&manager, &job_id, |job| {
        for (id, candidate) in downloaded {
            job.candidates.insert(id, candidate);
        }
        job.status.candidates = job
            .candidates
            .values()
            .map(|candidate| candidate.metadata.clone())
            .collect();
        job.status.state = "completed".into();
        job.status.stage = format!(
            "{} candidate{} ready for review",
            job.status.candidates.len(),
            if job.status.candidates.len() == 1 {
                ""
            } else {
                "s"
            }
        );
        job.status.completed = job.status.candidates.len() as u32;
        job.status.total = job.status.candidates.len() as u32;
    });
    Ok(())
}

fn status_for(
    manager: &SharedImageGeneration,
    job_id: &str,
    project_id: &str,
) -> Result<ImageGenerationStatus, String> {
    let mut manager = manager
        .lock()
        .map_err(|_| "image generation state is unavailable".to_string())?;
    manager.reap_expired();
    let job = manager
        .jobs
        .get(job_id)
        .ok_or_else(|| "Image generation job was not found or has expired".to_string())?;
    if job.project_id != project_id {
        return Err("Image generation job does not belong to this project".into());
    }
    Ok(job.status.clone())
}

#[tauri::command]
pub async fn image_generate_start(
    core: tauri::State<'_, SharedCore>,
    settings: tauri::State<'_, SharedSettings>,
    jobs: tauri::State<'_, SharedImageGeneration>,
    mut request: ImageGenerationRequest,
) -> Result<ImageGenerationStatus, String> {
    crate::ai::ensure_active_project(core.inner(), &request.project_id)?;
    crate::ensure_project_ai_enabled(&request.project_id)?;
    let entity_id = request.entity_id.clone();
    crate::with_read_project(core, move |project| {
        project
            .get_entity(&entity_id)?
            .filter(|entity| !entity.deleted)
            .ok_or_else(|| CoreError::NotFound("image generation entity not found".into()))?;
        Ok(())
    })
    .await?;

    let provider = load_provider(settings.inner())?;
    if !provider.enabled {
        return Err("Enable AI image generation in Settings first".into());
    }
    let discovery_provider = provider.clone();
    let discovery =
        tauri::async_runtime::spawn_blocking(move || discover_provider(&discovery_provider))
            .await
            .map_err(|error| format!("image provider worker failed: {error}"))?
            .map_err(|error| error.message)?;
    validate_request(&mut request, &provider, &discovery).map_err(|error| error.message)?;

    let job_id = Uuid::new_v4().to_string();
    let cancel = Arc::new(AtomicBool::new(false));
    let status = ImageGenerationStatus {
        job_id: job_id.clone(),
        state: "queued".into(),
        stage: "Submitting to local ComfyUI".into(),
        completed: 0,
        total: request.output_count,
        queue_position: None,
        candidates: Vec::new(),
        error_code: None,
        error: None,
    };
    {
        let mut manager = jobs
            .lock()
            .map_err(|_| "image generation state is unavailable".to_string())?;
        manager.reap_expired();
        let active_jobs = manager
            .jobs
            .values()
            .filter(|job| {
                matches!(
                    job.status.state.as_str(),
                    "queued" | "running" | "downloading"
                )
            })
            .count();
        if active_jobs >= MAX_ACTIVE_JOBS {
            return Err("Too many image generations are already active".into());
        }
        if manager.jobs.len() >= MAX_RETAINED_JOBS {
            return Err("Discard an older image generation before starting another".into());
        }
        manager.jobs.insert(
            job_id.clone(),
            ImageGenerationJob {
                project_id: request.project_id.clone(),
                entity_id: request.entity_id.clone(),
                expires_at: Instant::now() + JOB_TTL,
                cancel: cancel.clone(),
                status: status.clone(),
                candidates: BTreeMap::new(),
                request: request.clone(),
                provider: provider.clone(),
                created_at: timestamp(),
            },
        );
    }
    let manager = jobs.inner().clone();
    let worker_job_id = job_id.clone();
    thread::spawn(move || {
        if let Err(error) = run_generation(
            manager.clone(),
            worker_job_id.clone(),
            provider,
            request,
            cancel,
        ) {
            fail_job(&manager, &worker_job_id, error);
        }
    });
    Ok(status)
}

#[tauri::command]
pub fn image_generation_status(
    core: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedImageGeneration>,
    job_id: String,
    project_id: String,
) -> Result<ImageGenerationStatus, String> {
    crate::ai::ensure_active_project(core.inner(), &project_id)?;
    status_for(jobs.inner(), &job_id, &project_id)
}

#[tauri::command]
pub fn image_generation_cancel(
    core: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedImageGeneration>,
    job_id: String,
    project_id: String,
) -> Result<ImageGenerationStatus, String> {
    crate::ai::ensure_active_project(core.inner(), &project_id)?;
    {
        let mut manager = jobs
            .lock()
            .map_err(|_| "image generation state is unavailable".to_string())?;
        manager.reap_expired();
        let job = manager
            .jobs
            .get_mut(&job_id)
            .ok_or_else(|| "Image generation job was not found or has expired".to_string())?;
        if job.project_id != project_id {
            return Err("Image generation job does not belong to this project".into());
        }
        job.cancel.store(true, Ordering::Relaxed);
        job.status.stage = "Cancelling local generation".into();
    }
    status_for(jobs.inner(), &job_id, &project_id)
}

#[tauri::command]
pub fn image_candidate_bytes(
    core: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedImageGeneration>,
    job_id: String,
    candidate_id: String,
    project_id: String,
) -> Result<Vec<u8>, String> {
    crate::ai::ensure_active_project(core.inner(), &project_id)?;
    let mut manager = jobs
        .lock()
        .map_err(|_| "image generation state is unavailable".to_string())?;
    manager.reap_expired();
    let job = manager
        .jobs
        .get(&job_id)
        .ok_or_else(|| "Image generation job was not found or has expired".to_string())?;
    if job.project_id != project_id {
        return Err("Image generation job does not belong to this project".into());
    }
    job.candidates
        .get(&candidate_id)
        .and_then(|candidate| candidate.bytes.clone())
        .ok_or_else(|| "Image candidate is unavailable or was already accepted".to_string())
}

fn safe_extension(mime_type: &str) -> &'static str {
    match mime_type {
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        _ => "png",
    }
}

fn sanitized_filename(filename: &str, mime_type: &str) -> String {
    let stem = PathBuf::from(filename.trim())
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("generated-illustration")
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || matches!(character, '-' | '_' | ' ') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let stem = stem.trim().trim_matches(['.', '-']).trim();
    format!(
        "{}.{}",
        if stem.is_empty() {
            "generated-illustration"
        } else {
            stem
        },
        safe_extension(mime_type)
    )
}

fn generation_provenance(job: &ImageGenerationJob, candidate: &ImageCandidate) -> Value {
    let mut context_entity_ids = BTreeSet::new();
    for item in &job.request.context {
        context_entity_ids.insert(item.entity_id.clone());
    }
    context_entity_ids.insert(job.entity_id.clone());
    json!({
        "schemaVersion": 1,
        "kind": "ai-image-generation",
        "finalPrompt": job.request.prompt,
        "negativePrompt": if job.request.negative_prompt.is_empty() { Value::Null } else { Value::String(job.request.negative_prompt.clone()) },
        "promptGeneration": job.request.prompt_provenance,
        "imageProvider": {
            "id": job.provider.id,
            "name": job.provider.name,
            "adapter": job.provider.adapter,
            "local": true
        },
        "imageModel": job.request.model,
        "modelIdentifier": job.request.model,
        "seed": candidate.seed,
        "dimensions": {"width": candidate.width, "height": candidate.height},
        "parameters": {
            "steps": job.request.steps,
            "guidanceScale": job.request.guidance_scale,
            "sampler": job.request.sampler,
            "scheduler": job.request.scheduler,
            "outputCount": job.request.output_count
        },
        "sourceReferenceImages": [],
        "contextEntities": context_entity_ids,
        "selectedContext": job.request.context,
        "visualStyle": Value::Null,
        "creationTimestamp": job.created_at,
        "sourceRequestId": job.status.job_id,
        "candidateId": candidate.id
    })
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn image_candidate_accept(
    core: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedImageGeneration>,
    job_id: String,
    candidate_id: String,
    project_id: String,
    entity_id: String,
    namespace: String,
    filename: String,
    request_id: String,
) -> Result<Asset, String> {
    crate::ai::ensure_active_project(core.inner(), &project_id)?;
    crate::ensure_project_ai_enabled(&project_id)?;
    let (bytes, mime_type, provenance, safe_filename, already_accepted) = {
        let mut manager = jobs
            .lock()
            .map_err(|_| "image generation state is unavailable".to_string())?;
        manager.reap_expired();
        let job = manager
            .jobs
            .get(&job_id)
            .ok_or_else(|| "Image generation job was not found or has expired".to_string())?;
        if job.project_id != project_id || job.entity_id != entity_id {
            return Err("Image candidate does not belong to this project entity".into());
        }
        let candidate = job
            .candidates
            .get(&candidate_id)
            .ok_or_else(|| "Image candidate was not found".to_string())?;
        (
            candidate.bytes.clone(),
            candidate.metadata.mime_type.clone(),
            generation_provenance(job, &candidate.metadata),
            sanitized_filename(&filename, &candidate.metadata.mime_type),
            candidate.accepted.clone(),
        )
    };
    if let Some(asset) = already_accepted {
        return Ok(asset);
    }
    let bytes = bytes.ok_or_else(|| "Image candidate bytes are unavailable".to_string())?;
    let temporary = std::env::temp_dir().join(format!(
        "daena-generated-image-{}-{}",
        candidate_id,
        safe_extension(&mime_type)
    ));
    std::fs::write(&temporary, &bytes)
        .map_err(|error| format!("Could not stage the generated image: {error}"))?;
    let source_path = temporary.to_string_lossy().into_owned();
    let result = crate::with_core(core, move |core| {
        core.project(crate::trusted_shell())?
            .register_asset_file_with_options(
                AssetFileInput {
                    entity_id,
                    namespace,
                    source_path,
                    filename: safe_filename,
                    mime_type,
                    provenance: Some(provenance),
                },
                None,
                Some(&request_id),
            )
    })
    .await;
    let _ = std::fs::remove_file(&temporary);
    let asset = result?;
    {
        let mut manager = jobs
            .lock()
            .map_err(|_| "image generation state is unavailable".to_string())?;
        if let Some(candidate) = manager
            .jobs
            .get_mut(&job_id)
            .and_then(|job| job.candidates.get_mut(&candidate_id))
        {
            candidate.bytes = None;
            candidate.accepted = Some(asset.clone());
            candidate.metadata.accepted_asset_id = Some(asset.id.clone());
        }
        if let Some(job) = manager.jobs.get_mut(&job_id) {
            job.status.candidates = job
                .candidates
                .values()
                .map(|candidate| candidate.metadata.clone())
                .collect();
        }
    }
    Ok(asset)
}

#[tauri::command]
pub fn image_candidate_discard(
    core: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedImageGeneration>,
    job_id: String,
    candidate_id: String,
    project_id: String,
) -> Result<ImageGenerationStatus, String> {
    crate::ai::ensure_active_project(core.inner(), &project_id)?;
    {
        let mut manager = jobs
            .lock()
            .map_err(|_| "image generation state is unavailable".to_string())?;
        manager.reap_expired();
        let job = manager
            .jobs
            .get_mut(&job_id)
            .ok_or_else(|| "Image generation job was not found or has expired".to_string())?;
        if job.project_id != project_id {
            return Err("Image generation job does not belong to this project".into());
        }
        if job
            .candidates
            .get(&candidate_id)
            .is_some_and(|candidate| candidate.accepted.is_some())
        {
            return Err("Accepted assets must be removed through the asset workflow".into());
        }
        if job.candidates.remove(&candidate_id).is_none() {
            return Err("Image candidate was not found".into());
        }
        job.status.candidates = job
            .candidates
            .values()
            .map(|candidate| candidate.metadata.clone())
            .collect();
        job.status.completed = job.status.candidates.len() as u32;
        job.status.total = job.status.candidates.len() as u32;
        job.status.stage = if job.status.candidates.is_empty() {
            "All temporary candidates were discarded".into()
        } else {
            format!("{} candidate(s) remain", job.status.candidates.len())
        };
    }
    status_for(jobs.inner(), &job_id, &project_id)
}

#[tauri::command]
pub fn image_generation_discard(
    core: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedImageGeneration>,
    job_id: String,
    project_id: String,
) -> Result<(), String> {
    crate::ai::ensure_active_project(core.inner(), &project_id)?;
    let mut manager = jobs
        .lock()
        .map_err(|_| "image generation state is unavailable".to_string())?;
    manager.reap_expired();
    let job = manager
        .jobs
        .get(&job_id)
        .ok_or_else(|| "Image generation job was not found or has expired".to_string())?;
    if job.project_id != project_id {
        return Err("Image generation job does not belong to this project".into());
    }
    job.cancel.store(true, Ordering::Relaxed);
    manager.jobs.remove(&job_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;
    use std::net::TcpListener;

    fn discovery() -> ImageProviderDiscovery {
        ImageProviderDiscovery {
            provider_id: "comfyui-local".into(),
            provider_name: "ComfyUI".into(),
            endpoint: "http://127.0.0.1:8188".into(),
            local: true,
            capabilities: capabilities(),
            models: vec!["world.safetensors".into()],
            samplers: vec!["euler".into()],
            schedulers: vec!["normal".into()],
        }
    }

    fn request() -> ImageGenerationRequest {
        ImageGenerationRequest {
            project_id: "/tmp/world".into(),
            entity_id: Uuid::new_v4().to_string(),
            prompt: "A fortified city at dusk".into(),
            negative_prompt: "modern cars".into(),
            model: "world.safetensors".into(),
            width: 1024,
            height: 1024,
            seed: 42,
            output_count: 2,
            steps: 24,
            guidance_scale: 7.0,
            sampler: "euler".into(),
            scheduler: "normal".into(),
            context: vec![ImageContextItem {
                entity_id: Uuid::new_v4().to_string(),
                label: "Culture: Eranian".into(),
                source_kind: "field".into(),
            }],
            prompt_provenance: ImagePromptProvenance {
                method: "selected-context".into(),
                llm_assisted: true,
                edited_after_assistance: true,
                text_provider_id: Some("lm-studio".into()),
                text_model: Some("writer".into()),
            },
        }
    }

    #[test]
    fn local_endpoint_rejects_remote_and_credentialed_urls() {
        assert!(validate_local_endpoint("http://127.0.0.1:8188").is_ok());
        assert!(validate_local_endpoint("http://[::1]:8188/base").is_ok());
        assert!(validate_local_endpoint("https://example.com").is_err());
        assert!(validate_local_endpoint("http://user@127.0.0.1:8188").is_err());
    }

    #[test]
    fn request_validation_enforces_model_dimensions_and_provider_controls() {
        let provider = ImageProviderSettings::default();
        let available = discovery();
        assert!(validate_request(&mut request(), &provider, &available).is_ok());
        let mut invalid = request();
        invalid.width = 100;
        assert_eq!(
            validate_request(&mut invalid, &provider, &available)
                .unwrap_err()
                .code,
            "invalid_configuration"
        );
        let mut missing_model = request();
        missing_model.model = "missing.safetensors".into();
        assert_eq!(
            validate_request(&mut missing_model, &provider, &available)
                .unwrap_err()
                .code,
            "model_unavailable"
        );
    }

    #[test]
    fn workflow_is_controlled_and_contains_only_expected_v1_nodes() {
        let value = workflow(&request(), "12345678-1234-1234-1234-123456789012");
        let nodes = value.as_object().unwrap();
        assert_eq!(nodes.len(), 7);
        let classes = nodes
            .values()
            .filter_map(|node| node.get("class_type").and_then(Value::as_str))
            .collect::<BTreeSet<_>>();
        assert_eq!(
            classes,
            BTreeSet::from([
                "CheckpointLoaderSimple",
                "CLIPTextEncode",
                "EmptyLatentImage",
                "KSampler",
                "VAEDecode",
                "SaveImage",
            ])
        );
        assert_eq!(value.pointer("/4/inputs/batch_size"), Some(&json!(2)));
        assert_eq!(value.pointer("/5/inputs/seed"), Some(&json!(42)));
    }

    #[test]
    fn prompt_submission_leaves_provider_prompt_id_assignment_to_comfyui() {
        let job_id = "12345678-1234-1234-1234-123456789012";
        let value = prompt_submission(&request(), job_id);
        assert_eq!(
            value.get("client_id"),
            Some(&json!(format!("daena-{job_id}")))
        );
        assert!(value.get("prompt").is_some());
        assert!(value.get("prompt_id").is_none());
    }

    #[test]
    fn generation_tracks_the_prompt_id_assigned_by_comfyui() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::<Vec<u8>>::new()));
        let captured = requests.clone();
        let server = thread::spawn(move || {
            for index in 0..3 {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request_bytes = Vec::new();
                let mut buffer = [0_u8; 4_096];
                let expected_length = loop {
                    let read = stream.read(&mut buffer).unwrap();
                    assert!(read > 0);
                    request_bytes.extend_from_slice(&buffer[..read]);
                    let Some(header_end) = request_bytes
                        .windows(4)
                        .position(|window| window == b"\r\n\r\n")
                        .map(|position| position + 4)
                    else {
                        continue;
                    };
                    let headers = String::from_utf8_lossy(&request_bytes[..header_end]);
                    let content_length = headers
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|value| value.trim().parse::<usize>().ok())
                        })
                        .unwrap_or_default();
                    if request_bytes.len() >= header_end + content_length {
                        break header_end + content_length;
                    }
                };
                request_bytes.truncate(expected_length);
                captured.lock().unwrap().push(request_bytes);

                let (content_type, body): (&str, Vec<u8>) = match index {
                    0 => ("application/json", br#"{"prompt_id":"provider-123"}"#.to_vec()),
                    1 => (
                        "application/json",
                        br#"{"provider-123":{"outputs":{"7":{"images":[{"filename":"candidate.png","subfolder":"Daena","type":"output"}]}},"status":{"status_str":"success"}}}"#.to_vec(),
                    ),
                    _ => ("image/png", b"\x89PNG\r\n\x1a\n".to_vec()),
                };
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .unwrap();
                stream.write_all(&body).unwrap();
            }
        });

        let mut request = request();
        request.output_count = 1;
        let job_id = Uuid::new_v4().to_string();
        let cancel = Arc::new(AtomicBool::new(false));
        let provider = ImageProviderSettings {
            endpoint,
            ..ImageProviderSettings::default()
        };
        let manager: SharedImageGeneration =
            Arc::new(Mutex::new(ImageGenerationManager::default()));
        manager.lock().unwrap().jobs.insert(
            job_id.clone(),
            ImageGenerationJob {
                project_id: request.project_id.clone(),
                entity_id: request.entity_id.clone(),
                expires_at: Instant::now() + JOB_TTL,
                cancel: cancel.clone(),
                status: ImageGenerationStatus {
                    job_id: job_id.clone(),
                    state: "queued".into(),
                    stage: "queued".into(),
                    completed: 0,
                    total: 1,
                    queue_position: None,
                    candidates: Vec::new(),
                    error_code: None,
                    error: None,
                },
                candidates: BTreeMap::new(),
                request: request.clone(),
                provider: provider.clone(),
                created_at: timestamp(),
            },
        );

        run_generation(manager.clone(), job_id.clone(), provider, request, cancel).unwrap();
        server.join().unwrap();
        let manager = manager.lock().unwrap();
        let job = manager.jobs.get(&job_id).unwrap();
        assert_eq!(job.status.state, "completed");
        assert_eq!(job.candidates.len(), 1);
        drop(manager);

        let captured = requests.lock().unwrap();
        assert!(String::from_utf8_lossy(&captured[1]).contains("GET /history/provider-123 "));
        let first = &captured[0];
        let body_start = first
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .unwrap()
            + 4;
        let submission: Value = serde_json::from_slice(&first[body_start..]).unwrap();
        assert!(submission.get("prompt_id").is_none());
    }

    #[test]
    fn provenance_records_reviewed_prompt_context_and_generation_parameters() {
        let request = request();
        let candidate = ImageCandidate {
            id: Uuid::new_v4().to_string(),
            filename: "output.png".into(),
            mime_type: "image/png".into(),
            size: 100,
            width: request.width,
            height: request.height,
            seed: request.seed,
            accepted_asset_id: None,
        };
        let job = ImageGenerationJob {
            project_id: request.project_id.clone(),
            entity_id: request.entity_id.clone(),
            expires_at: Instant::now() + JOB_TTL,
            cancel: Arc::new(AtomicBool::new(false)),
            status: ImageGenerationStatus {
                job_id: Uuid::new_v4().to_string(),
                state: "completed".into(),
                stage: "ready".into(),
                completed: 1,
                total: 1,
                queue_position: None,
                candidates: vec![candidate.clone()],
                error_code: None,
                error: None,
            },
            candidates: BTreeMap::new(),
            request,
            provider: ImageProviderSettings::default(),
            created_at: timestamp(),
        };
        let provenance = generation_provenance(&job, &candidate);
        assert_eq!(
            provenance.get("finalPrompt").and_then(Value::as_str),
            Some("A fortified city at dusk")
        );
        assert_eq!(
            provenance.pointer("/promptGeneration/llmAssisted"),
            Some(&Value::Bool(true))
        );
        assert_eq!(provenance.pointer("/dimensions/width"), Some(&json!(1024)));
        assert_eq!(
            provenance
                .get("contextEntities")
                .and_then(Value::as_array)
                .map(Vec::len),
            Some(2)
        );
    }
}
