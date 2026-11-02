import "vis-timeline/styles/vis-timeline-graph2d.min.css";
import "./plugin.css";
import manifestJson from "../manifest.json";
import { applyPluginAppearance } from "../../../plugin-sdk/src/theme";
import { createBrowserPluginRpcTransport, createPluginRpcClient } from "../../../plugin-sdk/src/index";
import type { ModuleManifest } from "../../../module-api/src/index";
import { mountTimeline } from "./index";
import { createTimelineProjectionSource } from "./projection-source";

const manifest = manifestJson as unknown as ModuleManifest;
const root = document.querySelector<HTMLElement>("#chronology");
const status = document.querySelector<HTMLElement>("#chronology-status");

async function start() {
  if (!root) throw new Error("Chronology root is missing");
  const client = createPluginRpcClient(createBrowserPluginRpcTransport());
  const bootstrap = await client.bootstrap();
  applyPluginAppearance(bootstrap.appearance, document.documentElement);
  const source = await createTimelineProjectionSource(client, manifest);
  source.reportSurfaceMeta = (meta) => {
    if (status) status.textContent = meta.subtitle;
  };
  mountTimeline(root, source);
}

start().catch((cause) => {
  const message = cause instanceof Error ? cause.message : String(cause);
  if (status) status.textContent = message;
  else if (root) root.textContent = message;
});
