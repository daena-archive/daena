# Database-First Storage Hard-Cut Plan

## Status and intent

This plan guides the direct replacement of the current repository-first
storage implementation with the database-first architecture in
[`STORAGE.md`](./STORAGE.md).

This is an alpha hard cut. There is no backward-compatible runtime, database
migration, compatibility reader, feature flag, dual-write period, or staged
coexistence of old and new storage authorities.

The version-2 portable project representation remains the project interchange
format because its JSON, Markdown, and native assets already satisfy the target
contract. That is not a compatibility layer: importing portable files is the
normal first-open and rebuild path defined by `STORAGE.md`.

## 1. Hard-cut contract

The implementation must follow these rules exactly.

1. **Pre-cut `.daena/` state is unsupported.** The current
   `.daena/index.sqlite`, WAL/SHM files, filesystem journals, receipts, backups,
   conflicts, locks, and other repository-first local artifacts are not read,
   migrated, copied, or promoted.
2. **The user or developer resets local state once.** Before opening an
   existing alpha project with the hard-cut build, remove its complete
   `.daena/` directory. The portable files remain and initialize a new runtime
   database.
3. **Runtime code does not perform the reset destructively.** If an existing
   database lacks the exact new storage role and schema version, opening returns
   a typed unsupported-storage error instructing the user to close Daena and
   remove `.daena/`. It does not inspect old rows or attempt conversion.
4. **No legacy database schema migration exists.** The new schema is created
   only for a missing database. An incompatible database is rejected.
5. **No old writer remains active or dormant.** The change that introduces the
   database-first writer also removes the repository-first writer and its
   receipts, recovery dispatch, and normal-path tests.
6. **No feature switch exists.** Tests, development builds, and production
   builds all use the same database-first path.
7. **No dual writes exist.** A mutation commits to SQLite with durable export
   intent. Portable files are written only by the new exporter.
8. **No compatibility fallback exists.** Failure in the new path is a source
   defect or typed operational error; it never falls back to a snapshot or
   filesystem-first mutation.
9. **No old-project fixture is supported.** Tests create either a fresh
   portable version-2 project with no `.daena/`, or a database created by the
   new runtime schema.
10. **Portable validation remains strict.** The hard cut removes old runtime
    machinery, not path, schema, reference, namespace, asset, Git-conflict, or
    deterministic-encoding safeguards.

## 2. What is retained and what is deleted

### Retained

- `project.json` portable format version 2;
- entity UUID directories;
- deterministic `entity.json`, field JSON, relationship JSON, asset ledgers,
  and plugin state JSON;
- deterministic Markdown documents;
- native asset files and SHA-256 validation;
- `FilesystemRepository` parsing and full validation for first import, rebuild,
  repair, and exceptional divergence;
- revision-aware logical core and broker APIs;
- request IDs and idempotency semantics, reimplemented in SQLite;
- namespace ownership and Rust authorization;
- Git staging of portable paths only; and
- safe path, staging, hashing, fsync, and atomic single-file replacement
  primitives that fit the new exporter.

### Deleted or replaced

- promotion or reuse of the repository-first `.daena/index.sqlite`;
- `ProjectStore::repository_first_mutation`;
- `ProjectStore::commit_canonical_snapshot`;
- `ProjectStore::sync_canonical*` as mutation paths;
- full-project snapshot serialization for normal mutations;
- `FileTransaction` as the project commit authority;
- `.daena/transactions/committed` request receipts;
- old `source_files` semantics as proof that a disposable index is current;
- normal-open full portable scans;
- snapshot-wide external reimport for ordinary changes;
- snapshot-wide search/map rebuilds after ordinary mutations;
- tests whose acceptance rule is “files commit before SQLite”; and
- documentation that calls files the live transaction authority or SQLite
  disposable.

Safe low-level filesystem helpers may be extracted and renamed before their old
container is deleted. Do not keep the old transaction abstraction merely to
avoid moving those helpers.

## 3. Starting point and completed baseline

The current implementation is repository-first:

- `ProjectStore::open_directory` scans the portable tree and conditionally
  reuses a matching disposable index.
- `ProjectStore::repository_first_mutation` builds an in-memory projection from
  files before applying a directory-backed mutation.
- `ProjectStore::commit_canonical_snapshot` stages a complete portable snapshot
  and commits files before refreshing SQLite.
- `ProjectStore::reconcile_external_changes` reimports a complete valid
  snapshot.
- `FileTransaction` and `.daena/transactions/` provide current crash recovery.

The pre-cut measurement work is recorded in
[`STORAGE_MIGRATION_BASELINE.md`](./STORAGE_MIGRATION_BASELINE.md). It is a
historical performance baseline only. Any statements there about future legacy
index handling are superseded by this hard-cut plan.

Do not extend Phase 0 with compatibility work. Preserve its benchmark command
and fixture procedure for before/after measurements.

## 4. Agent execution protocol

Before editing, the assigned agent must:

1. read `AGENTS.md`, `docs/STORAGE.md`, this plan, and the baseline;
2. run `rtk git status --short` and inspect overlapping staged, unstaged, and
   untracked work;
3. use the codebase graph for symbol and caller discovery;
4. trace every caller of the old storage functions named above;
5. inspect current storage, transaction, lifecycle, Git, plugin, asset, and
   persistence tests; and
6. present the scoped implementation plan and wait for explicit approval when
   the user has requested planning or review only.

During implementation:

- preserve unrelated user-authored changes;
- do not stage, commit, push, install toolchains, or update dependencies unless
  explicitly requested;
- use `rtk`, explicit Cargo manifests, `--locked`, and `--offline` when cached;
- make direct replacements rather than adapters around old behavior;
- delete obsolete callers and tests in the same change that replaces them;
- keep the checkout buildable and tested at every phase exit;
- add focused crash, persistence, and reopen tests with each boundary; and
- report source failures separately from unavailable platform verification.

At handoff, report:

- contracts and files changed;
- old code deleted;
- tests and direct lifecycle checks run;
- hard-cut assumptions required to open projects;
- acceptance items proven; and
- later-phase work intentionally not started.

## 5. Target component ownership

```text
ProjectStore / project session
  |-- runtime repository: SQLite rows, revisions, receipts
  |-- sync catalog: baselines, batches, items, conflicts
  |-- portable codec: deterministic version-2 representation
  |-- exporter: SQLite -> staged bytes -> checked portable replacement
  |-- reconciler: disk + baseline + SQLite -> import/export/conflict
  |-- rebuild service: portable files -> new runtime database
  `-- derived indexes: FTS, maps, relationships, AI-adjacent views
```

Move synchronization logic out of the already large `project.rs` where that
improves ownership. Suggested modules are `runtime_storage.rs`, `sync.rs`, and
`rebuild.rs`, but avoid an unrelated mechanical reorganization.

The runtime database remains `.daena/index.sqlite`. The name is not a promise
that old databases at that path are supported.

## 6. New runtime schema

The hard-cut schema is created from scratch. Do not write SQL that alters or
copies the old repository-first schema.

### 6.1 Storage metadata

Store at least:

- exact storage role and runtime schema version;
- project ID and portable format version;
- database epoch;
- exporter and reconciler contract version;
- clean-shutdown observation;
- synchronization summary; and
- last observed Git identity when Git is present.

Open succeeds only when the storage role and schema version exactly match the
hard-cut runtime. Missing or different metadata returns the typed reset-required
error. Do not branch on individual legacy tables or versions.

### 6.2 Durable runtime records

SQLite stores authoritative runtime rows for:

- entities;
- documents;
- fields;
- relationships;
- asset metadata;
- portable plugin data and migration history; and
- any durable shared project metadata required to reconstruct portable files.

Derived FTS, map, relationship, and cache tables remain explicitly rebuildable.

### 6.3 Record revisions

Every mutable semantic record has an opaque revision updated in the same
transaction as the row. Revisions include or are scoped by the database epoch,
so a clean rebuild invalidates all prior revision tokens.

### 6.4 Synchronization catalog

A `sync_records`-equivalent table records:

- stable semantic record key and kind;
- logical owner/ID;
- normalized portable path;
- baseline existence and content hash;
- current database hash and revision;
- clean, dirty, exporting, failed, or conflicted state;
- last completed batch; and
- typed diagnostic state.

Deletion is explicit. It is not inferred from a missing row.

### 6.5 Durable export work

`sync_batches`/`sync_items`-equivalent tables record:

- batch and request IDs;
- record key;
- create, replace, or remove operation;
- expected baseline existence/hash;
- target database revision/hash;
- staged native-payload path where needed;
- pending, staged, applied, completed, superseded, or conflicted state;
- attempt count and typed last error; and
- logical dependencies needed for a consistent batch.

Constraints prevent orphan items and incompatible active work for one record.
A newer coalesced edit supersedes an older target without advancing the
baseline.

### 6.6 Mutation receipts

SQLite stores request ID, request fingerprint where needed, serialized result,
and commit metadata in the same transaction as the mutation. An identical
retry returns the result. Reuse with incompatible inputs fails closed.

No filesystem receipt is read after the hard cut.

## 7. Phase 1 — Direct core replacement

### Objective

Replace the repository-first core in one hard-cut phase. Phase 1 is complete
only when every directory-backed mutation uses SQLite first and the old writer
has been deleted.

Agents may implement the work internally in the order below, but no intermediate
compatibility mode is an exit state.

### 7.1 Replace project open and initialization

Start from:

- `ProjectStore::open_directory`;
- `ProjectStore::open_database`;
- `ProjectStore::initialize`;
- `ProjectStore::rebuild_directory_index`;
- `project_database_path`; and
- `CoreService` project lifecycle.

Implement:

1. acquire a writable project-session lock for the open session;
2. read and validate `project.json`;
3. when `.daena/index.sqlite` is absent, scan the portable version-2 project,
   build `.daena/index.sqlite.next`, verify it, and atomically install it;
4. when the database exists, require the exact new storage role, schema, and
   project ID before opening;
5. return a reset-required error for every other database; and
6. remove conditional reuse, verification, or rebuild logic written for the old
   disposable index.

The reset-required error is a rejection boundary, not a compatibility reader.
It may state the `.daena/` deletion instruction but must not inspect or migrate
old project rows.

### 7.2 Add targeted portable import and rendering

Start from:

- `FilesystemRepository::scan`;
- `read_canonical_project` and `write_canonical_project`;
- `collect_canonical_sources`; and
- portable codec/path validation tests.

Implement:

1. semantic portable record keys for project metadata, entities, documents,
   field namespaces, source-owned relationship sets, asset ledgers, asset
   payloads, and plugin records;
2. full validated import for a missing database;
3. deterministic renderers for individual record types and logical dependency
   batches;
4. database hashes derived from those deterministic representations; and
5. baseline creation during fresh import.

Do not change portable format bytes unless a separately proven codec defect
requires an explicit format decision.

### 7.3 Add the synchronous exporter

Implement the correctness reference before adding background work:

1. read a consistent SQLite snapshot for the target revisions;
2. render only affected records;
3. stage and fsync them below `.daena/sync/<batch-id>/`;
4. validate bytes and target hashes;
5. verify all current target paths against their baselines;
6. atomically replace/remove targets with a final race check;
7. fsync affected parent directories;
8. record applied items idempotently;
9. advance baselines and complete the batch in SQLite; and
10. clean staging only after completion is durable.

Reuse safe low-level primitives from `transactions.rs`, then delete or rename
the old `FileTransaction` authority.

### 7.4 Port every mutation directly

Inventory every inbound caller of `repository_first_mutation`,
`sync_canonical*`, and `commit_canonical_snapshot`. Port all groups in this
phase:

- project metadata;
- entity create/update/archive/delete;
- document and combined entry saves;
- field upsert/delete and namespace operations;
- relationship create/update/delete;
- asset register/import/replace/delete and map creation;
- plugin enabled state and preserved data;
- plugin schema/data migrations and backup metadata; and
- seed/import/reset workflows.

Each mutation must perform, in one SQLite transaction:

1. caller revision validation;
2. targeted disk-baseline preflight for affected portable records;
3. durable row changes and cascades;
4. targeted derived-index changes;
5. new opaque revisions;
6. idempotent request receipt; and
7. durable export batch/items.

After commit, run the synchronous exporter. An export failure leaves the
runtime mutation committed, visibly dirty, and retryable.

### 7.5 Stream native assets

For asset import or replacement:

1. stream bytes to `.daena/sync/<batch-id>/` while computing size and SHA-256;
2. fsync staged bytes before committing metadata that references them;
3. commit asset metadata, revision, receipt, and export work atomically;
4. remove orphan staging when the SQLite transaction fails;
5. retain staging when portable placement fails after commit;
6. resume applied-but-unfinalized moves idempotently; and
7. never buffer a large map, video, or attachment entirely in memory.

Storage startup does not parse opaque map/provider payloads.

### 7.6 Delete the old implementation

Before Phase 1 exits:

1. delete `repository_first_mutation`;
2. delete `commit_canonical_snapshot`;
3. delete `sync_canonical*` mutation paths;
4. delete old filesystem request receipts and recovery dispatch;
5. delete old source-hash/index-freshness helpers that have no rebuild or
   reconciliation role;
6. remove repository-first branches from every mutation;
7. remove old tests or rewrite them against the new commit order;
8. remove old architecture comments and error messages; and
9. graph- and text-search for remaining callers.

Do not leave dead old code for a later cleanup phase.

### Phase 1 verification

#### Hard-cut behavior

- a portable version-2 project with no `.daena/` imports and opens;
- a database created by the new runtime reopens;
- a repository-first database is rejected with reset-required;
- no old database row is inspected, copied, or migrated;
- there is no runtime flag or fallback selecting the old writer; and
- all tests create clean new runtime state.

#### Mutation behavior

For every mutation group:

- transaction rollback is complete;
- request retry is idempotent;
- stale revision and changed baseline fail closed;
- runtime changes and export intent are atomic;
- portable paths contain only the exact changed set;
- search/map/relationship updates are targeted;
- immediate rendered state is correct;
- close/reopen preserves the result; and
- clean delete-`.daena` rebuild is equivalent.

#### Failure injection

Inject failure:

- before and after SQLite commit;
- before and after stage creation and fsync;
- before baseline preflight;
- before and after each portable replacement;
- before and after parent-directory sync;
- before and after applied-item recording;
- before and after baseline/batch completion; and
- during staging cleanup.

Every reopen resumes idempotently or enters a typed conflict/repair state. It
never invokes old recovery code.

### Phase 1 exit gate

SQLite is the only directory-project mutation authority, all portable writes
come from the synchronous exporter, and no repository-first writer, fallback,
receipt, or compatibility path remains.

## 8. Phase 2 — Background export and fast startup

### Objective

Move export latency off high-frequency mutations and make existing valid-runtime
open independent of project size.

### Implement

1. project-scoped export worker with start, drain, flush, and stop lifecycle;
2. document debounce/coalescing that always preserves the newest committed
   revision;
3. bounded retry with typed transient and permanent errors;
4. visible synchronization summary and per-record diagnostics;
5. minimal blocking open for an exact-version runtime database;
6. watcher startup that closes the open/reconcile event gap;
7. background offline-change reconciliation state; and
8. targeted baseline write gates until each record is reconciled.

No synchronous-export fallback may bypass the worker's durable queue. An
explicit flush uses the same queue and waits for a defined revision set.

### Design rules

- database commit latency is independent of exporter latency;
- flush waits for the requested revisions, not unrelated future edits;
- close stops new mutations before draining required revisions;
- termination before flush leaves the queue durable;
- permission, lock, and conflict errors do not spin indefinitely; and
- directory mtimes or watcher delivery are hints, never proof of no change.

### Verification

- rapid document edits coalesce to the newest revision;
- close/reopen preserves rapid edits;
- process termination at every worker state resumes correctly;
- UI never reports clean while failed or conflicted items remain;
- write preflight catches an offline edit before background reconciliation;
- two writable instances cannot coexist; and
- normal open does not call the full portable scanner.

Using the Phase 0 fixtures, measure existing clean open, dirty resume, document
and field commit, rapid coalescing, unchanged large-map open, and flush latency.

### Exit gate

Runtime commits and valid-runtime open use the fast path while portable
persistence remains durable, visible, and recoverable.

## 9. Phase 3 — Incremental external reconciliation

### Objective

Replace snapshot-wide external reimport with semantic three-way reconciliation.

### Start from

- `ProjectStore::reconcile_external_changes`;
- the Tauri project watcher;
- `ExternalChangeReport` and frontend handling;
- remaining source hash helpers;
- Git-unmerged detection; and
- unsaved draft/conflict recovery paths.

### Implement

1. watcher event coalescing by semantic record key;
2. create/replace/delete classification using baseline existence and hashes;
3. external-only import with the same parser/validators as fresh import;
4. dependency-scope validation and targeted derived updates;
5. two-sided conflict rows with DB, baseline, and disk identities;
6. compare/use-database/use-disk/manual-resolution actions;
7. draft preservation and recovery copies below the new `.daena/conflicts/`;
8. project-level divergence mode for Git or widespread changes; and
9. deletion of snapshot-wide ordinary reconciliation.

### Rules

- watcher events are hints; classification reads actual state;
- self-event suppression uses active batch target hashes;
- invalid disk data never replaces valid runtime rows;
- external-only import updates rows, revisions, baseline, and projections in one
  transaction without exporting identical bytes back;
- use-database preserves the external version before replacement;
- use-disk preserves or explicitly discards dirty runtime content; and
- unaffected reconciled records may remain writable unless a global invariant
  requires project read-only mode.

### Verification

- clean external Markdown/JSON create, edit, and delete;
- rapid writes and atomic editor replacements;
- invalid intermediate data followed by valid save;
- external edits racing every exporter boundary;
- unsaved draft plus external edit;
- Git checkout, merge conflict, resolution, rebase, reset, and branch switch;
- widespread changes yield one project-level decision; and
- close/reopen preserves unresolved conflict state.

### Exit gate

External-only changes import, database-only changes export, two-sided changes
conflict, and ordinary external edits never trigger a full-project snapshot
reimport.

## 10. Phase 4 — Git, backup, plugin, and destructive workflows

### Objective

Put every operation that depends on portable completeness behind one flush and
validation barrier.

### Audit

- `ProjectStore::git_preflight`, `git_commit`, reset, history, and remote
  helpers;
- portable export/archive flows;
- plugin backup/restore and migration chains;
- project close and application shutdown;
- clean database rebuild/repair UI; and
- AI acceptance/index paths that assume current portable files.

### Implement

1. one typed flush-barrier API accepting revision set and operation reason;
2. Git preflight after flush, with exact portable staging paths;
3. portable backup/export after flush and validation;
4. runtime recovery backup using consistent SQLite backup plus staged payloads;
5. clean rebuild and dirty/conflicted rebuild refusal;
6. explicit archive-and-discard for dirty state created by the new runtime;
7. plugin migration/restore integration with runtime transactions and export
   batches; and
8. post-Git reconciliation before writes resume.

The archive-and-discard workflow applies only to databases created by the new
runtime. Pre-cut repository-first state is unsupported and receives only the
reset-required instruction.

### Verification

- Git cannot stage dirty, failed, conflicted, invalid, or unmerged state;
- `.daena/` and SQLite sidecars never enter built-in staging;
- exact selected paths are staged after flush;
- portable backup restores from files alone;
- runtime recovery backup restores pending new-runtime export state;
- new-runtime dirty rebuild requires explicit archive/discard;
- plugin migrations are runtime-atomic and portable after flush;
- failed migration/restore retains the prior active version; and
- immediate UI state agrees with close/reopen behavior.

### Exit gate

Every operation claiming current portable completeness uses the common flush
barrier and has no repository-first special case.

## 11. Phase 5 — Final hardening

### Objective

Prove the hard cut is complete, close remaining whole-project hot paths, and
finish cross-platform verification.

### Required work

1. graph- and text-search for every removed old symbol and behavior;
2. delete any remaining old schema, receipt, journal, compatibility, fallback,
   or source-authority tests;
3. retain full portable scan/import only for missing database, explicit clean
   rebuild, repair, portable format work, and exceptional divergence;
4. assert small edits cannot enumerate or render the full project;
5. update all active architecture, Git, plugin, AI, and operator documentation;
6. validate error text for reset-required versus new-runtime dirty recovery;
7. run the full crash and lifecycle matrix; and
8. re-index the repository knowledge graph.

### Cross-platform verification

Cover macOS, Linux, and Windows behavior for:

- atomic replacement;
- file and directory fsync;
- path casing and symlinks;
- filesystem watchers;
- SQLite WAL/SHM and locked files;
- process/session locks; and
- large streamed asset moves.

### Exit gate

No code or active documentation recognizes, migrates, reads, or falls back to
the repository-first database/runtime. The only supported states are portable
version-2 files with no runtime database, or an exact-version database created
by the new implementation.

## 12. Required test matrix

### Runtime transactions

- create, update, delete, cascade, and rollback;
- expected revision success/failure;
- identical and incompatible request retry;
- transaction plus export-intent atomicity;
- database epoch invalidation after clean rebuild;
- foreign-key and integrity checks;
- second writable process rejection; and
- exact-version open versus reset-required rejection.

### Export and crash recovery

- every failure point listed in Phase 1;
- create, replace, and remove;
- multi-record logical batches;
- coalesced superseded work;
- permission, disk-full, and locked-target failure;
- applied-but-unfinalized recovery; and
- cleanup after completed work.

### External changes

- clean, dirty, and conflicted records;
- create, replace, and delete;
- invalid UTF-8/JSON/Markdown/path/reference/namespace/asset state;
- duplicated, missing, and rapid watcher events;
- edits during startup reconciliation and export; and
- Git merge, checkout, rebase, reset, and branch divergence.

### Assets

- allowed zero-byte types;
- large streaming imports without full memory buffering;
- mismatched size/hash;
- missing staged or portable payload;
- crash before and after database reference creation;
- opaque map/provider bytes on startup; and
- delete/replace confined to owned paths.

### Portability

- deterministic JSON and Markdown golden files;
- clean delete-`.daena` rebuild equivalence;
- fresh clone equivalence;
- plugin state and migration-history reconstruction;
- grants and sessions remain local;
- derived projection rebuild equivalence; and
- portable backup restore.

### Hard-cut rejection

- repository-first database returns reset-required;
- old WAL, filesystem receipt, journal, or source hash is never read;
- no old schema migration SQL exists;
- no feature flag selects storage authority;
- no mutation fallback exists; and
- reset followed by portable import creates a valid new runtime.

### User-visible persistence

- immediate rendered state;
- visible saving, dirty, failed, and conflict state;
- close/reopen after normal, rapid, failed-export, and conflict cases;
- explicit flush feedback;
- reset-required guidance for pre-cut local state;
- new-runtime dirty rebuild warning lists affected records; and
- conflict actions preserve the unchosen side.

## 13. Repository checks

Use the exact commands supported by the checkout. The expected cached baseline
is:

```sh
rtk cargo test --manifest-path src-tauri/Cargo.toml --locked --offline
rtk cargo clippy --manifest-path src-tauri/Cargo.toml --locked --offline --all-targets -- -D warnings
rtk deno task check
rtk deno task build
rtk deno task check:plugin-contract
rtk deno task check:plugin-isolation
rtk deno task test:plugin-conformance
```

Run focused `daena-core` tests first. Do not install a host target/toolchain when
the established containerized verification route is available.

## 14. Performance evidence

Preserve the baseline fixture and command from
`STORAGE_MIGRATION_BASELINE.md`. Record:

- platform, filesystem, build profile, SQLite/WAL configuration;
- fixture entity/file/asset counts and total asset bytes;
- open to first usable read;
- background reconciliation time;
- SQLite commit latency;
- export stage/replacement latency;
- flush latency;
- changed record/path counts; and
- any full scan, snapshot serialization, asset hash, FTS rebuild, or map
  rebuild.

Expected ordinary complexity:

| Operation | Expected work |
| --- | --- |
| Existing exact-version open | Session/database metadata, not project size |
| Document edit | One document row, targeted FTS, one export record |
| Field edit | One namespace record and affected projections |
| Relationship edit | One source-owned record and affected indexes |
| Asset import | Streamed payload plus one ledger/payload batch |
| External single-file edit | One record plus dependency closure |
| Missing database import | Full portable project size |

Do not claim a percentage improvement without the same fixture, machine,
profile, and procedure.

## 15. Stop conditions

Stop the phase and report evidence when:

- a change would weaken a `STORAGE.md` invariant;
- overlapping user-authored changes cannot be preserved safely;
- the portable format must unexpectedly change;
- a crash can create runtime rows without export intent;
- a write can overwrite an unchecked baseline;
- code attempts to inspect, migrate, copy, or fall back to pre-cut local state;
- a new-runtime dirty rebuild can discard data without explicit archive/discard;
- repeated fixes fail before capturing the live error and artifact identity; or
- completion requires materially broader authority than the phase grants.

Tooling limitations are not proof of a source defect. Passing unit tests alone
is not proof of persistence, lifecycle, Git, conflict, or rendered behavior.

## 16. Completion definition

The hard cut is complete when:

- SQLite is the sole normal runtime mutation authority;
- the old local database is rejected, never migrated;
- portable version-2 files initialize a fresh new runtime;
- every portable mutation commits durable export intent atomically;
- no repository-first writer, receipt, recovery, fallback, or feature flag
  remains;
- existing exact-version projects open without a blocking full scan;
- ordinary edits scale with changed records;
- dirty, failed, and conflicted state is visible and recoverable;
- Git, backup, close, and rebuild use verified flush barriers;
- all plugin, asset, revision, retry, and authority boundaries remain intact;
- clean clones and checkpoints rebuild completely; and
- active documentation describes only the hard-cut architecture.
