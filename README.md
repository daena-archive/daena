# Daena Archive

![Daena Archive logo](static/branding/logo.png)

> “Daēnā (Avestan pronunciation: [dʌeːnaː]) is a Zoroastrian concept representing insight and revelation.”
>
> — [Wikipedia](https://en.wikipedia.org/wiki/Daena)

Daena Archive is an offline-first studio for turning scattered ideas into
insight, then turning insight into stories. Everything lives in one portable
project — no separate databases for lore, timelines, or manuscripts — stored
as plain files on your machine, kept useful by the app's graph, search, and
writing tools. It's local-first: your files stay yours, the app works offline,
and it's **free** and fully [**open source**](LICENSE).

## Current status

Daena Archive is in active early development (v0.1.0). The core authoring
workflow and plugin platform are implemented and evolving; this is not yet a
production-stable release.

- **Core authoring**: shared entities, documents, fields, relationships, assets,
  graph views, and full-text search.
- **Modules**: Lore, Timeline, and Writing Studio are enabled by default; Maps
  ships as a bundled **beta** module, disabled by default and enableable in
  **Settings → Plugins**.
- **Plugin platform**: a Rust-authoritative broker with capability grants,
  declarative manifests, and isolated execution paths.
- **AI** (optional, local-first): authoring assistance that runs fully against
  local OpenAI-compatible providers with no account or cloud dependency.
  Remote providers are used only with explicit per-request consent. Includes
  rewrite proposals with preview/diff and retrieval-backed generation with
  disposable semantic indexes.
- **Git**: optional built-in integration with a **Settings → Git** panel.

## What Daena Archive provides

- A durable entity graph for people, places, factions, artifacts, cultures, and
  other shared story concepts.
- Freeform documents, structured fields, typed relationships, search, and asset
  attachments.
- First-party Lore, Timeline, and Writing Studio modules that share the same
  project data instead of maintaining isolated copies.
- A bundled Maps module (beta) for provider-backed maps that stay linked to the
  shared entity model.
- Portable, local project folders with Markdown and JSON canonical data, a
  disposable SQLite index, and optional Git integration.
- A brokered plugin platform with declarative and sandboxed extension support.
- Optional, local-first AI assistance: provider-neutral, offline-capable
  authoring aid that proposes changes without becoming a second data model or
  mutation authority.

## Project format

Each project is a self-contained directory:

```text
My World/
├── project.json
├── entities/
│   └── <entity-id>/
│       ├── entity.json
│       ├── document.md
│       ├── fields/
│       ├── relationships.json
│       └── assets.json
├── plugins/
├── assets/
│   ├── images/
│   ├── videos/
│   ├── maps/
│   └── files/
├── .daena/                 # disposable index and local recovery state
└── .gitignore
```

`project.json`, entity records, and Markdown documents are canonical project
data. `.daena/` contains the disposable local index and recovery state; it can
be deleted and rebuilt without deleting project content. Files attached through
the UI are copied into the project and recorded with a SHA-256 hash. Git can be
initialized, inspected, and committed from the project menu.

## Development

The application uses Rust, Tauri, Svelte 5, TypeScript, Vite, and Deno.

```bash
deno install --node-modules-dir=auto
deno task dev
```

To run the desktop shell:

```bash
deno task tauri dev
```

Useful frontend and plugin checks:

```bash
deno task check
deno task build
deno task check:plugin-contract
deno task check:plugin-isolation
deno task test:plugin-conformance
deno task test:plugin-transport
```

Useful Rust checks:

```bash
cargo test --manifest-path crates/daena-ai/Cargo.toml --locked --offline
cargo test --manifest-path src-tauri/Cargo.toml --locked --offline
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --offline --all-targets -- -D warnings
```

Plugin authors should start with the [definitive plugin guide](docs/PLUGIN_SDK.md).

## Cross-compile for Windows

The repository includes a Docker workflow for building the Windows installer
without installing Windows targets on the host:

```bash
docker build -f Dockerfile.windows -t daena-windows-builder .
docker run --rm \
  -v "$PWD:/app" \
  -v daena-deno-modules:/app/node_modules \
  -v daena-deno-cache:/root/.cache/deno \
  -w /app \
  daena-windows-builder \
  deno task --node-modules-dir=auto tauri build \
    --runner cargo-xwin \
    --target x86_64-pc-windows-msvc
```

## Documentation

- [Architecture](docs/ARCHITECTURE.md)
- [AI integration and phase gates](docs/AI_INTEGRATION.md)
- [Plugin SDK](docs/PLUGIN_SDK.md)
- [Plugin platform plan](docs/PLUGIN_PLATFORM_PLAN.md)
- [Maps integration plan](docs/MAP_INTEGRATION_PLAN.md)
- [Git integration](docs/GIT_INTEGRATION.md)

## License

Daena Archive is available under the [Apache License 2.0](LICENSE).
