# ADR 0005: FMG provider boundary and binary transport

- Status: Accepted
- Date: 2026-08-06

## Context

FMG is a capable but evolving browser application. Its save format and runtime
model must remain provider-owned, while Daena must preserve map identity and
semantic links without giving a sandboxed webview filesystem, network, Tauri,
or unrestricted host access.

## Decision

FMG will run only as a pinned, locally built static tree inside the sandboxed
Maps child webview. A versioned Maps adapter will be the sole caller of FMG
internals. Daena core communicates with it using provider-neutral intents and
will eventually offer session-bound, revision-checked streaming asset handles
for source bytes. The adapter, not the host, translates selectors and viewport
operations.

FMG source bytes remain opaque native assets. Daena-owned entities, links,
layers, dates, and relationships remain canonical text/JSON fields and are
never embedded in a `.map` file. Provider feature IDs are best-effort selectors
with fallback geometry; an invalid selector is unresolved, never silently
retargeted.

Each upstream update must pin a commit, retain the MIT notice, rebuild from the
locked inputs, run adapter fixtures, and review a patch ledger. A patch may
only establish the narrow wrapper boundary or disable incompatible browser
paths; it may not make Daena depend on undocumented globals.

## Consequences

This isolates provider churn and protects the project boundary, but adds a
maintained adapter, reproducible-build cache, packaging size, and packaged
Tauri acceptance tests. The Phase 0 spike has not yet selected a patch or
changed any public contract.
