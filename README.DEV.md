# Daena Archive Development Guide

Daena uses Rust, Tauri 2, Svelte 5, TypeScript, Vite, and Deno.

For plugin development, please refer [here](docs/PLUGIN_SDK.md).

## Setup

From the repository root, install the JavaScript dependencies:

```bash
deno install --node-modules-dir=auto
```

The repository has no root `Cargo.toml`. Rust commands must point to the
specific manifest they are checking.

## Run Daena

Run the frontend in a browser:

```bash
deno task dev
```

Run the full desktop application through Tauri:

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

## Frontend Checks

Run the main type and contract checks:

```bash
deno task check
deno task format:check
deno task build
```

Run plugin and SDK checks:

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

Useful focused checks include:

```bash
deno task test:markdown
deno task check:writing-tabs
deno task check:timeline-calendar
deno task check:maps:native-vector
deno task check:maps:physical
deno task check:maps:atlas
```

Run `deno task` to list every available task in the repository.

## Rust Checks

Run tests for the core crates and desktop shell with the cached dependency set:

```bash
cargo test --manifest-path crates/daena-core/Cargo.toml --locked --offline
cargo test --manifest-path crates/daena-ai/Cargo.toml --locked --offline
cargo test --manifest-path crates/daena-atlas/Cargo.toml --locked --offline
cargo test --manifest-path crates/daena-plugin-api/Cargo.toml --locked --offline
cargo test --manifest-path crates/daena-plugin-host/Cargo.toml --locked --offline
cargo test --manifest-path src-tauri/Cargo.toml --locked --offline
```

Run the desktop shell's lint checks:

```bash
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --offline --all-targets -- -D warnings
```

The standalone `daena-physical` crate can be checked with:

```bash
cargo test --manifest-path crates/daena-physical-spike/Cargo.toml --locked --offline
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
