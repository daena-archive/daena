# ADR 0001: Plugin isolation and trust levels

- Status: Accepted
- Date: 2026-08-03

## Decision

Daena Archive supports declarative and sandboxed plugins. Arbitrary third-party
JavaScript never runs in the trusted application webview. Sandboxed UI runs in
an application-controlled origin without Tauri APIs, host DOM access, ambient
filesystem, process, clipboard, dialog, shell, or unrestricted network access.
Optional background logic uses WASM with no ambient WASI authority.

The plugin SDK is framework-neutral. Svelte remains a first-party choice, not
part of the public contract. Trusted native extensions are deferred and are not
part of manifest version 1.

## Consequences

The host must provide a brokered message boundary and restrictive CSP before
third-party runtime installation is enabled. First-party modules must use the
same public SDK and cannot rely on their bundled status for extra authority.
