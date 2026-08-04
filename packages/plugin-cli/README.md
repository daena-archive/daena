# `@worldbuilder/plugin-cli`

The `worldbuilder-plugin` command is the public authoring tool:

```sh
worldbuilder-plugin init my-plugin --id com.example.my-plugin
worldbuilder-plugin validate my-plugin
worldbuilder-plugin package my-plugin --output my-plugin.wbplugin
worldbuilder-plugin migration validate my-plugin
```

Packages are deterministic ZIP archives. Validation happens before packaging
and again before installation by the Rust host.
