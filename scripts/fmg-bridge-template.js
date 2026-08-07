"use strict";
// Daena's only integration point with FMG. FMG globals remain inside the
// provider-owned child webview and are never imported by Daena modules.
// Pack cells not found is retained as a recognizable FMG diagnostic.
window.DAENA_HOST = true;

(async () => {
  const params = new URLSearchParams(location.search);
  const projectId = params.get("project");
  const requestedMapEntityId = params.get("mapEntityId");
  const requestedLinkId = params.get("linkId");
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
    if (!response.ok) throw new Error(value.error?.message || value.error || `Daena RPC failed (${response.status})`);
    return value;
  }
  const isSessionFailure = value => value?.ok === false && ["session.revoked", "session.stale", "session.expired", "session.invalid"].includes(value.error?.code);
  async function rpc(method, payload) {
    for (let attempt = 0; attempt < 2; attempt += 1) {
      if (!sessionId) sessionId = (await post({ op: "bootstrap", pluginId: "daena.maps", projectId })).sessionId;
      const requestId = `maps-fmg-${++requestSequence}`;
      const value = await post({ op: "rpc", request: { rpcVersion: 1, sessionId, requestId, method, payload } });
      if (value.ok) return value.result;
      if (attempt === 0 && isSessionFailure(value)) { sessionId = undefined; continue; }
      const error = new Error(value.error?.message || "Daena RPC failed");
      error.code = value.error?.code;
      throw error;
    }
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
      if (requestedMapEntityId) return { mapId: map.id, assetId: descriptor?.sourceAssetId ?? null };
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
      let saved;
      if (asset.assetId === null) {
        const transfer = await rpc("maps.asset.create.begin", { mapEntityId: asset.mapId, size: bytes.length });
        for (let offset = 0, chunk = 0; offset < bytes.length; offset += transfer.maxChunkBytes, chunk += 1) {
          const response = await fetch(transfer.url.replace(/\/0\?/, `/${chunk}?`), { method: "PUT", body: bytes.slice(offset, offset + transfer.maxChunkBytes) });
          if (!response.ok) throw new Error(`map source upload failed (${response.status})`);
        }
        saved = await rpc("maps.asset.create.commit", { handle: transfer.handle, contentHash: hash });
        asset.assetId = saved.id;
        asset.revision = saved.revision;
        asset.contentHash = saved.content_hash ?? saved.contentHash;
        await rpc("field.set", { entityId: asset.mapId, namespace: "maps", key: "map", value: { schemaVersion: 1, provider: { id: "azgaar-fmg", adapterVersion: 1, sourceFormat: "fmg-map" }, sourceAssetId: saved.id, previewAssetId: null, defaultView: { center: [0.5, 0.5], zoom: 1 } }, expectedRevision: "" });
      } else {
        const transfer = await rpc("asset.replace.begin", { assetId: asset.assetId, namespace: "maps", expectedRevision: asset.revision, size: bytes.length, mimeType: "application/x-fmg-map" });
        for (let offset = 0, chunk = 0; offset < bytes.length; offset += transfer.maxChunkBytes, chunk += 1) {
          const response = await fetch(transfer.url.replace(/\/0\?/, `/${chunk}?`), { method: "PUT", body: bytes.slice(offset, offset + transfer.maxChunkBytes) });
          if (!response.ok) throw new Error(`map source upload failed (${response.status})`);
        }
        saved = await rpc("asset.replace.commit", { handle: transfer.handle, contentHash: hash });
        asset.revision = saved.revision;
        asset.contentHash = saved.content_hash ?? saved.contentHash;
      }
      lastSavedHash = hash;
      setDirty(false);
      window.dispatchEvent(new CustomEvent("daena-map-saved", { detail: saved }));
      publishState("saved", { revision: saved.revision });
      // The source bytes changed, so provider-feature resolution may have
      // changed. Reconcile immediately so removed or renumbered features
      // surface as unresolved rather than on the next full index build.
      const reconciled = await rpc("maps.reconcile.links", { mapEntityId: mapId }).catch(() => null);
      if (Array.isArray(reconciled)) {
        const unresolved = reconciled.filter(item => !item.resolved).map(item => item.locationId);
        await refreshOverlay().catch(() => undefined);
        publishState("reconcile", { total: reconciled.length, unresolved });
      }
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
  let asset = { mapId: mapAsset.mapId, assetId: mapAsset.assetId, revision: null, contentHash: null };
  let source = null;
  if (asset.assetId !== null) {
    const metadata = await rpc("asset.read.begin", { assetId: asset.assetId, namespace: "maps" });
    asset = { mapId: asset.mapId, assetId: asset.assetId, revision: metadata.revision, contentHash: metadata.contentHash };
    source = metadata.size === 0 ? null : await loadAsset(asset.assetId);
  }
  mapId = asset.mapId;
  const featureCollections = { burg: "burgs", state: "states", province: "provinces", river: "rivers", marker: "markers" };
  const pointFor = feature => [feature.x / graphWidth, feature.y / graphHeight];
  const listFeatures = async query => Object.entries(featureCollections).flatMap(([kind, collectionName]) => (pack[collectionName] || []).filter(feature => (!query?.kind || query.kind === kind) && (!query?.text || String(feature.name || feature.i).toLowerCase().includes(query.text.toLowerCase()))).map(feature => ({ kind, id: String(feature.i), label: feature.name, point: pointFor(feature) })));
  const resolveAnchor = async anchor => {
    if (anchor.kind !== "provider-feature") return { resolved: true, point: anchor.point || anchor.fallbackPoint || anchor.points?.[0] || anchor.rings?.[0]?.[0] || null };
    const feature = (pack[featureCollections[anchor.featureKind]] || []).find(item => String(item.i) === String(anchor.featureId));
    return feature ? { resolved: true, point: pointFor(feature) } : { resolved: false, point: anchor.fallbackPoint };
  };
  // Capture support. FMG's selection state is not exposed by the vendored
  // bundle, so capture uses click-to-pick: the last pointer position inside
  // the map SVG, snapped to the nearest feature within ~2% of the graph.
  let lastPointer = null;
  window.addEventListener("pointerdown", event => {
    const svg = event.target?.closest?.("svg");
    if (!svg) return;
    const rect = svg.getBoundingClientRect();
    if (!rect.width || !rect.height) return;
    lastPointer = [(event.clientX - rect.left) / rect.width, (event.clientY - rect.top) / rect.height];
  }, true);
  async function computeSelection() {
    const point = lastPointer;
    if (!point) return null;
    let nearest = null;
    let nearestDistance = Infinity;
    for (const [kind, collectionName] of Object.entries(featureCollections)) {
      for (const feature of pack[collectionName] || []) {
        if (!Number.isFinite(feature.x) || !Number.isFinite(feature.y)) continue;
        const fx = feature.x / graphWidth;
        const fy = feature.y / graphHeight;
        const distance = (fx - point[0]) ** 2 + (fy - point[1]) ** 2;
        if (distance < nearestDistance) {
          nearestDistance = distance;
          nearest = { kind, feature };
        }
      }
    }
    if (nearest && nearestDistance <= 0.0004) {
      return { kind: "provider-feature", provider: "azgaar-fmg", featureKind: nearest.kind, featureId: String(nearest.feature.i), fallbackPoint: pointFor(nearest.feature) };
    }
    return { kind: "point", point };
  }
  async function publishSelection() {
    if (!mapId) return;
    const anchor = await computeSelection().catch(() => null);
    void rpc("event.publish", { type: "daena.maps/selection@1", payload: { mapEntityId: mapId, anchor } }).catch(() => undefined);
    return anchor;
  }
  // Semantic overlay. The overlay is derived state: it is rebuilt from the
  // disposable projection on entity-changed events and after every save, and
  // never persists anything itself.
  let overlayFrame = null;
  let overlayDate = null;
  let pendingLinkId = requestedLinkId;
  let overlayRoot = null;
  function ensureOverlayRoot() {
    if (overlayRoot && overlayRoot.isConnected) return overlayRoot;
    const host = document.querySelector("#map") || document.body;
    if (host !== document.body && host.style.position === "") host.style.position = "relative";
    overlayRoot = document.createElement("div");
    overlayRoot.id = "daena-semantic-overlay";
    overlayRoot.style.cssText = "position:absolute;inset:0;pointer-events:none;overflow:hidden;z-index:2000;";
    host.appendChild(overlayRoot);
    return overlayRoot;
  }
  function overlayPoint(location) {
    if (location.anchorKind === "provider-feature" && location.featureKind && location.featureId) {
      const feature = (pack[featureCollections[location.featureKind]] || []).find(item => String(item.i) === String(location.featureId));
      if (feature && Number.isFinite(feature.x) && Number.isFinite(feature.y)) return pointFor(feature);
    }
    const [minX, minY, maxX, maxY] = location.bounds || [null, null, null, null];
    if (Number.isFinite(minX) && Number.isFinite(minY) && Number.isFinite(maxX) && Number.isFinite(maxY)) return [(minX + maxX) / 2, (minY + maxY) / 2];
    return null;
  }
  function inValidity(location) {
    if (!overlayDate) return true;
    const from = location.validity?.from ?? null;
    const to = location.validity?.to ?? null;
    if (from && overlayDate < from) return false;
    if (to && overlayDate > to) return false;
    return true;
  }
  function renderOverlay() {
    if (!overlayFrame) return;
    const root = ensureOverlayRoot();
    root.replaceChildren();
    for (const location of overlayFrame) {
      if (location.resolution === "unresolved" || !inValidity(location)) continue;
      const point = overlayPoint(location);
      if (!point) continue;
      const marker = document.createElement("div");
      marker.title = `${location.label || location.role} (${location.role})`;
      marker.style.cssText = "position:absolute;width:10px;height:10px;margin:-5px 0 0 -5px;border-radius:50%;background:#e11d48;border:1.5px solid #fff;box-shadow:0 1px 3px #0006;";
      marker.style.left = `${(point[0] * 100).toFixed(2)}%`;
      marker.style.top = `${(point[1] * 100).toFixed(2)}%`;
      root.appendChild(marker);
    }
  }
  function setOverlayDate(date) {
    if (date == null) {
      overlayDate = null;
      renderOverlay();
      return;
    }
    if (typeof date === "string") {
      overlayDate = date.slice(0, 10);
      renderOverlay();
      return;
    }
    const year = date.year;
    const month = date.month ? String(date.month).padStart(2, "0") : "01";
    const day = date.day ? String(date.day).padStart(2, "0") : "01";
    overlayDate = `${date.era === "BCE" ? "-" : ""}${String(year).padStart(4, "0")}-${month}-${day}`;
    renderOverlay();
  }
  async function focusByLink(linkId) {
    if (!overlayFrame) return false;
    const location = overlayFrame.find(item => item.id === linkId);
    if (!location || location.resolution === "unresolved") return false;
    const point = overlayPoint(location);
    if (!point) return false;
    zoomTo(point[0] * graphWidth, point[1] * graphHeight, 8, 500);
    return true;
  }
  async function refreshOverlay() {
    if (!mapId) return;
    const rows = await rpc("maps.locations.list", { mapEntityId: mapId }).catch(() => null);
    if (!Array.isArray(rows)) return;
    overlayFrame = rows;
    renderOverlay();
    if (pendingLinkId && (await focusByLink(pendingLinkId))) pendingLinkId = null;
  }
  async function subscribeCoreEvents() {
    await rpc("event.subscribe", { type: "daena.core/entity-changed@1" }).catch(() => undefined);
    window.setInterval(async () => {
      const events = await rpc("event.poll", { type: "daena.core/entity-changed@1" }).catch(() => []);
      if (Array.isArray(events) && events.length > 0) await refreshOverlay().catch(() => undefined);
    }, 2000);
  }
  // Publish a compact selection signal so the shell can enable or disable the
  // capture tool. Best-effort: publishing failures are never fatal.
  let lastSelectionPayload = null;
  const startSelectionWatcher = () => {
    window.setInterval(async () => {
      if (!mapId) return;
      const anchor = await computeSelection().catch(() => null);
      const key = JSON.stringify(anchor);
      if (key === lastSelectionPayload) return;
      lastSelectionPayload = key;
      void rpc("event.publish", { type: "daena.maps/selection@1", payload: { mapEntityId: mapId, anchor } }).catch(() => undefined);
    }, 900);
  };
  async function loadMapSource(bytes) {
    if (!bytes.length) throw new Error("map source is empty");
    await uploadMap(new File([bytes], "daena.map", { type: "application/octet-stream" }));
  }
  async function reloadSource() {
    if (!asset.assetId) { showDiagnostic(new Error("this map has no saved source yet")); return; }
    const bytes = await loadAsset(asset.assetId);
    await loadMapSource(bytes);
    lastSavedHash = await sha256(new TextEncoder().encode(prepareMapData()));
    setDirty(false);
    publishState("clean");
    await refreshOverlay().catch(() => undefined);
  }
  window.daenaMapProvider = { provider: "azgaar-fmg", capabilities: async () => ({ provider: "azgaar-fmg", adapterVersion: 1, featureKinds: Object.keys(featureCollections), supportsEditing: true }), load: loadMapSource, serialize: () => new TextEncoder().encode(prepareMapData()), listFeatures, resolveAnchor, captureSelection: publishSelection, focus: async anchor => { const result = await resolveAnchor(anchor); if (result.point) zoomTo(result.point[0] * graphWidth, result.point[1] * graphHeight, 8, 500); }, focusByLink, setSemanticOverlay: frame => { overlayFrame = Array.isArray(frame?.locations) ? frame.locations : null; setOverlayDate(frame?.date ?? null); }, setDate: setOverlayDate, save: () => saveAsset(asset), exportDraft, reloadSource, dispose: () => { delete window.daenaMapProvider; } };
  const originalSave = window.saveMap;
  window.saveMap = method => method === "machine" || method === "dropbox" || method === "storage" ? saveAsset(asset).catch(showDiagnosticUnlessConflict) : originalSave?.(method);
  window.addEventListener("keydown", event => { if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "s") { event.preventDefault(); void saveAsset(asset).catch(showDiagnosticUnlessConflict); } });
  if (source === null) {
    if (typeof window.generateMapOnLoad !== "function") throw new Error("new map source is empty and FMG generation is unavailable");
    await window.generateMapOnLoad();
    if (asset.assetId === null) setDirty(true);
    else await saveAsset(asset);
  } else {
    await window.daenaMapProvider.load(source);
    lastSavedHash = await sha256(new TextEncoder().encode(prepareMapData()));
    publishState("clean");
  }
  void subscribeCoreEvents();
  void refreshOverlay();
  startDirtyWatcher();
  startSelectionWatcher();
})().catch(error => {
  console.error("Daena Maps provider startup failed:", error);
  window.daenaMapDiagnostic?.(error);
});
