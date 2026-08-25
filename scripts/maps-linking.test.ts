import { JsonProviderAdapter, type MapAnchor } from "../packages/modules/maps/src/adapter.ts";

function bridge(): Promise<string> {
  return Deno.readTextFile(new URL("./fmg-bridge-template.js", import.meta.url));
}

Deno.test("Maps bridge exposes capture, overlay, date, and focus-by-link on the provider", async () => {
  const source = await bridge();
  const providerLiteral = source.split("window.daenaMapProvider = ")[1].split("\n")[0];
  for (const method of ["captureSelection", "setSemanticOverlay", "setDate", "focusByLink"]) {
    if (!providerLiteral.includes(method)) throw new Error(`provider is missing ${method}`);
  }
});

Deno.test("Maps bridge publishes selection events and reads the linkId URL parameter", async () => {
  const source = await bridge();
  if (!source.includes('type: "daena.maps/selection@1"')) {
    throw new Error("selection event is not published");
  }
  if (!source.includes('const requestedLinkId = params.get("linkId");')) {
    throw new Error("linkId URL parameter is not read");
  }
});

Deno.test("Maps bridge subscribes to core changes and reconciles links after save", async () => {
  const source = await bridge();
  if (!source.includes('type: "daena.core/entity-changed@1"')) {
    throw new Error("core change subscription is missing");
  }
  if (!source.includes("event.poll")) {
    throw new Error("core change polling is missing");
  }
  if (!source.includes('"maps.reconcile.links"')) {
    throw new Error("post-save link reconciliation is missing");
  }
  if (!source.includes('"maps.locations.list"')) {
    throw new Error("overlay location queries are missing");
  }
});

Deno.test("Maps bridge never renders unresolved or out-of-validity markers", async () => {
  const source = await bridge();
  const render = source.slice(source.indexOf("function renderOverlay"), source.indexOf("function setOverlayDate"));
  if (!render.includes(`resolution === "unresolved"`)) {
    throw new Error("unresolved locations are not skipped");
  }
  if (!render.includes("inValidity")) {
    throw new Error("validity filtering is not applied");
  }
});

Deno.test("Show on map switches the workspace section into Maps", async () => {
  const page = await Deno.readTextFile(new URL("../src/routes/+page.svelte", import.meta.url));
  if (!page.includes('section === "maps" && sandboxView?.renderer === "maps"')) {
    throw new Error("map surface still requires the Maps workspace section");
  }
  const marker = "async function openPluginView";
  const start = page.indexOf(marker);
  if (start < 0) throw new Error("openPluginView is missing");
  const openMaps = page.slice(start, start + 1800);
  if (!openMaps.includes('item.renderer === "maps"')) {
    throw new Error("openPluginView must handle the maps renderer");
  }
  if (!openMaps.includes('section = "maps"')) {
    throw new Error("openPluginView must set section to maps when opening a map surface");
  }
  if (!openMaps.includes("loreWikiOpen = false")) {
    throw new Error("opening a map must leave the Lore wiki surface");
  }
});

Deno.test("Json adapter captures provider-feature selections in anchor form", async () => {
  const adapter = new JsonProviderAdapter();
  await adapter.open(
    {
      mapId: "map-1",
      asset: { assetId: "asset-1", contentHash: "sha256:test", revision: 1 },
      dirty: false,
    },
    new TextEncoder().encode(
      JSON.stringify({
        features: [{ kind: "burg", id: "42", label: "Old Harbor", point: [0.613, 0.428] }],
      }),
    ),
  );
  adapter.focus({
    kind: "provider-feature",
    provider: "azgaar-fmg",
    featureKind: "burg",
    featureId: "42",
    fallbackPoint: [0.613, 0.428],
  });
  const anchor: MapAnchor | null = await adapter.captureSelection();
  await adapter.close({
    mapId: "map-1",
    asset: { assetId: "asset-1", contentHash: "sha256:test", revision: 1 },
    dirty: false,
  });
  if (!anchor || anchor.kind !== "provider-feature") throw new Error("captured anchor is not a provider-feature");
  if (anchor.featureId !== "42") throw new Error("captured feature id changed");
});

Deno.test("Maps manifest declares the selection event as publishable", async () => {
  const manifest = JSON.parse(
    await Deno.readTextFile(new URL("../packages/modules/maps/manifest.json", import.meta.url)),
  );
  const selection = (manifest.events?.publishes ?? []).find(
    (event: { name?: string }) => event.name === "daena.maps/selection",
  );
  if (!selection) throw new Error("selection event is not declared as publishable");
});
