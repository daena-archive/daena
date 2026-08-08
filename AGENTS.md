# Daena Archive contributor guidance

## Working agreement

- Inspect existing guidance and the current worktree before changing code. Preserve unrelated user-authored changes.
- Plan the intended vertical slice before editing; keep changes scoped to the requested phase.
- Do not stage, commit, or push unless the user explicitly asks.
- Use `rtk` to run shell commands. For example: `rtk git status`, `rtk cargo test`, and `rtk npm run build`.
- Use explicit Cargo manifests. This repository has no root `Cargo.toml`; Rust checks normally use `--manifest-path src-tauri/Cargo.toml --locked --offline` when dependencies are cached.
- This is a Tauri desktop project. Browser automation cannot interact with the native app; validate desktop behavior through appropriate Tauri-native or rendered checks instead.

## Architecture authorities

- `docs/ARCHITECTURE.md` defines the product and current architecture.
- `docs/PLUGIN_PLATFORM_PLAN.md` defines plugin-platform acceptance criteria.
- `docs/PLAIN_TEXT_STORAGE_PLAN.md` defines canonical project-storage behavior.
- Project files are canonical. `.daena/index.sqlite` is disposable derived state and must be rebuildable from the canonical files.

## Code discovery

This project uses codebase-memory-mcp to maintain a knowledge graph of the codebase.

Always prefer MCP graph tools over grep, glob, or file search for code discovery:

1. `search_graph` — find functions, classes, routes, and variables by pattern.
2. `trace_path` — trace callers and callees.
3. `get_code_snippet` — read a specific function or class.
4. `query_graph` — run Cypher queries for complex patterns.
5. `get_architecture` — obtain a high-level project summary.

Run `index_repository` first if the project is not indexed. Fall back to `rg` only for string literals, error messages, configuration values, non-code files, or when graph results are insufficient.

## Verification

- Validate changes in proportion to their risk with focused tests plus the relevant build/type checks.
- For storage work, verify canonical round trips and recovery after deleting `.daena/`; account for external edits and malformed paths where applicable.
- Passing unit tests alone do not prove rendered UI behavior, plugin-webview lifecycle, or persistence/recovery behavior. Exercise the relevant boundary directly.
