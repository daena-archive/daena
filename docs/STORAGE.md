# Daena Archive storage architecture

This is the single storage authority for Daena Archive. It replaces the
earlier storage proposal, file-canonical plan, and migration-status notes.

## 1. Authority and scope

Daena is an offline-first desktop authoring studio. The Rust core owns durable
project identity, validation, persistence, revisions, migrations, derived
indexes, recovery, and authorization. Tauri, Svelte, bundled modules, and
plugins are adapters or clients; they do not define storage authority.

The live project authority is the SQLite runtime database in `.daena/` and
its machine-local runtime state. Portable project files are generated,
validated checkpoints for Git, backup, inspection, interchange, and recovery.
They are not a second live database and are never continuously reconciled with
runtime rows.

This is a hard alpha cut. An incompatible runtime database is rejected with a
reset-required diagnostic. There are no legacy readers, dual writers,
three-way merges, compatibility migrations, or fallback storage authorities.

## 2. Portable project format

The portable tree contains the user-authored project representation:

```text
project.json
entities/<entity-uuid>/
  entity.json
  document.md
  fields/<owner>--<namespace>.json
  relationships.json
  assets.json
plugins/<plugin-id>.json
assets/{images,videos,maps,files}/
checkpoint.json
```

The project manifest, entity records, Markdown documents, strict JSON files,
plugin data, and native asset bytes are deterministic and path-normalized.
Entity UUIDs are stable references. Names, types, timestamps, soft-delete
state, fields, relationships, and assets may change without changing an
entity's identity.

`checkpoint.json` is generated output and records the portable format, project
identity, content generation, and the sorted path, size, and SHA-256 digest of
every other portable file. It is written after the complete checkpoint has
been installed and is never included in its own inventory. The checkpoint
manifest is the only persistent file inventory; SQLite does not retain a
per-path source-hash catalog.

`.daena/`, `.git/`, editor files, temporary files, and runtime-only indexes are
not portable content and are excluded from the checkpoint manifest.

## 3. Runtime database

`.daena/index.sqlite` contains the current runtime state: entities, documents,
fields, relationships, assets, module/plugin state, migration history,
idempotency receipts, opaque revisions, and derived search/map projections.
The runtime database may be deleted only when the portable checkpoint is clean
and validated. Derived projections are disposable and rebuildable; deleting a
projection must not delete authored content.

Runtime metadata contains the hard-cut storage identity, database epoch,
content generation, exported generation, checkpoint digest, and the last
export error. Sync state is derived as follows:

- `failed` when an export error is present;
- `pending` when content generation exceeds exported generation; and
- `clean` when generations match and the checkpoint digest is valid.

The database epoch participates in opaque revisions. Replacing the runtime
database creates a new epoch and invalidates revisions issued by the previous
runtime.

## 4. Runtime mutation and generation rules

Normal reads and mutations use SQLite. A successful mutation is durable when
its SQLite transaction commits; it does not wait for portable hashing or file
export. The same transaction records the mutation receipt, updates durable
rows and revisions, and advances `content_generation` when portable content
has changed. Derived-only maintenance does not create a portable generation.

Request IDs make retries idempotent. Reusing a request ID with different input
is a conflict. Revision-protected updates, deletes, document saves, field and
relationship changes, and asset operations reject stale revisions.

## 5. Checkpoint export

Export is one-way and generation-based. One project-scoped worker coalesces
wakeups; a lost wake is recoverable because the generation comparison is
durable.

An export attempt:

1. opens a consistent SQLite read transaction and captures generation `G`;
2. renders a complete deterministic portable snapshot from that transaction;
3. stages runtime asset bytes and verifies their declared hashes;
4. validates the staged file inventory and builds the checkpoint manifest;
5. installs portable files with safe replacement and removes stale
   manifest-owned paths;
6. writes `checkpoint.json` last; and
7. records `exported_generation = G` and the manifest digest only after the
   complete checkpoint succeeds.

If newer mutations commit while export is running, the completed checkpoint
still truthfully describes `G`; a later coalesced export handles the newer
generation. If export fails, the runtime remains authoritative, the error is
persisted, and the next barrier or reopen retries the latest generation.

The only barrier contract is:

```text
flush_checkpoint(reason) -> Result<Generation>
```

Git commit, portable backup, explicit save/export, recovery operations, and
clean lifecycle transitions use this barrier where a complete portable
checkpoint is required.

## 6. Open, rebuild, and explicit import

When a valid runtime database exists, opening it does not scan or hash the
portable tree. If the runtime is missing or incompatible, Daena validates the
complete portable tree, builds a new runtime database, rebuilds projections,
initializes generation-zero metadata, and writes a clean checkpoint manifest.

External portable changes never mutate live runtime rows automatically. A
user-requested import validates the project ID, paths, hashes, references,
plugin ownership, schemas, and assets as one complete checkpoint. It then:

1. rejects dirty runtime state unless the user first chooses a checkpoint
   barrier or a destructive replacement;
2. builds and validates `.daena/index.sqlite.next`;
3. archives the current runtime for recovery;
4. assigns a new database epoch; and
5. atomically installs and reopens the candidate database.

No valid subset is imported, and no path-level winner is selected silently.

## 7. Concurrency and filesystem notifications

The lifecycle handle protects project-session replacement. Read commands use
independent read-only SQLite connections. Runtime mutation transactions are
serialized by the project writer boundary.

Portable rendering, hashing, copying, Git, AI indexing, and event emission
must not become hidden mutation or reconciliation work. A filesystem watcher,
if enabled, watches only portable roots, excludes `.daena`, `.git`, temporary
and editor paths, and reports possible portable changes to the UI. It does not
scan, reconcile, import, export, open a write transaction, or wait for a
long-running worker from its callback.

## 8. Git, backups, plugins, and derived data

Git operates on portable files only. Git commits, resets, branch changes,
portable backups, and recovery actions are explicit user operations and use
typed checkpoint or recovery errors. Runtime databases, runtime indexes,
plugin grants, installed packages, and sessions remain machine-local.

Plugins and bundled modules may own portable data through declared namespaces
and schemas, but Rust remains the authority for validation, lifecycle,
authorization, migrations, and persistence. Plugin code receives no database,
filesystem, shell, process, or ambient Tauri handle.

Search indexes, map projections, relationship indexes, and similar structures
are derived from runtime rows and may be rebuilt without changing the
portable project.
