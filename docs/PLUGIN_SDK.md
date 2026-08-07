# Daena Archive plugins: the definitive authoring guide

This is the canonical guide for creating, testing, packaging, installing, and
maintaining Daena Archive plugins. The short package READMEs and the Plugins
panel are entry points only; use this document for the complete contract.

The public platform is provided by:

- `@daena-archive/plugin-sdk` — generated contract types, broker client, and
  migration helpers;
- `@daena-archive/plugin-cli` — the `daena-plugin` authoring CLI; and
- `@daena-archive/plugin-test-host` — an in-memory broker and lifecycle test
  host.

The normative contract is in [`schemas/plugin-manifest-v1.json`](../schemas/plugin-manifest-v1.json), [`schemas/plugin-rpc-v1.json`](../schemas/plugin-rpc-v1.json), and [`schemas/capability-registry-v1.json`](../schemas/capability-registry-v1.json). The architecture and security decisions are in [`PLUGIN_PLATFORM_PLAN.md`](PLUGIN_PLATFORM_PLAN.md).

## 1. Platform model

A plugin is an app-owned `.wbplugin` ZIP package. The package contains one
canonical `manifest.json` and the files referenced by its entrypoints. Plugin
identity, project binding, capability grants, storage ownership, and runtime
authority belong to the host—not to plugin JavaScript or WASM.

There are two author-facing plugin kinds:

- `declarative` contributes schemas, templates, views, commands, events, and
  services as manifest data. It has no arbitrary plugin runtime logic.
- `sandboxed` may include an isolated UI bundle and/or a background Wasm
  component. It communicates only through the brokered SDK API.

Third-party code never runs in the main application webview. Plugin UI has no
Tauri API, host DOM, filesystem, shell, process, dialog, clipboard, or ambient
network access. Background Wasm has no WASI imports or preopened directories;
host capabilities are the only authority.

## 2. Install the authoring tools

Inside this repository, use the checked-in CLI wrapper:

```sh
node scripts/plugin-cli.mjs --help
```

After the packages are published, install the CLI and SDK in an external
plugin project:

```sh
npm install --save-dev @daena-archive/plugin-cli
npm install @daena-archive/plugin-sdk
npm install --save-dev @daena-archive/plugin-test-host
```

Then invoke the CLI as:

```sh
npx daena-plugin --help
```

The repository's build and verification commands are:

```sh
npm run build:plugin-sdk
npm run build:plugin-examples
npm run test:plugin-cli
npm run test:plugin-conformance
npm run test:plugin-transport
npm run check
```

## 3. Create a plugin

Scaffold a valid package:

```sh
node scripts/plugin-cli.mjs init my-plugin \
  --id com.example.my-plugin \
  --name "My Plugin"
```

The generated layout is:

```text
my-plugin/
├── manifest.json
└── dist/
    └── ui/
        ├── index.html
        └── index.js
```

The same command is available as `npx daena-plugin init` in an external
project. Replace the generated UI with your built static bundle, then update
the manifest with the contributions and capabilities the plugin actually
needs.

## 4. Use an existing plugin

An existing plugin is any directory with a root `manifest.json` and all files
referenced by `entrypoints.ui` or `entrypoints.wasm`. It does not need to live
inside the Daena Archive monorepo.

Validate it before packaging:

```sh
node scripts/plugin-cli.mjs validate path/to/my-plugin
node scripts/plugin-cli.mjs migration validate path/to/my-plugin
```

The validator checks the manifest contract, declared ownership, migrations,
entrypoint files, package-relative paths, archive limits, and unsafe package
tree entries. The Rust host repeats security-sensitive validation before
installation.

## 5. The manifest

Every manifest v1 contains all of these top-level keys. Unknown keys are
rejected.

```json
{
  "manifestVersion": 1,
  "id": "com.example.my-plugin",
  "name": "My Plugin",
  "version": "1.0.0",
  "publisher": "com.example",
  "hostApi": ">=1.0.0 <2.0.0",
  "kind": "sandboxed",
  "entrypoints": { "ui": "dist/ui/index.html" },
  "capabilities": ["entity.read"],
  "dependencies": {},
  "namespaces": [],
  "schemas": [],
  "templates": [],
  "views": [],
  "commands": [],
  "services": { "provides": [], "consumes": [] },
  "events": { "publishes": [], "subscribes": [] },
  "migrations": []
}
```

Important rules:

- `id` and `publisher` are lowercase reverse-domain identifiers and are
  immutable after publication.
- `version` uses Semantic Versioning. `hostApi` is a host API version range.
- `entrypoints` must use package-relative paths. They may not be absolute,
  contain `..`, or contain Windows separators.
- `kind` is `declarative` or `sandboxed`.
- Every namespace has exactly one owning plugin. Schema and migration
  namespaces must be listed in `namespaces`.
- IDs for templates, views, commands, services, events, and migrations must
  be unique within the package.
- A package digest covers the manifest and every packaged file. Enabled state
  and grants are host/project state, not manifest state.

### Schemas, fields, and templates

Schemas declare entity types and fields in an owned namespace:

```json
{
  "namespaces": ["weather"],
  "schemas": [{
    "namespace": "weather",
    "entityTypes": ["forecast"],
    "fields": [
      { "key": "summary", "label": "Summary", "type": "text" },
      { "key": "temperature", "label": "Temperature", "type": "number" },
      { "key": "season", "label": "Season", "type": "enum", "options": ["spring", "summer", "autumn", "winter"] }
    ]
  }]
}
```

Supported field types are `text`, `number`, `boolean`, `date`, `enum`,
`entity-ref`, and `relationship`. Relationship fields must declare a
`relationshipType` and non-empty `targetEntityTypes`.

Templates provide initial values for declared fields and may include an
opening document. They cannot introduce fields or entity types that the
schema does not declare.

### Host-rendered views

Both bundled and third-party plugins may declare a host-rendered view. The
view is a JSON component tree; it is not executable HTML or JavaScript. The
same host component allowlist and Rust authorization rules apply to every
plugin.

```json
{
  "views": [{
    "id": "notes",
    "title": "Field Notes",
    "components": [
      { "type": "heading", "id": "intro", "text": "Field Notes" },
      { "type": "text", "id": "help", "text": "Review notes captured in this project." },
      { "type": "entity-list", "id": "recent", "title": "Recent notes", "entityType": "note", "limit": 10 },
      { "type": "entity-detail", "id": "selected", "title": "Selected note", "source": "recent" },
      { "type": "field-form", "id": "note-fields", "title": "Note color", "source": "recent", "namespace": "field-notes", "fields": ["color"], "editable": true },
      { "type": "button", "id": "refresh", "label": "Refresh", "command": "refresh" }
    ]
  }],
  "commands": [{
    "id": "refresh",
    "title": "Refresh",
    "action": { "type": "refresh-view" },
    "input": { "type": "object", "properties": {}, "required": [], "additionalProperties": false },
    "output": { "type": "object", "properties": { "type": { "type": "string" } }, "required": ["type"], "additionalProperties": false },
    "exposure": ["view"]
  }]
}
```

The component set is deliberately small: `heading`, `text`, `entity-list`,
`entity-detail`, `field-form`, and `button`. An entity list is the selection
source for detail/forms and may reference only an entity type declared by the
plugin; it requires `entity.read`. A detail component shows the selected
entity. A field form may reference only fields declared in an owned namespace,
requires `field.read:self`, and requires `field.write:self` when editable. A
button can invoke only a declared host action; the current action is
`refresh-view`.

The host fetches data and applies field writes only after checking the active
runtime, current project grant, source entity type, namespace, and manifest
field declaration. Plugins do not receive a DOM handle, callback, or arbitrary
component escape hatch. Commands default to `view` exposure when `exposure` is
omitted for compatibility. Phase 3 supports `view` exposure for host-rendered
buttons and `broker` exposure for host-routed invocations; menu and keyboard
shortcut surfaces are intentionally not part of this contract yet. Commands
may declare a small object-shaped input/output schema and required
capabilities. Rust validates the declared schema,
exposure, capability grants, and payload before invoking the host-owned action;
unknown input properties are rejected when `additionalProperties` is false.

User-facing manifest views appear in the host sidebar after the plugin is
enabled. Host-rendered views use the shared host renderer; sandboxed views use
an isolated child webview embedded in the workspace. The child webview keeps
its own plugin origin, CSP, initialization policy, and broker session; plugin
JavaScript still never runs in the host DOM. The plugin library remains the
management surface for installation, consent, upgrades, rollback, and
disablement. Built-in Lore, Timeline, and Writing navigation is also host-owned,
so it is not duplicated by empty module manifests.

## 6. Capabilities and broker access

Capabilities are deny-by-default. A manifest requests them; the user/project
grant is the authority. A plugin must declare and receive a capability before
the broker permits the operation.

| Capability | Purpose |
| --- | --- |
| `entity.read` | Read visible project entities. |
| `entity.write` | Create and update entities. |
| `entity.delete` | Delete entities; interactive confirmation applies. |
| `document.read` / `document.write` | Read or create/update entity documents. |
| `field.read:self` / `field.write:self` | Read or write fields in owned namespaces. |
| `field.read:shared` | Read fields explicitly shared by another plugin. |
| `relationship.read` / `relationship.write` | Read or create relationships. |
| `asset.read:self` | Read metadata for plugin-owned assets. |
| `asset.register` | Register a plugin-supplied asset into a caller-owned namespace. |
| `search.query` | Query the project search service. |
| `event.publish:<type>` / `event.subscribe:<type>` | Publish or subscribe to declared events. |
| `service.provide:<name>` / `service.call:<name>` | Provide or call declared services. |

There is no generic `filesystem`, `shell`, `process`, `dialog`, `tauri`, or
unrestricted `network` capability. Plugins never receive arbitrary local
paths.

Set `shared: true` on an owned schema field to export it read-only to other
plugins. A reader still needs `field.read:shared`; the owning plugin retains
the only write authority.

## 7. Build a sandboxed plugin

UI assets are static files served in a separate host-created webview. Use the
host-provided broker transport with the framework-neutral SDK:

```ts
import {
  createBrowserPluginRpcTransport,
  createPluginRpcClient,
} from "@daena-archive/plugin-sdk";

const transport = createBrowserPluginRpcTransport();
const client = createPluginRpcClient(transport);
const bootstrap = await client.bootstrap();
const entries = await client.listEntities("forecast");
```

Mutable broker calls are revision-aware. Use the revision returned by a read
and pass it back on the mutation; the SDK transport supplies a request ID for
retry-safe operations.

```ts
const [entry] = await client.listEntities("forecast");
const updated = await client.updateEntity(
  entry.id,
  "Evening forecast",
  "forecast",
  { expectedRevision: entry.revision },
);
```

Entity updates change the entry name/type; document and namespaced field
mutations use the corresponding `document.save` and `field.set` broker
payloads, each with that record's own revision. Stale revisions fail with a
typed `revision-conflict` error. Do not retry a mutation with a newly observed
revision unless the user or plugin has explicitly decided how to merge the
change.

`createBrowserPluginRpcTransport` reads the host-assigned plugin ID from the
webview document and the current project ID from its URL, then performs the
session bootstrap against the same-origin `/__rpc` endpoint. It adds a unique
request ID, serializes the versioned envelope, correlates the response, bounds
payloads, and turns broker failures into `PluginRpcException`. Tests may inject
`pluginId`, `projectId`, `endpoint`, and `fetch`; production plugins should not
replace the transport with a caller-selected identity or a Tauri command.

The bootstrap response contains the host-assigned plugin ID, session, project,
version, API range, grants, and optional features. Do not invent identity or
call Tauri commands directly.

The client exposes typed convenience methods for entity list/create/update/
delete, event publish/subscribe/poll, and service calls. Use `client.call`
only for a method explicitly documented by the RPC contract.

Wasm entrypoints must be compiled binary Wasm and export the host-required
`run` function. They must not import WASI, environment, filesystem, network,
clock, randomness, or process APIs. The example fixture can be regenerated
with:

```sh
npm run build:plugin-examples
```

## 8. Events and services

Declare event and service contracts in the manifest before requesting their
capabilities. Events are asynchronous and at-most-once; subscribers should
re-query core state when they need authoritative data.

```ts
await client.subscribeEvent("com.example.weather/forecast-updated", 1);
await client.publishEvent("com.example.weather/forecast-updated", 1, { entityId });
const events = await client.pollEvents("com.example.weather/forecast-updated", 1);
```

Services are versioned request/response contracts. Only one provider for a
service major is active in a project. Calls have deadlines and return a typed
provider-unavailable error when the provider is missing.

WASM providers use the synchronous `wb.service.sync.v1` ABI. A provider exports
`memory`, `alloc(i32) -> i32`, and `handle_json(i32, i32) -> i64`; the host writes
UTF-8 JSON into allocated memory and decodes the returned `(len << 32) | ptr`
value as UTF-8 JSON. Requests and responses are bounded by the broker payload
limit. Background event loops are not supported; cancellation stops waiting at
the broker boundary and the provider is drained or quarantined during lifecycle
shutdown.

## 9. Migrations

Migrations are declarative, contiguous, and package-owned. Runtime plugin code
cannot submit arbitrary migration JSON.

```ts
import {
  createMigrationOperation,
  migration,
  validateMigrationChain,
} from "@daena-archive/plugin-sdk";

const migrationV2 = migration({
  id: "weather-v2",
  from: 1,
  to: 2,
  recovery: "backup",
  operations: [createMigrationOperation("add-field", "weather", {
    field: { key: "confidence", label: "Confidence", type: "number" },
  })],
});

const errors = validateMigrationChain([migrationV2], ["weather"]);
```

The first migration normally goes from `0` to `1`. Each next migration must
start at the previously stored version. Migrations run transactionally after
the host creates a backup. Disabling or uninstalling code does not delete
plugin data.

Validate the chain from the CLI:

```sh
node scripts/plugin-cli.mjs migration validate path/to/my-plugin
```

## 10. Test with the fake host

Use the test host for deterministic broker and lifecycle tests:

```ts
import { FakePluginHost } from "@daena-archive/plugin-test-host";

const host = new FakePluginHost({
  manifest,
  projectId: "test-project",
  grants: ["entity.read"],
});

const client = host.client();
const bootstrap = await client.bootstrap();
const entities = await client.listEntities();
```

The fake host assigns identity, enforces grants, rejects unknown methods,
supports event/service test doubles, and can revoke sessions. The exported
`FakePluginLifecycleHost` covers install, enable, upgrade, rollback,
uninstall-code, and separate data deletion. It is a contract test double, not
a replacement for packaged Tauri/webview tests.

Run the repository conformance suite:

```sh
npm run test:plugin-conformance
```

This exercises the public examples plus bundled Lore, Timeline, and Writing
manifests.

## 11. Package and install

Package an existing plugin directory:

```sh
node scripts/plugin-cli.mjs validate path/to/my-plugin
node scripts/plugin-cli.mjs migration validate path/to/my-plugin
node scripts/plugin-cli.mjs package path/to/my-plugin \
  --output ./my-plugin.wbplugin
node scripts/plugin-cli.mjs validate ./my-plugin.wbplugin
```

The package CLI performs author-time checks. The Rust host independently
checks archive paths, file limits, collisions, symlinks, manifest references,
digests, signatures, compatibility, and unsigned-package consent before any
plugin code executes.

To install it in Daena Archive:

1. Open the **Plugins** panel.
2. Select **Install package…** and choose the `.wbplugin` file.
3. Review the publisher, digest, requested capabilities, and unsigned-package
   warning if applicable.
4. Confirm installation, then use the plugin's **Enable** action for the
   current project.

Installing a package does not automatically enable it in every project.
Installed code is global to the application profile; enablement, grants,
stored data versions, health, and failure state are project-scoped.

## 12. Upgrade, rollback, and removal

Install the newer `.wbplugin` version with the same immutable plugin ID. In
the Plugins panel:

- **Update** previews capability changes and migration requirements before
  consent and activation.
- **Rollback** restores the selected previous code version and the pre-upgrade
  project backup when required.
- **Uninstall code** removes a retained package version. If it is selected by
  the current project, disable the plugin first; uninstall then detaches that
  code version while preserving plugin-owned project data. Code selected by
  another project remains protected until that project is detached.
- **Delete project data** is separate, explicit, and destructive; disabling or
  uninstalling code preserves plugin-owned project data by default.
- **Retry** is available for a quarantined plugin after repeated failures.

Upgrades may add optional APIs in a host minor release. A plugin's declared
`hostApi` range must intersect the host's supported API range. Stored data
version, plugin version, and host API version are independent.

## 13. Compatibility and security checklist

Before publishing a plugin, verify:

- all manifest keys and nested objects match v1 exactly;
- the plugin ID, publisher, version, and host API range are valid;
- every capability is necessary, declared, and tested with both allow and deny
  cases;
- every namespace, schema, template, relationship, service, event, and
  migration is owned and declared;
- UI code uses only the broker transport and never imports Tauri APIs;
- Wasm is binary, exports `run`, and has no ambient imports;
- migrations are contiguous and tested against backup/rollback behavior;
- `validate`, migration validation, package creation, and archive validation
  all pass; and
- the packaged `.wbplugin` is tested through the application's Plugins panel.

## 14. Canonical references

This guide is the author-facing source of truth. The following files define the
machine-readable or architectural contract it summarizes:

- [`crates/daena-plugin-api/src/lib.rs`](../crates/daena-plugin-api/src/lib.rs)
  — the Rust manifest types and `validate_manifest`: the enforcement boundary
  and the source of truth for the contract.
- [`crates/daena-plugin-api/src/rpc.rs`](../crates/daena-plugin-api/src/rpc.rs)
  — the Rust RPC payload and envelope types.
- [`crates/daena-plugin-api/src/catalog.rs`](../crates/daena-plugin-api/src/catalog.rs)
  — the RPC method catalog: methods, payload shapes, revision and capability
  requirements.
- [`schemas/plugin-manifest-v1.json`](../schemas/plugin-manifest-v1.json)
- [`schemas/plugin-rpc-v1.json`](../schemas/plugin-rpc-v1.json)
- [`schemas/plugin-error-v1.json`](../schemas/plugin-error-v1.json)
- [`schemas/capability-registry-v1.json`](../schemas/capability-registry-v1.json)
  — the JSON Schemas are generated from the Rust types by
  `npm run gen:plugin-contract`; do not hand-edit them.
- [`packages/plugin-sdk/src/generated.ts`](../packages/plugin-sdk/src/generated.ts)
  — the TypeScript contract types, generated from the schemas.
- [`packages/plugin-sdk/src/index.ts`](../packages/plugin-sdk/src/index.ts) —
  `validatePluginManifest`, the TypeScript rule validator kept in parity with
  the Rust validator by the dual-validator conformance test
  (`npm run test:plugin-conformance`).
- [`schemas/fixtures/manifest/`](../schemas/fixtures/manifest/) — the shared
  fixture battery the conformance and structural checks run against.
- [`PLUGIN_PLATFORM_PLAN.md`](PLUGIN_PLATFORM_PLAN.md)
- [`adr/0006-rust-first-contract-generation.md`](adr/0006-rust-first-contract-generation.md)
  — the decision that Rust owns the contract and schemas/TS are generated
  artifacts.
