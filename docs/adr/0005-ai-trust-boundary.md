# ADR 0005: AI uses a host-owned trust boundary

- Status: Accepted
- Date: 2026-08-08

## Decision

AI orchestration and provider access belong to the trusted Rust application
boundary. Every request carries an `AiCaller` constructed by the host from the
trusted shell or an already-authorized plugin session. Plugin payloads cannot
provide caller identity, project identity, capabilities, credentials, or
provider transport details. Context access is resolved before retrieval and is
the intersection of AI grants, ordinary data-read grants, resource scopes,
session binding, and host privacy policy.

`daena-ai` is runtime-neutral and must not depend on `daena-core`, Tauri,
Svelte, a provider SDK, or plugin runtime code. `daena-core` therefore remains
usable when AI is disabled or unavailable.

## Consequences

The host owns provider routing, cancellation, deadlines, limits, diagnostics,
and disclosure policy. Plugins receive semantic AI operations only. Rust
authorization is authoritative; frontend checks are advisory.

## Phase 0 threat-model record

This ADR is also the Phase 0 threat-model record. The assets are project
content and metadata, provider credentials and endpoints, caller/session
identity, generated proposals, and provenance. The principal threats are
prompt-injection content, forged plugin/session/project identity, unauthorized
context retrieval, provider or credential disclosure, unbounded provider
output, and late events crossing cancellation or revocation boundaries.

The controls are host-created caller scope, Rust authorization before
retrieval, isolated plugin transport, no model tools or direct mutation,
provider-neutral host routing, explicit remote-disclosure policy, bounded
schemas/context/output/event queues, provenance-bearing results, and
cancellation/deadline/revocation handling. Residual prompt-injection risk is
accepted only because project content remains data and the model receives no
tools, credentials, filesystem access, network authority, or mutation path.
