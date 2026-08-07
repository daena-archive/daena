import {
  FMG_PROVIDER,
  JsonProviderAdapter,
  MapEditorController,
  UnavailableProviderAdapter,
  diagnosticForAssetState,
  type MapAnchor,
  type MapEditorSession,
  type MapProviderAdapter,
  type ProviderEvent,
} from "../packages/modules/maps/src/adapter.ts";

const fixture = new TextEncoder().encode(JSON.stringify({
  features: [
    { kind: "burg", id: "42", label: "Old Harbor", point: [0.613, 0.428] },
  ],
}));

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
    adapter.open({
      mapId: "map-1",
      asset: { assetId: "asset-1", contentHash: "sha256:test", revision: 1 },
      dirty: false,
    }).then(() => { throw new Error("open unexpectedly succeeded"); }, () => undefined),
    adapter.save({
      mapId: "map-1",
      asset: { assetId: "asset-1", contentHash: "sha256:test", revision: 1 },
      dirty: true,
    }).then(() => { throw new Error("save unexpectedly succeeded"); }, () => undefined),
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
  if (bridge.includes("window.daenaMapDiagnostic?.(error); if (!new URLSearchParams(location.search).get(\"mapEntityId\")) window.generateMapOnLoad?.()")) {
    throw new Error("startup failure still falls through to FMG generation");
  }
  if (!bridge.includes("if (!mapAsset) { await window.generateMapOnLoad?.(); return; }")) {
    throw new Error("empty-map generation path changed unexpectedly");
  }
});

Deno.test("provider contract fixture round-trips and does not retarget selectors", async () => {
  const adapter = new JsonProviderAdapter();
  const events: string[] = [];
  const unsubscribe = adapter.subscribe((event) => events.push(event.type));
  const loaded = await adapter.load(fixture);
  if (loaded.provider !== FMG_PROVIDER || loaded.dirty || !events.includes("ready")) throw new Error("fixture did not become ready");
  const features = await adapter.listFeatures({ kind: "burg", text: "harbor" });
  if (features.length !== 1 || features[0].id !== "42") throw new Error("feature selector fixture failed");
  const resolved = await adapter.resolveAnchor({ kind: "provider-feature", provider: FMG_PROVIDER, featureKind: "burg", featureId: "42", fallbackPoint: [0.1, 0.1] });
  if (!resolved.resolved || resolved.point?.[0] !== 0.613) throw new Error("feature did not resolve");
  const unresolved = await adapter.resolveAnchor({ kind: "provider-feature", provider: FMG_PROVIDER, featureKind: "burg", featureId: "removed", fallbackPoint: [0.1, 0.1] });
  if (unresolved.resolved || unresolved.point?.[0] !== 0.1) throw new Error("removed selector retargeted");
  const serialized = await adapter.serialize();
  if (new TextDecoder().decode(serialized) !== new TextDecoder().decode(fixture)) throw new Error("fixture source was not deterministic");
  unsubscribe();
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
  emit(event: ProviderEvent): void { for (const listener of this.listeners) listener(event); }
  async capabilities() { return { provider: FMG_PROVIDER, adapterVersion: 1, featureKinds: ["burg"], supportsEditing: false }; }
  async load(_source: Uint8Array) { return { provider: FMG_PROVIDER, sourceHash: "sha256:test", dirty: false }; }
  async serialize() { return new Uint8Array(); }
  async open(_session: MapEditorSession) {}
  async save() { return { bytes: new Uint8Array(), hash: "sha256:test" }; }
  async close(session: MapEditorSession) { return session.dirty; }
  async listFeatures() { return []; }
  async captureSelection() { return null; }
  async resolveAnchor(anchor: MapAnchor) { return { resolved: false, point: anchor.kind === "point" ? anchor.point : null }; }
  async focus() {}
  async setSemanticOverlay() {}
  subscribe(listener: (event: ProviderEvent) => void) { this.listeners.add(listener); return () => this.listeners.delete(listener); }
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
  if (!saved.hash.startsWith("sha256:") || saved.bytes.length !== fixture.length) throw new Error("save result is invalid");
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
  if (!bridge.includes("publishState(\"clean\")")) throw new Error("boot state is not published");
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
