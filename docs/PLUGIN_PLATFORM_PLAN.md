# Third-party plugin platform architecture and implementation plan

## Status and purpose

This document is the architecture decision and delivery plan for evolving
Daena Archive's build-time module system into a third-party plugin platform. It
refines the runtime-extension architecture in `ARCHITECTURE.md` where this
document is more specific. It does not change the canonical
project data model described there.

Plugin authors should use the [definitive plugin authoring guide](PLUGIN_SDK.md)
for the current manifest, SDK, testing, packaging, installation, and lifecycle
workflow. This plan remains the architectural record and phase authority.

The canonical-storage Phase 5 contract is now reflected across this platform:
broker reads carry opaque revisions, mutable calls require `expectedRevision`,
the RPC envelope owns retry `requestId`s, and the host administration view
reports lifecycle, selection, grants, installed versions, rollback state, and
dependency resolution from Rust-owned state.

The current implementation uses Rust-owned broker authority for plugin
identity, capabilities, sessions, revisions, request IDs, and project data.
Bundled and third-party plugin UIs run in isolated webviews and communicate
through the versioned RPC contract. Frontend checks remain advisory; the Rust
broker is the enforcement boundary.

For the future AI broker surface, see [`AI_INTEGRATION.md`](./AI_INTEGRATION.md).
AI grants do not imply project-data grants, provider access, or network access.

The target platform has one non-negotiable rule:

> Every third-party operation is attributed to a host-assigned plugin identity
> and authorized by Rust at the point where it reaches core data or an operating
> system resource.

## Decisions

### 1. Extension classes and trust levels

Daena Archive will support three extension classes:

1. **Declarative plugins** contribute schemas, templates, commands, menus, and
   projections expressed as data. They contain no executable plugin code and
   are the preferred extension type.
2. **Sandboxed plugins** contain a UI bundle and, optionally, a background WASM
   component. Their only access to Daena Archive is a versioned, brokered API.
3. **Trusted native extensions** may be considered later for functionality that
   WASM cannot provide. They are out of scope for the first public platform and
   must be presented as equivalent to installing a native application, not as
   sandboxed plugins.

Arbitrary third-party JavaScript will never execute in the main application
webview. Native dynamic libraries will not be supported by the initial plugin
platform.

First-party plugins will use the same manifest, broker, lifecycle, and public
SDK as third-party plugins. They may be bundled and signed by the application,
but they will not receive private data APIs. The trusted application shell is
not a plugin and keeps a separate privileged Tauri command surface.

### 2. Runtime isolation

Plugin UI runs in a sandboxed webview on a distinct application-controlled
origin. It receives no Tauri API, no access to the host DOM, and no ambient
filesystem, process, clipboard, dialog, shell, or network authority. A strict
Content Security Policy is mandatory for the host and plugin webviews.

The host and UI exchange structured messages over one brokered channel. The
host validates the sender webview, session, method, payload schema, payload
size, and capability before forwarding a request.

Background plugin logic runs as WebAssembly using WASI with no preopened
directories, inherited environment, network sockets, clocks, randomness, or
process APIs unless a specific host function grants the operation. Each
instance has memory, execution-time, and fuel limits. A plugin that repeatedly
exceeds limits is stopped and marked failed.

UI plugins may ship framework-generated static assets, but the SDK contract is
framework-neutral. Svelte is recommended, not required.

### 3. Authority and backend boundary

Rust is the only authority boundary. TypeScript capability helpers exist for
developer feedback but are not enforcement.

The trusted shell continues to use privileged commands that are available only
to the main webview. Plugin webviews receive only bootstrap and broker access.
The plugin API is exposed through a narrow operation such as:

```text
plugin_rpc(session_id, request_id, method, payload)
```

The host creates `session_id`; plugin code cannot choose its identity. A
session is bound to:

- installed plugin ID and package digest;
- plugin version and API version;
- current project ID;
- originating runtime/webview instance;
- granted capabilities and scopes;
- activation generation; and
- expiry and revocation state.

Every broker method verifies the session before calling a core service.
Disabling, upgrading, uninstalling, closing the project, or restarting the
plugin revokes its sessions immediately.

Raw Tauri project, migration, Git, backup, restore, file-dialog, opener, and
filesystem commands are never available to plugin webviews. Plugin requests do
not accept a caller-supplied plugin ID when the identity can be derived from the
session.

### 4. Capabilities and resource scopes

Capabilities are host-defined identifiers with documented payload and resource
scopes. A manifest requests capabilities; it does not grant them. Installation
records the user's grants, and the Rust broker enforces them.

The initial capability vocabulary is:

| Capability                           | Scope and meaning                                                                      |
| ------------------------------------ | -------------------------------------------------------------------------------------- |
| `entity.read`                        | Read live core entities in the current project.                                        |
| `entity.write`                       | Create and update entities; delete requires `entity.delete`.                           |
| `entity.delete`                      | Soft-delete entities after explicit host UI confirmation when initiated interactively. |
| `document.read`                      | Read documents attached to visible entities.                                           |
| `document.write`                     | Create or update documents.                                                            |
| `field.read:self`                    | Read fields in namespaces owned by the caller.                                         |
| `field.read:shared`                  | Read fields explicitly exported by another plugin.                                     |
| `field.write:self`                   | Write schema-valid fields in namespaces owned by the caller.                           |
| `relationship.read`                  | Read relationships involving visible entities.                                         |
| `relationship.write`                 | Create relationships using registered relationship types.                              |
| `asset.read:self`                    | Read metadata for caller-owned assets; bytes require an explicit broker request.       |
| `asset.register`                      | Register a plugin-supplied asset into a caller-owned namespace.                       |
| `search.query`                       | Query the core search service.                                                         |
| `event.publish:<type>`               | Publish a declared event type.                                                         |
| `event.subscribe:<type>`             | Subscribe to a declared event type.                                                    |
| `service.provide:<name>`             | Register a declared service implementation.                                            |
| `service.call:<name>`                | Call a declared service.                                                               |
| `network:<origin>`                   | Make brokered requests to an approved HTTPS origin. Not in the first release.          |
| `clipboard.read` / `clipboard.write` | Brokered clipboard operations with host policy. Not granted by default.                |

There is no generic `filesystem`, `shell`, `process`, `dialog`, `tauri`, or
unrestricted `network` capability. File import/export uses host-owned dialogs
and handles; plugins never receive arbitrary local paths.

Capabilities are denied by default. Read access to core entities is project
wide because shared entities are the product's integration model. Structured
plugin data remains private to its namespace unless its schema explicitly
exports fields as shared.

### 5. Canonical manifest

There will be one JSON manifest per package. The Rust `daena-plugin-api` crate
owns the contract types; the versioned JSON Schemas and the TypeScript SDK
declarations are generated from those Rust types by `npm run gen:plugin-contract`
and are build artifacts, not hand-edited sources. Handwritten duplicate contract
types are removed. Cross-reference rules (namespace ownership, migration
contiguity, template-field typing) stay handwritten in Rust's
`validate_manifest` and are mirrored in the TypeScript `validatePluginManifest`,
with parity enforced by a dual-validator conformance test over a shared fixture
battery. See [ADR 0006](adr/0006-rust-first-contract-generation.md).

The initial manifest contains:

```json
{
  "manifestVersion": 1,
  "id": "com.example.genealogy",
  "name": "Genealogy",
  "version": "1.2.0",
  "publisher": "com.example",
  "hostApi": ">=1.0.0 <2.0.0",
  "kind": "sandboxed",
  "entrypoints": {
    "ui": "dist/ui/index.html",
    "wasm": "dist/service.wasm"
  },
  "capabilities": ["entity.read", "field.write:self"],
  "dependencies": {},
  "namespaces": [],
  "schemas": [],
  "templates": [],
  "views": [],
  "commands": [],
  "services": { "provides": [], "consumes": [] },
  "events": { "publishes": [], "subscribes": [] },
  "migrations": []
}
```

Rules:

- IDs are lowercase reverse-domain identifiers and are immutable.
- Semantic Versioning is used for plugin and host API versions.
- Entrypoints are package-relative paths and cannot escape the package root.
- Namespace, service, event, view, command, and migration identifiers are
  unique within the package; globally addressable identifiers are prefixed by
  the plugin ID.
- Unknown manifest keys are rejected for manifest version 1. Future optional
  features arrive through a new manifest version or explicitly versioned
  extension blocks.
- A package digest covers the manifest and every packaged file. The manifest's
  migration checksums are part of that digest.
- Enabled state and granted capabilities are not stored in the package
  manifest; they belong to host/project state.

### 6. Package format, installation, and trust

Plugin packages use the `.wbplugin` extension and are deterministic ZIP
archives containing the manifest, static UI assets, optional WASM, schemas,
migrations, licenses, and signature metadata.

The installer performs these steps before any code executes:

1. Copy the package into a staging directory owned by Daena Archive.
2. Enforce compressed/uncompressed size, file-count, and path-length limits.
3. Reject absolute paths, `..`, duplicate normalized paths, symlinks, hard
   links, device files, and case-colliding paths.
4. Validate the manifest and all referenced files and schemas.
5. Calculate and record the package digest.
6. Verify a signature when present and display publisher trust status.
7. Resolve host API and plugin dependencies.
8. Show requested capabilities and changes from the previously installed
   version.
9. Install atomically into an app-owned, versioned directory.
10. Activate only after the project-specific enable action succeeds.

The first release supports local packages signed or unsigned. Unsigned packages
are clearly marked and require an explicit install confirmation. A future
registry adds publisher verification and revocation, but the runtime does not
depend on a marketplace.

Installed packages are global to the application profile. Enablement,
capability grants, data versions, and failure state are per project. Multiple
versions may be retained for rollback, but exactly one version of a plugin is
active in a project.

Uninstall removes installed code only after no project actively uses that
version. Plugin-owned project data is preserved by default and can be deleted
only through a separate, explicit data-removal flow with backup.

### 7. Plugin-to-plugin interaction

Plugins never import one another's runtime code and never receive direct
references to another plugin's objects. Interaction is mediated by the host in
three ways:

1. **Shared core data** uses stable entity IDs, relationships, and explicitly
   shared fields.
2. **Events** provide asynchronous, one-way notification.
3. **Services** provide versioned request/response operations.

#### Dependencies

A dependency declares a plugin ID, Semantic Version range, and whether it is
required or optional. Required dependencies must resolve before activation.
Optional dependencies expose feature availability through the SDK. Dependency
cycles are rejected, including cycles containing only service dependencies.
Activation follows a topological order and deactivation follows the reverse
order.

#### Events

Event names are globally qualified, for example
`com.example.timeline/event-created@1`. Each event version has a JSON Schema.
Publish and subscribe rights are declared separately and require grants.

Delivery is asynchronous and at-most-once within the active application
session. Events are not a durable job queue. Subscribers must re-query core
state when they need authoritative data. The broker enforces schema validation,
payload limits, per-plugin queue limits, and rate limits. A slow subscriber
cannot block the publisher or core transaction.

Core events use the `daena.core` namespace and are emitted only after the
corresponding database transaction commits. Event payloads contain stable IDs
and minimal change metadata, not complete private records.

#### Services

A service has a globally qualified name and major version, plus a schema for
each method's request, success result, and declared errors. Only one provider
for a service major version is active in a project. A consumer calls through
the broker, which verifies both parties, capabilities, schemas, payload limits,
and dependency declarations.

Calls have deadlines and cancellation. They do not run inside a core database
transaction. The broker detects re-entrant service-call cycles per request and
returns an error instead of deadlocking. Provider failure is returned as a
typed `provider-unavailable` error. Optional consumers must degrade gracefully.

Services are appropriate for domain calculations or projections. Core storage
operations remain core APIs and cannot be replaced by plugin services in the
first release.

### 8. Lifecycle and failure behavior

The lifecycle state machine is:

```text
discovered -> validated -> installed -> resolved -> activating -> active
                                      \-> incompatible
active -> deactivating -> resolved
activating/active/deactivating -> failed -> resolved or quarantined
installed/resolved -> uninstalling -> removed
```

Activation is project-scoped and consists of:

1. Resolve API, dependencies, grants, and namespace ownership.
2. Back up the project if migrations are required.
3. Apply declared migrations transactionally.
4. Start the isolated runtime and create a session.
5. Register views, commands, event subscriptions, and services.
6. Mark the plugin active only after every required step succeeds.

Failed activation removes registrations, revokes the session, stops the
runtime, and leaves the plugin disabled. Successfully committed migrations are
not automatically reversed; package upgrade rollback must use a declared
forward recovery migration or restore the pre-upgrade project backup.

Deactivation revokes new requests first, cancels in-flight work, unregisters
services/subscriptions/commands/views, waits for a short bounded grace period,
then terminates the runtime. Project close and application shutdown use the same
path. Plugin code cannot veto shutdown.

Three crashes or resource-limit terminations during one application session
quarantine the plugin for that project until the user explicitly retries. The
host records bounded diagnostic information without including document bodies
or plugin-private data by default.

### 9. Data ownership, schemas, and migrations

The core continues to own the database, entity graph, documents, relationships,
assets, and search index. Plugins own meaning, schemas, and presentation—not
database connections or private database files.

Every namespace has exactly one owning plugin ID. A plugin may define multiple
namespaces. Field reads and writes are checked against registered schemas and
ownership in Rust. A schema can mark individual fields as shared read-only data
for other plugins; other plugins never write them.

Relationship types are registered and qualified by owner. Relationships may
refer to any visible entity, which is the supported cross-plugin linking
mechanism.

Migrations are declarative and packaged. Runtime code cannot submit arbitrary
migration JSON. The plugin manager selects the exact migration chain from the
installed, digest-verified manifest and verifies:

- plugin identity and namespace ownership;
- contiguous stored-data versions;
- migration ID uniqueness and recorded checksum;
- operation/schema validity;
- destructive-operation recovery policy; and
- compatibility with the target package version.

Migrations execute in one SQLite transaction after a host-owned backup is
successfully created. The migration history records plugin ID, package digest,
migration ID, from/to data versions, operation checksum, and timestamp.

Disabling or uninstalling code does not delete plugin data. Export/import
preserves unknown plugin namespaces, manifests needed to identify their owner,
module state, grants, and migration history. Imported projects do not
automatically install or execute missing plugins.

### 10. API and compatibility policy

The public host API follows Semantic Versioning:

- patch releases fix behavior without changing schemas;
- minor releases add optional methods, fields, capabilities, or event/service
  versions; and
- major releases may remove or change contracts.

The manifest declares a supported host API range. Installation rejects an empty
intersection with the current host. A plugin may probe optional features using
SDK feature discovery; it must not infer support from application version.

RPC methods and data structures are explicitly versioned. Unknown methods and
fields fail closed unless that schema marks them extensible. Deprecated APIs
remain available for at least one host major release, with diagnostics during
development and packaging. Revision conflicts are typed broker failures rather
than best-effort overwrites; request IDs are retained for retryable mutations.

Stored data version is independent of package and host API versions. Downgrades
are allowed only when the target package declares compatibility with the
current stored data version; otherwise rollback requires restoring the backup.

### 11. Concurrency and resource limits

The Rust core exposes asynchronous application services. SQLite access remains
serialized per project initially, but blocking database and filesystem work is
run outside Tauri's async event-loop threads. Transactions are short and never
wait on plugin code or plugin-to-plugin calls.

The broker applies configurable hard limits with conservative defaults:

- maximum RPC and event payload size;
- maximum concurrent calls per plugin and per project;
- bounded event queues;
- service deadlines;
- WASM memory and fuel limits;
- UI and service startup timeouts; and
- rate limits for expensive search, asset, and future network operations.

Cancellation propagates from project close, plugin disable, caller cancellation,
and deadlines. Core mutations use request IDs for safe retry where an operation
could otherwise be duplicated.

### 12. User experience and administration

The application provides a plugin administration surface showing:

- installed and project-enabled plugins;
- version, publisher, signature, and package digest;
- compatibility and dependency state;
- requested and granted capabilities with plain-language descriptions;
- owned namespaces and provided/consumed services;
- update capability changes and migration requirements;
- runtime health, last failure, and retry action; and
- disable, rollback, uninstall-code, and separately confirmed delete-data
  actions.

Capability changes on upgrade require renewed consent. Non-sensitive additive
changes such as new views do not. A plugin cannot draw or phrase the host-owned
permission dialog.

## Required code structure

Refactor the backend into explicit boundaries:

```text
crates/
  daena-core/          # Project model and application services
  daena-plugin-api/    # Manifest, RPC, capability, event/service types
  daena-plugin-host/   # Catalog, resolver, sessions, broker, runtimes
src-tauri/                    # Trusted Tauri adapter and application assembly
packages/
  plugin-sdk/                 # Generated types and framework-neutral client
  plugin-test-host/           # Fake broker and conformance helpers
  modules/                    # Bundled plugins using only the public SDK
schemas/
  plugin-manifest-v1.json
  plugin-rpc-v1.json
```

`daena-core` must not depend on Tauri or plugin runtime implementations.
It exposes typed services for entities, documents, fields, relationships,
assets, search, migrations, and project lifecycle. The Tauri shell adapter and
plugin broker both call these services with different authority contexts.

`daena-plugin-host` contains:

- `PluginCatalog` for installed packages and retained versions;
- `ManifestValidator` and package verifier;
- `DependencyResolver`;
- `GrantStore`;
- `PluginManager` lifecycle coordinator;
- `SessionRegistry` with revocation and activation generations;
- `Broker` with method-level authorization and schema validation;
- `EventBus` and `ServiceRegistry`;
- UI-webview and WASM runtime adapters; and
- bounded audit/diagnostic records.

The existing `ModuleContext` becomes an SDK client backed by broker RPC. It no
longer imports the trusted `project` Tauri client. The existing frontend
`ModuleRegistry` becomes host UI state fed by the Rust plugin manager rather
than the source of truth.

## Delivery plan

### Phase 0: Specify and lock the public contract

Create the manifest and RPC contract types in Rust, generate the JSON Schemas
and TypeScript declarations from them, and define the capability registry,
lifecycle state machine, and error model. Add ADRs for isolation, package
trust, plugin-to-plugin communication, and data ownership. Update
`ARCHITECTURE.md` to link to this document.

**Exit gate:** One canonical Lore manifest validates identically in Rust and
TypeScript; JSON Schemas and TypeScript declarations are generated from the
Rust contract types, and there are no handwritten duplicate contract types.

### Phase 1: Extract the Rust core

Move project behavior out of Tauri command handlers into `daena-core`.
Introduce an authority context on operations that require resource ownership,
without yet changing trusted-shell behavior. Move blocking work off event-loop
threads and establish typed core errors.

**Exit gate:** Existing behavior and all current tests pass through the core
service; `daena-core` has no Tauri dependency.

### Phase 2: Add catalog, identity, and authorization

Implement the plugin catalog, manifest validator, package digests, grants,
sessions, namespace ownership, and the Rust broker. Add a Tauri capability that
allows only plugin bootstrap and RPC from plugin webviews. Keep installer input
limited to a development directory during this phase.

**Exit gate:** An adversarial test plugin cannot read/write another namespace,
call undeclared operations, forge identity, call trusted Tauri commands, or use
a revoked session.

### Phase 3: Convert bundled modules

Generate canonical manifests for Lore and Timeline, remove their hardcoded Rust
manifests and enable branches, and run both through broker-backed SDK contexts.
Treat any required private API as a platform design defect.

**Exit gate:** Lore and Timeline contain no imports of the trusted Tauri client,
are enabled and disabled by `PluginManager`, and pass existing cross-module,
export/import, migration, and disablement scenarios.

This is the minimum point at which the platform is on solid architectural
ground. Do not ship third-party installation before this gate.

### Phase 4: Lifecycle, events, and services

Implement activation rollback, cancellation, health/quarantine, dependency
resolution, typed events, typed services, cycle detection, deadlines,
backpressure, and post-commit core events. Add an optional Timeline service and
a small consumer test plugin to prove plugin-to-plugin interaction.

**Exit gate:** Required/optional dependencies, provider loss, slow subscribers,
service timeouts, re-entrant calls, disablement, and project shutdown have
deterministic tested behavior.

### Phase 5: Sandboxed runtimes

Add isolated plugin webviews, restrictive CSPs, validated message bridging, and
the WASM runtime with resource limits. Remove any development path that loads
third-party code into the main webview.

**Exit gate:** Browser/runtime tests demonstrate that plugin code cannot access
the host DOM, Tauri APIs, local files, environment, processes, or undeclared
network origins, while normal SDK calls succeed.

### Phase 6: Installer, upgrades, and recovery

Implement `.wbplugin` verification, atomic installation, publisher signatures,
capability consent, retained versions, upgrade planning, migration selection,
rollback, uninstall-code, and explicit delete-data flows.

**Exit gate:** Corrupt, oversized, path-traversing, symlinked, incompatible,
tampered, and unauthorized packages fail before execution. Upgrade failure
restores the previous active code and a usable project state.

### Phase 7: Public SDK and author tooling

Publish compiled SDK artifacts and declarations. Add a packaging/validation CLI,
fake host, conformance suite, example declarative plugin, example sandboxed UI
plugin, example WASM service plugin, compatibility documentation, and migration
authoring tools.

**Exit gate:** A plugin can be authored outside the monorepo, validated, tested,
packaged, installed, enabled, upgraded, rolled back, and uninstalled using only
public documentation and tools.

### Phase 8: Registry readiness, not registry dependency

Define publisher identity, signature rotation, revocation metadata, transparency
and review requirements, but keep local package installation functional. A
registry or marketplace is a distribution layer, never an authorization layer.

**Exit gate:** The same verified package behaves identically whether installed
locally or obtained from a registry.

## Test and release requirements

The plugin platform is not release-ready until automated tests cover:

- manifest parsing, canonicalization, unknown fields, and version ranges;
- archive traversal, links, collisions, bombs, limits, signatures, and digests;
- identity forgery, stale/replayed sessions, revocation, and origin binding;
- every capability's allow and deny path in Rust;
- namespace ownership and shared-field read-only behavior;
- migration identity, checksum, ordering, backup, rollback, and tampering;
- dependency resolution, missing optional dependencies, conflicts, and cycles;
- event schemas, post-commit ordering, queue overflow, and slow consumers;
- service schemas, deadlines, cancellation, provider failure, and call cycles;
- lifecycle cleanup after activate failure, disable, crash, project close, and
  application shutdown;
- WASM fuel/memory limits and unavailable ambient OS functionality;
- webview CSP, host DOM isolation, absent Tauri APIs, and message validation;
- upgrade consent, capability escalation, data-version incompatibility, and
  rollback;
- export/import with installed, missing, disabled, and newer plugins; and
- conformance of all bundled plugins with the public SDK.

Security-sensitive parsers and broker request validation receive fuzz tests.
Lifecycle and broker concurrency receive stress tests. Passing unit tests alone
is insufficient; packaged application tests must exercise real Tauri webview
isolation on every supported desktop platform.

## Explicitly deferred decisions

The following features are deferred, with their default behavior decided now:

- **Marketplace:** deferred; local verified packages work without it.
- **Cloud execution or sync:** deferred; plugins are local and project-scoped.
- **Arbitrary internet access:** deferred; denied. Later access is HTTPS,
  origin-scoped, brokered, rate-limited, and separately granted.
- **Native extension ABI:** deferred; unsupported packages are rejected.
- **Multiple providers for one service:** deferred; exactly one active provider
  per service major version.
- **Durable plugin event queues:** deferred; events are session-local and
  at-most-once.
- **Plugin-owned databases:** deferred and disallowed; use namespaced core data.
- **Headless automation:** deferred; when introduced it uses the same broker and
  explicit non-interactive grants.
- **Mobile/web plugin runtime:** deferred; the manifest may later declare target
  compatibility, but version 1 targets the desktop host only.

No unresolved architectural choice above is required to begin implementation.
Any future change to identity, isolation, authority, package integrity, data
ownership, or interaction semantics requires a new ADR and compatibility plan.

## Immediate next work

Begin with Phase 0 only. Produce, review, and approve these artifacts before
refactoring implementation code:

1. `schemas/plugin-manifest-v1.json`;
2. `schemas/plugin-rpc-v1.json` and the common error envelope;
3. the capability registry with request/resource mappings;
4. Rust contract types in the proposed `daena-plugin-api` crate;
5. generated JSON Schemas and TypeScript SDK types (generated from the Rust
   contract types by `npm run gen:plugin-contract`, not hand-written);
6. canonical Lore and Timeline manifests; and
7. ADRs for isolation, authority, packaging trust, and inter-plugin contracts.

Implementation should then follow the phase gates in order. In particular,
installer UI or marketplace work must not jump ahead of backend identity,
authorization, bundled-plugin conversion, and runtime isolation.

## Contract reconciliation and generation record

This appendix is the implementation record of the contract-reconciliation
effort. It supersedes the interim `plan/` documents and preserves their
load-bearing decisions and current state. The overall decision — Rust owns the
contract, schemas and TypeScript are generated artifacts — is [ADR 0006]
(adr/0006-rust-first-contract-generation.md).

### Representations are unified under Rust

The five parallel representations that previously disagreed are now derived
from one source:

| Representation | Location | Role |
| -------------- | -------- | ---- |
| Rust contract types + `validate_manifest` | `crates/daena-plugin-api/src/lib.rs` | Single source of truth |
| RPC payload/envelope types | `crates/daena-plugin-api/src/rpc.rs` | Pins exact wire names |
| RPC method catalog | `crates/daena-plugin-api/src/catalog.rs` | 32 methods, payload, revision, capability |
| JSON schemas | `schemas/plugin-{manifest,rpc,error}-v1.json`, `schemas/capability-registry-v1.json` | Generated build artifacts |
| TypeScript contract types | `packages/plugin-sdk/src/generated.ts` | Generated build artifact |
| TS rule validator | `packages/plugin-sdk/src/index.ts` (`validatePluginManifest`) | Mirror of Rust rules, conformance-tested |

JSON Schema cannot express the cross-reference *rules* (namespace ownership,
migration contiguity, template-field typing), so rules stay handwritten in Rust
and are mirrored in TypeScript. Shapes are generated; parity is enforced.

### Canonical RPC method catalog

32 executable methods in `RPC_METHOD_CATALOG`:

```
entity.list  entity.get  entity.create  entity.update  entity.delete
document.list  document.save
field.read  field.list  field.set
relationship.list  relationship.create  relationship.delete
asset.list  asset.register  asset.read.begin
asset.replace.begin  asset.replace.commit  asset.transfer.cancel
search.query
maps.asset.create.begin  maps.asset.create.commit
maps.recovery.export.begin  maps.recovery.export.commit
maps.recovery.list  maps.recovery.restore
maps.locations.list  maps.reconcile.links
event.publish  event.subscribe  event.poll
service.call
```

### Resolved contract decisions

- **`asset.import` capability renamed to `asset.register` (breaking).** The old
  capability implied a host file-picker import with no executable method; only
  `asset.register` (plugin-supplied file) exists. The capability registry,
  `KNOWN_CAPABILITIES`, the SDK validator's `knownCapabilities`, bundled
  manifests, the frontend `checkCapability`, and this document's §4 all use
  `asset.register`.
- **`maps.*` methods are first-class broker methods.** Previously
  `maps.locations.list` and `maps.reconcile.links` dispatched but had no
  capability arm (broker-unreachable). Both now map to `asset.read:self`;
  `reconcile.links` rebuilds the disposable map projection and does not mutate
  canonical files.
- **Capability-alias names are grouping keys, not methods.** `entity.read`,
  `entity.write`, `document.read`, `document.write`, `relationship.read`,
  `relationship.write`, `asset.read`, `field.write`, and `service.provide` were
  `required_capabilities` arms with no dispatch target; a call would authorize
  then fail with "unknown plugin RPC method". They are excluded from the
  catalog and return `method.unknown`; the host `required_capabilities` is now
  a catalog lookup.
- **`service.provide` is vestigial as a method.** Providers register at
  activation; the manifest `services.provides` is the mechanism.
- **Wire-name fidelity.** Payloads mix snake_case and camelCase by design
  (`source_id`, `expectedRevision`, `mapEntityId`, `fileName`). The Rust payload
  structs in `rpc.rs` pin these exact wire names through serde renames, so
  `generated.ts` matches what the host actually sends.

### Validation parity

Rust `validate_manifest` and TS `validatePluginManifest` were aligned rule by
rule:

- Rust gained semver `-pre`/`+build` parsing, `is_host_api_range`, and `.`-path
  segment rejection.
- TS gained Rust's template-preset typing, relationship
  `entityTypes`/`targetEntityTypes` duplicate/empty checks, and non-empty
  `views`/`commands` rule checks.
- `schemas/fixtures/manifest/` holds an 18-fixture battery (one manifest per
  rejection class) indexed in `index.json`. Both validators must agree with the
  indexed `expected` outcomes, enforced by
  `crates/daena-plugin-api/tests/fixture_battery.rs`, `npm run
  check:manifest-fixtures`, and the dual-validator conformance test
  (`npm run test:plugin-conformance`, which also runs 42 broker checks and
  lifecycle install/enable/upgrade/rollback/uninstall against the test host).
- Intentionally non-mutual rules (e.g. TS rejects duplicate capabilities and
  unknown migration-item keys; Rust rejects duplicate command exposure) remain
  outside the battery as non-contract-critical.

### Generation pipeline and drift guard

- `npm run gen:plugin-contract` runs the `gen-contract` bin
  (`crates/daena-plugin-api/src/bin/gen-contract.rs`, `--features gen`) to
  emit the four schemas, then converts them to
  `packages/plugin-sdk/src/generated.ts`. The SDK `dist` is rebuilt with
  `npm run build:plugin-sdk`.
- `npm run check:plugin-contract` (in `npm run check`) regenerates everything
  into a temp directory and byte-diffs the committed schemas, `generated.ts`,
  and `dist/generated.*` against the fresh output. Any contract change that is
  not followed by a regen fails the check; a gen-gated cargo test
  (`committed_schemas_match_generation`) does the same in-process.
- `schemas/fixtures/manifest/` is gitignored and regenerable:
  `npm run gen:manifest-fixtures` re-derives the 18 fixtures plus `index.json`
  from the Lore manifest (`packages/modules/lore/manifest.json`), so the
  directory contents can be deleted and rebuilt on demand (~50 ms). The
  dependent checks auto-generate first via npm `pre` hooks
  (`check:manifest-fixtures`, `test:plugin-conformance`), so a fresh clone
  needs no manual step. `npm run check:manifest-fixtures` additionally
  regenerates to a temp directory and byte-diffs the on-disk fixtures
  (`check-manifest-fixtures-drift.mjs`), failing with "is stale" when hand-edited
  files drift from the generator.
- The `daena-plugin validate` CLI compiles
  `schemas/plugin-manifest-v1.json` with `ajv` (draft 2020-12, strict, with the
  non-standard `uint32` format registered) at startup and prepends
  `schema:<instancePath>` errors to the TS validator's output in both validate
  paths, so shape-only rejections are caught too.
- The Phase 0–5 exit gates (all representations agree on the frozen contract,
  generated types reviewed and tests green, full suites green, drift guard
  fails on an intentional Rust change without regen, docs match code) are all
  **met**.
