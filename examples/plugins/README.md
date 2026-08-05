# Plugin examples

See the [definitive Worldbuilder plugin guide](../../docs/PLUGIN_SDK.md) for
the complete authoring and packaging workflow. These examples correspond to
the guide's host-rendered declarative, sandboxed UI, and executable Wasm
sections. The declarative Field Notes example opens a real host-component
surface from the host sidebar after the plugin is enabled. Ink Tools is a
richer standalone sandboxed UI example: it demonstrates structured layout,
local interaction, responsive styling, and visible isolation boundaries
without using Tauri APIs or host DOM access. It also reads a small project
entry summary through the broker and stores scratch notes in its owned
`ink-tools` namespace.
