# Daena Archive architecture

## Purpose and authority

Daena Archive is an offline-first authoring studio for fictional worlds and
stories. This document is the project-wide architecture authority for the
product model, host boundaries, project storage, bundled modules, and
runtime plugins.

It consolidates the former product and architecture documents. Detailed
contracts remain in the focused plans below:

- [`STORAGE.md`](./STORAGE.md) defines the authoritative runtime database,
  portable project format, synchronization, recovery, and rebuild contracts.
- [`STORAGE_MIGRATION.md`](./STORAGE_MIGRATION.md) defines the alpha hard cut
  to the database-first writer and durable exporter.
- [`PLAIN_TEXT_STORAGE_PLAN.md`](./PLAIN_TEXT_STORAGE_PLAN.md) records the
  historical portable format and file-canonical planning work.
- [`PLUGIN_PLATFORM_PLAN.md`](./PLUGIN_PLATFORM_PLAN.md) defines plugin
  isolation, package trust, broker authorization, lifecycle, and compatibility.
- [`PLUGIN_SDK.md`](./PLUGIN_SDK.md) is the definitive plugin authoring guide.
- [`GIT_INTEGRATION.md`](./GIT_INTEGRATION.md) defines the optional built-in Git
  integration.
- [`MAP_INTEGRATION_PLAN.md`](./MAP_INTEGRATION_PLAN.md) defines provider-backed
  maps and map/entity integration.
- [`AI_INTEGRATION.md`](./AI_INTEGRATION.md) defines the future AI subsystem.

Those documents may add detail but must not contradict the boundaries here.
ADRs in [`adr/`](./adr/) record narrower decisions and security constraints.

## Product direction

Daena is a private, local-first desktop workspace where authors build a shared
world model and write from it. The product is organized around one durable
entity graph rather than separate databases for lore, timelines, maps, or
manuscripts.

The current product direction includes:

- shared entities with stable identity, prose documents, typed fields,
  relationships, and assets;
- first-party Lore, Timeline, Writing Studio, and Maps experiences backed by
  the same core records;
- optional runtime plugins that use the same public contracts as bundled
  modules;
- deterministic, file-based project data that remains usable outside Daena;
- explicit user-controlled Git snapshots around portable project files; and
- future provider-neutral AI assistance that never becomes a second data model
  or mutation authority.

Collaboration, cloud synchronization, public publishing, mobile targets, and
unrestricted native extensions are separate future products, not hidden
assumptions in the core architecture.

## Non-negotiable principles

1. **Core owns durable truth.** The Rust core owns identity, persistence,
   validation, revisions, migrations, indexing, recovery, and resource
   authorization.
2. **SQLite owns runtime truth; files own portability.** Runtime mutations are
   authoritative in SQLite. Markdown, strict JSON, and native asset bytes form
   clean portable checkpoints for Git, inspection, external editing, and
   reconstruction.
3. **Modules add meaning, not storage silos.** A map pin, timeline event, and
   manuscript reference can point to the same entity without duplicating it.
4. **Rust is the authority boundary.** Frontend checks are advisory; requests
   are authorized again at the host/core boundary.
5. **Plugins are isolated.** Third-party code never enters the main webview and
   never receives ambient Tauri, filesystem, shell, process, or network access.
6. **Users control destructive or external actions.** Git commits, resets,
   pushes, imports, plugin grants, and destructive data operations require
   explicit host UI actions.
7. **Clean checkpoints must rebuild.** Derived projections are disposable, and
   a clean portable checkpoint must reconstruct the project. Dirty runtime
   database state is not disposable and must never be silently discarded.
8. **Public contracts are framework-neutral.** Svelte is the first-party UI
   choice, not a requirement for modules or plugins.

## System shape

```text
                         Trusted application shell
                    Svelte 5 / TypeScript / Tauri UI
                         │                    │
                         │ typed commands     │ plugin bootstrap + RPC
                         ▼                    ▼
                 ┌──────────────┐       ┌───────────────┐
                 │ Tauri adapter│       │ Plugin host   │
                 └──────┬───────┘       │ catalog       │
                        │               │ grants        │
                        │               │ sessions      │
                        │               │ broker        │
                        │               └──────┬────────┘
                        ▼                      │ authorized core calls
                 ┌────────────────────────────┴──────┐
                 │ Rust core: project, storage, sync  │
                 │ entities, docs, fields, links,    │
                 │ assets, search, migrations, Git    │
                 └────────────────┬───────────────────┘
                                  │
                    SQLite runtime transaction store
                      │ durable synchronization
                      ▼
                     portable project directory
              Markdown / JSON / native assets / project manifest
```

`daena-core` has no Tauri or plugin-runtime dependency. The Tauri adapter and
plugin broker call typed core services with different authority contexts.
Plugin webviews and WASM runtimes can reach project data only through the
versioned broker contract.

## Portable project model

A directory project has this shape:

```text
project.json
entities/<entity-uuid>/
  entity.json
  document.md
  fields/<plugin-id>--<namespace>.json
  relationships.json
  assets.json
plugins/<plugin-id>.json
assets/{images,videos,maps,files}/
.daena/
  index.sqlite                 # authoritative runtime database
  sync/<request-id>/new/
  project.lock  export.lock
  backups/ conflicts/ local/
.gitignore
```

The stable entity UUID is the filesystem address. Names, types, timestamps,
and soft-delete state are metadata and may change without changing references.
Documents contain author content in deterministic Markdown. Structured records
use strict JSON. Assets retain their native bytes and are referenced by
project-relative paths, hashes, MIME types, and ownership metadata.

The `.daena/` directory is machine-local and ignored by Git. It contains the
authoritative runtime database, durable synchronization state, recovery
material, conflict copies, and derived indexes. When synchronization is clean,
the portable files reconstruct the same durable project content. When it is
dirty, `.daena/index.sqlite` may contain newer committed work and is not safe
to delete. Plugin grants, installed packages, runtime sessions, and capability
state are machine-local as well. Portable plugin interpretation state belongs
in `plugins/<plugin-id>.json`.

## Storage and consistency

The target database-first path commits runtime rows, opaque revisions,
idempotency receipts, and durable portable-export intent in one SQLite
transaction. A synchronization worker renders only affected deterministic
records, verifies their last-sync baselines, and advances the portable
checkpoint. Git, backup, export, close, and rebuild use explicit flush barriers.

An existing valid database may serve reads without a blocking full-tree scan.
Offline file reconciliation continues in the background, while writes verify
the affected portable baselines before committing. If no usable database
exists, the core validates the complete portable representation and constructs
a new runtime database and derived projections.

Invalid external edits are diagnostic-only and do not replace the last valid
runtime rows. Unmerged Git files, malformed paths, invalid references,
namespace violations, and asset-hash failures block affected operations.
External-only edits may import after validation; two-sided changes and unsaved
drafts use typed conflict/recovery paths rather than silent overwrites.

All mutable broker-visible records expose opaque revisions. Updates, deletes,
document saves, field and relationship mutations, and asset registration use
the observed revision. Retryable mutations retain request IDs across Rust,
the SDK, bundled modules, and the test host.

Search, map projections, relationship indexes, and other views are derived
from durable runtime rows. They may be rebuilt, discarded, or temporarily
reported as stale without changing project content.

The database-first writer is the current hard-cut storage boundary. Each
mutation commits runtime rows, derived updates, its receipt, and exact sync
intent together; the exporter then updates only the affected portable files
from durable staging under `.daena/sync/`. There is no dual-authority writer
or fallback writer. See [`STORAGE_MIGRATION.md`](./STORAGE_MIGRATION.md)
for the recovery and synchronization contract.

## Core and trusted shell

`crates/daena-core` owns:

- project open/create/close, runtime database access, and portable codecs;
- stable entities, documents, fields, relationships, and assets;
- deterministic serialization, validation, synchronization baselines, runtime
  transactions, and durable export queues;
- search and disposable projections;
- module/plugin project state, migrations, backups, and recovery;
- typed `CoreError` results and authority-aware operations; and
- explicit Git helpers for portable preflight, snapshots, commits, resets,
  remotes, and lease-protected pushes.

`src-tauri` is the trusted application adapter. It resolves platform paths,
assembles services, serializes command inputs/outputs, owns host dialogs and
external URLs, and runs blocking core work through Tauri's task facilities.
Its privileged commands are available to the main shell only. Git, raw file
access, migration controls, backup/restore, and arbitrary Tauri commands are
not part of the plugin broker.

## Modules and plugins

Bundled modules and runtime plugins use the same manifest, SDK, lifecycle, and
broker-backed public contract. A manifest declares identity, version, kind,
capabilities, schemas, templates, views, commands, dependencies, namespaces,
and migrations. The Rust plugin API is the source for generated JSON Schemas,
TypeScript declarations, and contract fixtures.

The host aggregates enabled module templates and views into the workspace, but
the module does not receive a database handle, filesystem handle, raw Tauri
invoke function, or private host API. Views mount into a host-provided surface
and return cleanup handles. Disabled modules disappear from navigation and
command catalogs while their portable data remains available for recovery and
export.

### Runtime isolation and authority

Third-party UI runs in an isolated, application-controlled webview origin with
no host DOM or ambient Tauri access. Background logic runs in bounded WASM/WASI
when supported. Both communicate through a versioned broker envelope such as:

```text
plugin_rpc(session_id, request_id, method, payload)
```

Rust binds each session to the installed plugin identity and package digest,
version, current project, runtime/webview instance, granted capabilities,
activation generation, expiry, and revocation state. The broker validates the
session, origin, payload schema, size, capability, namespace ownership, and
revision before forwarding the request.

Capabilities are declared by a manifest and granted by the user; they are not
self-granted by plugin code. The vocabulary includes scoped entity/document
reads and writes, namespace-owned fields/assets, search, relationships, events,
and services. There is no generic filesystem, shell, process, dialog, Tauri, or
unrestricted network capability. Disabling, upgrading, uninstalling, closing a
project, or restarting a plugin revokes its sessions.

## Migrations and versioning

Package versions, host API versions, and stored data versions are separate.
Public API major versions are incompatible; compatible minor changes are
validated against the host-supported range. Stored data versions advance only
through deterministic, declared migrations.

Each migration has a unique ID, source and target versions, an operation list,
and an explicit recovery policy for destructive operations. The core validates
contiguity and namespace ownership, creates a recovery backup when required,
executes the migration transactionally, and records the migration ID and
checksum. Errors roll back the transaction and leave the prior version active.

The alpha cut is reset-oriented: pre-format-version-2 projects and pre-cut
`.daena/` runtime state receive no legacy reader, migration, feature flag, or
dual-write path. Existing version-2 portable files initialize a new runtime
database after `.daena/` is removed. Plugin data migrations remain required
when a plugin package changes its own schema or stored data version.

## Git and external integrations

Git is an optional, user-controlled snapshot tool around portable files. The
Rust core flushes committed runtime changes, validates the resulting portable
checkpoint, rejects unresolved conflicts, and presents exact staging paths
before a commit. The Settings → Git surface supports selective portable
commits, history browsing, read-only snapshot previews, explicit hard reset,
remotes, and lease-protected remote recovery. `.daena/` and SQLite files are
never staged by built-in Git helpers.

Maps are normal shared entities, not a parallel identity table. Provider source
files remain opaque native assets; map locations, roles, dates, relationships,
and story metadata remain Daena-owned project fields and links. Provider
adapters are replaceable and must not force provider-specific data into the
shared entity model.

AI, when implemented, must use core retrieval and broker authorization. It may
assemble context and propose changes, but users accept every mutation through
the same revision-aware runtime transaction and portable synchronization path.
Provider credentials, network access, temporary generations, and derived
retrieval state do not become portable project data.

## Verification and evolution

Changes to this architecture must preserve these exit properties:

- a fresh project opens and a clean portable checkpoint reconstructs after
  deleting `.daena/`;
- dirty runtime state cannot be silently rebuilt or discarded;
- portable round trips are deterministic and preserve external edits or report
  typed conflicts;
- revisions, request IDs, capability checks, namespace ownership, and session
  revocation are enforced at the Rust boundary;
- plugin UI and WASM remain isolated from the main webview and native shell;
- Git never stages nonportable paths or unresolved/unflushed portable state;
- derived search, map, relationship, and view projections can be rebuilt.

Use the focused plans and ADRs for phase-specific acceptance criteria. When
implementation and documentation diverge, verify the current source and tests,
then update this architecture and the relevant focused authority together.
