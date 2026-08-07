"use strict";
// Daena's only integration point with FMG. FMG globals remain inside the
// provider-owned child webview and are never imported by Daena modules.
// Pack cells not found is retained as a recognizable FMG diagnostic.
window.DAENA_HOST = true;

(async () => {
  const params = new URLSearchParams(location.search);
  const projectId = params.get("project");
  const requestedMapEntityId = params.get("mapEntityId");
  function showDiagnostic(error) {
    const message = error instanceof Error ? error.message : String(error);
    let panel = document.getElementById("daena-map-diagnostic");
    if (!panel) {
      panel = document.createElement("div");
      panel.id = "daena-map-diagnostic";
      panel.style.cssText = "position:fixed;z-index:2147483647;left:16px;right:16px;bottom:16px;padding:12px 16px;border:1px solid #d97706;border-radius:8px;background:#fffbeb;color:#78350f;font:14px system-ui,sans-serif;box-shadow:0 4px 18px #0003";
      document.body.appendChild(panel);
    }
    panel.textContent = `Daena Maps: ${message}`;
  }
  window.daenaMapDiagnostic = showDiagnostic;
  let requestSequence = 0;
  let sessionId;
  let mapId = null;
  let lastSavedHash = null;
  let dirty = false;
  let savingNow = false;
  async function post(body) {
    const response = await fetch("/__rpc", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) });
    const value = await response.json();
    if (!response.ok || value.ok === false) throw new Error(value.error?.message || value.error || `Daena RPC failed (${response.status})`);
    return value;
  }
  async function rpc(method, payload) {
    if (!sessionId) sessionId = (await post({ op: "bootstrap", pluginId: "daena.maps", projectId })).sessionId;
    const requestId = `maps-fmg-${++requestSequence}`;
    const value = await post({ op: "rpc", request: { rpcVersion: 1, sessionId, requestId, method, payload } });
    if (!value.ok) throw new Error(value.error?.message || "Daena RPC failed");
    return value.result;
  }
  async function sha256(bytes) {
    const hashBuffer = await crypto.subtle.digest("SHA-256", bytes);
    return `sha256:${Array.from(new Uint8Array(hashBuffer), value => value.toString(16).padStart(2, "0")).join("")}`;
  }
  function publishState(status, detail = null) {
    if (!mapId) return;
    void rpc("event.publish", { type: "daena.maps/state@1", payload: { mapEntityId: mapId, status, detail } }).catch(() => undefined);
  }
  function setDirty(value) {
    if (dirty === value) return;
    dirty = value;
    publishState(value ? "dirty" : "clean");
  }
  async function firstMapAsset() {
    const maps = await rpc("entity.list", { entityType: "daena.maps:map" });
    for (const map of maps) {
      if (requestedMapEntityId && map.id !== requestedMapEntityId) continue;
      const field = await rpc("field.read", { entityId: map.id, namespace: "maps", key: "map" });
      const descriptor = Array.isArray(field) ? field[0]?.value : field?.value ?? field;
      if (descriptor?.sourceAssetId) return { mapId: map.id, assetId: descriptor.sourceAssetId };
    }
    if (requestedMapEntityId) throw new Error(`requested map is unavailable: ${requestedMapEntityId}`);
    return null;
  }
  async function loadAsset(assetId) {
    const handle = await rpc("asset.read.begin", { assetId, namespace: "maps" });
    const response = await fetch(handle.url);
    if (!response.ok) throw new Error(`map source read failed (${response.status})`);
    return new Uint8Array(await response.arrayBuffer());
  }
  async function exportDraft() {
    const bytes = new TextEncoder().encode(prepareMapData());
    const contentHash = await sha256(bytes);
    const transfer = await rpc("maps.recovery.export.begin", { mapEntityId: mapId, size: bytes.length });
    for (let offset = 0, chunk = 0; offset < bytes.length; offset += transfer.maxChunkBytes, chunk += 1) {
      const response = await fetch(transfer.url.replace(/\/0\?/, `/${chunk}?`), { method: "PUT", body: bytes.slice(offset, offset + transfer.maxChunkBytes) });
      if (!response.ok) throw new Error(`map draft export failed (${response.status})`);
    }
    return rpc("maps.recovery.export.commit", { handle: transfer.handle, contentHash });
  }
  async function saveAsset(asset) {
    if (savingNow) return;
    savingNow = true;
    publishState("saving");
    try {
      const bytes = new TextEncoder().encode(prepareMapData());
      const hash = await sha256(bytes);
      const transfer = await rpc("asset.replace.begin", { assetId: asset.assetId, namespace: "maps", expectedRevision: asset.revision, size: bytes.length, mimeType: "application/x-fmg-map" });
      for (let offset = 0, chunk = 0; offset < bytes.length; offset += transfer.maxChunkBytes, chunk += 1) {
        const response = await fetch(transfer.url.replace(/\/0\?/, `/${chunk}?`), { method: "PUT", body: bytes.slice(offset, offset + transfer.maxChunkBytes) });
        if (!response.ok) throw new Error(`map source upload failed (${response.status})`);
      }
      const saved = await rpc("asset.replace.commit", { handle: transfer.handle, contentHash: hash });
      asset.revision = saved.revision;
      asset.contentHash = saved.content_hash ?? saved.contentHash;
      lastSavedHash = hash;
      setDirty(false);
      window.dispatchEvent(new CustomEvent("daena-map-saved", { detail: saved }));
      publishState("saved", { revision: saved.revision });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      if (message.includes("asset revision conflict")) {
        try {
          const draft = await exportDraft();
          const fileName = draft.fileName ?? String(draft.path ?? "").split("/").pop();
          publishState("conflict", { path: draft.path ?? null, fileName });
        } catch (exportError) {
          publishState("conflict", {});
        }
      } else {
        publishState("error", { message });
      }
      throw error;
    } finally {
      savingNow = false;
    }
  }
  const isConflictMessage = message => String(message).includes("asset revision conflict");
  function showDiagnosticUnlessConflict(error) {
    if (!isConflictMessage(error instanceof Error ? error.message : error)) showDiagnostic(error);
    throw error;
  }
  const startDirtyWatcher = () => {
    window.setInterval(async () => {
      if (savingNow || !mapId) return;
      try {
        const hash = await sha256(new TextEncoder().encode(prepareMapData()));
        if (lastSavedHash !== null && hash !== lastSavedHash) setDirty(true);
        else if (dirty && hash === lastSavedHash) setDirty(false);
      } catch (_) { /* FMG is not ready or mid-operation; try again on the next tick */ }
    }, 1500);
  };
  const waitForProvider = () => new Promise(resolve => {
    const poll = () => typeof window.uploadMap === "function" && typeof window.prepareMapData === "function" ? resolve() : setTimeout(poll, 25);
    poll();
  });
  await waitForProvider();
  const mapAsset = await firstMapAsset();
  if (!mapAsset) { await window.generateMapOnLoad?.(); return; }
  const metadata = await rpc("asset.read.begin", { assetId: mapAsset.assetId, namespace: "maps" });
  const asset = { mapId: mapAsset.mapId, assetId: mapAsset.assetId, revision: metadata.revision, contentHash: metadata.contentHash };
  mapId = asset.mapId;
  const source = metadata.size === 0 ? null : await loadAsset(mapAsset.assetId);
  const featureCollections = { burg: "burgs", state: "states", province: "provinces", river: "rivers", marker: "markers" };
  const pointFor = feature => [feature.x / graphWidth, feature.y / graphHeight];
  const listFeatures = async query => Object.entries(featureCollections).flatMap(([kind, collectionName]) => (pack[collectionName] || []).filter(feature => (!query?.kind || query.kind === kind) && (!query?.text || String(feature.name || feature.i).toLowerCase().includes(query.text.toLowerCase()))).map(feature => ({ kind, id: String(feature.i), label: feature.name, point: pointFor(feature) })));
  const resolveAnchor = async anchor => {
    if (anchor.kind !== "provider-feature") return { resolved: true, point: anchor.point || anchor.fallbackPoint || anchor.points?.[0] || anchor.rings?.[0]?.[0] || null };
    const feature = (pack[featureCollections[anchor.featureKind]] || []).find(item => String(item.i) === String(anchor.featureId));
    return feature ? { resolved: true, point: pointFor(feature) } : { resolved: false, point: anchor.fallbackPoint };
  };
  async function loadMapSource(bytes) { await uploadMap(new File([bytes], "daena.map", { type: "application/octet-stream" })); }
  async function reloadSource() {
    const bytes = await loadAsset(asset.assetId);
    await loadMapSource(bytes);
    lastSavedHash = await sha256(new TextEncoder().encode(prepareMapData()));
    setDirty(false);
    publishState("clean");
  }
  window.daenaMapProvider = { provider: "azgaar-fmg", capabilities: async () => ({ provider: "azgaar-fmg", adapterVersion: 1, featureKinds: Object.keys(featureCollections), supportsEditing: true }), load: loadMapSource, serialize: () => new TextEncoder().encode(prepareMapData()), listFeatures, resolveAnchor, focus: async anchor => { const result = await resolveAnchor(anchor); if (result.point) zoomTo(result.point[0] * graphWidth, result.point[1] * graphHeight, 8, 500); }, save: () => saveAsset(asset), exportDraft, reloadSource, dispose: () => { delete window.daenaMapProvider; } };
  const originalSave = window.saveMap;
  window.saveMap = method => method === "machine" || method === "dropbox" || method === "storage" ? saveAsset(asset).catch(showDiagnosticUnlessConflict) : originalSave?.(method);
  window.addEventListener("keydown", event => { if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "s") { event.preventDefault(); void saveAsset(asset).catch(showDiagnosticUnlessConflict); } });
  if (source === null) {
    if (typeof window.generateMapOnLoad !== "function") throw new Error("new map source is empty and FMG generation is unavailable");
    await window.generateMapOnLoad();
    await saveAsset(asset);
  } else {
    await window.daenaMapProvider.load(source);
    lastSavedHash = await sha256(new TextEncoder().encode(prepareMapData()));
    publishState("clean");
  }
  startDirtyWatcher();
})().catch(error => {
  console.error("Daena Maps provider startup failed:", error);
  window.daenaMapDiagnostic?.(error);
});
