# Daena Archive developer shortcuts.
# Canonical docs: README.DEV.md, docs/ARCHITECTURE.md, docs/storage/STORAGE.md.
# JS tasks run via `deno task` (see package.json scripts).
# Rust commands use the root workspace Cargo.toml.

.DEFAULT_GOAL := help

DENO ?= deno
CARGO ?= cargo
CARGO_FLAGS ?= --locked --offline
DENO_INSTALL_FLAGS ?= --node-modules-dir=auto

.PHONY: help install dev dev-web dev-desktop preview build build-desktop \
	test test-js test-unit test-plugins test-maps test-rust \
	check lint lint-js lint-rust format format-check clippy fmt-check

help: ## Show available commands
	@awk 'BEGIN {FS = ":.*##"} /^[a-zA-Z0-9_.-]+:.*##/ {printf "  \033[36m%-14s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

install: ## Install frontend dependencies
	$(DENO) install $(DENO_INSTALL_FLAGS)

dev: ## Run the full desktop app via Tauri (filesystem, Git, maps, plugins)
	$(DENO) task $(DENO_INSTALL_FLAGS) tauri dev

dev-web: ## Run the frontend in a browser only (no native APIs)
	$(DENO) task dev

dev-desktop: dev ## Alias for dev (full desktop app via Tauri)

preview: ## Preview the production frontend build
	$(DENO) task preview

build: ## Build the frontend bundle
	$(DENO) task build

build-desktop: ## Build the desktop app for the current platform
	$(DENO) task $(DENO_INSTALL_FLAGS) tauri build

test: test-js test-rust ## Run all tests (JS + Rust)

test-js: ## Run the full JS suite (unit + plugins + maps)
	$(DENO) task test

test-unit: ## Run JS unit/integration tests (shell, theme, ai, language, markdown, ...)
	$(DENO) task test:unit

test-plugins: ## Run plugin contract, isolation, conformance, and transport tests
	$(DENO) task test:plugins

test-maps: ## Run maps tests (native-vector, physical, atlas)
	$(DENO) task test:maps

test-rust: ## Run Rust tests across the workspace
	$(CARGO) test --workspace $(CARGO_FLAGS)

check: ## Typecheck frontend + plugin contract (svelte-check)
	$(DENO) task check

lint: lint-js lint-rust ## Run all linters (JS + Rust)

lint-js: check format-check ## Lint JS/TS/Svelte (svelte-check + prettier check)

lint-rust: clippy fmt-check ## Lint Rust (clippy strict + fmt check)

format: ## Format JS/TS/Svelte with prettier
	$(DENO) task format

format-check: ## Check JS/TS/Svelte formatting without writing
	$(DENO) task format:check

clippy: ## Run clippy with warnings denied across the workspace
	$(CARGO) clippy --workspace $(CARGO_FLAGS) --all-targets -- -D warnings

fmt-check: ## Check Rust formatting without writing
	$(CARGO) fmt -- --check
