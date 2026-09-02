// AI runtime core.
use super::*;

pub type SharedAiRuntime = Arc<Mutex<AiRuntime>>;
pub(super) const MAX_BUFFERED_REQUESTS: usize = 32;
pub(super) const MAX_BUFFERED_EVENTS: usize = 64;
pub(super) const AI_CANCEL_POLL_INTERVAL: Duration = Duration::from_millis(50);
pub(super) const MAX_PROVIDER_FRAME_BYTES: usize = 64 * 1024;
pub(super) const AI_CHUNKER_VERSION: &str = "chunker.v1";
#[derive(Debug, Clone)]
pub(super) struct RetrievalSource {
    pub(super) entity_id: Option<String>,
    pub(super) canonical_path: Option<String>,
    pub(super) summary: Option<String>,
}

pub(super) type RetrievalSourceIds = BTreeMap<String, RetrievalSource>;

#[derive(Default)]
pub struct AiRuntime {
    pub(super) cancellations: HashMap<String, Arc<std::sync::atomic::AtomicBool>>,
    pub(super) events: HashMap<String, VecDeque<AiStreamEvent>>,
    pub(super) request_order: VecDeque<String>,
    pub(super) provider: Option<Arc<dyn AiProvider>>,
    pub(super) citations: HashMap<String, Vec<SourceRef>>,
    pub(super) index: Option<AiIndex>,
    pub(super) index_cancel: Option<Arc<std::sync::atomic::AtomicBool>>,
    pub(super) index_state: Option<IndexState>,
}

/// Request payload for `AiProvider` implementations. Fields are read by providers,
/// not by this crate's orchestration path.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ProviderRequest {
    pub request_id: String,
    pub caller: daena_ai::AiCaller,
    pub model: String,
    pub instruction: String,
    pub context: String,
    pub output_contract: Option<serde_json::Value>,
    pub deadline: Duration,
}

pub trait AiProvider: Send + Sync {
    fn generate(
        &self,
        request: ProviderRequest,
        cancelled: Arc<std::sync::atomic::AtomicBool>,
    ) -> Vec<AiStreamEvent>;
}

#[cfg(test)]
#[derive(Debug, Default)]
pub struct FakeLoopbackProvider;

#[cfg(test)]
impl AiProvider for FakeLoopbackProvider {
    fn generate(
        &self,
        request: ProviderRequest,
        cancelled: Arc<std::sync::atomic::AtomicBool>,
    ) -> Vec<AiStreamEvent> {
        if cancelled.load(std::sync::atomic::Ordering::Relaxed) {
            return vec![AiStreamEvent {
                sequence: 0,
                request_id: request.request_id,
                phase: "cancelled".into(),
                delta: None,
                output: None,
                error: None,
            }];
        }
        let output = if request.output_contract.is_some() {
            serde_json::json!({"summary": format!("Draft for {}", request.context)}).to_string()
        } else {
            format!("Rewritten: {}", request.context)
        };
        if output.len() > DEFAULT_LIMITS.max_output_bytes {
            return vec![AiStreamEvent {
                sequence: 0,
                request_id: request.request_id,
                phase: "failed".into(),
                delta: None,
                output: None,
                error: Some(AiError::OutputValidationFailed.to_string()),
            }];
        }
        let request_id = request.request_id;
        vec![
            AiStreamEvent {
                sequence: 0,
                request_id: request_id.clone(),
                phase: "delta".into(),
                delta: Some(output.clone()),
                output: None,
                error: None,
            },
            AiStreamEvent {
                sequence: 0,
                request_id,
                phase: "completed".into(),
                delta: None,
                output: Some(output),
                error: None,
            },
        ]
    }
}

impl AiRuntime {
    #[cfg(test)]
    pub fn with_provider(provider: Arc<dyn AiProvider>) -> Self {
        Self {
            provider: Some(provider),
            ..Self::default()
        }
    }
}
