# Plugin registry remainder

The registry design is in [`PLUGIN_REGISTRY.md`](../plugins/PLUGIN_REGISTRY.md). Host refresh, catalog install, and `daena-plugin keygen` / `sign` already live in this repository.

Still outside this repository:

1. Protect `main` on `plugin-registry` and enable Pages.
2. Optional CI crawl of publisher GitHub Releases to fill `catalog.json` (unsigned assets omitted). Empty catalog is valid.
