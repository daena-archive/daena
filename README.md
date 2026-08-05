# Daena Archive

> “Daēnā (Avestan pronunciation: [dʌeːnaː]) is a Zoroastrian concept representing insight and revelation.”
>
> — [Wikipedia](https://en.wikipedia.org/wiki/Daena)

Daena Archive is an offline-first studio for developing fictional worlds and
writing the books that grow from them. It brings world bibles, timelines,
reference pages, manuscripts, relationships, and assets into one portable
workspace.

The name reflects the product’s purpose: turning scattered ideas into insight,
then turning insight into stories.

## What Daena Archive provides

- A durable entity graph for people, places, factions, artifacts, cultures, and
  other shared story concepts.
- Freeform documents, structured fields, typed relationships, search, and asset
  attachments.
- First-party Lore, Timeline, and Writing Studio modules that share the same
  project data instead of maintaining isolated copies.
- Portable, local project folders with SQLite storage, JSON snapshots for
  recovery, and optional Git integration.
- A brokered plugin platform with declarative and sandboxed extension support.

## Project format

Each project is a self-contained directory:

```text
My World/
├── project.json
├── daena.sqlite
├── .gitignore
└── assets/
    ├── images/
    ├── videos/
    ├── maps/
    └── files/
```

Files attached through the UI are copied into the project and recorded with a
SHA-256 hash. Git can be initialized, inspected, and committed from the
project menu. JSON snapshots are used internally for backups and recovery.

## Development

The application uses Rust, Tauri, Svelte 5, TypeScript, Vite, and Deno.

```bash
deno install --node-modules-dir=auto
deno task dev
```

Useful checks and builds:

```bash
deno task check
deno task build
deno task check:plugin-contract
deno task check:plugin-isolation
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
