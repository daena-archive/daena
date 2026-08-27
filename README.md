# Daena Archive

![Daena Archive logo](static/branding/logo.png)

[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Version: v0.1.0-alpha](https://img.shields.io/badge/version-v0.1.0--alpha-orange.svg)](#current-status)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey.svg)](#system-requirements)
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

Daena is in active early development (**v0.1.0-alpha**) and is **not production
ready**. APIs, storage, and plugin contracts may change before `v1.0`. Track
progress in [GitHub Releases](../../releases) and [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

The first beta is expected in early September 2026.

## Download

**Option 1 — Download the alpha (recommended):**

1. Go to [Releases](../../releases) and download the installer for your OS.
2. Install and open Daena Archive, then create a new project folder.

**Option 2 — Build from source:**

```bash
deno install --node-modules-dir=auto
deno task tauri build
```

See [`README.DEV.md`](README.DEV.md) for full prerequisites.

Each project is a folder on disk (`project.json`, `entities/`, `.daena/`). Back it up like any folder. See [`docs/STORAGE.md`](docs/STORAGE.md).

## System requirements

* **OS:** macOS 13+, Windows 10+, or Linux with WebKitGTK 4.1
* **Runtime:** WebView2 (Windows, usually preinstalled), 200 MB disk + projects
* **Build only:** Rust 1.85+ (tested 1.98), Deno 2.x (tested 2.9.5), `cargo` with `clippy`/`rustfmt`

## Quick start

1. **Create a project** — `File → New Project` and pick an empty folder.
2. **Add a world bible entry** — `People` → `New` → name, document, and structured fields.
3. **Connect ideas** — link a character to a faction, language, and place; links are bidirectional.
4. **Write** — open `Writing Studio` → `Manuscript` and link text to world entries.
5. **Explore** — `Search` across entities, `Timeline` for dates, `Maps` for locations.

## Features

### Build a world bible

Create and organize entries for the things that make up your world:

- People and characters
- Places and regions
- Factions and organizations
- Cultures, artifacts, and ideas
- Custom fields and notes for anything else

Each entry can have a readable document, structured details, relationships,
images, and other files.

### Connect your ideas

Link entries together so your world is easy to explore. A character can belong
to a faction, speak a language, come from a place, and appear in a story. Links
work in both directions, turning scattered notes into a connected reference.

Search across your project, filter collections, and move from one related entry
to another without losing context.

### Write stories alongside your notes

Writing Studio gives you two places to work:

- **Manuscripts** for stories, essays, and other long-form writing
- **Reference pages** for research, setting notes, plot ideas, and supporting material

Your writing can link directly to the people, places, and events in your world.

### Keep track of history

Timeline helps you organize events, encounters, eras, and calendars. Add dates,
locations, and participants, then see how the important moments in your world
fit together.

You can use familiar Gregorian dates or create a calendar with your own years,
months, weeks, seasons, and eras.

### Create fictional languages

The Language workspace helps you develop a language without requiring a
linguistics background. Keep its overview and vocabulary together, then add:

- Sounds and phonemes
- Writing systems
- Grammar notes
- Word forms and morphology
- Example sentences and translations

### Make maps part of the story

Maps are connected to your notes instead of being isolated images. Attach
places, characters, events, and other entries to map locations, then move
between the map and the related information.

The Maps workspace currently supports creating physical worlds and editing
OpenLayers vector maps (including image import). Maps are still a beta feature
and are being actively improved.

### Optional AI assistance

AI is optional and opt-in. You can run models locally with providers such as
LM Studio or Ollama, so AI assistance can work without an internet connection.
Remote providers are also supported if you choose to use them.

Daena's AI is an assistant, not an autonomous content generator. It can work
with the writing already in front of you or suggest ideas based on your existing
documents, relationships, and structured information. It does not create content
or change your project on its own. Results are shown as editable proposals, so
you decide what to keep.

### Keep your work under your control

Your project stays on your computer and can be backed up like any other folder.
Daena also provides:

- Portable project files that are easy to inspect and move
- Local and recovery backups
- Optional Git snapshots, history, remotes, and restore tools
- Search indexes that can be rebuilt without losing your authored content

### Add more through plugins

Daena supports optional **plugins** (including built-in modules) so new tools
can be added without changing the core of your project. Plugins run with
limited, user-approved access rather than unrestricted access to your files or
system. See [`docs/PLUGIN_SDK.md`](docs/PLUGIN_SDK.md) and
[`docs/PLUGIN_PLATFORM_PLAN.md`](docs/PLUGIN_PLATFORM_PLAN.md).

## FAQ

**Is my project compatible between versions?** `v0.1.0-alpha` storage may change
before `v1.0`. Keep Git snapshots or backups before upgrading. See
[`docs/STORAGE.md`](docs/STORAGE.md).

**Where is my data?** In the folder you picked at `New Project` — `project.json`
at the root, `entities/` and `assets/` for content, `.daena/` for the live
index. Move or copy the folder to move your project.

**Does Daena need the internet?** No. Writing, maps, languages, and local AI all
work offline. Remote AI and Git remotes are opt-in.

**Are plugins safe?** Plugins declare `capabilities` and need your consent to
read/write outside their namespace. See `docs/PLUGIN_SDK.md`.

## Contributing

Daena is, and forever will be, free and open source. It has no subscriptions,
paid features, or any other forms of monetization. If you find it useful, you
can help by [reporting bugs](../../issues), contributing code, or sharing it
with someone who enjoys worldbuilding.

## About Daena

Daena began as a personal attempt to organize the worlds created during
daydreams and sleepless nights. When I failed to find an application that suited
all my needs, I decided to create my own app.

Daena is free and open source under the [Apache License 2.0](LICENSE).
