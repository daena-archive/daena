# ADR 0006: Phase 2 AI runtime assembly

- Status: Accepted
- Date: 2026-08-08

## Decision

`daena-ai` remains the provider-neutral contract and policy crate. The Tauri
application owns the LM Studio adapter because it owns loopback endpoint
configuration, native event delivery, and application lifecycle. The broker
does not construct provider requests directly: it validates the public RPC
payload, derives the session-bound caller, and invokes the injected AI runtime.

The provider seam is injectable in Rust tests. Production requests use the
Tauri-owned LM Studio adapter; deterministic broker tests use a loopback fake.
Both paths emit the same bounded `AiStreamEvent` lifecycle and pass through the
same structured schema validation, deadline, cancellation, and result rules.

## Consequences

- `daena-ai` contracts remain independent of Tauri and plugin runtime code.
- Provider I/O stays inside the trusted application boundary.
- Broker lifecycle tests do not require LM Studio or network access.
- Moving the adapter into `daena-ai` remains possible if its transport becomes
  runtime-independent; that change requires a new decision rather than a
  plugin-contract change.
