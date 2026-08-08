//! Provider-neutral AI contracts and deterministic test primitives.
//!
//! This crate deliberately has no Tauri, provider SDK, plugin runtime, or
//! project-storage dependency. It is the Phase 0 contract boundary only.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt;
use std::time::{Duration, Instant};

pub const PROMPT_TEMPLATE_VERSION: &str = "ai.prompt.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub max_input_bytes: usize,
    pub max_output_bytes: usize,
    pub max_schema_bytes: usize,
    pub max_schema_depth: usize,
    pub stream_queue_length: usize,
    pub max_concurrent_requests: usize,
    pub embedding_batch_size: usize,
    pub max_image_bytes: usize,
    pub max_image_count: usize,
    pub default_deadline: Duration,
    pub temporary_result_ttl: Duration,
}

pub const DEFAULT_LIMITS: Limits = Limits {
    max_input_bytes: 128 * 1024,
    max_output_bytes: 64 * 1024,
    max_schema_bytes: 32 * 1024,
    max_schema_depth: 8,
    stream_queue_length: 64,
    max_concurrent_requests: 2,
    embedding_batch_size: 8,
    max_image_bytes: 8 * 1024 * 1024,
    max_image_count: 1,
    default_deadline: Duration::from_secs(30),
    temporary_result_ttl: Duration::from_secs(10 * 60),
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CallerKind {
    TrustedShell,
    AuthorizedPlugin { plugin_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiCaller {
    pub kind: CallerKind,
    pub project_id: String,
    pub capabilities: Vec<String>,
    pub resource_scopes: Vec<String>,
    pub session_generation: u64,
    pub request_id: String,
}

impl AiCaller {
    pub fn trusted_shell(project_id: impl Into<String>, request_id: impl Into<String>) -> Self {
        Self {
            kind: CallerKind::TrustedShell,
            project_id: project_id.into(),
            capabilities: vec![],
            resource_scopes: vec![],
            session_generation: 0,
            request_id: request_id.into(),
        }
    }

    pub fn authorized_plugin(
        plugin_id: impl Into<String>,
        project_id: impl Into<String>,
        capabilities: Vec<String>,
        resource_scopes: Vec<String>,
        session_generation: u64,
        request_id: impl Into<String>,
    ) -> Self {
        Self {
            kind: CallerKind::AuthorizedPlugin {
                plugin_id: plugin_id.into(),
            },
            project_id: project_id.into(),
            capabilities,
            resource_scopes,
            session_generation,
            request_id: request_id.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Operation {
    GenerateText,
    GenerateStructured,
    GenerateImage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RetrievalMode {
    None,
    ExplicitOnly,
    Related,
    Project,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalPolicy {
    pub mode: RetrievalMode,
    pub seed_ids: Vec<String>,
    pub allowed_source_kinds: Vec<String>,
    pub relationship_depth: u8,
    pub passage_count: u16,
    pub include_shared_fields: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiResult {
    pub request_id: String,
    pub operation: Operation,
    pub task_id: String,
    pub output: serde_json::Value,
    pub citations: Vec<SourceRef>,
    pub prompt_template_version: String,
    pub completion_reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationLimits {
    pub max_output_bytes: usize,
    pub deadline_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiRequest {
    pub request_id: String,
    pub caller: AiCaller,
    pub operation: Operation,
    pub task_id: String,
    pub user_instruction: String,
    pub immediate_context: serde_json::Value,
    pub output_contract: Option<serde_json::Value>,
    pub generation_limits: GenerationLimits,
    pub stream: bool,
    pub prompt_template_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRef {
    pub source_kind: String,
    pub entity_id: Option<String>,
    pub document_id: Option<String>,
    pub canonical_path: Option<String>,
    pub revision: String,
    pub content_hash: String,
    pub byte_start: Option<u64>,
    pub byte_end: Option<u64>,
    pub excerpt_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiEvent {
    Started,
    TextDelta(String),
    StructuredDelta(serde_json::Value),
    Usage {
        input_bytes: usize,
        output_bytes: usize,
    },
    Completed,
    Cancelled,
    Failed(AiError),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SequencedEvent {
    pub sequence: u64,
    pub event: AiEvent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiError {
    ProviderUnavailable,
    ModelNotFound,
    CapabilityUnavailable,
    AuthenticationFailed,
    RateLimited,
    ContextTooLarge,
    InvalidProviderResponse,
    OutputValidationFailed,
    RemoteContextDenied,
    Cancelled,
    DeadlineExceeded,
    IndexUnavailable,
    QueueFull,
}

impl fmt::Display for AiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for AiError {}

#[derive(Debug)]
pub struct Cancellation {
    cancelled: bool,
}
impl Default for Cancellation {
    fn default() -> Self {
        Self::new()
    }
}
impl Cancellation {
    pub fn new() -> Self {
        Self { cancelled: false }
    }
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
}

#[derive(Debug)]
pub struct BoundedEventStream {
    queue: VecDeque<SequencedEvent>,
    capacity: usize,
    next_sequence: u64,
    terminal: bool,
}
impl BoundedEventStream {
    pub fn new(capacity: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            capacity: capacity.max(2),
            next_sequence: 0,
            terminal: false,
        }
    }
    pub fn push(&mut self, event: AiEvent) -> Result<(), AiError> {
        if self.terminal {
            return Ok(());
        }
        let is_terminal = matches!(
            event,
            AiEvent::Completed | AiEvent::Cancelled | AiEvent::Failed(_)
        );
        // Reserve one slot so a bounded producer can always publish its
        // terminal event after the final non-terminal item.
        if self.queue.len() >= self.capacity
            || (!is_terminal && self.queue.len() + 1 >= self.capacity)
        {
            return Err(AiError::QueueFull);
        }
        self.queue.push_back(SequencedEvent {
            sequence: self.next_sequence,
            event,
        });
        self.next_sequence += 1;
        if is_terminal {
            self.terminal = true;
        }
        Ok(())
    }
    pub fn pop(&mut self) -> Option<SequencedEvent> {
        self.queue.pop_front()
    }
    pub fn is_terminal(&self) -> bool {
        self.terminal
    }
    pub fn len(&self) -> usize {
        self.queue.len()
    }
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct FakeProvider {
    pub scripted_events: Vec<AiEvent>,
    pub limits: Limits,
}
impl FakeProvider {
    pub fn new(scripted_events: Vec<AiEvent>) -> Self {
        Self {
            scripted_events,
            limits: DEFAULT_LIMITS,
        }
    }
    pub fn run(
        &self,
        request: &AiRequest,
        cancellation: &Cancellation,
        started: Instant,
    ) -> BoundedEventStream {
        let mut stream = BoundedEventStream::new(self.limits.stream_queue_length);
        stream
            .push(AiEvent::Started)
            .expect("default stream capacity must admit Started");
        let mut output_bytes = 0usize;
        for event in &self.scripted_events {
            if cancellation.is_cancelled() {
                let _ = stream.push(AiEvent::Cancelled);
                break;
            }
            if started.elapsed() > Duration::from_millis(request.generation_limits.deadline_ms) {
                let _ = stream.push(AiEvent::Failed(AiError::DeadlineExceeded));
                break;
            }
            let delta_bytes = match event {
                AiEvent::TextDelta(delta) => delta.len(),
                AiEvent::StructuredDelta(value) => match serde_json::to_vec(value) {
                    Ok(bytes) => bytes.len(),
                    Err(_) => {
                        let _ = stream.push(AiEvent::Failed(AiError::InvalidProviderResponse));
                        break;
                    }
                },
                _ => 0,
            };
            output_bytes = output_bytes.saturating_add(delta_bytes);
            if output_bytes > request.generation_limits.max_output_bytes {
                let _ = stream.push(AiEvent::Failed(AiError::OutputValidationFailed));
                break;
            }
            if stream.push(event.clone()).is_err() {
                let _ = stream.push(AiEvent::Failed(AiError::QueueFull));
                break;
            }
        }
        if !stream.is_terminal() {
            let _ = stream.push(AiEvent::Completed);
        }
        stream
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Deserialize)]
    struct ContractFixture {
        operation: Operation,
        task_id: String,
        #[serde(default)]
        user_instruction: String,
        #[serde(default)]
        immediate_context: serde_json::Value,
        #[serde(default)]
        output_contract: Option<serde_json::Value>,
        stream: bool,
        expected_events: Vec<String>,
    }

    fn request(deadline_ms: u64) -> AiRequest {
        AiRequest {
            request_id: "req-1".into(),
            caller: AiCaller::trusted_shell("project", "req-1"),
            operation: Operation::GenerateText,
            task_id: "test".into(),
            user_instruction: "hello".into(),
            immediate_context: serde_json::json!({}),
            output_contract: None,
            generation_limits: GenerationLimits {
                max_output_bytes: DEFAULT_LIMITS.max_output_bytes,
                deadline_ms,
            },
            stream: true,
            prompt_template_version: PROMPT_TEMPLATE_VERSION.into(),
        }
    }

    fn fixture_request(fixture: &ContractFixture) -> AiRequest {
        AiRequest {
            request_id: format!("fixture-{}", fixture.task_id),
            caller: AiCaller::trusted_shell("project", "fixture-request"),
            operation: fixture.operation,
            task_id: fixture.task_id.clone(),
            user_instruction: fixture.user_instruction.clone(),
            immediate_context: fixture.immediate_context.clone(),
            output_contract: fixture.output_contract.clone(),
            generation_limits: GenerationLimits {
                max_output_bytes: DEFAULT_LIMITS.max_output_bytes,
                deadline_ms: DEFAULT_LIMITS.default_deadline.as_millis() as u64,
            },
            stream: fixture.stream,
            prompt_template_version: PROMPT_TEMPLATE_VERSION.into(),
        }
    }

    fn event_name(event: &AiEvent) -> &'static str {
        match event {
            AiEvent::Started => "Started",
            AiEvent::TextDelta(_) => "TextDelta",
            AiEvent::StructuredDelta(_) => "StructuredDelta",
            AiEvent::Usage { .. } => "Usage",
            AiEvent::Completed => "Completed",
            AiEvent::Cancelled => "Cancelled",
            AiEvent::Failed(_) => "Failed",
        }
    }

    fn streamed_event_names(
        fixture: &ContractFixture,
        scripted_events: Vec<AiEvent>,
    ) -> Vec<String> {
        let mut stream = FakeProvider::new(scripted_events).run(
            &fixture_request(fixture),
            &Cancellation::new(),
            Instant::now(),
        );
        let mut names = Vec::new();
        while let Some(event) = stream.pop() {
            names.push(event_name(&event.event).to_owned());
        }
        names
    }

    #[test]
    fn stream_has_one_terminal_state_and_ignores_late_events() {
        let mut stream = BoundedEventStream::new(4);
        stream.push(AiEvent::Started).unwrap();
        stream.push(AiEvent::Completed).unwrap();
        stream.push(AiEvent::TextDelta("late".into())).unwrap();
        assert!(stream.is_terminal());
        assert_eq!(stream.len(), 2);
    }

    #[test]
    fn stream_is_bounded() {
        let mut stream = BoundedEventStream::new(2);
        stream.push(AiEvent::Started).unwrap();
        assert_eq!(
            stream.push(AiEvent::TextDelta("x".into())),
            Err(AiError::QueueFull)
        );
        stream.push(AiEvent::Completed).unwrap();
        assert!(stream.is_terminal());
    }

    #[test]
    fn contract_fixtures_deserialize_and_match_fake_provider_events() {
        let text: ContractFixture =
            serde_json::from_str(include_str!("../fixtures/text-generation.json")).unwrap();
        assert_eq!(text.operation, Operation::GenerateText);
        assert!(text.stream);
        assert_eq!(
            streamed_event_names(&text, vec![AiEvent::TextDelta("rewritten".into())]),
            text.expected_events
        );

        let structured: ContractFixture =
            serde_json::from_str(include_str!("../fixtures/structured-generation.json")).unwrap();
        assert_eq!(structured.operation, Operation::GenerateStructured);
        assert!(structured.stream);
        assert_eq!(
            streamed_event_names(
                &structured,
                vec![AiEvent::StructuredDelta(serde_json::json!({
                    "biography": "A careful archivist."
                }))],
            ),
            structured.expected_events
        );
    }

    #[test]
    fn fake_provider_enforces_aggregate_structured_output_limit() {
        let mut request = request(1000);
        request.generation_limits.max_output_bytes = 10;
        let mut stream = FakeProvider::new(vec![AiEvent::StructuredDelta(
            serde_json::json!({"biography": "more than ten bytes"}),
        )])
        .run(&request, &Cancellation::new(), Instant::now());
        let events: Vec<_> = (0..4).filter_map(|_| stream.pop()).collect();
        assert!(events
            .iter()
            .any(|event| event.event == AiEvent::Failed(AiError::OutputValidationFailed)));
    }

    #[test]
    fn fake_provider_honors_cancellation() {
        let mut cancellation = Cancellation::new();
        cancellation.cancel();
        let mut stream = FakeProvider::new(vec![AiEvent::TextDelta("never".into())]).run(
            &request(1000),
            &cancellation,
            Instant::now(),
        );
        let events: Vec<_> = (0..4).filter_map(|_| stream.pop()).collect();
        assert!(events.iter().any(|event| event.event == AiEvent::Cancelled));
    }

    #[test]
    fn fake_provider_honors_deadline() {
        let mut stream = FakeProvider::new(vec![AiEvent::TextDelta("never".into())]).run(
            &request(0),
            &Cancellation::new(),
            Instant::now() - Duration::from_millis(1),
        );
        let events: Vec<_> = (0..4).filter_map(|_| stream.pop()).collect();
        assert!(events
            .iter()
            .any(|event| event.event == AiEvent::Failed(AiError::DeadlineExceeded)));
    }

    #[test]
    fn caller_scope_is_host_constructed() {
        let caller = AiCaller::authorized_plugin(
            "com.example.lore",
            "project",
            vec!["ai.text.generate".into()],
            vec!["entity:read".into()],
            7,
            "req",
        );
        assert_eq!(caller.project_id, "project");
        assert_eq!(caller.session_generation, 7);
        assert!(matches!(caller.kind, CallerKind::AuthorizedPlugin { .. }));
    }
}
