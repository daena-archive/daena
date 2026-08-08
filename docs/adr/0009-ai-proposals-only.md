# ADR 0009: AI output is a proposal until accepted

- Status: Accepted
- Date: 2026-08-08

## Decision

Generated text, structured values, and later image bytes are temporary
application state. The lifecycle is streaming, complete, editable preview,
then accepted, discarded, or expired. Acceptance rechecks the target revision
and uses the existing revision-aware core/document/field/asset operation. AI
requests never mutate canonical project files directly.

Provider failures, cancellation, malformed output, stale revisions, and index
failures leave canonical data unchanged.

