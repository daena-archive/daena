# ADR 0010: Phase 4 provider-neutral vector baseline

- Status: Accepted
- Date: 2026-08-08

## Decision

Phase 4 begins with provider-neutral primitives in `crates/daena-ai`: a
deterministic Markdown block chunker, normalized embedding metadata, exact
cosine search, hash-keyed embedding reuse, and reciprocal-rank fusion of
lexical and semantic ranks. Markdown documents and structured field/relationship
records use deterministic chunkers. Chunk identities include source identity, source
hash, chunker version, byte range, and normalized text hash. Heading ancestry
is retained as derived chunk metadata while byte ranges continue to refer to
the original canonical source bytes.

The first slice does not add a provider, Tauri command, plugin capability, or
canonical storage dependency. `AiIndex` provides the disposable SQLite storage
primitive for the host-owned `.daena/ai/index.sqlite`; cancellable runtime
embedding and indexing orchestration are host-owned Phase 4 runtime behavior. A
model/provider/serializer/dimension change clears reusable embeddings
and makes the semantic index require rebuilding.

## Consequences

- Vector ranking and hybrid fusion are deterministic and testable offline.
- `daena-core` remains usable when AI is absent or unavailable.
- Embedding reuse is safe only when metadata is compatible and chunk hashes
  match.
- The host adapter normalizes LM Studio vectors before validation, batches
  requests, reuses compatible chunk hashes, and publishes each source
  generation atomically. Rebuild cancellation preserves the prior generation
  and reports a stale state for lexical fallback.
- Trusted-shell `ai_index_search` now exercises semantic search and lexical plus
  reciprocal-rank fusion over derived runtime search views; plugin retrieval remains
  broker-authorized and cannot call this shell-only command.
- The fixture benchmark threshold is Recall@3 >= 1.0 and nDCG@3 >= 1.0 for the
  two authorized queries, with zero forbidden-source candidates for the private
  marker query. Offline evidence is 20 `daena-ai` tests, including persistence
  reopen, cancellation, model invalidation, structured chunking, exact cosine
  ordering, fusion, and the corpus benchmark; the Tauri suite has 38 passing
  tests and the shell check is clean.
- Canonical watcher/reconciliation events do not start background embedding
  work yet. The manual rebuild is incremental by chunk hash and cancellable;
  automatic post-edit scheduling remains a documented follow-up slice.
- The remaining release evidence is a live LM Studio `/embeddings` rebuild and
  rendered verification of the index controls. Phase 4 must not be called
  exit-gated until that manual check passes.
