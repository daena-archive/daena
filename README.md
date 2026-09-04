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

- **Build a world bible** — Organize characters, places, factions, cultures, and custom entries with documents, structured details, relationships, and reversible archiving.
- **Houses and family trees** — Track families, lineages, and typed relationships with explorable tree views linked to Lore.
- **Connect your ideas** — Link entries bidirectionally with rich relationship details, plus search, hover cards, breadcrumbs, and Quick Open.
- **Write stories alongside your notes** — Draft manuscripts with series → book → chapter outlines and reference pages, with inline `@` mentions to world entries.
- **Keep track of history** — Organize events and eras with Gregorian or custom calendars (including BCE) in a synced chronology view.
- **Create fictional languages** — Build lexicons, sounds, writing systems, grammar, word forms, and glossed examples without linguistics expertise.
- **Make maps part of the story** — Attach entries to offline vector, image-backed, or generated physical maps with Atlas tiles and PNG/SVG/PDF export (beta).
- **Bring in notes you already have** — Import Markdown, text, ZIP, HTML, DOCX, Obsidian vaults, and MediaWiki XML via a review-before-commit pipeline.
- **Make Daena match your world** — Customize types, fields, templates, and relationship metadata per project with live impact previews.
- **Find your way around** — Navigate with Quick Open, breadcrumbs, history, workspace guides, light/dark themes, and Project Center.
- **Optional AI assistance** — Opt-in per-project text/image help (local-first via LM Studio/Ollama/ComfyUI) that proposes editable, cited changes without auto-modifying your project.
- **Keep your work under your control** — Stay offline with portable files, backups, optional Git snapshots, update channels, and rebuildable indexes.
- **Add more through plugins** — Extend with permission-scoped plugins without changing your core project. See [`docs/PLUGIN_SDK.md`](docs/PLUGIN_SDK.md) and [`docs/PLUGIN_PLATFORM_PLAN.md`](docs/PLUGIN_PLATFORM_PLAN.md).

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
