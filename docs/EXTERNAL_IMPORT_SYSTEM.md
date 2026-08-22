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

1. Markdown, plain text, HTML, DOCX, and recursive folders;
2. ZIP, followed by Obsidian as a specialization of Markdown;
3. streaming MediaWiki-compatible XML and conservative wikitext preservation;
4. third-party importer plugins; and
5. ODT and RTF after their dedicated parser, fidelity, and security work is
   planned and accepted.

Markdown initially remains one staged object per file. Headings are preserved
in the document body and do not create entities automatically. Obsidian adds
frontmatter, aliases, wikilinks, embeds, attachments, and vault-resolution
rules while producing the same staged contract.

### Deferred ODT and RTF support

ODT and RTF are explicitly deferred beyond Iteration 6. They are not advertised
by the built-in importer and are not selectable in the current UI. Each format
needs a maintained parser path, malformed-input and resource-limit fixtures,
conversion-quality expectations, attachment/link handling, and commit plus
clean-rebuild coverage before it can be enabled. This deferral does not change
the neutral staged contract or block a future first-party or capability-gated
plugin adapter.

### Obsidian adapter policy

Obsidian is a separate, folder-only built-in importer profile so generic
Markdown behavior remains unchanged. It accepts Markdown notes and supported
attachments, excludes root `.obsidian` and `.trash` directories with a visible
diagnostic, and preserves note bodies and unsupported plugin syntax verbatim.

YAML frontmatter is parsed conservatively into generic staged fields, aliases,
tags, and an optional entity-type hint. Raw frontmatter is retained. Unsupported
nested YAML is retained as text and reported rather than discarded or executed.
Wikilinks and embeds are resolved vault-wide by normalized path, Markdown path,
filename, title, and alias. Heading and block fragments resolve to their owning
note. A unique match is resolved, multiple matches are staged as ambiguous with
candidate note IDs, and absent targets remain missing; the importer never
invents a target. Attachment bytes still pass signature, size, hash, source
re-read, atomic commit, and clean-rebuild validation.

### MediaWiki adapter policy

MediaWiki is a separate, file-only built-in importer for UTF-8 MediaWiki-
compatible XML exports. It uses an event stream rather than building an XML
tree, rejects DTDs and non-predefined entity references, bounds XML depth,
pages, per-page wikitext, total staged wikitext, diagnostics, and source bytes,
and checks cancellation throughout the stream. The current defaults allow a
source file up to 8 GiB while staging at most 10,000 pages, 16 MiB per latest
page revision, and 512 MiB of latest-revision wikitext. Result paging and local
spill remain owned by the shared import-session layer.

Each page becomes one generic staged object. Stable identity uses the native
page ID when present; source hierarchy groups pages by numeric namespace. Only
the newest revision by timestamp and revision ID is retained. Omitted older
revisions are reported once with their count and revision-history import
remains a non-goal. Exact latest-revision wikitext is retained as the canonical
document body and in transient raw review data; it is not presented as a
lossless Markdown conversion. Site, namespace, page, and revision metadata are
available to preview and field mapping.

Categories become staged tags, mapping selectors, and hierarchy hints rather
than automatic semantic relationships. Internal links are resolved after the
stream as resolved, ambiguous, or missing. File/image links remain visible but
not applicable because XML dumps do not contain their binary files. A unique
redirect target adds a staged alias and relationship hint while retaining the
redirect page for review. Template invocations are preserved as raw structured
source data. Infobox named parameters become generic staged fields and
low-confidence field hints; no template is treated as a Daena schema.

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

### HTML conversion policy

HTML is parsed with an HTML5 parser and converted to Markdown during analysis;
imported HTML is never rendered directly or granted a privileged origin. The
converter preserves headings, paragraphs, emphasis, code, lists, quotations,
basic tables, safe links, and images. It removes active or embedded document
content such as scripts, styles, frames, objects, templates, SVG, and MathML;
event-handler and other unconsumed attributes never enter the converted body.

Only relative references, fragments, protocol-relative URLs, and explicit
`http`, `https`, or `mailto` targets survive conversion. Root-absolute,
backslash-containing, control-character, and other URI schemes are removed
with visible diagnostics. Relative links and images then pass through the same
normalization, missing-target reporting, asset signature, hash, ownership, and
commit checks as authored Markdown.

DOM node/depth and converted-output limits fail closed. Parser recovery and
removed content produce reviewable warnings. Original HTML is retained only in
the transient staged item's raw source data for review; the committed document
is sanitized Markdown, so active source bytes do not enter canonical project
content.

### DOCX conversion policy

DOCX is treated as an untrusted OOXML ZIP package and converted into one
Markdown document. Before XML parsing, every package entry passes portable-path,
duplicate/case-collision, special-file, depth, entry-count, per-entry size,
total expanded-size, and compression-ratio checks. Required content types and
the main Word document part must be present. XML parsing disables DTDs and uses
an explicit node ceiling; malformed or excessive packages fail closed.

The converter preserves core title metadata, headings, paragraphs, common run
formatting, hyperlinks, numbered/bulleted lists, line breaks, simple tables,
and supported embedded images. Images are resolved through OOXML relationships,
validated by extension and byte signature, hashed during analysis, and re-read
from the unchanged DOCX package at commit. This works for direct files, folders,
and DOCX files nested in an imported ZIP without extracting either package.

Comments, note bodies, headers/footers, revisions, fields, merged-table details,
embedded objects, macros, and other unsupported structures are never guessed.
They are omitted or simplified with reviewable diagnostics. Core-properties XML
and the package-entry manifest are retained as transient staged raw data; the
committed document is Markdown, and active or unconverted OOXML parts are not
copied into canonical content.

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
limits and safe extraction, and the HTML/DOCX parsers that pass quality and
security fixtures. ODT and RTF remain deferred as described above.

**Exit gate:** nested folders and ZIPs produce equivalent staged structure;
HTML and DOCX produce reviewable Markdown without active content; traversal,
symlinks, bombs, malformed documents/XML, unsafe targets, and missing assets are
safely blocked or reported; successful document and asset imports round-trip
through checkpoint rebuild without path or hash drift.

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
- Iteration 3: implemented; the project menu opens a shared import workflow with
  importer/source selection, live progress, paged item inspection, diagnostics,
  enabled-manifest-derived entity/field/relationship choices, global/folder/item
  overrides, and a deterministic generation-bound candidate plan. Closing or
  cancelling cleans the session without mutating project content.
- Iteration 4: implemented for Markdown/plain-text/folder imports. The server
  rebuilds and validates plans against the current enabled manifests and project
  generation, detects repeated source identities, requires explicit conflict
  decisions, and supports create/skip/map-to-existing. Commit uses one
  receipt-backed transaction with revision preconditions, warning acknowledgement,
  idempotent retry, canonical source-identity fields, session cleanup, and a
  per-item result report. Replace and merge remain intentionally disabled.
- Iteration 5: implemented. The delivered slice preserves raw YAML
  frontmatter without rewriting the Markdown body, discovers standard Markdown
  links/images (including reference links), resolves normalized relative paths,
  reports missing or escaping targets, and preflights referenced PNG/JPEG/GIF/
  WebP/PDF attachments by signature, size, and SHA-256. Commit reopens the
  project-bound source through symlink-refusing normalized paths, verifies the
  analyzed hash and size, and adds attachment rows in the same receipt-backed
  transaction as their created or mapped owner. Attachment bytes and metadata
  are covered by checkpoint and clean-rebuild tests. ZIP sources now pass the
  same staging/validation/commit path without extracting to disk; central-
  directory preflight rejects non-UTF-8, absolute, traversal, platform-prefix,
  duplicate/case-colliding, link/special-file, oversized, excessive-depth, and
  high-compression-ratio entries before content parsing. Folder/ZIP equivalence,
  malformed/traversal/bomb rejection, cancellation checks, archive attachment
  commit, and clean rebuild are covered. HTML and HTM sources now use a bounded
  HTML5-to-Markdown converter that retains the original bytes in transient staged
  review data, preserves safe document structure, routes converted links and
  images through the Markdown resolver, removes active/embedded content and
  unsafe targets with diagnostics, and rejects excessive DOM complexity. Quality,
  malformed-input, active-content, link/asset, limit, commit, and clean-rebuild
  fixtures cover the enabled path. DOCX sources now pass bounded OOXML package
  and DTD-disabled XML preflight, preserve common document structure and core
  title metadata as Markdown, resolve safe hyperlinks, and import signature-
  checked embedded images from direct, folder, or nested-ZIP sources. Traversal,
  malformed package/XML, active/unsupported content diagnostics, conversion
  quality, attachment re-read, commit, and clean-rebuild fixtures cover the
  enabled DOCX path. ODT/RTF parsers remain explicitly deferred and will only be
  enabled with their own format-specific quality and security fixtures.
- Iteration 6: implemented. A separate folder-only Obsidian importer reuses the
  staged analysis, mapping, validation, and atomic commit pipeline while leaving
  generic Markdown semantics unchanged. It parses a bounded YAML subset into
  generic fields, aliases, tags, and type hints while retaining raw frontmatter;
  preserves note bodies and plugin syntax; excludes configuration/trash folders;
  resolves path, filename, title, and alias wikilinks plus note/attachment embeds;
  and reports ambiguous, missing, partially parsed, and unsupported data for
  review. Representative, ambiguity, missing-target, generic-compatibility,
  folder-only, attachment commit, and clean-rebuild fixtures cover the profile.
- Iteration 7: implemented. A file-only MediaWiki importer streams UTF-8 XML
  without constructing a complete XML tree, rejects DTD/entity expansion,
  enforces explicit source/page/content/depth/diagnostic limits, reports
  progress, and supports cancellation. It stages the latest revision per page,
  namespaces, source metadata, redirects, categories, internal links, raw
  wikitext, templates, and infobox field hints through the existing preview,
  mapping, validation, atomic commit, and report pipeline. Fixtures cover
  multi-revision selection, metadata and structure preservation, link and
  redirect resolution, malformed XML and DTD rejection, page limits,
  cancellation, commit, checkpoint, and clean rebuild. Full revision history,
  wikitext-to-Markdown conversion, and media-file retrieval remain out of scope.
- Iteration 8: planned, not yet implemented.
