# ADR 0001: Plugin platform boundary

- Status: Accepted
- Decided: 2026-08-03
- Consolidated: 2026-08-26

## Context

Daena supports first-party modules and installable third-party plugins, but
project integrity and user privacy cannot depend on plugin behavior. Plugins
also need to exchange data without creating private storage silos or importing
one another's runtime code.

## Decision

Plugins are declarative or sandboxed. Untrusted JavaScript never runs in the
trusted application webview. Sandboxed UI runs at an application-controlled
origin without access to the host DOM or ambient Tauri, filesystem, process,
shell, dialog, clipboard, or network authority. Optional background execution
uses bounded WebAssembly without ambient WASI authority. Native extensions are
outside the plugin contract.

Rust is the sole authorization boundary. The host creates each plugin session
and binds it to the installed package, originating runtime, current project,
grants, activation generation, expiry, and revocation state. A plugin cannot
assert its own identity or authority. The broker validates the session,
origin, payload, capability, resource scope, revision, and request identity
before invoking a core operation. Frontend checks are advisory.

The core owns entities, documents, relationships, assets, search, storage, and
project lifecycle. Plugins own schemas, namespaces, templates, and
presentation, but plugin-authored data remains in the core project model. One
plugin owns each namespace. Interoperability uses stable entity IDs,
explicitly shared fields, versioned events, and versioned services; plugins do
not share live objects or import another plugin's code.

Plugin packages are deterministic archives whose manifest and contents are
validated and covered by a digest before execution. Installation, enablement,
capability grants, and project data are independent state. Unsigned local
packages require explicit user consent and visible trust labeling. A future
registry or publisher signature may inform installation trust but cannot
become an authorization dependency.

Bundled modules use the same public plugin contract. Application-shell
features that require privileged host access remain host features rather than
privileged plugins.

## Consequences

- Plugin clients are broker-backed and never receive the trusted project
  client or arbitrary native commands.
- Mutable calls use opaque revisions, and retryable calls keep their broker
  request identity so authorization and idempotency survive every layer.
- Disablement, upgrade, uninstall, project close, and application restart
  revoke affected sessions without deleting plugin-owned project data.
- Events are bounded runtime notifications, not a durable integration log.
  Services have explicit schemas, versions, deadlines, cancellation, and
  cycle protection.
- The public SDK remains framework-neutral; Svelte is an implementation choice
  for Daena's own UI.

## Decision history

- 2026-08-03: isolation, Rust authority, package integrity, shared core data
  ownership, and versioned interoperability were accepted together.
- 2026-08-26: four overlapping records were consolidated here without changing
  the boundary.
