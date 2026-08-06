# Plain-Text Project Storage Architecture Plan

## Summary

Daena projects will use ordinary files as the only canonical source of project
data. Author documents will be stored as Markdown, structured records will be
stored as strict JSON, and imported assets will remain in their native formats.

SQLite will remain, but only as a disposable local index. A project must open
and recover all of its content when the database is absent. The database and
all other machine-local state will be ignored by Git and rebuilt from the
canonical files.

This is an alpha format reset. Existing SQLite projects do not need a migration
path, compatibility reader, or dual-write period.

## Goals

- Produce focused, understandable Git diffs for documents and structured data.
- Allow authors to edit Markdown and JSON with tools outside Daena.
- Preserve stable entity IDs and cross-module relationships across renames.
- Keep plugin data namespaced and enforce ownership in the Rust core.
- Make the SQLite index safe to delete and deterministic to rebuild.
- Prevent crashes, concurrent saves, or external edits from silently losing
  canonical data.
- Preserve the existing broker boundary: modules and plugins use logical APIs,
  never arbitrary project filesystem access.

## Non-goals

- Supporting the current SQLite-canonical alpha project format.
- Maintaining a long-lived SQLite/file dual-write mode.
- Turning Git into a collaboration or synchronization protocol inside Daena.
- Automatically resolving semantic Git conflicts.
- Exposing project paths or raw file handles to third-party plugins.
- Storing capability consent or runtime sessions in version-controlled files.

## Canonical Project Layout

```text
project.json
entities/
  <entity-uuid>/
    entity.json
    document.md
    fields/
      <plugin-id>--<namespace>.json
    relationships.json
    assets.json
plugins/
  <plugin-id>.json
assets/
  images/
  videos/
  maps/
  files/
.daena/
  index.sqlite
  transactions/
  backups/
  conflicts/
  local/
.gitignore
```

The UUID directory is the stable address of an entity. It never changes when
the entity is renamed, retyped, archived, or displayed through a different
module. Human-readable names belong in metadata rather than paths so ordinary
edits do not create filesystem moves.

### `project.json`

`project.json` is the root manifest and contains only portable metadata:

```json
{
  "formatVersion": 2,
  "id": "6f21a771-eec6-4833-9a56-89b5cfc8f126",
  "name": "Eldermere",
  "createdAt": "2026-08-05T10:30:00Z"
}
```

The manifest must not contain absolute paths, database locations, installed
package locations, capability grants, or other machine-specific state.

### `entity.json`

The entity record owns stable identity and lifecycle metadata:

```json
{
  "id": "018f89df-b93e-7ad0-a07f-08b1441d1550",
  "name": "The Glass Coast",
  "type": "place",
  "deleted": false,
  "createdAt": "2026-08-05T10:32:00Z",
  "updatedAt": "2026-08-05T11:14:00Z",
  "document": {
    "id": "018f89e1-3d7b-73bb-b7c1-c83de04102e1",
    "path": "document.md"
  }
}
```

An archived entity remains on disk with `deleted: true`. Physical removal is a
separate maintenance operation and must reject removal while live references
or retained recovery requirements exist.

### `document.md`

The document contains only author content. It has no generated front matter;
its identity and other metadata remain in `entity.json`. Canonical documents
use UTF-8, LF line endings, a final newline, and deterministic CommonMark/GFM.
A small documented subset of sanitized inline HTML may represent formatting
that CommonMark cannot express directly.

Markdown is the only canonical author-document format in version 2. The rich
editor must parse Markdown when loading and serialize Markdown when saving.
HTML is a rendered/editor representation, not stored project data.

### Namespaced fields

Each `fields/<plugin-id>--<namespace>.json` file contains one JSON object whose
keys are the schema-defined field names and whose values are arbitrary JSON
values. Separating namespaces prevents unrelated plugins from rewriting the
same file and reduces Git merge conflicts.

The filename is derived only from a validated plugin ID and local namespace.
The core must reject separators, traversal, ambiguous normalization, case-only
collisions, and namespace ownership mismatches.

### Relationships

`relationships.json` stores relationships whose source is the containing
entity. The source ID is therefore implicit and cannot disagree with the path:

```json
{
  "relationships": [
    {
      "id": "018f89ec-25fc-7816-8b47-6f80905f2868",
      "targetId": "018f89df-b93e-7ad0-a07f-08b1441d1550",
      "type": "daena.lore:located-in",
      "metadata": {}
    }
  ]
}
```

Incoming relationships are derived by the SQLite index. Relationship IDs are
globally unique, relationship types are qualified by their owner, and targets
must resolve to known entities. Relationships to archived entities may remain
for recovery, but normal queries exclude them unless explicitly requested.

### Asset records and bytes

`assets.json` stores records owned by the containing entity. Every record has a
stable ID, owning namespace, original filename, MIME type, byte size, SHA-256
content hash, creation time, and normalized project-relative path.

Asset bytes remain below `assets/` in their appropriate native formats. The
core imports them through a host-owned file picker, stages and hashes them
before commit, rejects traversal and symlink escapes, and verifies their hash
during project validation. Plugins receive brokered asset operations rather
than filesystem paths.

### Portable plugin data state

`plugins/<plugin-id>.json` stores only state needed to interpret project data:

- plugin ID and namespace ownership;
- stored data version;
- schema version and schema checksum;
- selected package compatibility information, without an installation path;
- applied migration IDs, package digests, operation checksums, versions, and
  timestamps; and
- preserved state for an unavailable or disabled plugin.

Capability grants, installation consent, desired activation, active sessions,
runtime health, quarantine state, and diagnostics are local security decisions.
They belong under `.daena/local/` or the application profile and must not become
trusted merely because they arrived through Git. A cloned project keeps plugin
data but does not execute plugin code until the package is installed and local
consent is granted.

## Canonical Encoding Rules

All JSON files use strict JSON with:

- UTF-8 without a byte-order mark;
- LF line endings and one final newline;
- two-space indentation;
- deterministic lexicographic object-key ordering;
- arrays ordered only where order is semantically meaningful;
- no duplicate keys, comments, NaN, infinities, or implicit scalar coercion;
  and
- an explicit format/schema version at each independently versioned boundary.

The parser rejects unknown required-version fields, malformed UUIDs, absolute
paths, path traversal, duplicate stable IDs, case-colliding canonical paths,
invalid namespace ownership, dangling live references, invalid field values,
and Git-unmerged structured files. Diagnostics always include the canonical
path and a stable error code.

Canonical serializers must produce byte-identical output for logically
identical data. They must not rewrite unrelated files, reorder meaningful
arrays, or update timestamps when the logical record did not change.

## Core Storage Boundary

Introduce a filesystem-backed repository beneath the existing Rust core
service. `ProjectStore` should no longer expose SQLite as the persistence
abstraction. The repository owns parsing, validation, journaled mutations,
filesystem watching, and index coordination.

The public module and plugin APIs retain logical operations for entities,
documents, fields, relationships, assets, and search. They never expose a raw
database connection, project path, arbitrary filesystem method, or Tauri
invoke function.

Reads return an opaque content revision computed from the canonical bytes that
make up the logical record. Every update, delete, document save, field mutation,
relationship mutation, and asset registration supplies the revision observed
by the caller. A mismatched revision returns a typed conflict instead of
overwriting a newer external or concurrent edit.

Entity creation does not need an expected revision. Retryable mutations retain
request IDs so a broker retry cannot duplicate relationships, assets, or other
records.

## Reliable File Transactions

Renaming one file is atomic on supported local filesystems, but a logical Daena
mutation may update several canonical files. Multi-file mutations therefore
use a recoverable roll-forward journal:

1. Acquire the project writer lock.
2. Read and compare every expected source revision.
3. Validate the complete proposed logical state, including references and
   plugin ownership, before writing canonical paths.
4. Write new files and imported asset bytes below
   `.daena/transactions/<request-id>/new/`.
5. Fsync staged files and directories.
6. Write and fsync a journal containing target paths, expected old hashes, new
   hashes, and the ordered replacement set.
7. Atomically replace canonical targets one at a time and fsync their parent
   directories.
8. Mark the journal complete, update the disposable index, then remove staging
   data.

If the process stops after the journal is durable, opening the project verifies
the hashes and rolls the already-validated transaction forward. If it stops
before that point, staging is discarded. Recovery never guesses whether a
partially written logical mutation should be kept.

Canonical files commit before index updates. If SQLite work fails, the files
remain authoritative, the index is marked stale, and a rebuild is scheduled.
Index failure must never cause valid canonical data to be rolled back.

Only one Daena process may hold the project writer lock. Read-only inspection
can continue where safe. External editors are not expected to honor the lock,
so content revisions and the watcher remain mandatory.

## External Edit Reconciliation

The core watches `project.json`, `entities/`, `plugins/`, and `assets/` with a
debounced filesystem watcher. It suppresses events produced by its own known
transaction hashes rather than relying only on timestamps.

- A valid external edit to a clean record is parsed, indexed, and emitted to
  the UI automatically.
- An edit overlapping an unsaved in-app draft preserves the draft and creates
  a conflict state. The UI offers compare, reload disk, overwrite using an
  explicit new revision, or save the draft as a recovery copy.
- Invalid JSON, schema violations, hash mismatches, missing assets, or broken
  references keep the affected record read-only and surface diagnostics.
- The last valid index may continue serving unaffected records, but it is never
  presented as proof that invalid source files are healthy.
- Git-unmerged files are detected through Git where available. Daena does not
  auto-resolve them or commit while they remain unresolved.

Recovery copies belong in `.daena/conflicts/` and are ignored by Git. They are
never silently promoted into canonical project data.

## Disposable SQLite Index

The index lives at `.daena/index.sqlite`. Its WAL, SHM, journal, temporary
rebuild database, and all other `.daena/` contents are ignored by Git.

The index contains normalized projections for entities, documents, fields,
relationships, assets, plugin data state, and FTS. A source-file table records
every canonical path, content hash, parsed format version, and logical revision.
No row exists without enough source information to prove where it came from.

### Full rebuild

1. Validate `project.json` and the canonical path structure.
2. Enumerate files without following symlinks or crossing the project root.
3. Parse and validate each independent record.
4. Run a second pass for global ID uniqueness, references, namespaces, schemas,
   migration history, and asset hashes.
5. Populate `.daena/index.sqlite.next` in one SQLite transaction.
6. Build FTS and all derived projections.
7. Run foreign-key, integrity, and source-count checks.
8. Close database handles and atomically replace the previous index.

A fresh clone with no `.daena/` directory must rebuild and open normally. If a
rebuild fails and an older index exists, Daena may retain it for unaffected
read-only views while clearly reporting that it is stale. On a fresh project
with invalid canonical files, opening stops in repair mode rather than creating
a partial authoritative interpretation.

Incremental watcher updates use the exact same parser, validators, and
normalization code as a full rebuild.

## Git Integration

The generated `.gitignore` includes:

```gitignore
.daena/
daena.sqlite
daena.sqlite-*
```

The old database patterns remain ignored during alpha development so a stale
local database cannot be committed accidentally.

Before the built-in Git commit operation, Daena must:

1. flush pending autosaves;
2. finish or recover journaled transactions;
3. validate changed canonical files;
4. ensure the index represents their current hashes;
5. reject unresolved Git conflicts and invalid records; and
6. show exactly which canonical files and assets will be staged.

Daena continues to perform commits only through an explicit user action. It
does not automatically commit autosaves or resolve merges.

## Markdown Editor Contract

The editor loads Markdown into its internal rich-text model and serializes the
model back to deterministic Markdown. Supported editor features must have a
defined round trip:

- headings, paragraphs, emphasis, strong text, strikeout, links, inline code,
  fenced code blocks, block quotes, lists, and horizontal rules use
  CommonMark/GFM;
- underline and alignment use a narrowly allowed sanitized HTML representation
  if retained in the toolbar; and
- unsupported HTML, scripts, event handlers, unsafe URLs, and arbitrary style
  attributes are rejected or sanitized before rendering.

Loading and saving a document without logical edits must not change its bytes.
Formatting commands must not rewrite unrelated sections of a document. Source
fixtures cover every supported editor construct and repeated round trips.

## Public Contract Changes

- `DocumentRecord.body` contains Markdown and its canonical `format` is only
  `"markdown"` for format version 2.
- Mutable entity/document records expose an opaque revision.
- Update and delete operations require an expected revision and return the new
  revision.
- `ProjectInfo.database` is replaced with index/cache status; the physical path
  is not part of the plugin contract.
- Project APIs expose typed validation diagnostics, index state, external-change
  notifications, and conflicts.
- The broker preserves capability checks, namespace ownership, project/session
  binding, request IDs, and revocation before reaching the repository.
- Search remains an API over derived data. A stale or rebuilding index is a
  typed state rather than an empty successful result.

Because the product is in alpha, these contracts can change directly. The
repository should update generated Rust/TypeScript types, bundled modules,
test-host contracts, and SDK conformance fixtures together rather than carry
deprecated storage variants.

## Plugin Schema and Data Migrations

Plugin data migrations remain necessary even though the old project format is
not migrated. A plugin upgrade may change its own fields and schema version.

The verified package manifest selects an exact contiguous migration chain. The
core validates ownership, checksums, source/target versions, and destructive
recovery requirements. All affected entity field files and the plugin ledger
are committed through one filesystem journal transaction. Runtime plugin code
cannot submit arbitrary migration operations.

Pre-migration compatibility backups are stored locally below `.daena/backups/`
and include the affected canonical files plus hashes. Restore is a new
journaled mutation and never writes directly over active files without revision
checks. Successful migration history remains portable in
`plugins/<plugin-id>.json`; backup locations do not.

## Implementation Sequence

### Phase 1: Format and codec

- Define Rust types and strict schemas for every canonical JSON file.
- Implement deterministic serialization, path normalization, validation, and
  golden fixtures.
- Replace the alpha project initializer with the version-2 directory layout and
  `.gitignore`.
- Delete assumptions that `daena.sqlite` is canonical; no compatibility path is
  retained.

Exit gate: a project fixture round-trips to byte-identical canonical files and
invalid path/schema fixtures fail with stable diagnostics.

### Phase 2: Rebuildable repository and index

- Introduce the filesystem repository boundary.
- Implement complete canonical scanning, cross-record validation, and atomic
  index rebuild.
- Port reads and search to the new index while ensuring every returned record
  is traceable to current source hashes.

Exit gate: deleting `.daena/` and reopening reconstructs an equivalent project
and search index without data loss.

### Phase 3: Journaled mutations

- Implement the writer lock, expected revisions, transaction journal, crash
  recovery, idempotent request IDs, and atomic asset imports.
- Port entity, document, field, relationship, asset, plugin-state, backup, and
  plugin-migration mutations.
- Remove SQLite-authoritative writes rather than supporting dual persistence.

Exit gate: crash injection at every transaction step produces either the
complete previous state or the complete new state after recovery.

### Phase 4: Markdown editor and external edits

- Convert the editor input/output contract from stored HTML to Markdown.
- Add deterministic Markdown parsing/serialization and sanitization fixtures.
- Add filesystem watching, conflict state, diagnostics, recovery copies, and
  UI reconciliation.

The Phase 4 implementation uses a deterministic supported Markdown subset in
the rich editor. Tauri runs a debounced background filesystem watcher over the
directory-backed project and reconciles valid source changes into SQLite only
after the canonical scan succeeds. Invalid scans remain diagnostic-only; the
last valid projection is not presented as healthy, and a changed document with
an unsaved draft is surfaced as a conflict. Draft recovery copies are written
below `.daena/conflicts/` and are never included in canonical scans. Git-
unmerged canonical files are reported before scanning, and the built-in commit
path refuses to proceed while those diagnostics remain.

Exit gate: supported rich editing round-trips through Markdown, external clean
edits refresh live, and conflicting edits never overwrite an unsaved draft.

### Phase 5: Git and plugin contract completion

- Add Git preflight validation and precise staging previews.
- Update broker RPC schemas, generated SDK types, bundled modules, plugin test
  host, conformance tests, and administration state.
- Update `docs/PLAN.md`, `docs/ARCHITECTURE.md`, plugin ADRs, SDK documentation,
  and the example project so files are canonical and SQLite is explicitly
  derived.

The Phase 5 Git boundary exposes a typed preflight/staging preview. It recovers
pending transactions, validates the current canonical scan and source-index
hashes, rejects unmerged files and pre-staged noncanonical work, and reports
the exact canonical paths and asset paths that the explicit commit action will
stage. The broker and generated SDK contracts carry canonical revisions on
reads; mutable calls require `expectedRevision`, while the RPC envelope keeps
the retry `requestId` authoritative. Bundled modules, the in-memory plugin
test host, and administration view use the same contract.

Exit gate: bundled and third-party test plugins use only revision-aware broker
APIs, and a fresh Git clone opens after rebuilding its ignored index.

## Test Plan

### Format and validation

- Golden tests for byte-stable JSON and Markdown output.
- Reject duplicate keys, invalid UTF-8, unsafe paths, symlinks, case collisions,
  invalid IDs, duplicate IDs, dangling references, namespace violations, and
  asset hash mismatches.
- Verify renaming or retyping an entity does not move its directory or break
  references.
- Verify archiving preserves documents, fields, relationships, and assets.

### Index and search

- Delete the complete `.daena/` tree and rebuild an equivalent index.
- Compare entity, document, field, relationship, asset, plugin-state, and FTS
  projections before and after rebuild.
- Verify incremental watcher indexing and full rebuild interpret identical
  bytes identically.
- Test empty, stale, corrupt, interrupted, and concurrently requested rebuilds.

### Transactions and concurrency

- Inject termination before and after every journal/staging/replacement/fsync
  step.
- Verify recovery is idempotent and leaves no mixed logical mutation.
- Verify stale revisions cannot overwrite app, plugin, or external-editor
  changes.
- Verify duplicate request IDs cannot duplicate entities, relationships, or
  assets.
- Exercise lock acquisition, second-instance read-only behavior, and lock
  cleanup after abnormal termination.

### External edits and Git

- Test clean document and metadata edits, rapid write bursts, atomic editor
  replacements, deletes, invalid intermediate saves, and self-event suppression.
- Test an external edit during a dirty draft and every explicit resolution
  choice.
- Test Git-unmerged Markdown and JSON, resolved merges, branch switches, and
  index rebuild after checkout.
- Confirm `.daena/` and SQLite files never appear in ordinary Git status while
  canonical edits produce focused diffs.

### Editor and plugins

- Round-trip every supported editor construct through Markdown repeatedly.
- Reject unsafe embedded HTML and URLs without corrupting surrounding content.
- Verify plugin access is still capability-, session-, project-, and
  namespace-bound.
- Verify plugin data survives disablement, missing code, clone/rebuild, schema
  migration, failed migration, and local backup restore.
- Verify a cloned project never inherits capability consent or activates plugin
  code automatically.

### Repository validation

Run the cached repository checks after each phase:

```sh
cargo test --manifest-path src-tauri/Cargo.toml --locked --offline
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --offline --all-targets -- -D warnings
deno task check
deno task build
deno task check:plugin-contract
deno task check:plugin-isolation
deno task test:plugin-conformance
```

Also run targeted filesystem tests on macOS, Linux, and Windows because atomic
replacement, file watching, path casing, locked SQLite handles, and directory
fsync behavior vary by platform.

## Acceptance Criteria

- A project containing no SQLite database opens with all canonical data intact.
- Deleting `.daena/` changes performance only; it never removes project data.
- Author edits appear as Markdown diffs and structured changes appear as focused
  JSON diffs.
- Stable IDs keep relationships valid across renames and module projections.
- A crash during any core mutation cannot leave an unrecoverable partial logical
  write.
- Concurrent or external edits cannot be silently overwritten.
- Invalid canonical files produce actionable diagnostics and are never replaced
  by stale index rows.
- Plugin data remains portable while grants and execution consent remain local.
- Plugins continue to operate only through the authorized broker and receive no
  new filesystem authority.
- A fresh Git clone can rebuild all indexes deterministically and pass project
  validation without an export/import step.

## Decisions

- Canonical structured data uses strict deterministic JSON, not YAML or TOML.
- External edits are watched and reconciled while a project is open.
- Only portable plugin data, schema versions, and migration history are tracked;
  capability grants and activation consent are machine-local.
- Assets retain their native formats and are verified by SHA-256.
- SQLite is always disposable derived state.
- The alpha release adopts the new format directly and provides no project-data
  migration from the existing SQLite-canonical layout.
