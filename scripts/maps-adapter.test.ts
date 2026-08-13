import {
  FMG_PROVIDER,
  IMAGE_PROVIDER,
  ImageMapAdapter,
  JsonProviderAdapter,
  MapEditorController,
  UnavailableProviderAdapter,
  diagnosticForAssetState,
  selectProviderAdapter,
  type MapAnchor,
  type MapEditorSession,
  type MapProviderAdapter,
  type ProviderEvent,
} from "../packages/modules/maps/src/adapter.ts";

const fixture = new TextEncoder().encode(
  JSON.stringify({
    features: [{ kind: "burg", id: "42", label: "Old Harbor", point: [0.613, 0.428] }],
  }),
);

Deno.test("Maps adapter fallback fails closed when a provider is unavailable", async () => {
  const adapter = new UnavailableProviderAdapter();
  const diagnostic = adapter.getDiagnostic();
  if (adapter.provider !== FMG_PROVIDER || diagnostic.code !== "provider-unavailable") {
    throw new Error("unexpected provider diagnostic");
  }
  await adapter.close({
    mapId: "map-1",
    asset: { assetId: "asset-1", contentHash: "sha256:test", revision: 1 },
    dirty: false,
  });
  await Promise.all([
    adapter
      .open({
        mapId: "map-1",
        asset: { assetId: "asset-1", contentHash: "sha256:test", revision: 1 },
        dirty: false,
      })
      .then(
        () => {
          throw new Error("open unexpectedly succeeded");
        },
        () => undefined,
      ),
    adapter
      .save({
        mapId: "map-1",
        asset: { assetId: "asset-1", contentHash: "sha256:test", revision: 1 },
        dirty: true,
      })
      .then(
        () => {
          throw new Error("save unexpectedly succeeded");
        },
        () => undefined,
      ),
  ]);
});

Deno.test("Maps asset diagnostics are stable", () => {
  if (diagnosticForAssetState("missing").code !== "asset-missing") throw new Error("missing diagnostic changed");
  if (diagnosticForAssetState("malformed").code !== "asset-malformed") throw new Error("malformed diagnostic changed");
  if (diagnosticForAssetState("conflict").code !== "asset-conflict") throw new Error("conflict diagnostic changed");
});

Deno.test("Maps bridge does not generate an FMG prompt after startup failure", async () => {
  const bridge = await Deno.readTextFile(new URL("./fmg-bridge-template.js", import.meta.url));
  if (!bridge.includes("window.daenaMapDiagnostic?.(error)")) throw new Error("startup failure is not reported");
  if (
    bridge.includes(
      'window.daenaMapDiagnostic?.(error); if (!new URLSearchParams(location.search).get("mapEntityId")) window.generateMapOnLoad?.()',
    )
  ) {
    throw new Error("startup failure still falls through to FMG generation");
  }
  if (!bridge.includes("data-daena-link-open")) throw new Error("toolbar Link button is missing");
  if (!bridge.includes("linkArming")) throw new Error("opt-in link arming is missing");
  if (!bridge.includes("data-daena-link-x")) throw new Error("editable link coordinates are missing");
  if (!bridge.includes("max-height:240px")) throw new Error("scrollable entity list is missing");
  if (!bridge.includes("daena-link-chrome")) throw new Error("in-FMG link chrome is missing");
  if (!bridge.includes("maps.locations.upsert")) throw new Error("link upsert RPC is missing");
  if (!bridge.includes("maps.locations.create_and_link")) throw new Error("create-and-link RPC is missing");
  if (!bridge.includes("startPick")) throw new Error("pick mode for entity-to-map linking is missing");
  if (!bridge.includes("daena-save-chrome")) throw new Error("in-FMG save chrome is missing");
  if (!bridge.includes("commitFirstSave")) throw new Error("first-save name commit is missing");
  if (!bridge.includes('rpc("entity.create"')) throw new Error("draft first-save must create the map entity");
  if (!bridge.includes("showNameForm") || !bridge.includes("data-daena-name-form")) {
    throw new Error("first-save in-overlay name form is missing");
  }
  if (/\bwindow\.prompt\s*\(/.test(bridge)) {
    throw new Error("first-save must not call window.prompt in the plugin webview");
  }
  if (bridge.includes('fields: [{ namespace: "maps", key: "map"')) {
    throw new Error("first-save must not pass object map descriptors through entity.create fields");
  }
  if (!bridge.includes("requestSave")) throw new Error("unified save entrypoint is missing");
  if (bridge.includes("if (!mapAsset) { await window.generateMapOnLoad?.(); return; }")) {
    throw new Error("draft maps must not early-return before wiring the provider");
  }
  if (!bridge.includes("waitForUploadedPack") || !bridge.includes('"routes" in window.pack.cells')) {
    throw new Error("saved-map load must wait for pack.cells.routes before prepareMapData");
  }
});

Deno.test("provider contract fixture round-trips and does not retarget selectors", async () => {
  const adapter = new JsonProviderAdapter();
  const events: string[] = [];
  const unsubscribe = adapter.subscribe((event) => events.push(event.type));
  const loaded = await adapter.load(fixture);
  if (loaded.provider !== FMG_PROVIDER || loaded.dirty || !events.includes("ready"))
    throw new Error("fixture did not become ready");
  const features = await adapter.listFeatures({ kind: "burg", text: "harbor" });
  if (features.length !== 1 || features[0].id !== "42") throw new Error("feature selector fixture failed");
  const resolved = await adapter.resolveAnchor({
    kind: "provider-feature",
    provider: FMG_PROVIDER,
    featureKind: "burg",
    featureId: "42",
    fallbackPoint: [0.1, 0.1],
  });
  if (!resolved.resolved || resolved.point?.[0] !== 0.613) throw new Error("feature did not resolve");
  const unresolved = await adapter.resolveAnchor({
    kind: "provider-feature",
    provider: FMG_PROVIDER,
    featureKind: "burg",
    featureId: "removed",
    fallbackPoint: [0.1, 0.1],
  });
  if (unresolved.resolved || unresolved.point?.[0] !== 0.1) throw new Error("removed selector retargeted");
  const serialized = await adapter.serialize();
  if (new TextDecoder().decode(serialized) !== new TextDecoder().decode(fixture))
    throw new Error("fixture source was not deterministic");
  unsubscribe();
  await adapter.dispose();
});

Deno.test("Image Map adapter is selected without FMG internals", async () => {
  const adapter = new ImageMapAdapter();
  const adapters = new Map<string, MapProviderAdapter>([[IMAGE_PROVIDER, adapter]]);
  if (selectProviderAdapter(IMAGE_PROVIDER, adapters) !== adapter) {
    throw new Error("image provider was not selected by id");
  }
  const capabilities = await adapter.capabilities();
  if (capabilities.provider !== IMAGE_PROVIDER || capabilities.featureKinds.length !== 0) {
    throw new Error("image adapter advertised FMG features");
  }
  const loaded = await adapter.load(new Uint8Array([1, 2, 3]));
  if (loaded.provider !== IMAGE_PROVIDER || loaded.dirty) throw new Error("image load did not stay clean");
  const point = await adapter.resolveAnchor({ kind: "point", point: [0.2, 0.8] });
  if (!point.resolved || point.point?.[0] !== 0.2) throw new Error("point anchors must resolve in place");
  const feature = await adapter.resolveAnchor({
    kind: "provider-feature",
    provider: FMG_PROVIDER,
    featureKind: "burg",
    featureId: "42",
    fallbackPoint: [0.5, 0.5],
  });
  if (feature.resolved) throw new Error("image maps must not resolve FMG selectors");
  if ((await adapter.listFeatures()).length !== 0) throw new Error("image maps must not expose provider features");
  await adapter.dispose();
});

const sessionFor = (mapId: string): MapEditorSession => ({
  mapId,
  asset: { assetId: "asset-1", contentHash: "sha256:test", revision: 1 },
  dirty: false,
});

class StubProviderAdapter implements MapProviderAdapter {
  readonly provider = FMG_PROVIDER;
  private readonly listeners = new Set<(event: ProviderEvent) => void>();
  emit(event: ProviderEvent): void {
    for (const listener of this.listeners) listener(event);
  }
  async capabilities() {
    return { provider: FMG_PROVIDER, adapterVersion: 1, featureKinds: ["burg"], supportsEditing: false };
  }
  async load(_source: Uint8Array) {
    return { provider: FMG_PROVIDER, sourceHash: "sha256:test", dirty: false };
  }
  async serialize() {
    return new Uint8Array();
  }
  async open(_session: MapEditorSession) {}
  async save() {
    return { bytes: new Uint8Array(), hash: "sha256:test" };
  }
  async close(session: MapEditorSession) {
    return session.dirty;
  }
  async listFeatures() {
    return [];
  }
  async captureSelection() {
    return null;
  }
  async resolveAnchor(anchor: MapAnchor) {
    return { resolved: false, point: anchor.kind === "point" ? anchor.point : null };
  }
  async focus() {}
  async setSemanticOverlay() {}
  subscribe(listener: (event: ProviderEvent) => void) {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }
  async dispose() {}
}

Deno.test("Map editor controller opens clean and close without discarding work", async () => {
  const adapter = new JsonProviderAdapter();
  const controller = new MapEditorController(adapter);
  const events: string[] = [];
  controller.subscribe((event) => events.push(event.type));
  await controller.open(sessionFor("map-1"), fixture);
  if (controller.getSession()?.dirty) throw new Error("session opened dirty");
  if (events.includes("dirty")) throw new Error("clean open emitted a dirty event");
  if (await controller.close()) throw new Error("clean close discarded work");
  if (controller.getSession() !== null) throw new Error("session was not cleared on close");
  await controller.close();
});

Deno.test("Map editor controller tracks dirty and saved transitions", async () => {
  const adapter = new JsonProviderAdapter();
  const controller = new MapEditorController(adapter);
  const events: string[] = [];
  controller.subscribe((event) => events.push(event.type));
  await controller.open(sessionFor("map-1"), fixture);
  adapter.markDirty();
  if (!controller.getSession()?.dirty) throw new Error("provider dirty signal was not tracked");
  if (!events.includes("dirty")) throw new Error("dirty event was not emitted");
  const saved = await controller.save();
  if (!saved.hash.startsWith("sha256:") || saved.bytes.length !== fixture.length)
    throw new Error("save result is invalid");
  controller.confirmSaved(2, saved.hash);
  const session = controller.getSession();
  if (!session || session.dirty || session.asset.revision !== 2 || session.asset.contentHash !== saved.hash) {
    throw new Error("confirmSaved did not settle the session");
  }
  if (await controller.close()) throw new Error("saved close discarded work");
});

Deno.test("Map editor controller reports typed conflicts and forwards fatal errors", async () => {
  const adapter = new StubProviderAdapter();
  const controller = new MapEditorController(adapter);
  const events: string[] = [];
  controller.subscribe((event) => events.push(event.type));
  controller.reportConflict(diagnosticForAssetState("conflict"));
  if (!events.includes("conflict")) throw new Error("conflict event was not emitted");
  await controller.open(sessionFor("map-1"));
  adapter.emit({ type: "fatal-error", message: "provider crashed" });
  if (!events.includes("fatal")) throw new Error("fatal error was not forwarded");
  await controller.close();
});

Deno.test("Map editor controller discards unsaved work on dirty close", async () => {
  const adapter = new JsonProviderAdapter();
  const controller = new MapEditorController(adapter);
  await controller.open(sessionFor("map-1"), fixture);
  adapter.markDirty();
  if (!(await controller.close())) throw new Error("dirty close did not discard work");
});

Deno.test("Maps bridge publishes editor state and recovery flows", async () => {
  const bridge = await Deno.readTextFile(new URL("./fmg-bridge-template.js", import.meta.url));
  if (!bridge.includes('type: "daena.maps/state@1"')) throw new Error("state publishing channel is missing");
  if (!bridge.includes('rpc("event.publish"')) throw new Error("event publish is not wired");
  if (!bridge.includes('publishState("saving"')) throw new Error("saving state is not published");
  if (!bridge.includes("asset revision conflict")) throw new Error("conflict discrimination is missing");
  if (!bridge.includes('publishState("conflict"')) throw new Error("conflict state is not published");
  if (!bridge.includes("exportDraft")) throw new Error("draft export is missing");
  if (!bridge.includes("maps.recovery.export.begin")) throw new Error("recovery export begin is missing");
  if (!bridge.includes("maps.recovery.export.commit")) throw new Error("recovery export commit is missing");
  if (!bridge.includes("maps.asset.create.begin")) throw new Error("first-save create begin is missing");
  if (!bridge.includes("maps.asset.create.commit")) throw new Error("first-save create commit is missing");
  if (!bridge.includes("this map has no saved source yet")) throw new Error("reload guard for unsaved maps is missing");
  if (!bridge.includes("map source is empty")) throw new Error("empty source guard is missing");
  if (!bridge.includes("setDirty(true)")) throw new Error("fresh map generation is not marked dirty");
  if (!bridge.includes('publishState("clean")')) throw new Error("boot state is not published");
  if (!bridge.includes("showDiagnosticUnlessConflict")) throw new Error("conflict diagnostics are not suppressed");
  if (!bridge.includes("if (savingNow) return;")) throw new Error("concurrent saves are not guarded");
  if (!bridge.includes("startDirtyWatcher")) throw new Error("dirty watcher is missing");
  if (!bridge.includes("reloadSource")) throw new Error("source reload is missing");
});

Deno.test("Maps bridge self-heals a revoked session by re-bootstrapping", async () => {
  const bridge = await Deno.readTextFile(new URL("./fmg-bridge-template.js", import.meta.url));
  if (!bridge.includes("session.revoked")) throw new Error("revoked session code is not handled");
  if (!bridge.includes("session.stale")) throw new Error("stale session code is not handled");
  if (!bridge.includes("session.expired")) throw new Error("expired session code is not handled");
  if (!bridge.includes("session.invalid")) throw new Error("invalid session code is not handled");
  if (!bridge.includes("sessionId = undefined")) throw new Error("session is not cleared for re-bootstrap");
  if (!bridge.includes("attempt < 2")) throw new Error("rpc does not retry after re-bootstrap");
});
