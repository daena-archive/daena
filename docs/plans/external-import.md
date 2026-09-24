# External import remainder

The import product is in [`EXTERNAL_IMPORT_SYSTEM.md`](../import/EXTERNAL_IMPORT_SYSTEM.md). Built-in Markdown, text, HTML, DOCX, folders, ZIP, Obsidian, and MediaWiki import are shipped. ODT and RTF are not part of the product.

## Plugin importer ecosystem

Add importer declarations to the canonical Rust plugin contract, generate JSON Schema and TypeScript SDK types, implement broker discovery and bounded opaque source reads, and add plugin test-host conformance fixtures. Importer availability follows enabled service/capability state.

**Exit gate:** a test plugin can detect and analyze a new format into the neutral contract without storage knowledge or project-write authority; malformed, oversized, timed-out, disabled, and revoked providers fail closed; bundled and plugin output pass identical core validation.
