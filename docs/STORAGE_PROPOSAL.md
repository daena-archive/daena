# Daena Storage Architecture Proposal

## Status

**Proposal**

This document proposes a new storage model for Daena that replaces the current disk-first architecture with a database-first runtime model while preserving plain-text files as the durable, portable, Git-friendly representation of a project.

The goal is to simplify the storage subsystem, drastically improve startup and mutation performance, and retain the ability to inspect, version, diff, recover, and reconstruct Daena projects without depending on a proprietary database blob.

---

## 1. Summary

Daena should use two representations of project state, each with a distinct responsibility:

- **SQLite is the canonical runtime store.**
- **Plain-text project files are the portable persisted representation.**

During normal operation, all reads and writes go through SQLite. Changes committed to SQLite are then synchronized to disk, either immediately or through a short debounced or batched persistence cycle.

When no database exists, Daena reconstructs the database from the project files.

When both the database and project files exist, Daena treats SQLite as canonical for normal operation while detecting external changes to the files. If the database and files have diverged from their last known synchronized state, Daena presents the conflict to the user rather than silently choosing one side.

If the database is stale, corrupted, incompatible, or substantially divergent from the files, the user can rebuild it. Rebuilding means discarding the local SQLite database and reconstructing a new one from the portable project files.

This gives Daena a simple model:

> SQLite is the working state.  
> Plain text is the portable project format.  
> Synchronization keeps them aligned.  
> Git operates on the portable project format.  
> SQLite can always be reconstructed from it.

---

## 2. Motivation

The current storage architecture treats the filesystem representation as the primary source of truth and SQLite as a derived projection.

That model has two major problems.

### 2.1 Excessive complexity

A disk-first model requires Daena to make filesystem operations behave like database transactions.

Logical changes may span several files and must remain recoverable across partial writes, crashes, interrupted staging operations, stale projections, and external edits. This introduces substantial machinery around:

- transaction journals,
- staged writes,
- rollback and roll-forward behavior,
- source scanning,
- hashing,
- projection invalidation,
- startup verification,
- synchronization ordering,
- filesystem consistency.

Much of this complexity exists because the filesystem is being asked to serve as the authoritative transactional store.

SQLite already provides these semantics directly and more reliably.

### 2.2 Poor performance

The disk-first architecture also creates unnecessary work on critical paths.

Startup may require scanning, hashing, parsing, or validating large portions of the project before the application can safely use its database projections.

Similarly, mutations may trigger filesystem serialization and synchronization work that should not be part of the core application transaction.

This becomes particularly problematic for large or opaque files. For example, adding a `.map` file can cause project startup to take several seconds.

That is unacceptable for a local-first desktop application where unchanged projects should normally open almost immediately.

---

## 3. Design Goals

The new architecture should satisfy the following goals.

### 3.1 Fast startup

If a valid database already exists, Daena should be able to open it directly without reparsing the entire project representation.

Startup cost should primarily depend on opening and validating SQLite, not on project size.

### 3.2 Fast mutations

Application mutations should commit through SQLite transactions without requiring synchronous serialization of the entire affected filesystem representation.

### 3.3 Git-friendly projects

Project state intended for version control must remain represented as meaningful plain-text and ordinary asset files.

Git users should see changes such as:

```text
entities/characters/alice/entity.json
entities/characters/alice/document.md
maps/world.map
```

rather than a constantly changing database blob.

The local SQLite database should normally remain ignored by Git.

### 3.4 Recoverability

Deleting the SQLite database must not destroy the project.

Daena must be able to recreate all durable project data from the portable files.

### 3.5 Explicit conflict handling

If the database and files diverge independently, Daena must not silently overwrite either side.

The user should remain in control of conflict resolution.

### 3.6 Simple mental model

The behavior of the storage system should be explainable without requiring knowledge of filesystem journaling or projection internals.

---

## 4. Core Model

The architecture uses two representations.

### 4.1 Runtime representation

SQLite stores the state Daena actively operates on.

This includes canonical runtime project data and may also include derived structures such as:

- search indexes,
- map indexes,
- relationship projections,
- RAG-related indexes or metadata,
- plugin projections,
- caches,
- synchronization metadata.

During normal application operation, SQLite is authoritative.

### 4.2 Portable representation

The filesystem contains the representation intended for:

- Git,
- external editing,
- inspection,
- backup,
- interchange,
- project reconstruction.

This may include formats such as:

- JSON,
- Markdown,
- images,
- maps,
- attachments,
- other ordinary project assets.

The portable representation must contain enough durable state to reconstruct the canonical project database.

Derived data does not need to be stored there unless it is itself meaningful project content.

---

## 5. Source-of-Truth Rules

The term "source of truth" depends on the project lifecycle.

### 5.1 No database exists

If the project contains no usable Daena database, the filesystem representation is authoritative.

Daena performs a full import:

```text
project files
    ↓
parse and validate
    ↓
create SQLite database
    ↓
populate canonical tables
    ↓
build derived indexes and projections
    ↓
open project
```

This is the expected path for:

- a fresh Git clone,
- a copied project without local state,
- an explicitly requested rebuild,
- recovery after database loss.

### 5.2 Database exists

If a valid database exists, SQLite is authoritative during normal runtime.

Daena should open the database directly and should not require a full project re-import before making the project available.

### 5.3 Rebuild mode

If the user explicitly rebuilds the database, authority temporarily returns to the filesystem.

The existing database is discarded and recreated from the portable files.

After the rebuild completes, SQLite again becomes the canonical runtime store.

---

## 6. Normal Mutation Flow

Application code should interact with SQLite, not directly with the portable project files.

A normal mutation should look conceptually like this:

```text
user action
    ↓
application service
    ↓
SQLite transaction
    ↓
COMMIT
    ↓
mark affected portable records dirty
    ↓
persistence queue
    ↓
serialize affected records
    ↓
atomic disk writes
    ↓
update synchronization metadata
```

The SQLite transaction is the application-level durability boundary.

The filesystem synchronization step occurs after the database commit.

This removes filesystem serialization from the core transaction path.

---

## 7. Disk Persistence

Changes committed to SQLite must eventually be persisted to the portable project representation.

Two persistence strategies may coexist.

### 7.1 Immediate persistence

Some mutations may be written to disk immediately after commit.

This is appropriate when:

- the operation is infrequent,
- the output is small,
- immediate external visibility is desirable.

### 7.2 Debounced or batched persistence

High-frequency mutations should normally be coalesced.

For example, document editing should not rewrite a Markdown file after every keystroke.

A conceptual flow is:

```text
DB mutation
    ↓
mark record dirty
    ↓
short debounce
    ↓
serialize latest state once
```

Batching should be an optimization of persistence, not a change to database semantics.

### 7.3 Forced flushes

Daena should provide internal flush points for operations where the portable representation should be fully synchronized, such as:

- project close,
- application shutdown,
- explicit "save" or "sync" operation if exposed,
- Git-related workflows,
- export or backup operations,
- before operations that depend on the filesystem representation being current.

---

## 8. Synchronization Metadata

Daena needs to distinguish between:

- changes made by Daena,
- changes made externally,
- genuine two-sided conflicts.

Directly comparing the current database state with the current disk state is insufficient.

Daena should track the **last synchronized state** of each portable record.

Conceptually:

```text
current database state
        ↘
      last-sync state
        ↗
current disk state
```

The last-sync state may be represented using hashes, revisions, or equivalent fingerprints.

Timestamps should not be the primary mechanism because they are unreliable across:

- Git operations,
- file copying,
- clock changes,
- filesystems with different timestamp precision,
- external tooling.

---

## 9. Change Classification

For each synchronized record, Daena can compare:

- the current database representation,
- the last synchronized representation,
- the current disk representation.

This produces several cases.

### 9.1 No external change

```text
disk == last-sync
```

The disk representation has not changed externally.

If the database is newer, Daena may safely persist DB → disk.

### 9.2 External-only change

```text
database == last-sync
disk != last-sync
```

The portable file changed outside Daena while the database copy did not.

This can be treated as an external update.

Depending on the type of data and product UX, Daena may:

- import it automatically,
- notify the user,
- or ask for confirmation.

The important point is that this is not a two-sided conflict.

### 9.3 Database-only change

```text
database != last-sync
disk == last-sync
```

Daena has unsynchronized runtime changes.

The database version may safely be written to disk.

### 9.4 Genuine conflict

```text
database != last-sync
disk != last-sync
```

Both sides changed independently after the last synchronization point.

Daena must not silently choose one.

The user should be asked how to resolve the conflict.

---

## 10. Conflict Resolution

Conflicts should be presented at the highest meaningful semantic level available.

Possible resolutions include:

- keep the database version,
- use the disk version,
- review differences,
- resolve manually,
- rebuild the database from disk when divergence is widespread.

Not every file difference should necessarily become an individual prompt. Daena should distinguish isolated record conflicts from project-wide divergence.

---

## 11. Substantial Divergence

Some external operations can change a large fraction of the project at once.

Examples include:

- switching Git branches,
- rebasing,
- resetting to another commit,
- restoring a backup,
- replacing project files,
- checking out an older revision.

In these situations, presenting hundreds of individual conflicts would be poor UX.

Daena should detect substantial divergence and treat it as a project-level event.

A suitable prompt might explain that the project files differ significantly from the local database and offer options such as:

- **Rebuild database from files**
- **Keep local database**
- **Review changes**

The exact threshold or heuristic is an implementation detail, but the architecture should explicitly support this mode.

---

## 12. Database Rebuild

A rebuild is an explicit recovery and reconciliation operation.

Conceptually:

```text
flush or discard pending DB → disk changes as appropriate
    ↓
close SQLite
    ↓
remove or archive local database
    ↓
create new database
    ↓
import complete portable project representation
    ↓
rebuild derived indexes and projections
    ↓
establish new synchronization baseline
    ↓
open project
```

The rebuild operation should be useful when:

- the database is missing,
- the database is corrupted,
- the schema is incompatible,
- the user explicitly wants disk to win,
- a Git operation substantially replaced project state,
- there are too many conflicts to resolve individually,
- synchronization metadata is no longer trustworthy.

Rebuild should be conceptually cheap from a product perspective even if it is computationally expensive: it is a recovery path, not part of normal startup.

---

## 13. Startup Behavior

Startup should distinguish between normal operation and reconstruction.

### 13.1 Normal startup

When a valid database exists:

```text
open project
    ↓
open SQLite
    ↓
validate DB/schema metadata
    ↓
make project available
    ↓
perform lightweight reconciliation as needed
```

Daena should not perform a full parse of every entity, document, map, and asset before the project becomes usable.

External-change detection may occur incrementally or after the project has opened.

### 13.2 First open or rebuild

When no usable database exists:

```text
scan project representation
    ↓
parse durable project files
    ↓
construct SQLite database
    ↓
build projections/indexes
    ↓
establish synchronization metadata
    ↓
open project
```

This is expected to take longer and should be treated as initialization, not normal startup.

---

## 14. Filesystem Watching

Filesystem watchers remain useful, but their role changes.

They no longer define canonical project state.

Instead, watchers are used to identify possible external modifications and trigger reconciliation.

Watcher events should be treated as hints rather than absolute truth because filesystem notification APIs may:

- merge events,
- duplicate events,
- omit events,
- report temporary files,
- behave differently across platforms.

The actual reconciliation decision should be based on stored synchronization metadata and content fingerprints.

---

## 15. Crash Semantics

The new architecture intentionally separates database durability from filesystem synchronization.

### 15.1 Crash before database commit

The mutation did not happen.

SQLite transaction semantics handle rollback.

### 15.2 Crash after database commit but before disk persistence

The mutation exists in SQLite but the portable representation is stale.

This is not database corruption.

On the next startup, Daena can detect the unsynchronized database state and persist it to disk.

### 15.3 Crash during disk write

Portable files should be updated using atomic replacement where practical.

A failed filesystem write must not roll back an already committed database transaction.

Instead, the corresponding record remains dirty and can be retried.

This is significantly simpler than treating multi-file filesystem updates as the authoritative transaction.

---

## 16. Transaction Boundaries

SQLite provides the primary transactional boundary.

A logical Daena operation may update:

- entities,
- relationships,
- metadata,
- documents,
- plugin-owned canonical data,
- synchronization state.

These changes should commit atomically through SQLite where required.

Filesystem persistence happens afterward.

This eliminates the need for the filesystem itself to emulate ACID semantics for normal application mutations.

Custom filesystem transaction journals should therefore be avoided unless a specific portable-format operation genuinely requires them.

---

## 17. Dirty Tracking

Daena should track which portable records require persistence.

Dirty tracking may occur at a semantic level such as:

- entity,
- document,
- relationship set,
- project metadata,
- map,
- plugin-owned record.

The persistence layer should only serialize affected records.

A change to one entity must not require re-exporting or rescanning the entire project.

Dirty state should survive long enough to recover from failed writes or application interruption.

The exact representation of dirty state is an implementation concern, but it should be part of the storage model.

---

## 18. Git Integration

Git is a primary reason to retain the plain-text representation.

The new architecture should preserve several important properties.

### 18.1 Meaningful diffs

Project changes remain visible as ordinary text and asset changes.

### 18.2 No database churn

The local SQLite database should normally be ignored.

Git operations should not create large binary diffs for routine project edits.

### 18.3 Fresh clones remain complete

A project clone that does not contain `.daena/index.sqlite` must still contain all durable project information required to reconstruct it.

### 18.4 Git operations are treated as external changes

Operations such as checkout, merge, rebase, and reset modify the portable representation.

Daena should reconcile those changes against the runtime database rather than assuming either side automatically wins.

Large Git-driven changes should generally favor a project-level rebuild workflow instead of hundreds of record-level prompts.

---

## 19. Derived Data

Not all SQLite content needs a portable representation.

Data that can be reconstructed should remain local where possible.

Examples may include:

- FTS indexes,
- search projections,
- cached graph structures,
- map indexes,
- embedding indexes,
- RAG chunk metadata,
- thumbnail caches,
- plugin caches,
- performance-oriented denormalizations.

This allows Daena to use SQLite and related local indexes aggressively without polluting Git history.

The general rule should be:

> If losing the database must not lose user-authored project information, that information belongs in the portable representation.

Everything else may be treated as derived local state.

---

## 20. Plugin Model

Plugins should normally interact with storage through Daena's database/storage APIs rather than writing project files directly.

This keeps:

- transactional behavior consistent,
- synchronization centralized,
- conflict handling predictable,
- portable serialization under Daena's control.

If plugins own durable project data, they should define or register how that data maps to the portable representation.

Plugin-owned derived data may remain SQLite-only.

---

## 21. Project Invariants

The new architecture should maintain the following invariants.

### Invariant 1

A successfully committed application mutation exists in SQLite before it is considered complete at the runtime layer.

### Invariant 2

All durable user-authored project state can eventually be represented in the portable project files.

### Invariant 3

A project can be reconstructed from the portable representation without access to the previous SQLite database.

### Invariant 4

Daena never silently overwrites independently modified DB and disk state when both have changed since the last synchronization point.

### Invariant 5

Normal startup does not require rebuilding or fully reparsing the portable representation.

### Invariant 6

Filesystem synchronization failure does not corrupt or roll back already committed SQLite state.

### Invariant 7

Derived local indexes may be deleted and rebuilt without loss of durable project information.

---

## 22. Non-Goals

This architecture does not attempt to make SQLite itself a collaboration format.

It also does not require:

- committing the database to Git,
- keeping DB and disk byte-for-byte synchronized after every keystroke,
- treating filesystem watcher events as transactional truth,
- performing a full consistency scan at every startup,
- making every derived database structure portable,
- implementing automatic semantic merge for every conflict type.

Those capabilities may be added independently if future requirements justify them.

---

## 23. Performance Expectations

The architecture should be designed around the following expectation:

> The common path must optimize for an existing, unchanged local project.

For that case:

- startup should primarily open SQLite,
- reads should come directly from indexed database structures,
- mutations should commit through SQLite,
- serialization should be incremental,
- unchanged large assets should not be reparsed or rehashed unnecessarily.

Full project parsing is acceptable during:

- initial import,
- explicit rebuild,
- migration,
- recovery,
- exceptional reconciliation.

It should not be part of routine project opening.

---

## 24. Architectural Simplification

The main benefit of this proposal is not merely that SQLite is faster than filesystem scans.

It changes where complexity belongs.

The disk-first model requires Daena to maintain strong transactional correctness across ordinary files.

The database-first model instead uses SQLite for transactional runtime correctness and treats the filesystem representation as synchronized durable output.

This reduces the number of responsibilities on the critical path:

### Previous model

```text
mutation
    ↓
serialize canonical files
    ↓
stage filesystem transaction
    ↓
update files
    ↓
scan / verify
    ↓
update database projection
    ↓
rebuild derived state
```

### Proposed model

```text
mutation
    ↓
SQLite transaction
    ↓
COMMIT
    ↓
queue affected records
    ↓
incrementally persist portable files
```

The resulting system should be easier to reason about, easier to test, and significantly faster.

---

## 25. Proposed Lifecycle

The complete project lifecycle can be summarized as follows.

### Fresh clone

```text
portable files
    ↓
no DB found
    ↓
import
    ↓
SQLite created
    ↓
normal runtime
```

### Normal editing

```text
UI
    ↓
SQLite
    ↓
dirty tracking
    ↓
batched portable-file persistence
```

### External file edit

```text
filesystem watcher
    ↓
compare disk / DB / last-sync
    ↓
import, ignore, or prompt
```

### Git branch switch

```text
large filesystem divergence
    ↓
detect project-level mismatch
    ↓
recommend rebuild
    ↓
recreate SQLite from branch files
```

### Lost database

```text
delete or missing SQLite
    ↓
import portable files
    ↓
rebuild all derived state
```

---

## 26. Conclusion

Daena should stop treating ordinary project files as its transactional runtime database.

SQLite is better suited to:

- transactions,
- indexed reads,
- relational integrity,
- high-frequency mutation,
- derived projections,
- search,
- local caches,
- scalable runtime access.

Plain-text and ordinary asset files are better suited to:

- Git,
- human inspection,
- interchange,
- backup,
- external editing,
- long-term portability.

The architecture should embrace those strengths instead of forcing either representation to serve both roles.

The proposed storage model is therefore:

> **Database-first at runtime, plain-text-first for portability and reconstruction.**

This preserves Daena's local-first and Git-friendly characteristics while removing a substantial amount of storage complexity and making high-performance project access the default rather than an optimization.
