# Daena Storage Architecture

## Status and authority

This document is the definitive target architecture for Daena project storage.
It replaces the design in [`STORAGE_PROPOSAL.md`](./STORAGE_PROPOSAL.md) and
supersedes the source-of-truth, transaction, startup, and SQLite-role decisions
in [`PLAIN_TEXT_STORAGE_PLAN.md`](./PLAIN_TEXT_STORAGE_PLAN.md).

The existing portable project format remains version 2 unless this document
explicitly changes it. The current implementation is still repository-first;
[`STORAGE_MIGRATION.md`](./STORAGE_MIGRATION.md) defines a hard cut to this
architecture. Pre-cut `.daena/` state is unsupported and receives no database
migration, compatibility reader, feature flag, dual-write period, or fallback.

This document governs:

- authority and durability boundaries;
- the SQLite runtime store;
- the portable project representation;
- synchronization, conflicts, and recovery;
- startup, shutdown, Git, backup, and rebuild behavior;
- plugin and asset storage boundaries; and
- the invariants every implementation phase must preserve.

## 1. Decision

Daena uses two representations with different jobs:

- **SQLite is the authoritative runtime store and primary transaction
  boundary.** Application reads and mutations operate on SQLite.
- **Plain-text files and native assets form the portable project
  representation.** They are intended for Git, external editing, inspection,
  interchange, backup, and reconstruction.
- **A durable synchronization queue connects them.** Every runtime mutation
  that affects portable data records its export work in the same SQLite
  transaction as the mutation.
- **The portable representation is a checkpoint, not a second live
  authority.** A clean checkpoint reconstructs the same durable project state.
  A dirty runtime database may contain newer committed work.

The concise mental model is:

> SQLite is the working project; files are its portable checkpoints;
> synchronization advances the checkpoint; external edits are reconciled.
> External edits are reconciled through a three-way comparison.

## 2. Corrections to the proposal

The proposal's direction is valid, subject to the following mandatory
clarifications.

1. **Database commit and portable checkpoint are different durability
   states.** After a SQLite commit but before export completes, deleting or
   rebuilding the database can lose the newer mutation. Daena must expose this
   as dirty state and must never call such a rebuild lossless.
2. **Rebuild is conditional, not automatically safe.** Rebuilding from files
   is lossless only when the synchronization state is clean. Dirty, failed, or
   conflicted state must be flushed, recovered, archived, or explicitly
   discarded by the user.
3. **Native assets are not ordinary database rows.** SQLite owns their logical
   metadata, but large bytes remain managed files. New or replaced bytes must
   be durably staged before a database row is allowed to reference them.
4. **Fast startup cannot permit blind writes.** The application may show
   database-backed reads before a complete offline-change scan, but every
   mutation must verify the affected portable paths against the last-sync
   baseline until reconciliation is complete.
5. **A multi-file export is not filesystem-atomic.** The database and its
   durable export queue are the recovery authority while a batch is in
   progress. Portable files are declared a complete checkpoint only after the
   whole logical batch finishes.

These are correctness requirements, not implementation options.

## 3. Terminology

**Runtime state**
: Project data committed to SQLite and visible through core APIs.

**Portable state**
: Deterministic JSON, Markdown, and native asset files below the project root.

**Portable record**
: The smallest semantic unit independently classified for synchronization,
  such as project metadata, an entity record, a document, a field namespace,
  a source-owned relationship set, an asset ledger, an asset payload, or a
  plugin state record.

**Baseline**
: The last representation known to have equivalent database and disk content
  for a portable record. It includes existence as well as a content hash.

**Dirty**
: The database representation differs from the baseline and has pending export
  work.

**Clean checkpoint**
: No portable record is dirty, exporting, failed, or conflicted, and the
  portable files validate as the representation recorded by the database.

**External change**
: A disk representation that differs from the baseline and was not produced by
  Daena's current export batch.

**Conflict**
: Database and disk representations both changed from the same baseline, or an
  external change cannot be safely classified or imported.

**Rebuild**
: Construction of a new runtime database from a validated portable checkpoint.

## 4. Authority by lifecycle state

| State | Runtime authority | Permitted behavior |
| --- | --- | --- |
| No usable database | Portable files | Validate and import into a new database |
| Valid database, clean | SQLite | Open immediately; reconcile external changes |
| Valid database, dirty | SQLite | Resume export; do not rebuild from files silently |
| Export in progress | SQLite plus durable queue | Resume or finish the recorded batch |
| External-only change | Baseline classifies disk as newer | Validate and import the changed dependency scope |
| Database-only change | SQLite | Export the queued target representation |
| Two-sided change | Neither side wins automatically | Enter typed conflict resolution |
| Widespread external divergence | User decision | Review, preserve DB, or rebuild from files |
| Corrupt database | Portable checkpoint plus recovery artifacts | Archive first; recover or rebuild with stated loss boundary |

SQLite authority never grants callers a raw database handle. The Rust core
remains the only mutation, validation, revision, authorization, and
synchronization authority.

## 5. Durability contract

### 5.1 Mutation states

A portable mutation progresses through these states:

```text
request
  -> SQLite transaction and durable export intent
  -> runtime committed
  -> portable bytes staged and validated
  -> portable paths replaced
  -> synchronization baseline advanced
  -> clean checkpoint
```

The SQLite mutation, opaque record revisions, idempotency receipt, and export
intent must commit atomically. A process crash cannot produce changed runtime
rows without recoverable knowledge of the required export.

The UI may use the runtime-committed state immediately. It must retain a
visible or queryable saving/synchronizing state until portable persistence is
complete. An explicit save, close, Git, export, or portable-backup operation
must report failure if its required flush does not finish.

### 5.2 What recovery guarantees

- A crash before SQLite commit leaves the mutation unapplied.
- A crash after SQLite commit leaves either completed export work or durable
  pending export work in the database.
- A crash during file replacement is resumed from the durable queue. Already
  replaced paths are verified rather than blindly rewritten.
- A clean portable checkpoint can recreate all durable project content without
  the previous database.
- A dirty database can contain committed content that the portable checkpoint
  does not yet contain. Manual deletion of that database is destructive.
- If the database becomes unrecoverable while dirty, Daena can guarantee the
  last clean portable checkpoint, not unexported work. It must say so plainly
  and preserve the damaged database and WAL for recovery attempts.

### 5.3 Flush barriers

The following operations require a successful portable flush barrier:

- built-in Git status intended for commit, staging, commit, reset, or branch
  operations;
- portable export, archive, copy, and backup;
- an explicit user save or sync command;
- database rebuild or discard;
- project format migration;
- plugin data migration when portable state changes; and
- project close when the product promises that all edits are saved.

Application shutdown should attempt the same barrier. If the operating system
terminates the process before it completes, the durable database queue remains
the recovery path on next open.

## 6. Runtime database

The runtime database remains at `.daena/index.sqlite` and is ignored by Git.
The name is retained to avoid unnecessary project-layout churn even though the
database is no longer merely an index.

### 6.1 Required content

SQLite stores:

- authoritative runtime rows for entities, documents, fields, relationships,
  asset metadata, portable plugin data, and migration history;
- opaque logical revisions and a database epoch;
- idempotent mutation receipts keyed by request ID;
- portable-record baselines and database content hashes;
- durable export batches and items;
- conflict and diagnostic state needed to block unsafe writes;
- project identity and portable-format compatibility metadata; and
- derived FTS, map, relationship, AI-adjacent, and performance projections as
  appropriate.

Derived data must remain distinguishable from durable runtime rows so it can
be rebuilt without modifying project content.

### 6.2 Required metadata

At minimum, the database records:

- runtime schema version;
- project ID from `project.json`;
- portable format version;
- database epoch;
- last clean-shutdown observation;
- synchronization state and dirty count;
- last observed Git identity when Git is present; and
- exporter/reconciler versions needed to reject incompatible local state.

Database schema version and portable project format version are independent.
A runtime schema migration must not rewrite project files unless the portable
format also changes.

### 6.3 SQLite configuration

The core owns SQLite configuration. WAL mode is appropriate for runtime
responsiveness, but durability settings must be selected deliberately and
tested. The implementation must:

- enable foreign-key enforcement on every connection;
- use transactions for every logical mutation;
- checkpoint or preserve WAL correctly before database backup or replacement;
- avoid sharing a `rusqlite::Connection` across unsupported thread boundaries;
- serialize writers through a core-owned project session; and
- verify schema and project identity before serving writes.

### 6.4 Revisions and retries

Every broker-visible mutable record exposes an opaque revision. A mutation
must supply the observed revision where a record already exists. The core
checks it inside the same SQLite transaction as the write.

Revisions are scoped to a database epoch. Rebuilding the database creates a
new epoch, invalidating all previously issued revisions. Callers must reload.

Retryable mutations retain request IDs. A repeated request returns the stored
result or a typed incompatible-retry error; it never duplicates an entity,
relationship, asset, or migration.

## 7. Portable project representation

The portable layout remains:

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
  sync/
  conflicts/
  backups/
  local/
.gitignore
```

`.daena/` is machine-local. A fresh clone excludes it and reconstructs a new
database from the portable files.

### 7.1 Portable content

- `project.json` carries portable project identity, name, creation time, and
  portable format version.
- `entity.json` carries stable identity and entity metadata. The UUID
  directory is never derived from the display name.
- `document.md` contains deterministic Markdown, never editor-private HTML or
  a serialized editor state.
- `fields/*.json` contains one plugin/namespace-owned field map per file.
- `relationships.json` contains relationships owned by that source entity.
- `assets.json` contains logical asset metadata and hashes for assets attached
  to the entity.
- `plugins/<plugin-id>.json` contains portable interpretation state, data/schema
  versions, migration history, enabled state, and preserved data.
- `assets/**` contains native payload bytes.

Capability grants, installed packages, execution consent, sessions, provider
credentials, runtime health, diagnostics, and caches are not portable project
data.

### 7.2 Encoding and path rules

Structured files use strict UTF-8 JSON with duplicate-key rejection, two-space
indentation, deterministic key ordering, LF line endings, and a final newline.
Markdown uses UTF-8, LF, deterministic supported syntax, and a final newline.

The storage boundary rejects:

- unknown fields at a versioned strict boundary;
- invalid UUIDs, namespaces, ownership, or references;
- absolute paths, traversal, root escape, unsafe symlinks, and case collisions;
- invalid UTF-8, duplicate JSON keys, NaN, infinities, or implicit coercion;
- duplicate stable IDs and dangling live references;
- Git conflict markers in structured data; and
- asset size or SHA-256 mismatches when payload validation is required.

Known OS metadata files may be ignored only by the explicit allowlist already
covered by tests. Other unexpected entries remain diagnostics.

### 7.3 Determinism

Equivalent database state must serialize to byte-identical portable bytes.
Serializers must not change timestamps or reorder meaningful arrays merely
because export ran. A record export must not rewrite an unrelated path.

## 8. Synchronization model

### 8.1 Record identity

Synchronization is semantic, not merely path-based. Each record has a stable
key such as:

```text
project:<project-id>
entity:<entity-id>
document:<document-id>
fields:<entity-id>:<plugin-id>:<namespace>
relationships:<source-entity-id>
asset-ledger:<entity-id>
asset-payload:<asset-id>
plugin:<plugin-id>
```

The baseline records the expected portable path, whether it exists, its
content hash, the corresponding database hash/revision, and the last completed
batch. Deletion is a first-class state, not an absent metadata row.

### 8.2 Durable export queue

Every database transaction that changes portable content inserts or coalesces
export items in that same transaction. An export item identifies:

- logical batch and request ID;
- portable record key and target path;
- expected baseline existence/hash;
- target database revision/hash;
- operation: create, replace, or remove;
- staging source for native bytes when applicable;
- state, attempt count, and typed last error; and
- dependencies needed to serialize a consistent logical batch.

Document keystrokes may coalesce to the newest committed revision. Structural
operations and explicit saves may request immediate export. Coalescing must
not discard a request receipt or allow an older revision to overwrite a newer
one.

### 8.3 Export algorithm

For each logical batch, the exporter:

1. reads a consistent SQLite snapshot at the target revisions;
2. renders only affected deterministic portable records;
3. writes new bytes below `.daena/sync/<batch-id>/`, fsyncing files and needed
   directories;
4. validates rendered bytes and computes target hashes;
5. verifies every current target against the recorded baseline before applying
   any item;
6. atomically replaces or removes each target, checking again immediately
   before each replacement where an external editor could race;
7. fsyncs affected portable directories where the platform supports it;
8. records each applied item so a crash can resume idempotently;
9. advances all baselines and completes the batch in a SQLite transaction; and
10. removes staging only after completion is durable.

If a target no longer matches its baseline, export stops and records a typed
conflict. It never overwrites the external bytes automatically.

The exporter may reuse safe staging, hashing, path-normalization, and atomic
replacement primitives from the current filesystem transaction code. The old
repository-first full-snapshot transaction is not retained as a second
authority.

### 8.4 Multi-file visibility

Portable files can temporarily show part of an export batch because ordinary
filesystems do not provide atomic multi-file commits. Therefore:

- Git, backup, export, and rebuild are blocked until the batch completes;
- the database queue is the recovery authority during the batch;
- external reconciliation recognizes the current batch's target hashes;
- a crash resumes the batch before normal writes; and
- a missing database during a partial batch is repair mode, not an
  automatically valid import.

## 9. Native assets

SQLite stores asset identity, ownership, path, MIME type, size, hash, and
logical revision. Native payload bytes remain files; large assets are not
stored as SQLite BLOBs by default.

For a Daena-originated import or replacement:

1. stream bytes into a managed staging file below `.daena/sync/`;
2. compute size and SHA-256 while streaming;
3. fsync the staged payload;
4. commit asset metadata and an export item that references the staged payload
   in one SQLite transaction; and
5. move the staged payload to its portable path through the exporter.

A database transaction must never reference ephemeral bytes that can disappear
before export. Failed payload export leaves the asset dirty and retryable.

Fast startup must not re-read or re-hash unchanged payloads. Reconciliation may
use path identity and cheap filesystem metadata as a hint, but a claimed asset
change or a rebuild must ultimately verify content using SHA-256. Opaque map
files remain native payloads and are parsed only by the responsible provider or
projection path, not by general storage startup.

## 10. Startup and project sessions

### 10.1 Project lock

One Daena project session owns the SQLite writer and synchronization workers.
A second process may open only in an explicitly supported read-only mode. Lock
recovery must verify ownership rather than deleting a lock merely because it
is old.

### 10.2 Fast path with an existing database

Normal open performs the blocking minimum:

1. validate the project root and read `project.json`;
2. acquire the project session lock;
3. open SQLite and verify schema compatibility and project identity;
4. recover SQLite/WAL and inspect durable export state;
5. resume or classify interrupted export batches;
6. start the filesystem watcher before leaving an unobserved mutation window;
7. expose database-backed reads with an explicit reconciliation state; and
8. reconcile offline file and Git changes incrementally in the background.

Routine open must not parse every entity, document, map, or asset before
showing the project. Expensive integrity checks may run after open unless cheap
metadata or SQLite errors require repair mode.

### 10.3 Write gate during reconciliation

Until a record's disk state has been reconciled, a mutation touching it must
compare its current path existence/hash with the baseline inside the operation's
preflight. Creation checks the target path and case-folded collision set.

If the check differs, the mutation pauses for reconciliation or returns a
typed conflict. This preserves fast reads without allowing a stale database to
overwrite an offline edit.

### 10.4 Missing or unusable database

Daena scans and validates the complete portable representation, creates
`.daena/index.sqlite.next`, imports durable rows, builds derived indexes,
establishes baselines, verifies the result, and atomically installs the new
database. Invalid files open repair mode; they do not produce a partially
authoritative database.

## 11. External edits and conflicts

Filesystem events are hints. Classification uses database state, the baseline,
and current disk content:

| Database vs baseline | Disk vs baseline | Classification | Action |
| --- | --- | --- | --- |
| Same | Same | Clean | None |
| Changed | Same | Database-only | Export |
| Same | Changed | External-only | Validate and import |
| Changed | Changed | Two-sided | Conflict |

Existence participates in the comparison, so external creates and deletes use
the same rules.

An external-only import must:

- use the same parser and validators as a rebuild;
- validate the affected dependency closure, not only JSON syntax;
- commit runtime changes and the new baseline atomically;
- update derived projections incrementally; and
- emit new opaque revisions to invalidate stale callers.

Invalid external data creates diagnostics and makes the affected record or
dependency scope read-only. The last valid database row may be displayed as
stale context but is never presented as proof that disk is valid.

Conflict actions are explicit:

- **Use database version:** preserve the disk version as a recovery copy,
  adopt its hash as the reviewed baseline, then export the database version.
- **Use disk version:** preserve or archive the database version, validate the
  disk dependency scope, then import it.
- **Compare/manual resolution:** keep both sides until the user submits a new
  revision against the current conflict token.
- **Rebuild from files:** project-level destructive resolution, subject to the
  rebuild rules below.

Unsaved editor drafts are a third UI state above storage. They are never
silently replaced by either database reconciliation or disk import.

## 12. Substantial divergence and Git

Git checkout, merge, rebase, reset, or restoration can replace much of the
portable tree. A changed Git HEAD/index identity, many changed record keys, or
cross-record validation failures may trigger project-level divergence mode.

In divergence mode Daena:

- stops normal writes;
- preserves dirty database state and drafts;
- reports the scope and synchronization status;
- offers review, finish exporting the database, or rebuild from files; and
- never creates hundreds of automatic overwrite prompts.

Before built-in Git staging or commit, Daena must:

1. flush pending editor autosaves to SQLite;
2. complete all portable export batches;
3. validate changed portable records and unresolved Git state;
4. show the exact portable paths and assets to stage;
5. exclude `.daena/` and all SQLite sidecars; and
6. stage or commit only after explicit user action.

After a built-in operation that changes the worktree, Daena reconciles before
re-enabling writes. External Git commands are handled as external changes.

## 13. Rebuild, repair, and database replacement

### 13.1 Safe rebuild

A normal rebuild requires a clean checkpoint. Daena then:

1. closes or quiesces writers and synchronization workers;
2. takes a recoverable local database backup;
3. validates the full portable representation;
4. builds and verifies a new database at a temporary path;
5. creates a new database epoch and baselines;
6. atomically replaces the old database only after verification; and
7. retains the previous backup until the new database opens successfully.

### 13.2 Dirty rebuild

If the database is dirty, exporting, failed, or conflicted, Daena first offers
to flush or resolve it. Discarding runtime changes requires an explicit
destructive confirmation that identifies the affected records. Before discard,
Daena archives the database, WAL, staged payloads, synchronization queue, and
available drafts below machine-local recovery storage.

The product must not describe a dirty rebuild as repair without data loss.

### 13.3 Corruption and incompatibility

For a corrupt database, preserve the database and sidecars before attempting
SQLite recovery. If recovery cannot restore dirty rows, rebuild guarantees only
the last valid portable checkpoint.

During the alpha hard cut, a pre-cut or otherwise incompatible runtime schema
is never migrated. Open returns a typed reset-required error; after the user
removes `.daena/`, Daena constructs a new runtime database from the portable
version-2 files. Once a database has been created by the new architecture,
dirty state remains protected by the rebuild and recovery rules above. Any
future post-cut schema migration policy requires a separate architecture
decision.

## 14. Plugin, module, and AI boundaries

Plugins and bundled modules use logical core APIs. They never receive SQLite
connections, raw project paths, arbitrary filesystem access, or authority to
manage synchronization.

Portable plugin data is exported through registered, deterministic core-owned
codecs and remains namespace-owned. Plugin-derived caches may remain
SQLite-only. Installed packages, grants, sessions, runtime diagnostics, and
execution consent remain machine-local.

Plugin data migrations execute within the runtime transaction boundary and
queue the resulting portable changes atomically. Destructive migration backup
or package-switch workflows use a flush barrier before declaring success.

AI retrieval indexes remain separate, derived, permission-aware local state.
Accepted AI changes use the same revision-aware runtime mutations and portable
export queue as user or plugin changes.

## 15. Backup and export

Daena distinguishes:

- **portable backup:** flush, validate, then copy/archive only portable files;
- **runtime recovery backup:** use a consistent SQLite backup including the
  state represented by WAL and retain referenced staged payloads; and
- **Git snapshot:** flush and validate, then stage the exact portable changed
  set.

A portable backup taken without a successful flush must be labeled as the last
portable checkpoint, not as a backup of current runtime state.

## 16. Performance contract

The common path is an existing local project with a valid database.

- Blocking startup work is independent of entity and asset count except for
  cheap session/database metadata.
- Reads use SQLite and derived indexes.
- A small mutation updates only its logical database rows, revisions, derived
  rows, and export items.
- Export serializes only affected portable records.
- Document edits may debounce and coalesce without losing the newest committed
  revision.
- Unchanged native assets are not parsed or hashed on routine open.
- Full parsing is reserved for first import, explicit rebuild, repair, format
  migration, or exceptional reconciliation.

Performance claims require recorded fixtures and before/after measurements.
Correctness gates may not be removed merely to meet a timing target.

## 17. Invariants

1. A successful runtime mutation and all work needed to export it are committed
   atomically in SQLite.
2. No core path mutates portable project data without synchronization metadata
   and expected-baseline checks.
3. A clean portable checkpoint reconstructs all durable user-authored project
   state without the previous database.
4. Dirty database state is never silently discarded or described as already
   portable.
5. Independently changed database and disk versions are never silently
   overwritten.
6. A normal open with a valid database does not require a full portable scan
   before reads become available.
7. Writes cannot race ahead of reconciliation for the records they touch.
8. A failed export does not roll back a committed database mutation; it remains
   dirty, visible, diagnosable, and retryable.
9. Derived indexes can be deleted and rebuilt without altering durable runtime
   or clean portable content.
10. A fresh clone containing only portable files reconstructs an equivalent
    clean runtime project.
11. Plugins cannot bypass the Rust authority, revision, namespace, or
    synchronization boundaries.
12. Git, portable backup, export, and rebuild operate only across an explicit
    flush barrier.

## 18. Non-goals

This architecture does not provide:

- SQLite as a Git or collaboration format;
- byte-for-byte file synchronization after every keystroke;
- filesystem-wide atomic snapshots;
- automatic semantic merge for every conflict;
- unrestricted plugin filesystem access;
- cloud synchronization or multi-user concurrent editing;
- lossless recovery of unexported rows from an unrecoverably corrupt database;
- a legacy reader for pre-format-version-2 projects; or
- automatic activation of plugin code from cloned portable state.

## 19. Acceptance criteria

The storage migration is complete only when all of the following are proven:

- an unchanged indexed project opens without a blocking full-tree scan;
- application reads and writes use the runtime database directly;
- every portable mutation creates durable export intent in the same SQLite
  transaction;
- crash injection at every database, staging, replacement, and baseline step
  resumes without silent overwrite or lost queue state;
- clean close, Git, backup, and rebuild flush and validate portable state;
- dirty rebuild is blocked until resolved or explicitly archived/discarded;
- external-only edits import, database-only edits export, and two-sided edits
  conflict at semantic record scope;
- writes during background reconciliation check affected baselines;
- a fresh clone and a clean delete-`.daena/` rebuild produce equivalent data,
  search, map, relationship, and plugin projections;
- new and replaced native assets survive crashes between staging, commit, and
  portable placement;
- invalid paths, symlinks, schemas, references, namespaces, and asset hashes
  remain rejected with stable diagnostics;
- Git never stages `.daena/` or an unflushed/inconsistent portable state; and
- focused performance fixtures show ordinary edits scale with their changed
  record set rather than total project size.

## 20. Architecture summary

```text
                           logical core APIs
                                  |
                                  v
                    +-----------------------------+
                    | SQLite runtime transaction  |
                    | data + revisions + outbox   |
                    +--------------+--------------+
                                   |
                         durable export worker
                                   |
                       baseline/conflict checks
                                   |
                                   v
             Markdown + strict JSON + native asset files
                 Git / external tools / clone / rebuild
```

SQLite supplies fast, transactional runtime behavior. Portable files supply
meaningful long-lived interchange. The synchronization baseline and durable
queue make the boundary explicit instead of pretending both representations
commit atomically.
