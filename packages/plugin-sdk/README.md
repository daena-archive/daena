# `@worldbuilder/plugin-sdk`

Framework-neutral TypeScript SDK for Worldbuilder's brokered plugin API.

```ts
import { createPluginRpcClient } from "@worldbuilder/plugin-sdk";

const client = createPluginRpcClient(transportProvidedByTheHost);
const bootstrap = await client.bootstrap();
const entities = await client.listEntities();
```

The package publishes compiled ESM in `dist/` with declaration files. Runtime
identity, grants, storage ownership, and resource policy remain host-owned.
