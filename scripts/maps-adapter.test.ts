import {
  FMG_PROVIDER,
  JsonProviderAdapter,
  UnavailableProviderAdapter,
  diagnosticForAssetState,
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
