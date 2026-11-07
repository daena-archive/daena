import "./plugin.css";
import { flushSync, mount } from "svelte";
import manifestJson from "../manifest.json";
import {
  createBrowserPluginRpcTransport,
  createPluginRpcClient,
  onHostMessage,
  transferText,
  uploadAssetChunks,
} from "../../../plugin-sdk/src/index";
import { applyPluginAppearance } from "../../../plugin-sdk/src/theme";
import type { ModuleManifest } from "../../../module-api/src/index";
import { setDefaultEditorHost } from "../../../../src/lib/editor/host";
import { createBrokerEditorHost } from "../../../../src/lib/editor/broker-host.ts";
import WritingWorkspace from "./WritingWorkspace.svelte";
import {
  createWritingStatePublisher,
  parseWritingHostMessage,
  parseWritingMountParams,
  pickWritingManifest,
  WRITING_RPC_MAX_RESPONSE_BYTES,
  type WritingWorkspaceBridge,
} from "./plugin-protocol.ts";

/** Only used when schema.list does not list Writing; see pickWritingManifest. */
const bundledManifest = manifestJson as unknown as ModuleManifest;
const root = document.querySelector<HTMLElement>("#writing-root");

async function start(target: HTMLElement): Promise<void> {
  // Long documents arrive in one `document.list` answer, past the SDK's 256 KiB default.
  const transport = createBrowserPluginRpcTransport({ maxResponseBytes: WRITING_RPC_MAX_RESPONSE_BYTES });
  const client = createPluginRpcClient(transport);
  const bootstrap = await client.bootstrap();
  applyPluginAppearance(bootstrap.appearance, document.documentElement);

  // The effective manifest carries qualified entity type ids (`daena.writing:manuscript`) and the project's
  // schema overlay, which is what stored entities match against.
  const manifests = await client.call<ModuleManifest[]>("schema.list", {});
  const manifest = pickWritingManifest(manifests, bundledManifest);
  const launch = parseWritingMountParams(location.search);
  const publish = createWritingStatePublisher(client, (cause) => {
    console.warn("Writing state could not be published", cause);
  });

  // The webview stays mounted across entities, so the image fallback reads the entity being shown.
  let shownEntityId = launch.entityId;
  setDefaultEditorHost(
    createBrokerEditorHost({
      rpc: client,
      fetch: (url) => window.fetch(url),
      openLink: (href) => void publish({ kind: "open-link", href }),
      openEntity: (entityId, options) =>
        void publish(
          options?.newTab ? { kind: "open-entity", entityId, newTab: true } : { kind: "open-entity", entityId },
        ),
      namespace: manifest.schemas[0]?.namespace ?? "writing",
      launchEntityId: () => shownEntityId,
      uploadChunks: (transfer, bytes) => uploadAssetChunks(transfer, bytes),
      // The shell is the only writer; it refreshes an entity after its own saves.
      onEntityChanged: () => undefined,
      document,
      crypto,
    }),
  );

  const bridge: WritingWorkspaceBridge = {
    onHostMessage: (handler) =>
      onHostMessage((message) => {
        const parsed = parseWritingHostMessage(message);
        if (!parsed) return;
        if (parsed.type === "writing.load") shownEntityId = parsed.entityId;
        handler(parsed);
      }),
    publish,
    transferText: (text) => transferText(client, text),
  };

  mount(WritingWorkspace, { target, props: { rpc: client, bridge, initialEntityId: launch.entityId } });
  flushSync();
  void publish({ kind: "ready", entityId: launch.entityId });
}

if (root) {
  start(root).catch((cause) => {
    const message = cause instanceof Error ? cause.message : String(cause);
    console.error("Writing failed to start", cause);
    root.textContent = message;
  });
} else {
  console.error("Writing root is missing");
}
