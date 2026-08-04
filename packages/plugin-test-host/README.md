# `@worldbuilder/plugin-test-host`

The test host is a deterministic, in-memory implementation of the public
broker transport. Use it in plugin unit tests and call `runConformance` to
check identity attribution, capability denial, and unknown-method behavior.
It intentionally exposes no Tauri APIs, filesystem, network, or host paths.

`FakePluginLifecycleHost` exercises install, enable, upgrade, rollback,
uninstall-code, and separate data deletion semantics without requiring a Tauri
process.
