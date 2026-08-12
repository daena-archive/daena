# Daena Language module

Architecture and implementation brief for a first-class fictional-language workspace.

## 1. Purpose and scope

The Language module gives authors a focused place to document a fictional language and maintain its vocabulary. The first release must be useful without requiring linguistic expertise: create a language, record a few basic facts, and build a searchable lexicon.

The module must fit Daena rather than become a separate application inside it:

- a Language is a normal top-level Daena entity with stable identity;
- overview data uses the normal namespaced entity-field and document contracts;
- lexemes are Language-owned module records, not global entities;
- Rust/core remains the authority for validation, revisions, persistence, checkpointing, recovery, and authorization;
- bundled code uses the same public manifest, SDK, and broker contracts available to plugins;
- SQLite is live runtime authority, and deterministic portable files provide the checkpoint used for Git, inspection, and clean reconstruction;
- UI and creation behavior are derived from the enabled module manifest, not checks for a hard-coded Language module ID.

This document defines iterations 1–5 and the boundaries later iterations must preserve. It is not approval to implement the later feature list now.

## 2. Product model

### 2.1 Language entity

Register `language` as an entity type contributed by the bundled Language module. A Language receives all normal Daena entity behavior: ID, name, document, revision, soft deletion, global search, relationships, assets, checkpointing, and recovery.

Only the entity name is required. The template must not persist fake starter content or defaults merely to make the form look complete.

Iteration 1 overview fields are:

- native name;
- aliases;
- status;
- family;
- writing system;
- description or notes through the normal entity document.

All fields are optional. Treat pronunciation and fictional notation as author text; do not reject unfamiliar scripts or require IPA.

### 2.2 Language-owned records

A lexeme belongs to exactly one Language entity but is not itself an entity. This distinction is architectural:

```text
Daena entity graph
└── Language entity
    ├── common identity, document, fields, links, assets
    └── Language module workspace
        └── lexeme records
```

Do not model lexemes as global entities. Large lexicons would overwhelm global search, entity lists, and relationship views. Do not store the entire lexicon as one JSON array in an entity field either; that would create whole-lexicon conflicts, expensive rewrites, and unstable item identity.

Iteration 1 therefore introduces a generic **module-owned record collection** contract in core and the broker. Language is its first consumer. The primitive must be module-neutral so later bundled modules and third-party plugins can use the same facility.

Phonemes, orthographies, senses, paradigms, and samples are future record families. Do not create placeholder tables, empty screens, or speculative schemas for them in iteration 1.

## 3. Iteration 1

### 3.1 User outcome

An author can create a Language using only a name, edit its overview, and add, edit, delete, browse, and search vocabulary entirely offline. All data survives reopen and clean reconstruction from the portable checkpoint.

### 3.2 Required capabilities

1. Contribute the Language entity type and empty creation template through manifest v1.
2. Show enabled Language entities in a top-level Language workspace.
3. Create and open a Language through existing generic entity creation and selection behavior.
4. Edit the optional overview fields and entity document.
5. Create, read, update, delete, page, sort, and search lexemes scoped to one Language.
6. Give every lexeme a stable opaque ID and independent opaque revision.
7. Preserve retry idempotency and reject stale revisions.
8. Persist lexemes through the normal runtime transaction and checkpoint pipeline.
9. Rebuild lexemes and their derived search projection from a clean portable checkpoint.
10. Preserve Language data when the module is disabled while removing its active navigation and commands.

### 3.3 Lexeme contract

The minimum authored shape is:

```ts
type Lexeme = {
  id: RecordId;
  ownerEntityId: EntityId; // the Language
  lemma: string;
  partOfSpeech?: string;
  meanings: string[];
  pronunciation?: string;
  notes?: string;
  example?: {
    text: string;
    translation?: string;
  };
  revision: Revision;
};
```

Only `lemma` is required. `meanings` is list-shaped in storage even if the first editor uses a simple repeated-input control. Trim values for validation, preserve meaningful Unicode, and do not use the lemma or list position as identity. Duplicate lemmas are valid because homonyms exist.

The exact generated Rust and TypeScript names must follow repository conventions. Wire payloads also carry request IDs and expected revisions according to the existing RPC contract.

### 3.4 Non-goals

Iteration 1 does not include:

- phoneme inventories or IPA charts;
- phonotactic, sound-change, or language-evolution engines;
- orthography mapping or custom glyph/font tooling;
- structured grammar reference pages;
- conjugation, declension, morphology, or generated paradigms;
- multiple structured senses, etymology graphs, or form trees;
- interlinear glossing or corpus analysis;
- audio recording or playback;
- specialist import/export formats;
- automatic translation or AI generation;
- temporal modeling;
- a separate database or language-specific filesystem watcher;
- global lexeme entities or global lexeme search results;
- new cross-entity relationship workflows.

## 4. UI and interaction design

### 4.1 Navigation and workspace

Language appears as a normal enabled module. Its manifest and effective project configuration determine visibility and creation options.

Use the existing entity workspace for the Language overview and a focused module projection for the lexicon:

```text
Language workspace
├── language list/search + Create Language
└── selected Language
    ├── overview: name, fields, document, save state
    └── Open lexicon → focused lexicon projection
```

On narrow windows, the list may become a back-navigation level instead of a permanent column. Preserve the selected Language when opening and closing the lexicon projection.

Do not add lexemes to the main Daena entity list. The Language list may show native name, status, and a derived lexeme count, but it must remain visually quiet.

### 4.2 Overview

The overview uses the existing entity name/document editing behavior plus a small namespaced field form. Show only fields that work now. Use visible optional labels or supporting text; do not display disabled future sections.

Saving behavior must match Daena:

- edits receive visible saving, saved, and error feedback;
- failed saves preserve input;
- stale revisions become recoverable conflicts rather than silent overwrites;
- untouched manifest defaults do not mark the view dirty.

### 4.3 Lexicon

Use a list and a separate editor panel or dialog. Do not build an editable spreadsheet in iteration 1.

The list provides:

- search by lemma and meaning;
- columns or rows for lemma, part of speech, and first meaning;
- deterministic sort, initially lemma then stable ID;
- bounded page size with next/previous or incremental loading;
- an empty state with one clear **Add word** action.

The editor provides:

- required lemma;
- optional part of speech as editable text, optionally with non-binding suggestions;
- repeatable meanings;
- optional pronunciation, notes, example, and translation;
- Save and Cancel;
- explicit confirmation before deletion.

Keep the selected lexeme stable after save. Refresh only the affected record/list result rather than remounting the whole workspace. Search must be debounced and cancellable so older results cannot replace a newer query.

### 4.4 Accessibility and visual rules

- Use labels, not placeholders, as the only field description.
- Keep focus inside dialogs and return it to the invoking control on close.
- Make loading, empty, error, and conflict states visible to assistive technology.
- Use the established Daena spacing, form, button, and panel styles.
- Keep primary actions visible; do not hide Save or Add word in overflow menus.
- Avoid cramped multi-column forms and avoid horizontal scrolling at ordinary desktop widths.

## 5. Technical architecture

### 5.1 Bundled module package

Add a first-party package beside the existing bundled modules, conceptually:

```text
packages/modules/language/
  manifest.json
  package.json
  src/index.ts
```

Register it through the existing bundled-module catalog. Use a stable ID such as `daena.language` and namespace such as `language`, following the exact current naming rules.

The manifest remains **manifest version 1**. Add the Language schema, template, capabilities, view contribution, and migration declarations through existing contracts. If module-owned record collections require a new optional manifest member, extend manifest v1 backward-compatibly, update its Rust source type, regenerate JSON Schema and TypeScript, and keep old manifests valid. Do not introduce manifest v2 for this module.

The Language template must have:

- entity type `language`;
- name supplied by the generic create dialog;
- no required fields beyond the common entity name;
- empty field/document values so opening the template does not create authored content.

The specialized bundled view mounts through the public module API and returns its cleanup handle. It uses the SDK/broker client; it must not import the trusted project client or call Tauri directly.

### 5.2 Overview storage

Store overview values as Language-owned namespaced entity fields. Store prose in the normal entity document. This keeps overview data in the shared entity model and makes existing revisions, overlays, search, export, and external-edit behavior available without a parallel overview record.

Do not duplicate the entity name or document inside module-owned data.

### 5.3 Generic module-owned record primitive

Add one generic core model for records owned by a module collection and optionally scoped to an entity:

```text
ModuleRecord
  module_id
  collection             // e.g. "lexemes"
  id                     // opaque stable ID
  owner_entity_id        // required for Language lexemes
  value                  // schema-validated JSON object
  revision               // opaque, database-epoch aware
```

Timestamps may be included if that is the current core convention, but the client must not use them as identity or revision.

The collection declaration must define its JSON schema and limits. Core validates the declared schema, collection ownership, owner entity existence, payload size, text lengths, and list bounds on every mutation. Unknown collections fail closed. The broker binds `module_id` to the active session; callers cannot choose another module's namespace.

Use a generic runtime table such as `module_records`, not `language_lexemes`. Add appropriate uniqueness and owner/collection indexes. Keep search in a disposable derived projection; authored record rows remain durable runtime content.

### 5.4 Broker and SDK surface

Extend the public broker with narrow generic operations following current naming conventions:

- `record.list`
- `record.create`
- `record.update`
- `record.delete`

`record.list` accepts the collection, owner entity ID, bounded page request, deterministic sort, and optional text query. It never accepts raw SQL, arbitrary JSON predicates, a filesystem path, or an unrestricted namespace.

Add self-scoped read/write capabilities if existing capability vocabulary cannot express this boundary. Authorization must verify:

- active project and module session;
- module enabled state;
- declared collection and capability;
- collection ownership by the session's module;
- live owner entity with an allowed entity type;
- record ownership on get/update/delete;
- request ID identity and expected revision.

Generate the JSON Schema, RPC fixtures, TypeScript declarations, SDK helpers, and fake test-host behavior from the Rust contract. Do not add Language-specific Tauri commands or bypass the broker because Language is bundled.

### 5.5 Runtime persistence and portable checkpoint

Normal operations commit SQLite rows, idempotency receipts, revisions, and content generation in one core transaction. The normal checkpoint worker then renders portable data. Language does not write portable files directly.

Serialize module-owned collections into the existing portable plugin state at:

```text
plugins/daena.language.json
```

Extend that strict JSON contract with a deterministic records section. Keep records individually identifiable while using one array so the existing plugin-state codec can sort and validate all collections uniformly:

```json
{
  "dataVersion": 1,
  "records": [
    {
      "collection": "lexemes",
      "id": "...",
      "ownerEntityId": "...",
      "value": { "lemma": "...", "meanings": [] },
      "createdAt": "...",
      "updatedAt": "..."
    }
  ]
}
```

This extends the current plugin-state codec without replacing its enablement, namespace, schema, package, or migration data. Sort records deterministically by collection and ID. Omit runtime revisions, idempotency receipts, FTS data, UI selection, caches, and transient save state from portable authored data. Rebuild assigns revisions in the new database epoch.

Checkpoint validation must reject duplicate IDs, unknown collections, missing/deleted owner entities, namespace violations, invalid JSON, and schema-invalid payloads. A failed external import reports diagnostics and does not replace the last valid runtime state.

### 5.6 Search

Keep the two search domains separate:

- global Daena search indexes the Language entity and normal document/fields;
- `record.list` text search indexes lexemes only within the selected Language.

Build lexeme search as a disposable projection derived from schema-valid records. At minimum, index lemma and meanings. Scope every query by module, collection, and owner entity before ranking or pagination. Never insert lexemes into the global entity FTS table.

### 5.7 Validation limits

Choose explicit, tested bounds during implementation and keep them in the generated contract. At minimum bound:

- request and record payload bytes;
- lemma and individual text lengths;
- number and length of meanings;
- page size;
- search query length;
- records returned per response.

Bounds protect the host; they must not prescribe how a fictional language works. Preserve Unicode and user notation.

## 6. Implementation sequence

Implement only this vertical slice:

1. Add the generic module-record core model, storage, revisions, portable codec, and rebuild validation.
2. Add broker authorization, RPC schema, SDK helpers, and test-host support for module records.
3. Add the bundled Language manifest and register its entity schema/template.
4. Add the Language workspace with Overview and Lexicon.
5. Add scoped lexeme search and its rebuildable projection.
6. Verify iteration 1 exit gates.

Iteration 2:

1. Extend the lexeme schema with nested senses, forms, pronunciations, tags, and status while remaining backward-compatible.
2. Add allowlisted `record.list` sort and filter parameters.
3. Replace the lexicon editor with the richer form, filters, homonym workflow, and JSON import/export.
4. Verify iteration 2 exit gates.

Iteration 3:

1. Add Language-owned `phonemes`, `phonology`, and `orthographies` record collections.
2. Add allowlisted `symbol` and `name` sorts for those collections.
3. Add Sounds and Writing panes with optional IPA-style charts and grapheme-to-sound mappings.
4. Verify iteration 3 exit gates.

Iteration 4:

1. Add a Language-owned `grammar` record collection for Markdown topics.
2. Add an allowlisted `title` sort.
3. Add a Grammar pane with section navigation and typed lexeme/example links.
4. Verify iteration 4 exit gates.

Iteration 5:

1. Add a Language-owned `paradigms` collection with nested slots and rules.
2. Generate form previews from those rules without writing generated cells into lexemes.
3. Add a Forms pane plus lexeme overrides that distinguish authored from generated provenance.
4. Verify iteration 5 exit gates.

Do not begin samples, AI, or relationship-specific work as part of this slice.

## 7. Verification and exit gates

Iteration 1 is complete only when all of these are demonstrated:

- A Language can be created with only a name.
- Overview edits save without untouched defaults causing dirty state.
- Lexemes can be created, edited, paged, searched, and deleted within one Language.
- Duplicate lemmas remain distinct records.
- A lexeme cannot be read or mutated through another Language owner ID or another module session.
- Stale revisions and mismatched request-ID retries fail with typed errors.
- Lexemes never appear in entity lists or global search.
- Disable/re-enable hides and restores the active module UI without deleting authored data.
- Close/reopen preserves all Language data.
- A flushed clean checkpoint deterministically contains Language data.
- Deleting `.daena/` only after confirming a clean checkpoint and reopening reconstructs overview and lexemes.
- Malformed portable Language data yields diagnostics rather than partial silent import.
- Generated manifest/RPC schemas, TypeScript SDK, fixtures, and test host are in sync.
- Focused Rust tests, broker conformance tests, frontend checks, and a rendered desktop interaction pass succeed.
- The rendered pass covers empty, populated, loading, save failure, deletion confirmation, and stale-conflict states.

Iteration 2 is complete only when all of these are demonstrated:

- A lexeme can store multiple senses, examples, forms, and pronunciation variants with stable nested IDs.
- Iteration 1 lexemes open and save without losing lemma, meanings, pronunciation, or example.
- Status and tag filters and lemma/status/updated sorts return the expected page.
- Homonyms remain distinct; the editor reports other matches and can create another entry with the same lemma.
- JSON export round-trips authored lexeme values; JSON import recreates them as new records.
- Search still matches lemma, glosses, and nested definitions within one Language.
- Generated `record.list` schemas and TypeScript SDK include the new allowlisted list parameters.

Iteration 3 is complete only when all of these are demonstrated:

- A Language can store consonant and vowel inventory items, phonotactic notes, and one or more orthographies as module-owned records.
- IPA is optional; user-defined symbols are accepted.
- Incomplete inventories still save; unplaced sounds appear outside the chart rather than being rejected.
- Grapheme-to-sound mappings can name sounds by symbol without requiring a complete inventory.
- Phonology and orthography data survive checkpoint rebuild and stay scoped to one Language.
- Lexicon behavior from iterations 1 and 2 remains available in the same workspace.

Iteration 4 is complete only when all of these are demonstrated:

- Grammar topics can be created, edited, deleted, and grouped under word order, noun, pronoun, verb, modifier, clause, agreement, and other.
- Topic notes are Markdown and render a safe preview.
- Typed links can point at lexemes and examples in the same Language and open the linked word.
- Grammar records survive checkpoint rebuild and stay scoped to one Language.
- Sounds, writing, and lexicon panes remain available.

Iteration 5 is complete only when all of these are demonstrated:

- A Language can store inflectional and derivational paradigm tables as module-owned records.
- Generated-form previews fill from rules; more specific lemma-ending matches win over default rules.
- Irregular and other authored forms can override a generated cell and keep an explicit authored provenance.
- Changing a rule updates the generated preview and never deletes or rewrites authored forms, exceptions, or examples.
- Paradigm records survive checkpoint rebuild and stay scoped to one Language.
- Grammar, sounds, writing, and lexicon panes remain available.

## 8. Later iterations

Later work must extend the same Language entity and generic module-record boundary. Each iteration should be independently useful.

### Iteration 2: richer lexicon

Iteration 2 extends the same Language entity and `lexemes` collection. Senses, forms, examples, and pronunciation variants remain module-owned structured objects with stable IDs nested in the lexeme value. The generic record primitive is still scoped to an owner entity, not a parent record; nested families avoid a parallel identity/revision plane until a later generic parent-record contract exists.

Iteration 1 lexemes remain valid. On edit, `meanings`, a top-level `example`, and a single `pronunciation` are normalized into senses and pronunciation variants. Saves keep `meanings` as the list of sense glosses so list columns and search stay compatible.

#### User outcome

An author can document multiple senses and examples for a word, record alternate forms and pronunciations, tag and filter the lexicon, inspect homonyms, and round-trip a language's vocabulary through JSON without leaving the module.

#### Required capabilities

1. Structured senses with optional gloss, definition, usage notes, and multiple examples.
2. Alternate forms and pronunciation variants with optional notes.
3. Etymology and source notes on the lexeme.
4. Optional status and tags, with list filters and allowlisted sorts (`lemma`, `status`, `updatedAt`).
5. Homonym notice in the editor, **Add homonym**, and a **Homonyms only** list filter.
6. Lossless JSON export of authored lexeme values and import that recreates those values as new records.
7. Iteration 1 create/read/update/delete, paging, search, revision, and checkpoint behavior remains true.

`record.list` may accept allowlisted `sort`, `status`, `tag`, and `homonymsOnly` parameters. It still must not accept raw SQL or arbitrary JSON predicates.

#### Non-goals

Iteration 2 does not add separate sense/form/example collections, parent-record IDs, spreadsheet editing, specialist interchange formats, or phonology/grammar work.

### Iteration 3: phonology and orthography

Iteration 3 adds Language-owned record families beside `lexemes`:

- `phonemes` — inventory items with a required symbol and optional IPA, kind, articulatory features, and notes;
- `phonology` — optional syllable, stress, tone, and phonotactic notes for the language;
- `orthographies` — named writing systems with grapheme-to-sound mappings.

Formal validation stays optional. Charts are a derived view of authored features: consonants group by place and manner, vowels by height and backness. Missing features leave a sound unplaced instead of blocking save.

#### User outcome

An author can sketch a sound inventory and one or more writing systems without finishing an IPA chart first, then see a familiar grid fill in as features are added.

#### Required capabilities

1. Consonant and vowel (and optional tone/other) inventory records.
2. Optional IPA alongside user-defined notation.
3. Syllable structure, stress, tone, and phonotactic notes.
4. Multiple orthography records with grapheme-to-sound mappings.
5. Optional IPA-style consonant and vowel charts derived from authored features.
6. Iteration 1–2 lexicon behavior remains true.

#### Non-goals

Iteration 3 does not add phonotactic engines, sound-change rules, custom fonts, audio, or required IPA validity.

### Iteration 4: grammar reference

Iteration 4 adds a Language-owned `grammar` collection. Each topic has a title, a section, Markdown notes, and typed links to lexemes or examples. Suggested sections are word order, nouns, pronouns, verbs, modifiers, clauses, and agreement; authors may also file topics under other.

This is structured documentation, not an executable grammar engine. Links store lexeme (and optional example) IDs rather than creating global entities or cross-entity relationships.

#### User outcome

An author can keep a navigable grammar sketch next to the lexicon, write Markdown notes, and jump from a grammar mention to the linked word.

#### Required capabilities

1. Navigable grammar topics grouped by the suggested sections.
2. Markdown notes with a readable preview.
3. Typed links from topics to lexemes and examples in the same Language.
4. Iteration 1–3 lexicon, phonology, and orthography behavior remains true.

#### Non-goals

Iteration 4 does not generate paradigms, parse the language, or add interlinear samples.

### Iteration 5: morphology and paradigms

Iteration 5 adds a Language-owned `paradigms` collection. Each paradigm has a name, inflection or derivation kind, slots, and nested rules. Rules stay nested in the paradigm value so they do not need a parent-record identity plane.

Generation is a derived preview: prefix, suffix, replace-suffix, and identity operations compute cells from the lemma or typed stem. Generated cells are not written to the lexeme unless the author pins an override. Lexeme `forms` remain the authored store; pinned cells record `paradigmId`, `slotId`, and `provenance: override`. Existing alternate forms without those fields stay authored.

A lexeme may optionally point at a paradigm through `paradigmId` so the lexicon editor can show the same preview.

#### User outcome

An author can define a conjugation or derivation table, see generated forms for a stem, pin irregular overrides on a word, and change a rule without losing those exceptions.

#### Required capabilities

1. Inflectional and derivational paradigm records with named slots.
2. Nested rules with optional lemma-ending matches and affix operations.
3. Generated-form previews that label each cell generated, authored, or missing.
4. Authored overrides that win over generated cells and survive rule edits.
5. Iteration 1–4 lexicon, phonology, orthography, and grammar behavior remains true.

Changing a rule must never silently destroy authored forms, exceptions, or examples.

#### Non-goals

Iteration 5 does not add a full morphological parser, automatic paradigm inference, or sample/interlinear text.

### Iteration 6: samples and interlinear text

- sentence and paragraph samples;
- translation and transliteration;
- optional interlinear glossing;
- token-level lexeme links and grammar annotations;
- editable and readable rendered views.

Samples remain module-owned unless a later product decision explicitly promotes them.

### Iteration 7: advanced tools

- sound changes and historical development;
- language-family and dialect comparison;
- corpus statistics;
- audio and pronunciation assets;
- custom scripts, glyphs, and font previews;
- specialist interchange formats;
- provider-neutral AI suggestions that require explicit user acceptance.

Temporal modeling remains deferred until explicitly planned.

## 9. Decisions that must remain true

1. Language is a first-class Daena entity and top-level module experience.
2. Only a Language name is required at creation.
3. Overview data uses normal entity fields and documents.
4. Lexemes and future linguistic collections are module-owned records, not global entities by default.
5. The module-record facility is generic, schema-declared, revision-aware, and broker-authorized.
6. Bundled Language code has no private storage or Tauri bypass.
7. SQLite remains live runtime authority; deterministic portable checkpoints reconstruct clean state.
8. Manifest version stays at 1; additions are backward-compatible and generated from Rust contracts.
9. Future features add explicit record families instead of an unbounded metadata escape hatch.
10. Cross-entity relationships are normal Daena behavior, not an iteration-1 Language subsystem.
