# Daena Archive Development Guide

Daena uses Rust, Tauri 2, Svelte 5, TypeScript, Vite, and Deno.

For plugin development, please refer to the [definitive plugin guide](docs/PLUGIN_SDK.md).

## Prerequisites

* **Deno 2.x** (`deno --version`, tested 2.9.5) — JS/TS runtime, tasks, and `deno task check`
* **Rust 1.85+** (`rustc --version`, tested 1.98) with `cargo`, `clippy`, `rustfmt` — `rustup component add clippy rustfmt` (pin via `rust-toolchain.toml` if present)
* **Node 22+** (optional) — only if running `node scripts/...` directly; `deno task` is canonical
* **Tauri CLI 2** — `cargo install tauri-cli` for `deno task tauri ...`
* **System deps for Tauri 2** — see [Tauri prerequisites](https://tauri.app/start/prerequisites/)
* **Docker** (optional) — only for Windows cross-compile

## Setup

From the repository root, install the JavaScript dependencies:

```bash
deno install --node-modules-dir=auto
```

The repository uses a root Cargo workspace (`Cargo.toml`). Rust commands can run
across the entire workspace or focus on individual crates using `-p <crate>`.

## Run Daena

Run the frontend in a browser (no native APIs):

```bash
deno task dev
```

Run the full desktop application through Tauri (required for filesystem, Git, maps, plugins):

```bash
deno task tauri dev
```

Build the desktop application for the current platform:

```bash
deno task tauri build
```

Use the Tauri desktop app when testing filesystem access, project storage,
dialogs, Git, maps, plugins, or other native behavior. Browser development does
not reproduce those boundaries.

## Checks

Run all checks (type, contracts, plugins, maps):

```bash
deno task check
deno task test
```

Individual groups:

```bash
deno task check        # svelte-kit sync + svelte-check + plugin contract
deno task format:check # prettier --check
deno task build        # vite build

# JS unit / integration
deno task test         # unit + plugins + maps
deno task test:unit    # shell, theme, ai-stream, external-import, language, markdown, timeline, writing-tabs, git-ui
deno task test:plugins # plugin-contract, manifest-fixtures, isolation, conformance, declarative, cli, transport
deno task test:maps    # native-vector, physical, atlas

# Focused
deno task test:markdown
deno task check:writing-tabs
deno task check:timeline-calendar
deno task check:maps:native-vector
deno task check:maps:physical
deno task check:maps:atlas
```

Run `deno task` to list every available task.

### Plugin and SDK checks

```bash
deno task build:plugin-sdk
deno task build:plugin-examples
deno task check:plugin-contract
deno task check:plugin-isolation
deno task check:manifest-fixtures
deno task test:plugin-conformance
deno task test:plugin-declarative
deno task test:plugin-cli
deno task test:plugin-transport
```

## Rust Checks

Run tests across the workspace with the cached dependency set:

```bash
cargo test --workspace --locked --offline
```

To test a specific crate:

```bash
cargo test -p daena-core --locked --offline
cargo test -p daena-ai --locked --offline
cargo test -p daena-atlas --locked --offline
cargo test -p daena-plugin-api --locked --offline
cargo test -p daena-plugin-host --locked --offline
cargo test -p daena-physical --locked --offline
cargo test -p daena --locked --offline
```

Run lint and format checks (strict — warnings deny):

```bash
cargo clippy --workspace --locked --offline --all-targets -- -D warnings
cargo check --workspace --locked --offline
cargo fmt -- --check
```

## Plugin Development

Plugin authors should start with the [definitive plugin guide](docs/PLUGIN_SDK.md).
The checked-in CLI wrapper can be inspected with:

```bash
node scripts/plugin-cli.mjs --help
```

The guide covers scaffolding, manifest validation, packaging, installation,
capabilities, migrations, and the plugin test host.

## Cross-Compile for Windows

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

## Project Documentation

- [Architecture](docs/ARCHITECTURE.md)
- [Storage and recovery](docs/STORAGE.md)
- [AI integration and phase gates](docs/AI_INTEGRATION.md)
- [Language module](docs/LANGUAGE_MODULE.md)
- [Plugin SDK](docs/PLUGIN_SDK.md)
- [Plugin platform plan](docs/PLUGIN_PLATFORM_PLAN.md)
- [Maps, physical worlds, and Atlas](docs/MAPS.md)
- [Git integration](docs/GIT_INTEGRATION.md)

## License

Daena Archive is available under the [Apache License 2.0](LICENSE).
