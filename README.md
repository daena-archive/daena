# Worldbuilder

Worldbuilder is a local-first worldbuilding studio. Each world is stored in a portable project folder:

```text
My World/
├── project.json
├── worldbuilder.sqlite
├── .gitignore
└── assets/
    ├── images/
    ├── videos/
    ├── maps/
    └── files/
```

Open and close project folders from the app. Files attached through the UI are copied into the project and recorded with a SHA-256 hash. Git can be initialized, inspected, and committed from the project menu; JSON snapshots are used internally for backups and recovery.

Plugin authors should start with the [definitive plugin guide](docs/PLUGIN_SDK.md).

## Cross-compile

### Windows

```bash
docker build -f Dockerfile.windows -t worldbuilder-windows-builder .
```

```bash
docker run --rm \
  -v "$PWD:/app" \
  -v worldbuilder-deno-modules:/app/node_modules \
  -v worldbuilder-deno-cache:/root/.cache/deno \
  -w /app \
  worldbuilder-windows-builder \
  deno task --node-modules-dir=auto tauri build \
    --runner cargo-xwin \
    --target x86_64-pc-windows-msvc
```
