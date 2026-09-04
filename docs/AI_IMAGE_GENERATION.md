# AI image generation V1

## Scope

Daena V1 provides entity-scoped, local text-to-image generation through a
user-managed ComfyUI server. It does not expose a general ComfyUI graph editor,
ship a model runtime, use reference images, or fall back to a hosted provider.

The implemented workflow is:

```text
entity action
  -> visible context selection
  -> optional existing text-AI prompt authoring
  -> user-reviewed prompt
  -> controlled local ComfyUI workflow
  -> expiring temporary candidates
  -> explicit accept
  -> canonical entity asset plus provenance
```

## Configuration

Application settings contain independent text and image provider profiles. The
image profile is `AiSettings.imageProvider` and contains only machine-local
provider configuration: enabled state, provider identity, adapter, endpoint,
and selected model. This is an additive field in settings format version 1, so
existing text-provider settings remain intact and image generation starts
disabled.

The V1 adapter accepts only `http` loopback endpoints. It rejects credentials,
queries, fragments, non-loopback hosts, redirects, and unknown adapter kinds.
Connection checking uses ComfyUI `system_stats`; capability discovery verifies
the required built-in nodes and reads checkpoint, sampler, and scheduler choices
from `object_info`, with `models/checkpoints` used when available.

## Controlled workflow

Daena submits a fixed API-format graph composed of:

- `CheckpointLoaderSimple`;
- positive and negative `CLIPTextEncode` nodes;
- `EmptyLatentImage`;
- `KSampler`;
- `VAEDecode`; and
- `SaveImage`.

The user controls prompt, optional negative prompt, model, width, height, seed,
output count, steps, guidance, sampler, and scheduler. The host validates every
value against hard limits and the discovered provider choices. Daena never
accepts an arbitrary workflow graph from the frontend.

## Context and prompt authoring

The Lore action starts with a small default context: entity identity plus
non-empty structured fields whose names indicate appearance, clothing, species,
culture, occupation, era, architecture, or location. Article text, relations,
and map locations are visible but opt-in.

Prompt actions reuse the existing text provider and include build from entity,
build from selected context, rewrite, add detail, and simplify. Retrieval is
disabled for these calls so a remote text provider receives only the context
shown in the dialog. The final prompt is always editable and is never submitted
to ComfyUI without an explicit Generate action.

## Temporary candidate lifecycle

Generation runs outside the Tauri command lifetime. Jobs are bound to the open
project, expire after 15 minutes of inactivity, and expose queued, running, downloading,
completed, failed, and cancelled states. Daena polls ComfyUI history and queue
state, downloads output bytes itself, detects MIME type from bytes, and permits
only PNG, JPEG, and WebP within per-image and aggregate byte budgets.

Candidate handles remain host-owned. The frontend receives bytes only through a
project-bound command. Closing, discarding, project close, and expiry remove
temporary state. Cancellation stops provider work; the status handle remains
until normal cleanup. Accepted assets are not deleted when their job is discarded.

## Acceptance and provenance

Acceptance is the only operation that crosses from temporary generation state
into project state. It registers the bytes through the canonical asset service
as an attachment owned by the originating entity. Multiple candidates can be
accepted independently.

The asset `provenance` object stores:

- final prompt and optional negative prompt;
- manual or LLM-assisted prompt method and edit state;
- text provider and model when used;
- local image provider, adapter, and model identifier;
- seed, dimensions, output count, steps, guidance, sampler, and scheduler;
- selected context labels and entity IDs;
- source request and candidate IDs; and
- creation timestamp.

Provenance is a bounded JSON object in the runtime asset row and portable
`entities/<entity-id>/assets.json`. It participates in asset/entity revisions,
checkpoint export, and rebuild after deleting `.daena/`.

## Limits and errors

V1 limits prompts to 16 KiB, context to 64 items, outputs to four, dimensions to
multiples of eight between 64 and 4096 with a 16-megapixel ceiling, individual
images to 32 MiB, and all outputs to 96 MiB. At most two jobs may be active and
four retained at once. Generation has a 15-minute deadline.

The host distinguishes unavailable/connection failures, missing models,
unsupported capabilities, invalid configuration, insufficient resources,
provider errors, and authentication failures. Failure preserves all dialog
inputs for retry, and no error path selects another provider.
