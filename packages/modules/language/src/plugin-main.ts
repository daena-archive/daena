import "./plugin.css";
import { flushSync, mount } from "svelte";
import manifestJson from "../manifest.json";
import { createBrowserPluginRpcTransport, createPluginRpcClient, onHostMessage } from "../../../plugin-sdk/src/index";
import { applyPluginAppearance } from "../../../plugin-sdk/src/theme";
import { createBrokerModuleContext, type ModuleContext, type ModuleManifest, type UUID } from "../../../module-api/src/index";
import { setDefaultEditorHost } from "../../../../src/lib/editor/host";
import LanguageWorkspace from "./LanguageWorkspace.svelte";
import { createBrokerEditorHost } from "./plugin-editor-host.ts";
import {
  createLanguageStatePublisher,
  paneStatePayload,
  parseLanguageHostMessage,
  parseLanguageMountParams,
  pickLanguageManifest,
  type LanguageWorkspaceBridge,
} from "./plugin-protocol.ts";

/** Only used when schema.list does not list Language; see pickLanguageManifest. */
const bundledManifest = manifestJson as unknown as ModuleManifest;
const root = document.querySelector<HTMLElement>("#language-root");

async function start(target: HTMLElement): Promise<void> {
  const transport = createBrowserPluginRpcTransport();
  const client = createPluginRpcClient(transport);
  const bootstrap = await client.bootstrap();
  applyPluginAppearance(bootstrap.appearance, document.documentElement);

  // The effective manifest carries qualified entity type ids (`daena.language:language`)
  // and the project's schema overlay, which is what stored entities match against.
  const manifests = await client.call<ModuleManifest[]>("schema.list", {});
  const manifest = pickLanguageManifest(manifests, bundledManifest);

  const launch = parseLanguageMountParams(location.search);
  const entityId = launch.entityId;
  const publish = createLanguageStatePublisher(client, (cause) => {
    console.warn("Language state could not be published", cause);
  });

  setDefaultEditorHost(
    createBrokerEditorHost({
      rpc: client,
      fetch: (url) => window.fetch(url),
      openLink: (href) => void publish({ kind: "open-link", href }),
    }),
  );

  const bridge: LanguageWorkspaceBridge = {
    onPaneRequest: (handler) =>
      onHostMessage((message) => {
        const parsed = parseLanguageHostMessage(message);
        if (parsed?.type === "language.pane") handler(parsed.pane);
      }),
    onLeaveRequest: (handler) =>
      onHostMessage((message) => {
        const parsed = parseLanguageHostMessage(message);
        if (parsed?.type === "language.can-leave") handler(parsed.requestId);
      }),
    reportLeave: (message) => void publish(message),
  };

  const base = createBrokerModuleContext(manifest, transport, {
    focusEntityId: entityId ? (entityId as UUID) : undefined,
    moduleState: launch.moduleState,
    onModuleStateChange: (state) => {
      const payload = paneStatePayload(state);
      if (payload) void publish(payload);
    },
    onEntityDeleted: () => {
      // The broker reports a delete without its id; Language only ever deletes the entity it was launched for.
      if (entityId) void publish({ kind: "entity-changed", entityId, deleted: true });
    },
  });
  const context: ModuleContext = {
    ...base,
    entities: {
      ...base.entities,
      update: async (id, patch, options) => {
        const record = await base.entities.update(id, patch, options);
        void publish({ kind: "entity-changed", entityId: id, deleted: false });
        return record;
      },
    },
  };

  mount(LanguageWorkspace, { target, props: { context, bridge } });
  flushSync();
  void publish({ kind: "ready", pane: launch.pane, entityId });
}

if (root) {
  start(root).catch((cause) => {
    const message = cause instanceof Error ? cause.message : String(cause);
    console.error("Language failed to start", cause);
    root.textContent = message;
  });
} else {
  console.error("Language root is missing");
}
