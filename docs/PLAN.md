# Open Worldbuilding Studio — Architecture and MVP Plan

## Summary

Build an open-source, local-first authoring studio for fictional worlds and eventual book writing. The first goal is not feature breadth; it is to validate a durable module contract through a small, usable MVP.

The product will use a Rust/Tauri host with a Svelte 5, TypeScript, and Vite frontend. Its worldbuilding modules are first-party TypeScript packages that use the same public contracts future contributors will use.

## Architecture

The public plugin-platform contract and its phased migration plan are defined
in [`PLUGIN_PLATFORM_PLAN.md`](./PLUGIN_PLATFORM_PLAN.md). Bundled modules and
runtime-installable plugins share the broker-backed contract, including
revision-aware mutations and host-owned administration state.

- The core owns project storage, stable IDs, migrations, entity links, assets, search indexing, compatibility backups, and permission-checked native operations.
- A project is a portable directory: `project.json`, entity metadata, Markdown documents, structured JSON, and native assets are canonical; `.daena/index.sqlite` is disposable derived state. Git integration is optional and user-controlled.
- The canonical model is an entity graph with documents: entities have prose documents, optional schema-defined fields, typed relationships, references, and assets.
- Modules add meaning and presentation, not separate databases. A map pin, timeline event, and manuscript reference point to the same entity.
- Module data is namespaced and preserved when a module is disabled or uninstalled. Views and indexes are rebuildable derived data.
- The TypeScript module API is framework-neutral: modules register schemas, routes, commands, views, migrations, and capability requirements; UI views mount into a host element and return a cleanup handle.
- Built-in Git commits expose a typed preflight and exact canonical staging preview. They never stage unrelated work, unresolved merges, or a stale/invalid canonical index. Expanded Settings → Git behavior is defined in [`GIT_INTEGRATION.md`](./GIT_INTEGRATION.md).
- Broker reads expose opaque canonical revisions. Updates, deletes, document saves, field and relationship mutations, and asset registration require the observed revision; retryable mutations retain request IDs across Rust, SDK, bundled modules, and the test host.
- Svelte is the official first-party implementation choice, but modules are not required to expose Svelte components.

## MVP

- Write the architecture specification first, including data primitives, invariants, module lifecycle, migration rules, module manifest, capability model, and versioning policy.
- Implement the core project format and the smallest host API required to create, read, update, link, search, and migrate entities.
- Build a first-party **Lore** module with rich documents, entity types, templates, fields, links, and attachments.
- Build a deliberately different **Timeline** module that contributes event schemas and renders shared entities without owning duplicate data.
- Build a **Writing Studio** module with separate Manuscripts and Reference Pages collections backed by the same shared entity/document primitives.
- Require both modules to use only the public module API; treat any missing capability as a contract-design issue, not an excuse for privileged access.
- Include a small example world to validate relationships, renames, deletes, module disablement, export, and migration behavior.

## Roadmap

1. **Specification laboratory** — model cross-module scenarios and finalize a small version-one contract.
2. **Core and Lore** — ship a private, local-first world bible that is useful on its own.
3. **Timeline validation** — prove the contract supports a contrasting module and cross-module references.
4. **Maps or Boards** — test visual projections and module-owned annotations referencing shared entities.
5. **Runtime extensions** — add verified third-party package installation only after bundled modules and migrations are stable.
6. **Advanced writing workflows and mobile/web targets** — expand Manuscripts and Reference Pages after the shared editor and world model have proven durable.

## Stack

- Rust with Tauri for native application lifecycle, local storage, filesystem access, indexing, permissions, and packaging.
- Svelte 5, TypeScript, and Vite for the desktop frontend.
- SQLite for a disposable local index inside each project directory; canonical project content remains ordinary Markdown, JSON, and native asset files.
- A TypeScript monorepo for the shell, module API, and first-party modules.
- Build-time module registration for the MVP; no marketplace, arbitrary runtime code loading, or cloud sync in the initial release.

## Acceptance Criteria

- A project can create shared entities and attach prose, typed fields, links, and assets.
- Lore, Timeline, and Writing Studio are independently packaged modules and have no private host APIs.
- Writing Studio can create and edit Manuscripts and Reference Pages without duplicating Lore entities or core documents.
- A timeline event can reference a lore entity; renaming that entity preserves the reference.
- Disabling a module hides its views without deleting its project data.
- Module migrations are validated, transactional, versioned, and recoverable.
- The example world exports, imports, and rebuilds derived search and views without data loss.

## Defaults

- Optimize first for private, offline desktop authoring; collaboration, cloud sync, public publishing, and mobile are later concerns.
- Use structured fields as optional schema contributions around a freeform author document, rather than forcing every entry into a rigid template.
- Keep runtime-installable modules out of the MVP; prove the contracts with bundled modules first.
