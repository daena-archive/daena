# ADR 0003: AI trust, privacy, and proposal lifecycle

- Status: Accepted
- Decided: 2026-08-08
- Consolidated: 2026-08-26

## Context

AI features combine untrusted project content, optional local or remote
providers, retrieved evidence, credentials, streaming output, and requests
from both the trusted shell and plugins. Treating a model as an authority or
letting provider concerns leak into plugin contracts would bypass Daena's
security and storage model.

## Decision

AI orchestration and provider access are host-owned. Rust constructs caller
identity from the trusted shell or an authorized plugin session. Context
retrieval is limited by the intersection of AI grants, normal data-read
grants, resource scope, project binding, and privacy policy. Plugins cannot
supply identity, credentials, provider transport, or project authority.

The provider-neutral AI crate owns contracts and deterministic policy but has
no dependency on the core, Tauri, Svelte, provider SDKs, or plugin runtime.
The application adapter owns network transport, provider configuration,
native event delivery, lifecycle, deadlines, cancellation, and diagnostics.
Provider implementations are injectable so the same bounded lifecycle can be
tested without a live service.

Prompt policy, authorized plugin guidance, user instructions, immediate
context, retrieved evidence, and output contracts remain distinct. Project
content, filenames, metadata, and model output are untrusted data. Models
receive no credentials, tools, filesystem access, ambient network authority,
or direct mutation path.

AI output is temporary until the user accepts it. Text, structured values,
and generated media pass through preview and explicit acceptance. Acceptance
revalidates the destination revision and uses the ordinary core operation.
Failure, cancellation, malformed output, stale state, or retrieval failure
cannot alter canonical project data.

Retrieval chunks, embeddings, lexical passage indexes, caches, and indexing
diagnostics are machine-local derived state. They are bounded,
provenance-bearing, independently rebuildable, and never required to open a
canonical project. Reuse is valid only when source identity and hash,
chunking, provider, model, serializer, dimensions, and other compatibility
metadata agree. Publishing a rebuilt source generation is atomic; cancellation
preserves the previous usable generation.

Provider selection is explicit. Daena never silently fails over between local
and remote providers. Remote project disclosure requires an exact consent
record for the project, provider, and endpoint. Remote endpoints must use a
validated HTTPS path that rejects embedded credentials, redirects, loopback,
private destinations, and DNS rebinding. Credentials live only in native
operating-system secret storage and never in projects, settings payloads,
plugins, prompts, webviews, or logs. Missing consent or credentials fails
closed.

## Threats and controls

The protected assets are project content, provider credentials and endpoints,
caller identity, generated proposals, and provenance. Principal threats are
forged authority, unauthorized retrieval, prompt injection, provider or secret
disclosure, unbounded output, server-side request forgery, and late events
crossing cancellation or revocation boundaries.

Controls are host-created caller scope, Rust authorization before retrieval,
isolated plugin transport, explicit disclosure policy, strict endpoint and
secret handling, bounded schemas and queues, provenance-bearing results,
revision-aware acceptance, and terminal lifecycle guards. Residual prompt
injection risk is accepted only because project content remains data and the
model has no independent authority or mutation path.

## Consequences

- AI can be unavailable without making the core or canonical project unusable.
- Shell and plugin entry points share policy but keep distinct authorization
  scopes.
- Local and remote transport differences do not change the public proposal
  model.
- Derived retrieval state may report stale and fall back to an authorized
  retrieval mode, but it cannot silently broaden access.
- Generated media enters canonical storage only through the normal asset
  registration and acceptance boundary.

## Decision history

- 2026-08-08: the host-owned trust boundary, explicit routing, layered prompt
  model, disposable retrieval state, and proposals-only lifecycle were
  accepted.
- 2026-08-08: local provider assembly, deterministic retrieval primitives, and
  remote privacy controls were applied under that boundary.
- 2026-08-26: the overlapping phase records were consolidated here; delivery
  status remains in the AI project documentation rather than this ADR.
