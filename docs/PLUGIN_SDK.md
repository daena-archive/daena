# Worldbuilder plugin SDK

The public plugin surface is the `@worldbuilder/plugin-sdk` package. Plugin
identity is assigned by the host; SDK clients never send a plugin ID to core.
Use the generated declarations and `createPluginRpcClient` for broker calls.

## Authoring flow

```sh
worldbuilder-plugin init my-plugin --id com.example.my-plugin
worldbuilder-plugin validate my-plugin
worldbuilder-plugin migration validate my-plugin
worldbuilder-plugin package my-plugin --output my-plugin.wbplugin
```

The CLI is intentionally usable outside this repository. Validation checks the
manifest, migration chain, package-relative entrypoints, and archive paths.
The Rust host repeats all security-sensitive checks before installation.

After packaging, install the `.wbplugin` from the application's Plugins panel.
The host UI owns unsigned-package consent, project enablement, capability
grants, upgrades, rollback, uninstall-code, and separate project-data deletion.
These actions are also covered by `FakePluginLifecycleHost` in the conformance
suite so author tooling can verify lifecycle assumptions without a Tauri
process.

## Compatibility

`manifestVersion` and RPC contracts are independent of the host application
version. `hostApi` is a SemVer range. Patch releases preserve the contract;
minor releases add optional fields or methods; major releases may remove or
change contracts. Capabilities are denied by default and must be both declared
in the manifest and granted by the user/project.

Stored data versions are advanced only by contiguous declarative migrations.
Use the SDK's `migration`, `createMigrationOperation`, and
`validateMigrationChain` helpers; never submit migration operations from a
runtime RPC request.

## Testing

Use `@worldbuilder/plugin-test-host` for unit tests. It implements the same
transport shape as the broker, assigns a host-owned session, rejects unknown
methods, and enforces grants. `runConformance` provides a small baseline that
all public examples can run before packaging.

```ts
const host = new FakePluginHost({ manifest, grants: ["entity.read"] });
const client = host.client();
const bootstrap = await client.bootstrap();
```

The fake host is not a security substitute for the packaged Tauri application;
it is a deterministic contract and authorization test double.
