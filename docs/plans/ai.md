# AI remainder

Product and architecture are in [`AI_INTEGRATION.md`](../ai/AI_INTEGRATION.md).
This file is unfinished evidence and later gated slices. Verify the worktree
before treating any item as done.

## Pending evidence

Implementation for local text, brokered plugin AI, retrieval, remote-provider
privacy controls, and V1 ComfyUI image generation is present. Still pending:

- rendered Tauri validation of local rewrite with a running LM Studio instance;
- rendered Tauri / live local-provider checks for brokered plugin AI;
- live local-provider embedding rebuild, rendered retrieval-control checks, and
  automatic watcher scheduling;
- live remote / keychain / rendered privacy evidence.

## Later gated image slices

Not part of V1. Each needs its own delivery:

- provider-neutral plugin image capabilities;
- project visual profiles;
- image editing;
- reference images;
- vision analysis;
- image embeddings.

## Agentic tools

Not authorized. Before any model can invoke tools or perform multi-step
mutations, write a separate architecture plan and obtain explicit approval.
Minimum scope: tool schemas, per-step capabilities, user confirmation policy,
budgets, loop/deadlock prevention, durable audit, transactional rollback,
interruption, and adversarial testing.
