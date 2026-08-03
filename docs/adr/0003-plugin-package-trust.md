# ADR 0003: Package integrity and trust

- Status: Accepted
- Date: 2026-08-03

## Decision

Plugin packages use deterministic `.wbplugin` archives. Installation validates
paths, duplicates, links, size limits, the manifest, referenced files, and the
package digest before any code executes. Local unsigned packages are supported
initially but require explicit user confirmation and are clearly marked.

Publisher signatures and a registry are later distribution features. A
registry is never an authorization dependency. Installation, enablement,
capability grants, and project data state remain separate concerns.

## Consequences

The manifest and every packaged file are covered by the digest. Upgrade and
rollback behavior must preserve plugin-owned data by default. Installer work is
deferred until identity, authorization, bundled-plugin conversion, and runtime
isolation are complete.

