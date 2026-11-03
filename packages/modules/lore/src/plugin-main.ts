import "./plugin.css";
import manifestJson from "../manifest.json";
import { applyPluginAppearance } from "../../../plugin-sdk/src/theme";
import { createBrowserPluginRpcTransport, createPluginRpcClient } from "../../../plugin-sdk/src/index";
import type { ModuleManifest } from "../../../module-api/src/index";
import { mountLoreGraph } from "./index";
import { createLoreGraphSource } from "./projection-source";

const manifest = manifestJson as unknown as ModuleManifest;
const root = document.querySelector<HTMLElement>("#lore-graph-root");
const status = document.querySelector<HTMLElement>("#lore-status");

async function start() {
  if (!root) throw new Error("World graph root is missing");
  const client = createPluginRpcClient(createBrowserPluginRpcTransport());
  const bootstrap = await client.bootstrap();
  applyPluginAppearance(bootstrap.appearance, document.documentElement);
  const source = await createLoreGraphSource(client, manifest);
  source.reportSurfaceMeta = (meta) => {
    if (status) status.textContent = meta.subtitle;
  };
  mountLoreGraph(root, source);
}

start().catch((cause) => {
  const message = cause instanceof Error ? cause.message : String(cause);
  if (status) status.textContent = message;
  else if (root) root.textContent = message;
});
