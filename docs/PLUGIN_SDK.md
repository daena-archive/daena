# Worldbuilder plugins: the definitive authoring guide

This is the canonical guide for creating, testing, packaging, installing, and
maintaining Worldbuilder plugins. The short package READMEs and the Plugins
panel are entry points only; use this document for the complete contract.

The public platform is provided by:

- `@worldbuilder/plugin-sdk` — generated contract types, broker client, and
  migration helpers;
- `@worldbuilder/plugin-cli` — the `worldbuilder-plugin` authoring CLI; and
- `@worldbuilder/plugin-test-host` — an in-memory broker and lifecycle test
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
npm install --save-dev @worldbuilder/plugin-cli
npm install @worldbuilder/plugin-sdk
npm install --save-dev @worldbuilder/plugin-test-host
```

Then invoke the CLI as:

```sh
npx worldbuilder-plugin --help
```

The repository's build and verification commands are:

```sh
npm run build:plugin-sdk
npm run build:plugin-examples
npm run test:plugin-cli
npm run test:plugin-conformance
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

The same command is available as `npx worldbuilder-plugin init` in an external
project. Replace the generated UI with your built static bundle, then update
the manifest with the contributions and capabilities the plugin actually
needs.

## 4. Use an existing plugin

An existing plugin is any directory with a root `manifest.json` and all files
referenced by `entrypoints.ui` or `entrypoints.wasm`. It does not need to live
inside the Worldbuilder monorepo.

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
| `asset.import` | Ask the host to import a user-selected file. |
| `search.query` | Query the project search service. |
| `event.publish:<type>` / `event.subscribe:<type>` | Publish or subscribe to declared events. |
| `service.provide:<name>` / `service.call:<name>` | Provide or call declared services. |

There is no generic `filesystem`, `shell`, `process`, `dialog`, `tauri`, or
unrestricted `network` capability. Plugins never receive arbitrary local
paths.

## 7. Build a sandboxed plugin

UI assets are static files served in a separate host-created webview. Use the
host-provided broker transport with the framework-neutral SDK:

```ts
import { createPluginRpcClient } from "@worldbuilder/plugin-sdk";

const client = createPluginRpcClient(transportProvidedByTheHost);
const bootstrap = await client.bootstrap();
const entries = await client.listEntities("forecast");
```

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

## 9. Migrations

Migrations are declarative, contiguous, and package-owned. Runtime plugin code
cannot submit arbitrary migration JSON.

```ts
import {
  createMigrationOperation,
  migration,
  validateMigrationChain,
} from "@worldbuilder/plugin-sdk";

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
import { FakePluginHost } from "@worldbuilder/plugin-test-host";

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

To install it in Worldbuilder:

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
- **Uninstall code** removes a retained package version only when it is not
  selected by an active project.
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

- [`schemas/plugin-manifest-v1.json`](../schemas/plugin-manifest-v1.json)
- [`schemas/plugin-rpc-v1.json`](../schemas/plugin-rpc-v1.json)
- [`schemas/plugin-error-v1.json`](../schemas/plugin-error-v1.json)
- [`schemas/capability-registry-v1.json`](../schemas/capability-registry-v1.json)
- [`PLUGIN_PLATFORM_PLAN.md`](PLUGIN_PLATFORM_PLAN.md)
- [`packages/plugin-sdk/src/generated.ts`](../packages/plugin-sdk/src/generated.ts)
