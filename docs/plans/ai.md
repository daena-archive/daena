# AI delivery plan

Product and architecture are in [`AI_INTEGRATION.md`](../ai/AI_INTEGRATION.md). This file is the unfinished delivery sequence. Do not treat a phase as done from this file alone; verify the worktree.

Historical delivery status: **architecture approved; Phase 0 is complete, Phase
1 implementation is present, Phase 2 broker implementation is present, Phase 3
implementation is complete, and Phase 4 provider-neutral vector primitives are
implemented in the worktree**. Phase 4 disposable-index persistence, structured
chunking, local-provider embedding rebuild, cancellation, reuse, hybrid search, and
shell status controls are now implemented; live local-provider embedding/rebuild and
rendered-control checks remain manual evidence. The `daena-ai` contracts,
deterministic fake-provider tests, hard limits, ADRs, bounded-stream transport
decision/tests, local-provider discovery, normalized streaming/error handling,
buffered event lifecycle, and trusted-shell rewrite preview path are present.
The remaining Phase 1 evidence is a rendered Tauri validation with a running LM
Studio instance; Phase 2 broker execution, conformance, lifecycle, deadline,
and oversized-output evidence is complete, while rendered Tauri/local-provider
validation remains pending; Phase 4 implementation evidence is present, while
live provider, rendered-control, and automatic watcher scheduling evidence
remains pending. Phase 5 remote-provider privacy controls are implemented in
the worktree with explicit consent, cross-platform native credential storage, strict
HTTPS/SSRF validation, redirect blocking, and no-silent-fallback routing;
live remote/keychain/rendered privacy evidence remains pending. Agents must
verify the worktree and
current source before relying on this status.

---

## 14. Delivery plan

Only one phase may be implemented at a time. A phase is not complete because
types compile or unit tests pass; every stated exit gate must have evidence.

### Phase 0 — contracts, threat model, and fake provider

Deliver:

- confirmation that [ADR 0003](../adr/0003-ai-trust-privacy-and-proposals.md)
  captures the AI trust boundary, provider routing/privacy, prompt/context
  model, RAG index placement, and proposal-only mutation;
- `daena-ai` crate skeleton with provider-neutral request, result, event, error,
  caller, policy, and provenance types;
- fake provider and cancellation/deadline primitives;
- explicit hard limits and normalized error semantics;
- a recorded transport decision and contract-level bounded-stream tests;
  running Tauri/trusted-shell and isolated-plugin-webview validation is
  explicitly deferred to Phases 1 and 2;
- contract fixtures for text and structured generation;
- documentation links from [`ARCHITECTURE.md`](../ARCHITECTURE.md) and relevant plugin/storage plans.

Do not add a live provider, embeddings, or project mutation in this phase.

**Exit gate:** architecture tests prove one terminal stream state, bounded output,
deadline/cancellation behavior, caller-scope construction, and zero dependency
from `daena-core` to `daena-ai`. The transport decision and threat model are
recorded, and all existing checks still pass.

### Phase 1 — trusted-shell local text generation

Deliver:

- provider registry and model capability discovery;
- local-provider adapter for OpenAI-compatible text generation and streaming;
- machine-local settings without credentials;
- local endpoint validation and clear unavailable/model-missing states;
- host-owned prompt builder with explicit context only;
- shell UI for one vertical slice: rewrite selected document text;
- streaming, cancel, diff, discard, and revision-checked acceptance;
- redacted diagnostics and deterministic Rust-level fake-provider rewrite-path
  tests; rendered fake-provider UI evidence is part of the remaining Tauri
  validation.

This phase is complete in the current implementation. Structured generation,
remote providers, and RAG are delivered in the later phases below; the phase
description remains as the historical delivery boundary.

**Exit gate:** in the rendered Tauri app, a user can configure a local provider,
rewrite a selection, cancel mid-stream, inspect a diff, and accept through the
normal document save path. Provider failure and document revision conflict cause
no data loss. The same workflow must also pass deterministically with the fake
provider; rendered evidence for both paths remains pending.

### Phase 2 — structured generation and plugin broker API

Deliver:

- canonical Rust AI capabilities and RPC request/result schemas;
- generated schema/TypeScript SDK/test-host updates;
- broker authorization using session-derived caller scope;
- bounded start/status/cancel/result lifecycle;
- strict structured-output validation;
- shared proposal UI for text and structured values;
- one bundled Lore action, such as biography or schema-compatible field
  suggestions, using only public SDK operations;
- allow/deny tests for every AI/data-capability combination.

Context remains explicit and caller-supplied from already-readable data. No
semantic index or implicit project-wide retrieval yet.

For `generate_text`, `immediateContext` is an object containing a non-empty
`selection` string. For `generate_structured`, `immediateContext` is arbitrary
caller-supplied JSON and the required `outputContract` defines the strict
result shape. Both requests are bounded by the host deadline and output limit;
the provider never receives broker identity or capability data.

**Exit gate:** a bundled plugin and an external conformance fixture can generate
text/structured proposals through the public broker without provider details or
private APIs. Missing AI grants, missing data grants, revoked sessions, invalid
schemas, oversized output, and cancellation all fail closed. Accepted data uses
existing revision-aware operations.

### Phase 3 — shared context builder and deterministic retrieval

Deliver:

- permission-aware `AiCaller`/resolved retrieval scope;
- context recipes for selected entity, document, fields, and one/two-hop
  relationships;
- passage-level lexical retrieval with stable provenance;
- prompt templates and token budgeting described in Section 6;
- citation inspector and stale-citation handling;
- synthetic retrieval/evaluation corpus;
- consistency-analysis result schema.

No embeddings are required in this phase. This deliberately proves access,
provenance, ranking, and UI before adding semantic complexity.

**Exit gate:** grounded generation cites exact authorized source ranges;
forbidden namespace markers never enter candidates, prompts, logs, or results;
citation links open the correct source; deterministic retrieval and prompt
fixtures are byte-stable; prompt-injection fixtures cannot trigger tools or
mutation.

### Phase 4 — embeddings and hybrid RAG

Deliver:

- embedding capability in the internal provider registry;
- deterministic Markdown/structured chunkers;
- `.daena/ai/index.sqlite` with version/model metadata;
- incremental/cancellable embedding pipeline;
- exact vector search baseline and hybrid rank fusion;
- index state/rebuild UI and lexical fallback disclosure;
- model-change invalidation and embedding reuse by chunk hash;
- retrieval quality and performance benchmarks.

`ai.embed` remains host-internal.

**Exit gate:** deleting `.daena/ai/` and reopening/rebuilding changes no canonical
file and recovers equivalent source coverage; edits re-embed only affected
chunks; incompatible models trigger rebuild; core project access works while the
AI index is absent/failed; the evaluation corpus meets thresholds recorded in
this document with zero unauthorized retrievals.

### Phase 5 — remote providers and production privacy controls

Current implementation:

- OS-backed secret storage;
- provider-neutral OpenAI-compatible local and HTTPS adapters;
- HTTPS/origin/redirect validation;
- host-owned exact provider/project/endpoint consent;
- cost/usage display when available;
- no-silent-fallback enforcement;
- credential/log redaction tests and remote-provider documentation.

Remote support was sequenced after local workflows and RAG access controls were
proven; it is now part of the current provider-neutral implementation.

**Exit gate:** credentials never reach frontend/plugin memory, project files,
prompts, logs, or exports; remote calls cannot occur without matching policy;
endpoint changes renew consent; local-only mode is enforced in Rust; all privacy
and redirect/SSRF tests pass.

### Phase 6 — image generation and optional multimodal support

The entity-scoped V1 image-generation slice is implemented:

- host-owned local ComfyUI discovery, submission, status, cancellation, and
  bounded candidate commands;
- visible Lore context selection plus optional text-AI prompt authoring;
- bounded temporary binary handles and preview/regenerate UX;
- MIME/size validation and canonical asset acceptance with portable provenance;
- a common Lore entity action without entity-type hardcoding.

Provider-neutral plugin image capabilities, project visual profiles, image
editing, reference images, vision analysis, and image embeddings remain later,
separately gated slices.

**V1 exit gate:** generated bytes cannot bypass limits or asset validation;
discard and cancellation leave no canonical state; acceptance produces a normal
revision-aware Daena asset; no provider URL or arbitrary filesystem path crosses
the generation UI boundary; there is no local-to-remote fallback.

### Future phase — agentic tools (not authorized by this plan)

Before any model can invoke tools or perform multi-step mutations, write a
separate architecture plan and obtain explicit approval. Its minimum scope must
include tool schemas, per-step capabilities, user confirmation policy, budgets,
loop/deadlock prevention, durable audit, transactional rollback, interruption,
and adversarial testing. Completion of Phase 6 does not authorize it.

---

## 15. Instructions for implementation agents

Agents implementing this plan must follow this protocol.

### 15.1 Before editing

1. Read this document completely and identify the one approved phase/slice.
2. Read `AGENTS.md`, [`ARCHITECTURE.md`](../ARCHITECTURE.md), [`PLUGIN_PLATFORM_PLAN.md`](./PLUGIN_PLATFORM_PLAN.md), and
   [`STORAGE.md`](../storage/STORAGE.md); read the [`ADR index`](../adr/README.md) and SDK guide.
3. Run `rtk git status --short` and preserve unrelated staged, unstaged, and
   untracked work.
4. Use codebase-memory graph tools first for symbols/call paths. Verify important
   details in current source and diff.
5. Inspect the previous phase's evidence. Do not assume a phase is complete from
   documentation alone.
6. Write a concrete vertical-slice plan: files, contracts, trust boundaries,
   tests, rendered behavior, and exit-gate evidence.
7. Do not start code until the user explicitly approves that implementation
   slice. Approval to edit this architecture document is not approval to
   implement every phase.

### 15.2 While implementing

- Keep Rust as the authority and treat TypeScript checks as advisory.
- Change Rust contract sources first; regenerate schemas/SDK outputs; never
  hand-edit generated contract files.
- Keep provider-specific types behind adapters.
- Use core APIs for project data; do not read canonical files from AI or plugin
  code.
- Carry broker-derived identity/scope through retrieval and streaming.
- Add allow and deny tests together.
- Keep unaccepted output machine-local and mutations proposal-only.
- Preserve request IDs, revisions, cancellation, bounded queues, and project
  lifecycle semantics.
- Do not add remote disclosure, tool calling, fallback, telemetry, retention, or
  persistence merely because a provider SDK makes it easy.
- If a phase exposes a missing prerequisite, stop and update the plan or add a
  superseding ADR with user approval rather than silently broadening scope.

### 15.3 Verification

Use focused tests plus the relevant full checks. The normal command forms are:

```text
rtk cargo fmt -- --check
rtk cargo test --workspace --locked --offline
rtk cargo clippy --workspace --locked --offline --all-targets -- -D warnings
rtk cargo test -p daena-ai --locked --offline
rtk npm run check
rtk npm run check:plugin-contract
rtk npm run test:plugin-conformance
rtk npm run test:plugin-transport
```

Run only commands supported by the current manifests/scripts and document any
sandbox or missing-cache limitation separately from source failures. AI UI
acceptance requires a rendered Tauri-native check; browser-only automation does
not prove native behavior. Live providers are optional verification, never the
only proof. `daena-ai` is intentionally a standalone crate rather than a
`daena-core` or Tauri dependency; its own `Cargo.lock` is therefore expected
and the explicit crate test above is part of standard Phase 0 verification.

For storage/RAG phases also prove:

- byte-identical canonical files before and after AI-index rebuild;
- recovery after removing only `.daena/ai/`;
- core project open while the AI index is absent or invalid;
- external-edit/revision conflict behavior;
- no prompt/output/embedding files appear in Git staging preview.

### 15.4 Phase completion report

A completion report must state:

- approved slice and files changed;
- contract/security decisions;
- tests and rendered scenarios run with exact results;
- exit-gate evidence item by item;
- known limitations or deferred work;
- current `rtk git status --short` summary;
- a short pasteable prompt for the next phase.

Do not say “Phase N complete” when any gate is untested, blocked, or only
inferred. Do not stage, commit, or push unless explicitly asked.

Suggested next-phase handoff format:

```text
Read [`AI_INTEGRATION.md`](../ai/AI_INTEGRATION.md) and implement only Phase N, <named vertical slice>.
First inspect the current worktree and verify Phase N-1 evidence. Present a
file-level plan and wait for explicit approval before coding. Preserve unrelated
changes. Use Rust-owned contracts/authorization, regenerate derived SDK files,
and prove every listed exit gate with focused tests plus the relevant full
checks. Do not start Phase N+1.
```

---
