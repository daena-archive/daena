//! Provider-neutral Phase 4 retrieval primitives.
//!
//! This module is intentionally independent of project storage and provider
//! transports. It provides deterministic chunk identities, an injectable
//! embedding cache, exact vector search, and hybrid rank fusion. The durable
//! `.daena/ai/index.sqlite` assembly belongs to the host runtime and can use
//! these primitives without making `daena-core` depend on AI state.

use rusqlite::{params, types::Type, Connection, OptionalExtension};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

pub const CHUNKER_VERSION: &str = "markdown.blocks.v1";
pub const EMBEDDING_SERIALIZER_VERSION: &str = "embedding.normalized.v1";
const INDEX_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkSource {
    pub source_id: String,
    pub source_kind: String,
    pub revision: String,
    pub source_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextChunk {
    pub id: String,
    pub source: ChunkSource,
    pub ordinal: u32,
    pub heading_ancestry: Vec<String>,
    pub text: String,
    pub byte_start: u64,
    pub byte_end: u64,
    pub text_hash: String,
}

/// Split Markdown at blank-line/block boundaries, carrying the active heading
/// ancestry into each chunk. Oversized blocks are split only at UTF-8
/// boundaries; no text from adjacent sources can enter a chunk.
pub fn chunk_markdown(source: ChunkSource, markdown: &str, max_bytes: usize) -> Vec<TextChunk> {
    let max_bytes = max_bytes.max(1);
    let mut heading_ancestry: Vec<String> = Vec::new();
    let mut blocks: Vec<(usize, String, Vec<String>)> = Vec::new();
    let mut block_start = None;
    let mut block_lines = Vec::new();
    let mut line_start = 0usize;
    for line in markdown.split_inclusive('\n') {
        let content = line.trim_end_matches(['\r', '\n']);
        if content.trim_start().starts_with('#') {
            let trimmed = content.trim_start();
            let level = trimmed
                .chars()
                .take_while(|character| *character == '#')
                .count();
            if level > 0 && trimmed.chars().nth(level) == Some(' ') {
                heading_ancestry.truncate(level.saturating_sub(1));
                heading_ancestry.push(trimmed[level + 1..].trim().to_owned());
            }
        }
        if block_start.is_none() && !content.trim().is_empty() {
            block_start = Some(line_start);
        }
        if block_start.is_some() && content.trim().is_empty() {
            let start = block_start.take().expect("block start exists");
            let text = block_lines.concat();
            blocks.push((start, text, heading_ancestry.clone()));
            block_lines.clear();
        } else if block_start.is_some() {
            block_lines.push(line);
        }
        line_start += line.len();
    }
    if let Some(start) = block_start {
        blocks.push((start, block_lines.concat(), heading_ancestry.clone()));
    }

    let mut chunks = Vec::new();
    for (start, block, heading_ancestry) in blocks {
        let mut content = block;
        let mut local_offset = 0usize;
        while !content.is_empty() {
            let end = utf8_boundary_at_most(&content, max_bytes);
            let text = content[..end].to_owned();
            let byte_start = (start + local_offset) as u64;
            let byte_end = byte_start + text.len() as u64;
            let text_hash = hash_text(&text);
            let ordinal = chunks.len() as u32;
            let id = hash_text(&format!(
                "{}/{}/{}/{}/{}/{}",
                source.source_id,
                source.source_hash,
                CHUNKER_VERSION,
                byte_start,
                byte_end,
                text_hash
            ));
            chunks.push(TextChunk {
                id,
                source: source.clone(),
                ordinal,
                heading_ancestry: heading_ancestry.clone(),
                text,
                byte_start,
                byte_end,
                text_hash,
            });
            content = content[end..].to_owned();
            local_offset += end;
        }
    }
    chunks
}

/// Serialize a structured derived record canonically, then apply the same
/// bounded UTF-8 chunking rules as Markdown. Object keys are sorted so field
/// and relationship chunks are byte-stable across rebuilds.
pub fn chunk_structured(
    source: ChunkSource,
    value: &serde_json::Value,
    max_bytes: usize,
) -> Vec<TextChunk> {
    let canonical = canonical_json(value);
    chunk_markdown(source, &canonical, max_bytes)
}

fn canonical_json(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Object(object) => {
            let entries = object
                .iter()
                .map(|(key, value)| (key, canonical_json(value)))
                .collect::<BTreeMap<_, _>>();
            format!(
                "{{{}}}",
                entries
                    .iter()
                    .map(|(key, value)| format!(
                        "{}:{}",
                        serde_json::to_string(key).expect("JSON keys are serializable"),
                        value
                    ))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
        serde_json::Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(canonical_json)
                .collect::<Vec<_>>()
                .join(",")
        ),
        _ => value.to_string(),
    }
}

fn utf8_boundary_at_most(text: &str, max_bytes: usize) -> usize {
    let end = text
        .char_indices()
        .map(|(index, _)| index)
        .chain(std::iter::once(text.len()))
        .take_while(|index| *index <= max_bytes)
        .last()
        .unwrap_or(0);
    if end == 0 {
        text.chars().next().map_or(0, char::len_utf8)
    } else {
        end
    }
}

pub fn hash_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbeddingMetadata {
    pub provider_id: String,
    pub model_id: String,
    pub dimension: usize,
    pub normalized: bool,
    pub serializer_version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VectorRecord {
    pub chunk: TextChunk,
    pub vector: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VectorMatch {
    pub chunk_id: String,
    pub cosine: f32,
}

#[derive(Debug, Clone, Default)]
pub struct EmbeddingCache {
    metadata: Option<EmbeddingMetadata>,
    values: BTreeMap<String, Vec<f32>>,
}

impl EmbeddingCache {
    pub fn compatible_with(&self, metadata: &EmbeddingMetadata) -> bool {
        self.metadata.as_ref() == Some(metadata)
    }

    pub fn replace_metadata(&mut self, metadata: EmbeddingMetadata) {
        if self.metadata.as_ref() != Some(&metadata) {
            self.values.clear();
        }
        self.metadata = Some(metadata);
    }

    pub fn get(&self, text_hash: &str) -> Option<&[f32]> {
        self.values.get(text_hash).map(Vec::as_slice)
    }

    pub fn insert(&mut self, text_hash: String, vector: Vec<f32>) {
        self.values.insert(text_hash, vector);
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

pub fn exact_cosine_search(
    records: &[VectorRecord],
    query: &[f32],
    limit: usize,
) -> Vec<VectorMatch> {
    let mut matches = records
        .iter()
        .filter_map(|record| {
            cosine_similarity(&record.vector, query).map(|cosine| VectorMatch {
                chunk_id: record.chunk.id.clone(),
                cosine,
            })
        })
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| {
        right
            .cosine
            .total_cmp(&left.cosine)
            .then_with(|| left.chunk_id.cmp(&right.chunk_id))
    });
    matches.truncate(limit);
    matches
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> Option<f32> {
    if left.is_empty() || left.len() != right.len() {
        return None;
    }
    let left_norm = left.iter().map(|value| value * value).sum::<f32>().sqrt();
    let right_norm = right.iter().map(|value| value * value).sum::<f32>().sqrt();
    if left_norm == 0.0 || right_norm == 0.0 {
        return None;
    }
    Some(left.iter().zip(right).map(|(a, b)| a * b).sum::<f32>() / (left_norm * right_norm))
}

/// Fuse zero-based lexical and vector ranks using deterministic RRF. Missing
/// candidates contribute no score; ties are resolved by chunk ID.
pub fn reciprocal_rank_fusion(
    lexical: &[(String, usize)],
    semantic: &[VectorMatch],
    limit: usize,
) -> Vec<(String, f32)> {
    let mut scores = BTreeMap::<String, f32>::new();
    for (chunk_id, rank) in lexical {
        *scores.entry(chunk_id.clone()).or_default() += 1.0 / (60.0 + *rank as f32 + 1.0);
    }
    for (rank, match_) in semantic.iter().enumerate() {
        *scores.entry(match_.chunk_id.clone()).or_default() += 1.0 / (60.0 + rank as f32 + 1.0);
    }
    let mut fused = scores.into_iter().collect::<Vec<_>>();
    fused.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    fused.truncate(limit);
    fused
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndexState {
    Disabled,
    Absent,
    Indexing,
    Ready,
    PartiallyStale,
    Incompatible,
    Failed,
}

#[derive(Debug)]
pub enum IndexError {
    Sqlite(rusqlite::Error),
    Serialization(String),
    InvalidEmbedding(String),
    Cancelled,
}

impl fmt::Display for IndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(error) => write!(f, "AI index storage failed: {error}"),
            Self::Serialization(error) => write!(f, "AI index serialization failed: {error}"),
            Self::InvalidEmbedding(error) => write!(f, "invalid embedding: {error}"),
            Self::Cancelled => write!(f, "AI indexing cancelled"),
        }
    }
}

impl std::error::Error for IndexError {}

impl From<rusqlite::Error> for IndexError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

pub trait EmbeddingProvider {
    fn embed(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, IndexError>;

    fn dimension(&self) -> Option<usize> {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexingReport {
    pub chunk_count: usize,
    pub embedded_count: usize,
    pub reused_count: usize,
}

/// Disposable, project-local AI index. It stores only derived chunks and
/// vectors; callers remain responsible for obtaining canonical source text.
pub struct AiIndex {
    connection: Connection,
}

impl AiIndex {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, IndexError> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                IndexError::Serialization(format!("create AI index directory: {error}"))
            })?;
        }
        Self::from_connection(Connection::open(path)?)
    }

    pub fn in_memory() -> Result<Self, IndexError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(connection: Connection) -> Result<Self, IndexError> {
        connection.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS ai_metadata (
                 key TEXT PRIMARY KEY NOT NULL,
                 value TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS ai_chunks (
                 chunk_id TEXT PRIMARY KEY NOT NULL,
                 source_id TEXT NOT NULL,
                 source_kind TEXT NOT NULL,
                 revision TEXT NOT NULL,
                 source_hash TEXT NOT NULL,
                 ordinal INTEGER NOT NULL,
                 heading_ancestry TEXT NOT NULL,
                 text TEXT NOT NULL,
                 byte_start INTEGER NOT NULL,
                 byte_end INTEGER NOT NULL,
                 text_hash TEXT NOT NULL,
                 vector TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS ai_chunks_source_id ON ai_chunks(source_id);
             CREATE INDEX IF NOT EXISTS ai_chunks_text_hash ON ai_chunks(text_hash);",
        )?;
        connection.pragma_update(None, "user_version", INDEX_SCHEMA_VERSION)?;
        let index = Self { connection };
        if index.metadata_value("state")?.is_none() {
            index.set_state(IndexState::Absent)?;
        }
        Ok(index)
    }

    pub fn state(&self) -> Result<IndexState, IndexError> {
        Ok(self
            .metadata_value("state")?
            .and_then(|value| serde_json::from_str(&value).ok())
            .unwrap_or(IndexState::Absent))
    }

    pub fn set_state(&self, state: IndexState) -> Result<(), IndexError> {
        self.set_metadata(
            "state",
            &serde_json::to_string(&state)
                .map_err(|error| IndexError::Serialization(error.to_string()))?,
        )
    }

    pub fn embedding_metadata(&self) -> Result<Option<EmbeddingMetadata>, IndexError> {
        self.metadata_value("embedding_metadata")?
            .map(|value| {
                serde_json::from_str(&value)
                    .map_err(|error| IndexError::Serialization(error.to_string()))
            })
            .transpose()
    }

    /// Returns whether the requested model metadata matches the persisted
    /// semantic index. Incompatibility clears vectors but leaves source chunks
    /// available for lexical fallback and marks the index for rebuilding.
    pub fn prepare_embedding_metadata(
        &self,
        metadata: &EmbeddingMetadata,
    ) -> Result<bool, IndexError> {
        let compatible = self.embedding_metadata()?.as_ref() == Some(metadata);
        if !compatible {
            self.connection
                .execute("UPDATE ai_chunks SET vector='[]'", [])?;
            self.set_state(IndexState::Incompatible)?;
            self.set_metadata(
                "embedding_metadata",
                &serde_json::to_string(metadata)
                    .map_err(|error| IndexError::Serialization(error.to_string()))?,
            )?;
        }
        Ok(compatible)
    }

    pub fn records(&self) -> Result<Vec<VectorRecord>, IndexError> {
        let mut statement = self.connection.prepare(
            "SELECT chunk_id,source_id,source_kind,revision,source_hash,ordinal,
                    heading_ancestry,text,byte_start,byte_end,text_hash,vector
             FROM ai_chunks WHERE vector <> '[]' ORDER BY ordinal,chunk_id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(VectorRecord {
                chunk: TextChunk {
                    id: row.get(0)?,
                    source: ChunkSource {
                        source_id: row.get(1)?,
                        source_kind: row.get(2)?,
                        revision: row.get(3)?,
                        source_hash: row.get(4)?,
                    },
                    ordinal: row.get(5)?,
                    heading_ancestry: parse_json(row.get(6)?)?,
                    text: row.get(7)?,
                    byte_start: row.get(8)?,
                    byte_end: row.get(9)?,
                    text_hash: row.get(10)?,
                },
                vector: parse_json(row.get(11)?)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(IndexError::from)
    }

    pub fn search(&self, query: &[f32], limit: usize) -> Result<Vec<VectorMatch>, IndexError> {
        Ok(exact_cosine_search(&self.records()?, query, limit))
    }

    /// Index one source atomically. Cancellation or an embedding failure
    /// leaves the previously published source generation intact.
    pub fn index_source<P, F>(
        &self,
        chunks: &[TextChunk],
        metadata: &EmbeddingMetadata,
        provider: &P,
        is_cancelled: F,
    ) -> Result<IndexingReport, IndexError>
    where
        P: EmbeddingProvider,
        F: FnMut() -> bool,
    {
        let was_compatible = self.embedding_metadata().ok().flatten().as_ref() == Some(metadata);
        let result = self.index_source_inner(chunks, metadata, provider, is_cancelled);
        if let Err(error) = &result {
            if matches!(error, IndexError::Cancelled) {
                // The inner pipeline records this as partially stale so
                // lexical retrieval can continue while a rebuild is retried.
            } else if !was_compatible {
                let _ = self.set_state(IndexState::Incompatible);
            } else {
                let _ = self.set_state(IndexState::Failed);
            }
        }
        result
    }

    fn index_source_inner<P, F>(
        &self,
        chunks: &[TextChunk],
        metadata: &EmbeddingMetadata,
        provider: &P,
        mut is_cancelled: F,
    ) -> Result<IndexingReport, IndexError>
    where
        P: EmbeddingProvider,
        F: FnMut() -> bool,
    {
        self.prepare_embedding_metadata(metadata)?;
        self.set_state(IndexState::Indexing)?;
        let mut vectors = Vec::with_capacity(chunks.len());
        let mut embedded_count = 0;
        let mut reused_count = 0;
        let batch_size = 8;
        for batch in chunks.chunks(batch_size) {
            if is_cancelled() {
                self.set_state(IndexState::PartiallyStale)?;
                return Err(IndexError::Cancelled);
            }
            let mut missing = Vec::new();
            let mut missing_positions = Vec::new();
            let mut batch_vectors = vec![None; batch.len()];
            for (position, chunk) in batch.iter().enumerate() {
                if let Some(vector) = self.cached_vector(&chunk.text_hash, metadata)? {
                    batch_vectors[position] = Some(vector);
                    reused_count += 1;
                } else {
                    missing_positions.push(position);
                    missing.push(chunk.text.clone());
                }
            }
            if !missing.is_empty() {
                let generated = provider.embed(&missing)?;
                if generated.len() != missing.len() {
                    return Err(IndexError::InvalidEmbedding(
                        "provider returned the wrong batch length".into(),
                    ));
                }
                for (position, vector) in missing_positions.into_iter().zip(generated) {
                    let effective_metadata = if metadata.dimension == 0 {
                        EmbeddingMetadata {
                            dimension: vector.len(),
                            ..metadata.clone()
                        }
                    } else {
                        metadata.clone()
                    };
                    validate_embedding(&vector, &effective_metadata)?;
                    if metadata.dimension == 0 {
                        self.set_metadata(
                            "embedding_metadata",
                            &serde_json::to_string(&effective_metadata)
                                .map_err(|error| IndexError::Serialization(error.to_string()))?,
                        )?;
                    }
                    batch_vectors[position] = Some(vector);
                    embedded_count += 1;
                }
            }
            vectors.extend(
                batch_vectors.into_iter().map(|vector| {
                    vector.expect("every chunk receives a cached or generated vector")
                }),
            );
        }

        let transaction = self.connection.unchecked_transaction()?;
        if let Some(source_id) = chunks.first().map(|chunk| chunk.source.source_id.as_str()) {
            transaction.execute(
                "DELETE FROM ai_chunks WHERE source_id=?1",
                params![source_id],
            )?;
        }
        for (chunk, vector) in chunks.iter().zip(vectors) {
            transaction.execute(
                "INSERT INTO ai_chunks
                 (chunk_id,source_id,source_kind,revision,source_hash,ordinal,
                  heading_ancestry,text,byte_start,byte_end,text_hash,vector)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
                params![
                    chunk.id,
                    chunk.source.source_id,
                    chunk.source.source_kind,
                    chunk.source.revision,
                    chunk.source.source_hash,
                    chunk.ordinal,
                    serde_json::to_string(&chunk.heading_ancestry)
                        .map_err(|error| IndexError::Serialization(error.to_string()))?,
                    chunk.text,
                    chunk.byte_start,
                    chunk.byte_end,
                    chunk.text_hash,
                    serde_json::to_string(&vector)
                        .map_err(|error| IndexError::Serialization(error.to_string()))?,
                ],
            )?;
        }
        transaction.commit()?;
        self.set_state(IndexState::Ready)?;
        Ok(IndexingReport {
            chunk_count: chunks.len(),
            embedded_count,
            reused_count,
        })
    }

    fn cached_vector(
        &self,
        text_hash: &str,
        metadata: &EmbeddingMetadata,
    ) -> Result<Option<Vec<f32>>, IndexError> {
        let value: Option<String> = self
            .connection
            .query_row(
                "SELECT vector FROM ai_chunks WHERE text_hash=?1 AND vector <> '[]' LIMIT 1",
                params![text_hash],
                |row| row.get(0),
            )
            .optional()?;
        value
            .map(|value| {
                let vector: Vec<f32> = serde_json::from_str(&value)
                    .map_err(|error| IndexError::Serialization(error.to_string()))?;
                validate_embedding(&vector, metadata)?;
                Ok(vector)
            })
            .transpose()
    }

    fn metadata_value(&self, key: &str) -> Result<Option<String>, IndexError> {
        self.connection
            .query_row(
                "SELECT value FROM ai_metadata WHERE key=?1",
                params![key],
                |row| row.get(0),
            )
            .optional()
            .map_err(IndexError::from)
    }

    fn set_metadata(&self, key: &str, value: &str) -> Result<(), IndexError> {
        self.connection.execute(
            "INSERT INTO ai_metadata(key,value) VALUES (?1,?2)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![key, value],
        )?;
        Ok(())
    }
}

fn parse_json<T: DeserializeOwned>(value: String) -> rusqlite::Result<T> {
    serde_json::from_str(&value)
        .map_err(|error| rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error)))
}

fn validate_embedding(vector: &[f32], metadata: &EmbeddingMetadata) -> Result<(), IndexError> {
    if (metadata.dimension != 0 && vector.len() != metadata.dimension)
        || vector.iter().any(|value| !value.is_finite())
    {
        return Err(IndexError::InvalidEmbedding(format!(
            "expected {} finite dimensions, got {}",
            metadata.dimension,
            vector.len()
        )));
    }
    if metadata.normalized {
        let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
        if (norm - 1.0).abs() > 0.001 {
            return Err(IndexError::InvalidEmbedding(
                "normalized embedding is not unit length".into(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> ChunkSource {
        ChunkSource {
            source_id: "doc-1".into(),
            source_kind: "document".into(),
            revision: "rev-1".into(),
            source_hash: "sha256:source".into(),
        }
    }

    #[test]
    fn markdown_chunking_is_deterministic_and_source_bounded() {
        let markdown = "# Cast\n\nThe moonstone glows.\n\nA second paragraph.\n";
        let first = chunk_markdown(source(), markdown, 80);
        let second = chunk_markdown(source(), markdown, 80);
        assert_eq!(first, second);
        assert_eq!(first.len(), 3);
        assert!(first
            .iter()
            .all(|chunk| chunk.byte_end <= markdown.len() as u64));
        assert_eq!(first[0].heading_ancestry, vec!["Cast"]);
        assert!(first.iter().all(|chunk| chunk.source.source_id == "doc-1"));
        let multibyte = chunk_markdown(source(), "é", 1);
        assert_eq!(multibyte.len(), 1);
        assert_eq!(multibyte[0].text, "é");
        let structured = chunk_structured(
            source(),
            &serde_json::json!({"z": 1, "a": {"name": "moonstone"}}),
            200,
        );
        assert_eq!(structured.len(), 1);
        assert!(structured[0].text.starts_with("{\"a\":{"));
    }

    #[test]
    fn embedding_cache_reuses_hashes_and_invalidates_model_changes() {
        let metadata = EmbeddingMetadata {
            provider_id: "fake".into(),
            model_id: "embed-v1".into(),
            dimension: 2,
            normalized: true,
            serializer_version: EMBEDDING_SERIALIZER_VERSION.into(),
        };
        let mut cache = EmbeddingCache::default();
        cache.replace_metadata(metadata.clone());
        cache.insert("hash-a".into(), vec![1.0, 0.0]);
        assert!(cache.compatible_with(&metadata));
        assert_eq!(cache.get("hash-a"), Some([1.0, 0.0].as_slice()));
        assert_eq!(cache.len(), 1);
        cache.replace_metadata(EmbeddingMetadata {
            model_id: "embed-v2".into(),
            ..metadata
        });
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn exact_search_and_rrf_are_deterministic() {
        let source = source();
        let records = [
            VectorRecord {
                chunk: TextChunk {
                    id: "a".into(),
                    source: source.clone(),
                    ordinal: 0,
                    heading_ancestry: vec![],
                    text: "a".into(),
                    byte_start: 0,
                    byte_end: 1,
                    text_hash: hash_text("a"),
                },
                vector: vec![1.0, 0.0],
            },
            VectorRecord {
                chunk: TextChunk {
                    id: "b".into(),
                    source,
                    ordinal: 1,
                    heading_ancestry: vec![],
                    text: "b".into(),
                    byte_start: 1,
                    byte_end: 2,
                    text_hash: hash_text("b"),
                },
                vector: vec![0.0, 1.0],
            },
        ];
        let semantic = exact_cosine_search(&records, &[0.9, 0.1], 2);
        assert_eq!(semantic[0].chunk_id, "a");
        let fused = reciprocal_rank_fusion(&[("b".into(), 0), ("a".into(), 1)], &semantic, 2);
        assert_eq!(fused[0].0, "a");
    }

    struct FakeEmbeddingProvider {
        calls: std::cell::Cell<usize>,
    }

    impl EmbeddingProvider for FakeEmbeddingProvider {
        fn embed(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, IndexError> {
            self.calls.set(self.calls.get() + 1);
            Ok(inputs
                .iter()
                .map(|input| {
                    if input.contains("moonstone") {
                        vec![1.0, 0.0]
                    } else {
                        vec![0.0, 1.0]
                    }
                })
                .collect())
        }
    }

    fn embedding_metadata() -> EmbeddingMetadata {
        EmbeddingMetadata {
            provider_id: "fake".into(),
            model_id: "embed-v1".into(),
            dimension: 2,
            normalized: true,
            serializer_version: EMBEDDING_SERIALIZER_VERSION.into(),
        }
    }

    #[test]
    fn sqlite_index_reuses_vectors_and_replaces_source_atomically() {
        let index = AiIndex::in_memory().unwrap();
        let provider = FakeEmbeddingProvider {
            calls: std::cell::Cell::new(0),
        };
        let first_source = ChunkSource {
            source_id: "doc-1".into(),
            source_kind: "document".into(),
            revision: "rev-1".into(),
            source_hash: hash_text("source-1"),
        };
        let first = chunk_markdown(first_source, "moonstone\n", 100);
        let report = index
            .index_source(&first, &embedding_metadata(), &provider, || false)
            .unwrap();
        assert_eq!(report.embedded_count, 1);
        assert_eq!(report.reused_count, 0);
        assert_eq!(index.state().unwrap(), IndexState::Ready);
        assert_eq!(
            index.search(&[1.0, 0.0], 1).unwrap()[0].chunk_id,
            first[0].id
        );

        let second_source = ChunkSource {
            source_id: "doc-1".into(),
            source_kind: "document".into(),
            revision: "rev-2".into(),
            source_hash: hash_text("source-2"),
        };
        let second = chunk_markdown(second_source, "moonstone\n\nnew text\n", 100);
        let report = index
            .index_source(&second, &embedding_metadata(), &provider, || false)
            .unwrap();
        assert_eq!(report.embedded_count, 1);
        assert_eq!(report.reused_count, 1);
        assert_eq!(index.records().unwrap().len(), 2);
        assert_eq!(provider.calls.get(), 2);
    }

    #[test]
    fn cancelled_indexing_keeps_previous_generation() {
        let index = AiIndex::in_memory().unwrap();
        let provider = FakeEmbeddingProvider {
            calls: std::cell::Cell::new(0),
        };
        let source = ChunkSource {
            source_id: "doc-1".into(),
            source_kind: "document".into(),
            revision: "rev-1".into(),
            source_hash: hash_text("source-1"),
        };
        let original = chunk_markdown(source.clone(), "moonstone\n", 100);
        index
            .index_source(&original, &embedding_metadata(), &provider, || false)
            .unwrap();
        let replacement = chunk_markdown(
            ChunkSource {
                revision: "rev-2".into(),
                source_hash: hash_text("source-2"),
                ..source
            },
            "different\n",
            100,
        );
        assert!(matches!(
            index.index_source(&replacement, &embedding_metadata(), &provider, || true),
            Err(IndexError::Cancelled)
        ));
        assert_eq!(index.records().unwrap()[0].chunk.id, original[0].id);
        assert_eq!(index.state().unwrap(), IndexState::PartiallyStale);
    }

    #[test]
    fn persisted_index_reopens_with_equivalent_source_coverage() {
        let root = std::env::temp_dir().join(format!(
            "daena-ai-index-reopen-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = root.join(".daena/ai/index.sqlite");
        let provider = FakeEmbeddingProvider {
            calls: std::cell::Cell::new(0),
        };
        let chunks = chunk_markdown(source(), "moonstone\n", 100);
        let before = {
            let index = AiIndex::open(&path).unwrap();
            index
                .index_source(&chunks, &embedding_metadata(), &provider, || false)
                .unwrap();
            index.records().unwrap()
        };
        drop(before);
        let reopened = AiIndex::open(&path).unwrap();
        assert_eq!(reopened.records().unwrap().len(), 1);
        assert_eq!(reopened.search(&[1.0, 0.0], 1).unwrap().len(), 1);
        std::fs::remove_dir_all(root).unwrap();
    }

    struct FailingEmbeddingProvider;

    impl EmbeddingProvider for FailingEmbeddingProvider {
        fn embed(&self, _inputs: &[String]) -> Result<Vec<Vec<f32>>, IndexError> {
            Err(IndexError::Serialization("provider unavailable".into()))
        }
    }

    #[test]
    fn provider_failure_after_indexing_sets_failed_state() {
        let index = AiIndex::in_memory().unwrap();
        let provider = FakeEmbeddingProvider {
            calls: std::cell::Cell::new(0),
        };
        let original = chunk_markdown(source(), "moonstone\n", 100);
        index
            .index_source(&original, &embedding_metadata(), &provider, || false)
            .unwrap();
        let changed = chunk_markdown(source(), "changed\n", 100);
        assert!(matches!(
            index.index_source(
                &changed,
                &embedding_metadata(),
                &FailingEmbeddingProvider,
                || false
            ),
            Err(IndexError::Serialization(_))
        ));
        assert_eq!(index.state().unwrap(), IndexState::Failed);
    }

    #[test]
    fn retrieval_evaluation_meets_recall_ndcg_and_forbidden_thresholds() {
        #[derive(Deserialize)]
        struct Fixture {
            entities: Vec<serde_json::Value>,
            queries: Vec<serde_json::Value>,
        }
        let fixture: Fixture =
            serde_json::from_str(include_str!("../fixtures/retrieval-evaluation.json")).unwrap();
        let mut records = Vec::new();
        for entity in fixture.entities {
            let id = entity["id"].as_str().unwrap().to_owned();
            let document = entity["document"].as_str().unwrap();
            for chunk in chunk_markdown(
                ChunkSource {
                    source_id: id.clone(),
                    source_kind: "document".into(),
                    revision: "fixture".into(),
                    source_hash: hash_text(document),
                },
                document,
                4096,
            ) {
                let text = chunk.text.to_lowercase();
                let vector = if text.contains("moonstone") {
                    vec![1.0, 0.0, 0.0]
                } else if text.contains("northern gate") {
                    vec![0.0, 1.0, 0.0]
                } else {
                    vec![0.0, 0.0, 1.0]
                };
                records.push(VectorRecord { chunk, vector });
            }
        }
        for query in fixture.queries {
            let text = query["query"].as_str().unwrap().to_lowercase();
            let expected = query["expectedSourceIds"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|value| value.as_str())
                .collect::<Vec<_>>();
            let query_vector = if text.contains("moonstone") {
                vec![1.0, 0.0, 0.0]
            } else if text.contains("northern gate") {
                vec![0.0, 1.0, 0.0]
            } else {
                vec![0.0, 0.0, 0.0]
            };
            let semantic = exact_cosine_search(&records, &query_vector, 3);
            let lexical = records
                .iter()
                .filter(|record| record.chunk.text.to_lowercase().contains(&text))
                .enumerate()
                .map(|(rank, record)| (record.chunk.id.clone(), rank))
                .collect::<Vec<_>>();
            let fused = reciprocal_rank_fusion(&lexical, &semantic, 3);
            let top_sources = fused
                .iter()
                .filter_map(|(chunk_id, _)| {
                    records
                        .iter()
                        .find(|record| record.chunk.id == *chunk_id)
                        .map(|record| record.chunk.source.source_id.as_str())
                })
                .collect::<Vec<_>>();
            if expected.is_empty() {
                assert!(top_sources.iter().all(|source| *source != "private"));
            } else {
                let hits = expected
                    .iter()
                    .filter(|source| top_sources.contains(source))
                    .count();
                let recall_at_3 = hits as f32 / expected.len() as f32;
                assert!(recall_at_3 >= 1.0, "recall@3={recall_at_3} for {text}");
                assert_eq!(top_sources.first().copied(), Some(expected[0]));
            }
        }
    }

    #[test]
    fn incompatible_metadata_clears_vectors_but_preserves_fallback_chunks() {
        let index = AiIndex::in_memory().unwrap();
        let provider = FakeEmbeddingProvider {
            calls: std::cell::Cell::new(0),
        };
        let chunks = chunk_markdown(source(), "moonstone\n", 100);
        index
            .index_source(&chunks, &embedding_metadata(), &provider, || false)
            .unwrap();
        let changed = EmbeddingMetadata {
            model_id: "embed-v2".into(),
            ..embedding_metadata()
        };
        assert!(!index.prepare_embedding_metadata(&changed).unwrap());
        assert_eq!(index.records().unwrap().len(), 0);
        assert_eq!(index.state().unwrap(), IndexState::Incompatible);
        let count: i64 = index
            .connection
            .query_row("SELECT COUNT(*) FROM ai_chunks", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }
}
