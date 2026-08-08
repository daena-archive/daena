# ADR 0006: Provider routing is explicit and privacy-aware

- Status: Accepted
- Date: 2026-08-08

## Decision

Provider profiles are machine-local application state. A request uses an
explicit host-selected provider/model and data-boundary classification. Daena
never silently fails over from local to remote or between remote providers.
Remote disclosure requires host confirmation; credentials are never project,
plugin, webview, WASM, prompt, or log data.

Phase 0 defines conservative limits and normalized provider errors but adds no
live provider or credential store. Local inference remains a later phase.

