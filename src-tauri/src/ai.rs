use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use daena_ai::index::{
    AiIndex, ChunkSource, EmbeddingMetadata, EmbeddingProvider, IndexError, IndexState, TextChunk,
};
use daena_ai::{
    AiCaller, AiError, ContextBudget, RetrievalMode, RetrievalPolicy, RetrievedPassage, SourceRef,
    DEFAULT_GENERATION_DEADLINE, DEFAULT_LIMITS, PROMPT_TEMPLATE_VERSION,
};
use daena_core::{CoreError, ProjectStore};
use daena_plugin_api::{AiRetrievalMode, AiRetrievalPolicyPayload};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use crate::settings::{AppSettings, SettingsStore};

mod index;
mod orchestration;
mod provider;
mod retrieval;
mod runtime;
mod transport;

#[cfg(test)]
pub use self::index::index_status;
use self::index::*;
pub use self::index::{
    __cmd__ai_index_cancel, __cmd__ai_index_rebuild, __cmd__ai_index_search,
    __cmd__ai_index_status, __tauri_command_name_ai_index_cancel,
    __tauri_command_name_ai_index_rebuild, __tauri_command_name_ai_index_search,
    __tauri_command_name_ai_index_status,
};
pub use self::index::{
    ai_index_cancel, ai_index_rebuild, ai_index_search, ai_index_status, attach_project_index,
    detach_project_index,
};
use self::orchestration::*;
pub use self::orchestration::{
    __cmd__ai_cancel_text, __cmd__ai_generate_structured, __cmd__ai_generate_text,
    __cmd__ai_poll_text, __cmd__ai_provider_models, __cmd__ai_provider_status,
    __tauri_command_name_ai_cancel_text, __tauri_command_name_ai_generate_structured,
    __tauri_command_name_ai_generate_text, __tauri_command_name_ai_poll_text,
    __tauri_command_name_ai_provider_models, __tauri_command_name_ai_provider_status,
};
pub use self::orchestration::{
    ai_cancel_text, ai_generate_structured, ai_generate_text, ai_poll_text, ai_provider_models,
    ai_provider_status, ai_request_citations, ai_request_result, cancel_ai_request, poll_ai_events,
    remove_ai_citations, start_ai_request_mode, validate_structured_output,
    validate_structured_schema,
};
use self::provider::*;
pub use self::provider::{
    __cmd__ai_provider_clear_credential, __cmd__ai_provider_credential_status,
    __cmd__ai_provider_import_credential, __cmd__ai_provider_set_credential,
    __cmd__ai_remote_set_consent, __tauri_command_name_ai_provider_clear_credential,
    __tauri_command_name_ai_provider_credential_status,
    __tauri_command_name_ai_provider_import_credential,
    __tauri_command_name_ai_provider_set_credential, __tauri_command_name_ai_remote_set_consent,
};
pub use self::provider::{
    ai_provider_clear_credential, ai_provider_credential_status, ai_provider_import_credential,
    ai_provider_set_credential, ai_remote_set_consent, resolve_ai_provider,
};
pub use self::retrieval::build_retrieval_context;
pub(crate) use self::retrieval::ensure_active_project;
use self::retrieval::*;
#[cfg(test)]
pub use self::runtime::FakeLoopbackProvider;
use self::runtime::*;
pub use self::runtime::{AiProvider, AiRuntime, ProviderRequest, SharedAiRuntime};
use self::transport::*;
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiStreamEvent {
    pub sequence: u64,
    pub request_id: String,
    pub phase: String,
    pub delta: Option<String>,
    pub output: Option<String>,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests;
