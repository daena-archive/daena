# External import remainder

The import product is in [`EXTERNAL_IMPORT_SYSTEM.md`](../import/EXTERNAL_IMPORT_SYSTEM.md). Built-in Markdown, text, HTML, DOCX, folders, ZIP, Obsidian, and MediaWiki import are shipped. ODT and RTF are not part of the product.

## Plugin importer ecosystem

Shipped. Do not re-plan this slice.

`ImporterContribution` is on the Rust manifest and the generated schema and SDK. The host discovers enabled file importers and calls them with opaque bytes and a display name, never a path. Availability requires an active session, that importer's `service.provide:<importer-id>@1` grant, and an active provider owned by that plugin. Output is accepted only through `StagedImport::validate`. Reserved built-in ids cannot be claimed. Malformed, oversized, timed-out, disabled, and revoked providers fail closed. Folder sources are not part of this slice.

The import dialog lists those importers and uses the selected importer's extensions. Host `detect` exists; the shell selects an importer explicitly.
