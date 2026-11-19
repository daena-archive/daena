# `@daena-archive/plugin-sdk`

Use the definitive [Daena Archive plugin guide](../../docs/plugins/PLUGIN_SDK.md) for
the SDK contract, broker client, capabilities, migrations, testing, and
packaging workflow.

This package publishes compiled ESM and declaration files for that contract.

### Display language

Use `createPluginI18n()` to translate plugin UI with `t(source, values?)` and observe live language changes with `subscribe(listener)`. Put the plugin's dictionaries in `locales/daena/<language>.json` at the package root. For example, `{"Date Lens":"日付レンズ"}` in `locales/daena/ja.json` also translates the matching manifest name in Daena's menus. [Authoring guide and example](../../docs/plugins/PLUGIN_SDK.md#localizing-a-plugin).
