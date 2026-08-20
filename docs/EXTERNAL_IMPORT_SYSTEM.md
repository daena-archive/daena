# External import system

## Purpose and authority

This document defines Daena's requirements, architecture, and implementation
plan for migrating external material into an open project. It translates the
external product specification into the current Daena architecture. It is
subordinate to [`ARCHITECTURE.md`](./ARCHITECTURE.md),
[`STORAGE.md`](./STORAGE.md), and
[`PLUGIN_PLATFORM_PLAN.md`](./PLUGIN_PLATFORM_PLAN.md) where their ownership,
storage, security, or plugin boundaries apply.

Import is an explicit, user-reviewed migration operation. It is not live file
mirroring, synchronization, checkpoint recovery, or a compatibility reader for
older Daena project formats.

## Product requirements

### Required pipeline

Every source follows one pipeline:

```text
source selection
  -> importer detection and explicit selection when ambiguous
  -> neutral staged import
  -> analysis and validation
  -> preview and user mapping
  -> immutable validated plan
  -> one logical project commit
  -> import report
```

Source selection and analysis must not mutate the project. Importers understand
source formats; they do not understand or write Daena's SQLite schema,
portable-file layout, or checkpoint format. Daena core owns mapping,
validation, conflict resolution, commit, and reporting.

### Neutral staged model

The versioned staged contract must represent, without requiring a Daena entity
type:

- source and importer identity, source-relative paths, and stable source IDs;
- objects with titles, body content, source kind, hierarchy, tags, aliases,
  arbitrary fields, and raw metadata;
- internal, external, and embedded links, including resolved, ambiguous, and
  missing states;
- assets and attachment relationships without granting an importer a project
  filesystem path;
- conservative mapping hints, each distinguishable from a user decision;
- unsupported/raw source data and structured fatal, error, and warning
  diagnostics; and
- incremental analysis totals suitable for large sources.

Unknown information must be preserved as raw metadata or source material where
practical. Otherwise it must be named in the preview and final report. No
conversion path may silently discard it.

### Analysis, preview, and mapping

Analysis reports object, folder, link, asset, unresolved-reference,
unsupported-item, duplicate-candidate, and diagnostic counts. Large sources
must be processed incrementally with bounded memory and progress/cancellation
support.

The shared preview UI must allow inspection of every staged item and its source
data. Mapping can be global, source-category, folder, or item specific. Entity
types, fields, and relationship types must be discovered from enabled module
manifests and current project contracts; they must not be hardcoded by importer
or frontend source-name checks. Importer suggestions are defaults only and the
user can override ambiguous mappings.

Only currently available importers are shown. The project-level `Import`
action owns source choice, analysis progress, preview, mapping, validation,
warning acknowledgement, commit confirmation, and the result report.

### Validation and conflicts

Before commit, core produces an immutable import plan and validates at least:

- staged contract version and structural validity;
- duplicate source and staged IDs;
- required titles and selected entity mappings;
- field values and namespace ownership;
- link targets and ambiguous or missing references;
- duplicate candidates and explicit conflict decisions;
- portable filename and path conflicts;
- asset availability, size, MIME type, hash, and ownership; and
- every plugin-provided value against the same limits and schemas as bundled
  importers.

Blocking errors prevent commit. Warnings require explicit acknowledgement.
Supported conflict decisions are `create`, `skip`, `replace`, `merge`, and
`map to existing`; only decisions implemented with lossless, validated
semantics are enabled. Automatic merging is forbidden unless identity is
unambiguous and the decision remains visible in preview.

### Commit and report

Commit is one logical, receipt-backed core mutation. It must either install the
validated plan or leave runtime project content unchanged. Import transactions
must not wait on importer or plugin code. SQLite remains runtime authority;
after commit, the existing checkpoint worker renders the normal portable
project files. Import does not introduce a second writer or direct portable
file mutation path.

The result report includes source/importer identity, created and mapped
objects, skipped items, converted fields, link resolutions, conflict
decisions, assets, unsupported data, missing data, and all diagnostics. The
report returned by commit is authoritative for the operation. Persisting a
report is optional and, if added, uses an explicit Daena-owned project-metadata
contract rather than importer-controlled files.

### Source identity and re-import

Staged objects carry importer ID, importer contract version, source-relative
path, optional source-native ID, and a content fingerprint. Committed source
identity may be retained as Daena-owned metadata for duplicate detection and a
future explicit re-import workflow. It never enables background watching or
implicit synchronization.

### Initial formats

Delivery order is:

1. Markdown, plain text, and recursive folders;
2. ZIP and reliable rich-document parsers, followed by Obsidian as a
   specialization of Markdown;
3. streaming MediaWiki-compatible XML and conservative wikitext preservation;
4. third-party importer plugins.

Markdown initially remains one staged object per file. Headings are preserved
in the document body and do not create entities automatically. Obsidian adds
frontmatter, aliases, wikilinks, embeds, attachments, and vault-resolution
rules while producing the same staged contract.

### Security and resource limits

All imported bytes and plugin output are untrusted. The implementation must:

- reject traversal, absolute, malformed, and platform-prefix archive paths;
- reject or explicitly surface symlinks instead of following them;
- bound per-file bytes, total expanded bytes, item count, hierarchy depth,
  diagnostic count, parser time, plugin payload size, and concurrent work;
- detect decompression-ratio and expanded-size limits before ZIP extraction;
- disable XML external entities and DTD expansion;
- sanitize rendered HTML and never grant imported content a privileged origin;
- parse malformed binary formats in isolated, bounded code paths; and
- support cancellation on user request, project close, importer/plugin
  disable, and application shutdown.

Raw host paths are trusted-shell data and are not included in plugin payloads,
portable project metadata, or user-visible plugin diagnostics. Diagnostics use
source-relative paths.

## Architecture and contracts

### Ownership boundaries

`daena-core` owns the staged contract, validation, mapping model, conflict
model, immutable plan, transactional application, and report types. Bundled
format adapters may live beside this core contract but may call only staging
APIs during analysis.

`src-tauri` owns native file/folder dialogs, opaque source handles, background
job lifecycle, progress events, cancellation, and moving blocking filesystem
and parser work off the Tauri event loop.

The trusted Svelte shell owns the shared import workflow and renders only typed
core results. It derives selectable mappings from enabled contributions.

`daena-plugin-api` eventually declares versioned importer contributions and
bounded analyze/progress messages. `daena-plugin-host` discovers an enabled
provider, authorizes it, supplies a bounded opaque source reader, validates all
output, and revokes the job with plugin lifecycle. A plugin cannot receive an
ambient path, filesystem capability, database handle, project mutation method,
or commit callback.

### Sessions and staging lifetime

An analysis creates an opaque import session bound to the current project
generation, selected source, importer identity/version, and enabled-module
snapshot. Staged data is not canonical project content. Small sessions may stay
in memory; larger sessions may spill to bounded machine-local storage under
`.daena/local/`, with cleanup on cancel, successful commit, close, or expiry.
No persistent source catalog or portable staging directory is introduced.

The validated plan captures the observed project generation and revisions of
all existing targets. Commit fails with a typed conflict if project state or
enabled schema contributions changed after validation, forcing re-analysis or
re-validation rather than applying stale decisions.

### Importer contract

Each importer declares a stable ID, contract version, display metadata,
supported source kinds/extensions/MIME types, conservative detection rules,
options schema, and capabilities. Detection is bounded and side-effect free.
Ambiguous detection is shown to the user; filename extension alone does not
silently select a destructive interpretation.

Analysis emits staged batches and progress through a bounded sink. The host can
apply backpressure and cancel. Importers never receive a project mutation
interface. Bundled and plugin importers are validated through the same staged
contract and limits.

### Commit strategy

The existing `ProjectStore::create_entries_with_request` proves the required
receipt-backed transaction pattern, but is not the import commit API: it cannot
fully express new-object links, asset installation, merges, replacements, or
source metadata in one plan. Import receives a dedicated core transaction that
preallocates IDs, resolves all new-to-new and new-to-existing references,
validates every row and asset before beginning the transaction, writes all
runtime records plus the idempotency receipt together, and only then signals
the checkpoint worker.

Asset bytes are preflighted into bounded machine-local staging. Commit installs
content-addressed runtime assets with rollback-safe bookkeeping. No plugin code,
source parsing, network request, or user prompt occurs while the SQLite
transaction is open.

## Implementation plan

Each iteration is a bounded vertical slice with its own exit gate. Later work
is retained here but does not expand the first release implicitly.

### Iteration 1: staged contract and generic analysis

Implement the versioned neutral types, diagnostics, summary generation,
resource limits, deterministic Markdown/plain-text file staging, recursive
folder staging, stable source-relative ordering/identity, and symlink refusal.
No project mutation or UI is added in this iteration.

**Exit gate:** focused core tests prove that analysis leaves project state
untouched, produces deterministic staged output for files and folders,
preserves Markdown/text bytes as UTF-8 content, reports unsupported entries,
rejects invalid roots and non-UTF-8/oversized content safely, never follows
symlinks, and enforces item and total-byte limits.

### Iteration 2: trusted-shell analysis sessions

Add Tauri commands for importer discovery, file/folder selection, starting and
cancelling an analysis job, paged staged-item reads, and progress events. Bind
sessions to project lifecycle and spill large staged batches to bounded local
storage. Add typed frontend client contracts.

**Exit gate:** the native app can analyze a folder without mutation, report
progress, cancel promptly, page results, clean local staging on lifecycle
changes, and reject stale/guessed session IDs.

### Iteration 3: shared preview and mapping

Add the project-level Import entry point, source/importer chooser, analysis
summary, item inspector, diagnostics, and global/folder/item mapping controls.
Derive entity/field/relationship choices from enabled manifests. Produce an
immutable candidate plan without committing it.

**Exit gate:** a user can review every Markdown/text item, override suggested
mappings, see unsupported information and unresolved decisions, and close or
cancel without project mutation. Disabled module contributions disappear.

### Iteration 4: validation, atomic commit, and report

Add duplicate/source-identity detection, explicit conflict decisions, plan
validation, warning acknowledgement, generation/revision preconditions, the
dedicated receipt-backed import transaction, and the complete result report.
Start with create/skip/map-to-existing; keep replace/merge disabled until their
field, document, relationship, and asset semantics are specified and tested.

**Exit gate:** confirmed Markdown/text/folder imports are atomic and
idempotent; injected validation, transaction, and checkpoint failures do not
leave partial project content; successful imports survive close/reopen and a
clean rebuild after removing `.daena/`; stale plans fail closed; the report
matches every applied or skipped item.

### Iteration 5: Markdown completeness, ZIP, and assets

Add standard Markdown link/image discovery, safe relative resolution,
frontmatter preservation/mapping, preflighted assets, ZIP central-directory
limits and safe extraction, and only those HTML/DOCX/ODT/RTF parsers that pass
quality and security fixtures.

**Exit gate:** nested folders and ZIPs produce equivalent staged structure;
traversal, symlinks, bombs, malformed documents, and missing assets are safely
blocked or reported; successful asset imports round-trip through checkpoint
rebuild without path or hash drift.

### Iteration 6: Obsidian specialization

Build the Obsidian adapter on the Markdown path. Add YAML frontmatter aliases,
wikilinks, embeds, attachment discovery, and vault-wide resolved/ambiguous/
missing reference analysis. Preserve unsupported plugin syntax intact.

**Exit gate:** a representative vault retains useful hierarchy, Markdown,
frontmatter, aliases, links, embeds, and assets; ambiguous/missing links are
never invented and remain reviewable; reopen and clean rebuild preserve the
accepted result.

### Iteration 7: streaming MediaWiki

Add a streaming, external-entity-disabled XML adapter for latest page revisions,
namespaces, redirects, categories, links, raw wikitext, templates, and infobox
hints. Do not add revision-history import.

**Exit gate:** a large fixture stays within an explicit memory ceiling, can be
cancelled, preserves unconverted wikitext/source metadata, and completes the
same preview-plan-commit-report flow.

### Iteration 8: plugin importer ecosystem

Add importer declarations to the canonical Rust plugin contract, generate JSON
Schema and TypeScript SDK types, implement broker discovery and bounded opaque
source reads, and add plugin test-host conformance fixtures. Importer
availability follows enabled service/capability state.

**Exit gate:** a test plugin can detect and analyze a new format into the
neutral contract without storage knowledge or project-write authority;
malformed, oversized, timed-out, disabled, and revoked providers fail closed;
bundled and plugin output pass identical core validation.

## Explicit non-goals for the first release

- bidirectional sync, background watching, or live external mirrors;
- full source revision history;
- automatic semantic relationship inference;
- application-specific layout/widget reproduction;
- silent entity-type creation, destructive conversion, or automatic merge;
- unrestricted plugin filesystem/network access; and
- first-party support for every proprietary worldbuilding product.

## Implementation status

- Iteration 1: implemented in the core; the initial slice covers the neutral
  contract and deterministic generic Markdown/plain-text/folder analysis.
- Iteration 2: implemented; trusted-shell source handles, project-bound
  background sessions, progress/cancellation, paged results, lifecycle cleanup,
  bounded local spill storage, Tauri commands, and frontend client types are in
  place.
- Iterations 3-8: planned, not yet implemented.
