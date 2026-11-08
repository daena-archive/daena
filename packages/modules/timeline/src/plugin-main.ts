import "vis-timeline/styles/vis-timeline-graph2d.min.css";
import "./plugin.css";
import manifestJson from "../manifest.json";
import { applyPluginAppearance } from "../../../plugin-sdk/src/theme";
import {
  createBrowserPluginRpcTransport,
  createPluginRpcClient,
  type PluginRpcClient,
} from "../../../plugin-sdk/src/index";
import type { ModuleManifest } from "../../../module-api/src/index";
import { mountTimeline } from "./index";
import { createTimelineProjectionSource } from "./projection-source";
import { tabShortcutForKey } from "./tab-shortcuts";

const TIMELINE_SHORTCUT_EVENT_TYPE = "daena.timeline/shortcut@1";

function isTextInput(target: EventTarget | null): boolean {
  return (
    typeof Element !== "undefined" &&
    target instanceof Element &&
    target.closest('input, textarea, select, [contenteditable]:not([contenteditable="false"])') !== null
  );
}

/** Unmodified typing, and the usual edit chords, stay in the field. Tab chords are not these keys. */
function isPlainEditingKey(event: KeyboardEvent): boolean {
  if (event.altKey) return false;
  if (!event.metaKey && !event.ctrlKey) return true;
  const key = event.key.toLowerCase();
  return key === "a" || key === "c" || key === "v" || key === "x" || key === "z" || key === "y";
}

function forwardTimelineTabShortcut(client: PluginRpcClient, event: KeyboardEvent) {
  if (event.isComposing) return;
  if (isTextInput(event.target) && isPlainEditingKey(event)) return;
  const shortcut = tabShortcutForKey(
    {
      key: event.key,
      code: event.code,
      metaKey: event.metaKey,
      ctrlKey: event.ctrlKey,
      altKey: event.altKey,
      shiftKey: event.shiftKey,
      repeat: event.repeat,
    },
    typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform),
  );
  if (shortcut === null) return;
  event.preventDefault();
  event.stopPropagation();
  void client
    .call("event.publish", { type: TIMELINE_SHORTCUT_EVENT_TYPE, payload: { shortcut } })
    .catch((cause: unknown) => {
      console.warn("Chronology shortcut could not be published", cause);
    });
}

const manifest = manifestJson as unknown as ModuleManifest;
const root = document.querySelector<HTMLElement>("#chronology");
const status = document.querySelector<HTMLElement>("#chronology-status");

async function start() {
  if (!root) throw new Error("Chronology root is missing");
  const client = createPluginRpcClient(createBrowserPluginRpcTransport());
  const bootstrap = await client.bootstrap();
  applyPluginAppearance(bootstrap.appearance, document.documentElement);
  // Key events in this webview never reach the shell. Forward only the tab chords.
  window.addEventListener("keydown", (event) => forwardTimelineTabShortcut(client, event), true);
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
