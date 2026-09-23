# External import system

## Purpose and authority

This document is the product and architecture spec for migrating external
material into an open project. It is subordinate to
[`ARCHITECTURE.md`](./ARCHITECTURE.md),
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
objects, skipped items, converted fields, created relationships, conflict
decisions, assets, unsupported data, missing data, and all diagnostics. The
report returned by commit is authoritative for the operation. Persisting a
report is optional and, if added, uses an explicit Daena-owned project-metadata
contract rather than importer-controlled files.

### Source identity and re-import

Staged objects carry importer ID, importer contract version, source-relative
path, optional source-native ID, and a content fingerprint. Commit retains a
Daena-owned, source-specific provenance record containing that identity plus
the source kind, hierarchy, aliases, tags/categories, adapter metadata,
unmapped source fields, and discovered-link resolutions. This makes accepted
imports inspectable and rebuildable without exposing importer-controlled
namespaces or silently dropping sparse frontmatter/infobox data. Document-sized
raw content is not duplicated into provenance. The record supports duplicate
detection and a future explicit re-import workflow; it never enables background
watching or implicit synchronization.

### Explicit relationship mapping

The mapping UI offers only resolved internal links and embeds as relationship
sources. A user may map each source kind to a relationship type contributed by
an enabled manifest. Types with required relationship metadata are withheld
because the source adapters cannot satisfy that contract without an additional
mapping step. Validation rejects disabled or stale types, deduplicates repeated
source links to the same target, omits ambiguous/missing links and links to
skipped items with an aggregated warning, and carries only validated endpoints
into the immutable plan. Commit creates the relationships in the same atomic,
receipt-backed transaction as their entities and attachments. No category,
tag, redirect, or link becomes a semantic relationship unless the user maps it.

Global and folder field mappings are conditional: a mapped key is applied when
present and is not an error on sparse notes/pages. Unmapped fields are retained
in source provenance and summarized in one warning instead of generating a
warning per field per object.

### Supported formats

The built-in importer accepts:

1. Markdown, plain text, HTML, DOCX, and recursive folders;
2. ZIP, and Obsidian as a folder-only specialization of Markdown;
3. streaming MediaWiki-compatible XML with conservative wikitext preservation.

Third-party importer plugins are not shipped. ODT and RTF are not advertised
and are not selectable.

Markdown initially remains one staged object per file. Headings are preserved
in the document body and do not create entities automatically. Obsidian adds
frontmatter, aliases, wikilinks, embeds, attachments, and vault-resolution
rules while producing the same staged contract.

### Deferred ODT and RTF support

ODT and RTF are not part of the current product. They are not advertised
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
re-read, atomic commit, and clean-rebuild validation. Preview shows source-field
values, aliases, tags, metadata, link resolution, and the enabled relationship
types that can accept resolved links.

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
low-confidence field hints; no template is treated as a Daena schema. Accepted
categories, redirect aliases, namespace/revision/site metadata, template names,
and unmapped infobox values survive commit in Daena-owned source provenance.

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

`create`, `skip`, and `map to existing` are enabled. `replace` and `merge` are
not.

## Remaining work

### Plugin importer ecosystem

Add importer declarations to the canonical Rust plugin contract, generate JSON
Schema and TypeScript SDK types, implement broker discovery and bounded opaque
source reads, and add plugin test-host conformance fixtures. Importer
availability follows enabled service/capability state.

**Exit gate:** a test plugin can detect and analyze a new format into the
neutral contract without storage knowledge or project-write authority;
malformed, oversized, timed-out, disabled, and revoked providers fail closed;
bundled and plugin output pass identical core validation.

## Non-goals

- bidirectional sync, background watching, or live external mirrors;
- full source revision history;
- automatic semantic relationship inference;
- application-specific layout/widget reproduction;
- silent entity-type creation, destructive conversion, or automatic merge;
- unrestricted plugin filesystem/network access; and
- first-party support for every proprietary worldbuilding product.
