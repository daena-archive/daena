# Plugin author examples

Each directory is a package that can be validated and packaged independently
of the Worldbuilder checkout:

```sh
npx worldbuilder-plugin validate examples/plugins/ui
npx worldbuilder-plugin package examples/plugins/ui --output ink-tools.wbplugin
```

`declarative` contains only manifest contributions and host-rendered static UI.
`ui` demonstrates a sandboxed static bundle. `wasm-service` contains a minimal
executable Wasm service fixture. Regenerate it with
`npm run build:plugin-examples` when the example source changes.
