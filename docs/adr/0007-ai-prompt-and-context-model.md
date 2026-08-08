# ADR 0007: Prompt layers and project context are distinct

- Status: Accepted
- Date: 2026-08-08

## Decision

Requests keep host policy, authorized plugin guidance, the user instruction,
immediate context, retrieved project evidence, and the output contract as
separate layers. Project text, fields, filenames, and provider output are
untrusted data, never host instructions. Context is bounded, delimited, and
provenance-bearing. Models receive no tools, credentials, filesystem access,
network authority, or mutation path.

The initial prompt template identifier is `ai.prompt.v1`; templates are
application code and not canonical project content.

