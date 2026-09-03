# Daena Archive

![Daena Archive logo](static/branding/logo.png)

[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Version: v0.1.0-beta](https://img.shields.io/badge/version-v0.1.0--beta-orange.svg)](#current-status)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey.svg)](../../releases)
[![Tauri 2](https://img.shields.io/badge/Tauri-2-24C8DB.svg)](https://tauri.app)

> "Daena" (Avestan pronunciation: [dʌeːnaː]) is a Zoroastrian concept
> representing insight and revelation.
>
> - [Wikipedia](https://en.wikipedia.org/wiki/Daena)

Daena Archive is a free, open-source desktop app for building fictional worlds
and writing stories in them. It gives your notes, characters, places, history,
maps, languages, and manuscripts one connected home.

Daena works offline and keeps each project in a folder on your computer. You do
not need an account or a subscription.

## Current status

Daena is in active development (**v0.1.0-beta**) and is **not production
ready**. APIs, storage, and plugin contracts may change before `v1.0`. Track
progress in [GitHub Releases](../../releases) and [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

## Download

**Option 1 — Download the beta (recommended):**

1. Go to [Releases](../../releases) and download the installer for your OS.
2. Install and open Daena Archive, then create a new project folder.

**Option 2 — Build from source:**

```bash
deno install --node-modules-dir=auto
deno task tauri build
```

See [`README.DEV.md`](README.DEV.md) for full prerequisites.

## Features

### Build a world bible

Create and organize entries for the things that make up your world:

- People and characters
- Places and regions
- Factions and organizations
- Cultures, artifacts, and ideas
- Custom fields and notes for anything else

Each entry can have a readable document, structured details, relationships,
images, and other files. Entries share one stable identity across modules:
renaming or retyping never breaks existing links.

Archiving is reversible: everyday menus only archive, while
`Project Center → Archive` restores or permanently deletes.

### Houses and family trees

Houses tracks families and lineages alongside Lore people:

- House entities with documents, fields, members, and leadership roles
  (head, heir, founder, consort, member, custom)
- A `Tree` view for parents, partners, and house membership with typed
  relationships (biological, adoptive, marriage, partnership, and more)
- Bounded, explorable neighborhoods (three generations up/down by default)
  with expand/collapse, re-rooting, and house-scoped views
- Keyboard navigation, reduced-motion support, and direct round-trip to the
  Lore inspector for each person

### Connect your ideas

Link entries together so your world is easy to explore. A character can belong
to a faction, speak a language, come from a place, and appear in a story. Links
work in both directions, turning scattered notes into a connected reference.

Relationships carry their own details (kind, role, status, dates, notes), are
grouped by owning module in the inspector and wiki pages, and stay portable
even when a plugin schema changes.

Search across your project, filter collections, and move from one related entry
to another without losing context. Hover cards, interactive breadcrumbs, shell
history (back/forward), and `Quick Open` keep you oriented in large projects.

### Write stories alongside your notes

Writing Studio gives you two places to work:

- **Manuscripts** for stories, essays, and other long-form writing
- **Reference pages** for research, setting notes, plot ideas, and supporting material

Manuscripts support a containment outline (`series → book → chapter`) with
grouped, flat, and search views, parent paths, and `New in this manuscript`
for nested chapters. Your writing can link directly to the people, places, and
events in your world, including inline `@` entity mentions with hover cards.
Link URLs are validated so pasted targets stay safe.

### Keep track of history

Timeline helps you organize events, encounters, eras, and calendars. Add dates,
locations, and participants, then see how the important moments in your world
fit together.

You can use familiar Gregorian dates or create a calendar with your own years,
months, weeks, seasons, and eras — including BCE dates. The chronology view
stays in sync with creates, archives, restores, and type changes.

### Create fictional languages

The Language workspace helps you develop a language without requiring a
linguistics background. Keep its overview and vocabulary together, then add:

- Lexicon with senses, alternate forms, pronunciations, tags, homonyms, and
  JSON import/export
- Sounds and phoneme inventories with IPA-style charts
- Writing systems and grapheme-to-sound mappings
- Guided grammar workspace (syntax, nouns, pronouns, verbs, agreement,
  custom rules) with progressive disclosure and inline help
- Word forms, paradigms, and morphology with generated previews and authored
  overrides
- Example sentences, translations, and interlinear glosses linked back to
  the lexicon

### Make maps part of the story

Maps are connected to your notes instead of being isolated images. Attach
places, characters, events, and other entries to map locations, then move
between the map and the related information.

Daena supports three map models, all offline with the OpenLayers editor:

- **Authored vector maps** — draw points, lines, polygons, labels, and layers
  with undo/redo, snapping, styling, and map-local search
- **Image-backed maps** — import a validated image as a pixel-native
  background and author geometry above it
- **Generated physical worlds** — accept a deterministic terrain/climate/
  hydrology world, explore epochs, layer authored countries and routes above
  it, and use explicit `Detach for editing` to reshape generated coastlines
  into editable geometry

Layer visibility, locking, opacity, and styling are first-class. `Atlas Studio`
renders detailed interactive tiles from a map snapshot, and static export
produces PNG, SVG, or PDF with provenance.

Maps are still a beta feature and are being actively improved.

### Bring in notes you already have

`Project Center → Import material` migrates outside material through an explicit,
review-before-commit pipeline: analyze → preview and map → validate →
one atomic commit → report. Nothing is added until you confirm, and
unsupported content is named rather than silently dropped.

Supported sources:

- Markdown, plain text, and recursive folders
- ZIP archives (with traversal/bomb guards)
- HTML (converted to sanitized Markdown)
- DOCX (structure plus embedded images, converted to Markdown)
- Obsidian vaults (frontmatter, aliases, wikilinks, embeds, attachments)
- MediaWiki XML exports (streamed latest revisions, namespaces, redirects,
  categories, infobox hints)

You map each item to an entity type, field, or relationship from your enabled
modules; resolved links can become real Daena relationships in the same commit.

### Make Daena match your world

`Project Center → Fields & Types` customizes Lore, Timeline, Writing, and
Houses per project without touching package defaults:

- Enable/disable built-in types, add custom types, fields, and templates
- Relationship metadata, Timeline options, and appearance (icon/color)
- Live-data impact preview before risky saves, with revision-protected,
  idempotent updates

Language and Maps stay extension-managed until their specialized surfaces
render custom schema consistently.

### Find your way around

- `Quick Open` jump-to-anything with grouped results
- Interactive breadcrumbs and shell back/forward history
- Contextual workspace guides (tour/hint) for Lore, Timeline, and Language
- Light/dark/system themes
- `Project Center` for archive management, schema, and project-level actions

### Optional AI assistance

AI is optional and opt-in, configured per project rather than globally. Each
project can bind its own text provider (such as LM Studio or Ollama for fully
offline use) and its own image provider, plus project-level prompt-template
overlays on top of bundled defaults. Remote providers are also supported if
you choose to use them, with explicit per-project/provider/endpoint consent —
no silent fallback from local to remote.

Daena's AI is an assistant, not an autonomous content generator. It can:

- Rewrite, shorten, expand, or change the tone of selected text
- Propose schema-compatible structured values (biographies, traits, summaries)
- Surface consistency findings and grounded brainstorming from your existing
  documents, relationships, and structured information
- Generate entity portraits and concept art locally through a user-managed
  ComfyUI server, with expiring temporary candidates and explicit acceptance
  into canonical assets (with generation provenance)

It does not create content or change your project on its own. Retrieval is
permission-filtered with ranked, provenance-bearing citations (including stale
markers), and results are shown as editable proposals with diffs — so
you decide what to keep. Credentials stay in the OS keychain; AI indexes are
disposable machine-local state.

### Keep your work under your control

Your project stays on your computer and can be backed up like any other folder.
Daena also provides:

- Portable project files that are easy to inspect and move
- Local and recovery backups
- Optional Git snapshots with selective staging, snapshot history browsing,
  read-only file previews, hard-reset restore, remotes, lease-protected pushes
  (`--force-with-lease` only), and restore-from-remote recovery —
  all under `Settings → Git`
- Built-in update checker with stable/beta/alpha channels and an About section
- Search indexes that can be rebuilt without losing your authored content

### Add more through plugins

Daena supports optional **plugins** (including built-in modules) so new tools
can be added without changing the core of your project. Plugins run with
limited, user-approved access rather than unrestricted access to your files or
system. See [`docs/PLUGIN_SDK.md`](docs/PLUGIN_SDK.md) and
[`docs/PLUGIN_PLATFORM_PLAN.md`](docs/PLUGIN_PLATFORM_PLAN.md).

## About Daena

Daena is, and forever will be, free and open source. It has no subscriptions,
paid features, or any other forms of monetization. If you find it useful, you
can help by [reporting bugs](../../issues), contributing code, or sharing it
with someone who enjoys worldbuilding.

Daena began as a personal attempt to organize the worlds created during
daydreams and sleepless nights. When I failed to find an application that suited
all my needs, I decided to create my own app.

## License

Daena is licensed under the [Apache License 2.0](LICENSE).
