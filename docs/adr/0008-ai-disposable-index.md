# ADR 0008: AI retrieval state is disposable

- Status: Accepted
- Date: 2026-08-08

## Decision

Future chunks, embeddings, lexical passage indexes, and indexing diagnostics
live under `.daena/ai/`, separate from the core index. They are derived,
machine-local, rebuildable state and never block canonical project opening.
`daena-core` owns canonical files and does not depend on `daena-ai`.

Phase 0 defines the boundary only. The index and retrieval implementation are
Phase 3/4 work.

