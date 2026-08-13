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
      panel.style.cssText =
        "position:fixed;z-index:2147483647;left:16px;right:16px;bottom:16px;padding:12px 16px;border:1px solid #d97706;border-radius:8px;background:#fffbeb;color:#78350f;font:14px system-ui,sans-serif;box-shadow:0 4px 18px #0003";
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
  let fullscreen = false;
  async function post(body) {
    const response = await fetch("/__rpc", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    const value = await response.json();
    if (!response.ok) throw new Error(value.error?.message || value.error || `Daena RPC failed (${response.status})`);
    return value;
  }
  const isSessionFailure = (value) =>
    value?.ok === false &&
    ["session.revoked", "session.stale", "session.expired", "session.invalid"].includes(value.error?.code);
  async function rpc(method, payload) {
    for (let attempt = 0; attempt < 2; attempt += 1) {
      if (!sessionId) sessionId = (await post({ op: "bootstrap", pluginId: "daena.maps", projectId })).sessionId;
      const requestId = `maps-fmg-${++requestSequence}`;
      const value = await post({ op: "rpc", request: { rpcVersion: 1, sessionId, requestId, method, payload } });
      if (value.ok) return value.result;
      if (attempt === 0 && isSessionFailure(value)) {
        sessionId = undefined;
        continue;
      }
      const error = new Error(value.error?.message || "Daena RPC failed");
      error.code = value.error?.code;
      throw error;
    }
  }
  async function sha256(bytes) {
    const hashBuffer = await crypto.subtle.digest("SHA-256", bytes);
    return `sha256:${Array.from(new Uint8Array(hashBuffer), (value) => value.toString(16).padStart(2, "0")).join("")}`;
  }
  function publishState(status, detail = null) {
    if (!mapId) return;
    void rpc("event.publish", { type: "daena.maps/state@1", payload: { mapEntityId: mapId, status, detail } }).catch(
      () => undefined,
    );
  }
  function publishUiState(status, detail = null) {
    void rpc("event.publish", {
      type: "daena.maps/state@1",
      payload: { mapEntityId: mapId, status, detail },
    }).catch(() => undefined);
  }
  function setDirty(value) {
    if (dirty === value) {
      updateSaveChrome();
      return;
    }
    dirty = value;
    publishState(value ? "dirty" : "clean");
    updateSaveChrome();
  }
  async function firstMapAsset() {
    const maps = await rpc("entity.list", { entityType: "daena.maps:map" });
    for (const map of maps) {
      if (requestedMapEntityId && map.id !== requestedMapEntityId) continue;
      const field = await rpc("field.read", { entityId: map.id, namespace: "maps", key: "map" });
      const descriptor = Array.isArray(field) ? field[0]?.value : (field?.value ?? field);
      let assetId = descriptor?.sourceAssetId ?? null;
      // Repair orphans where .map bytes exist but sourceAssetId was never linked.
      if (!assetId) {
        const assets = await rpc("asset.list", { entityId: map.id, namespace: "maps" });
        const orphan = (Array.isArray(assets) ? assets : [])
          .filter((asset) => asset.namespace === "maps" && asset.size > 0)
          .sort((left, right) => String(right.created_at ?? "").localeCompare(String(left.created_at ?? "")))[0];
        if (orphan) {
          assetId = orphan.id;
          await rpc("field.set", {
            entityId: map.id,
            namespace: "maps",
            key: "map",
            value: {
              schemaVersion: 1,
              provider: { id: "azgaar-fmg", adapterVersion: 1, sourceFormat: "fmg-map" },
              sourceAssetId: orphan.id,
              previewAssetId: descriptor?.previewAssetId ?? null,
              defaultView: descriptor?.defaultView ?? { center: [0.5, 0.5], zoom: 1 },
            },
            expectedRevision: "",
          });
        }
      }
      if (requestedMapEntityId) return { mapId: map.id, assetId };
      if (assetId) return { mapId: map.id, assetId };
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
      const response = await fetch(transfer.url.replace(/\/0\?/, `/${chunk}?`), {
        method: "PUT",
        body: bytes.slice(offset, offset + transfer.maxChunkBytes),
      });
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
          const response = await fetch(transfer.url.replace(/\/0\?/, `/${chunk}?`), {
            method: "PUT",
            body: bytes.slice(offset, offset + transfer.maxChunkBytes),
          });
          if (!response.ok) throw new Error(`map source upload failed (${response.status})`);
        }
        saved = await rpc("maps.asset.create.commit", { handle: transfer.handle, contentHash: hash });
        asset.assetId = saved.id;
        asset.revision = saved.revision;
        asset.contentHash = saved.content_hash ?? saved.contentHash;
        // create.commit already links sourceAssetId on the map descriptor via the
        // trusted host path. Do not follow up with field.set: the maps schema
        // declares the descriptor as text, so an object value is rejected.
      } else {
        const transfer = await rpc("asset.replace.begin", {
          assetId: asset.assetId,
          namespace: "maps",
          expectedRevision: asset.revision,
          size: bytes.length,
          mimeType: "application/x-fmg-map",
        });
        for (let offset = 0, chunk = 0; offset < bytes.length; offset += transfer.maxChunkBytes, chunk += 1) {
          const response = await fetch(transfer.url.replace(/\/0\?/, `/${chunk}?`), {
            method: "PUT",
            body: bytes.slice(offset, offset + transfer.maxChunkBytes),
          });
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
        const unresolved = reconciled.filter((item) => !item.resolved).map((item) => item.locationId);
        await refreshOverlay().catch(() => undefined);
        publishState("reconcile", { total: reconciled.length, unresolved });
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      if (message.includes("asset revision conflict")) {
        try {
          const draft = await exportDraft();
          const fileName =
            draft.fileName ??
            String(draft.path ?? "")
              .split("/")
              .pop();
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
      updateSaveChrome();
    }
  }
  const isConflictMessage = (message) => String(message).includes("asset revision conflict");
  function showDiagnosticUnlessConflict(error) {
    if (!isConflictMessage(error instanceof Error ? error.message : error)) showDiagnostic(error);
    throw error;
  }
  let saveChrome = null;
  let asset = { mapId: null, assetId: null, revision: null, contentHash: null };
  function updateSaveChrome() {
    if (!saveChrome) return;
    const status = saveChrome.querySelector("[data-daena-save-status]");
    const button = saveChrome.querySelector("[data-daena-save-button]");
    if (!status || !button) return;
    if (savingNow) {
      status.textContent = "";
      status.setAttribute("aria-label", "Saving");
      status.title = "Saving";
      status.style.background = "#7aa2d5";
      button.disabled = true;
      updateLinkOpenButton();
      return;
    }
    if (dirty || asset.assetId === null) {
      const label = asset.assetId === null ? "Unsaved map" : "Unsaved changes";
      status.textContent = "";
      status.setAttribute("aria-label", label);
      status.title = label;
      status.style.background = "#d5ab6c";
      button.disabled = false;
      updateLinkOpenButton();
      return;
    }
    status.textContent = "";
    status.setAttribute("aria-label", "Saved");
    status.title = "Saved";
    status.style.background = "#8fc79b";
    button.disabled = true;
    updateLinkOpenButton();
  }
  function hideBackConfirmation() {
    const confirmation = saveChrome?.querySelector("[data-daena-back-confirm]");
    if (confirmation) confirmation.style.display = "none";
  }
  function showBackConfirmation() {
    const confirmation = saveChrome?.querySelector("[data-daena-back-confirm]");
    if (!confirmation) return;
    confirmation.style.display = "flex";
    confirmation.querySelector("[data-daena-back-stay]")?.focus();
  }
  function requestBack() {
    if (savingNow) return;
    if (dirty || asset.assetId === null) {
      showBackConfirmation();
      return;
    }
    fullscreen = false;
    publishUiState("back");
  }
  function ensureSaveChrome() {
    if (saveChrome && saveChrome.isConnected) return saveChrome;
    saveChrome = document.createElement("div");
    saveChrome.id = "daena-save-chrome";
    saveChrome.style.cssText =
      "position:fixed;z-index:2147483000;top:14px;right:14px;display:flex;flex-direction:column;align-items:stretch;gap:8px;padding:8px 10px;border:1px solid rgba(255,255,255,.18);border-radius:10px;background:rgba(32,40,36,.92);color:#f4f1ea;font:12px system-ui,sans-serif;box-shadow:0 8px 24px #0005;pointer-events:auto;min-width:0";
    saveChrome.innerHTML = `
      <div data-daena-save-row style="display:flex;align-items:center;gap:8px">
        <button data-daena-back type="button" aria-label="Back to map details" title="Back to map details" style="display:grid;place-items:center;width:32px;height:32px;appearance:none;border:1px solid rgba(255,255,255,.22);border-radius:7px;padding:0;background:transparent;color:#f4f1ea;cursor:pointer"><svg aria-hidden="true" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M19 12H5"/><path d="m12 19-7-7 7-7"/></svg></button>
        <button data-daena-link-open type="button" aria-label="Link location" title="Link location" style="display:grid;place-items:center;width:32px;height:32px;appearance:none;border:1px solid rgba(255,255,255,.22);border-radius:7px;padding:0;background:transparent;color:#f4f1ea;cursor:pointer"><svg aria-hidden="true" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/></svg></button>
        <button data-daena-save-button type="button" aria-label="Save map" title="Save map" style="display:grid;place-items:center;width:32px;height:32px;appearance:none;border:0;border-radius:7px;padding:0;background:#d5ab6c;color:#2c4032;cursor:pointer"><svg aria-hidden="true" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 3h12l2 2v16H5z"/><path d="M8 3v6h8V3"/><path d="M8 21v-6h8v6"/></svg></button>
        <span data-daena-save-status role="status" aria-live="polite" aria-label="Unsaved map" title="Unsaved map" style="display:block;width:8px;height:8px;flex:0 0 8px;margin-left:auto;border-radius:999px;background:#d5ab6c;box-shadow:0 0 0 2px rgba(255,255,255,.08)"></span>
      </div>
      <form data-daena-name-form style="display:none;gap:8px;margin:0">
        <label style="display:grid;gap:4px">
          <span style="opacity:.75;font-size:10px;letter-spacing:.08em;text-transform:uppercase">Map name</span>
          <input data-daena-name-input type="text" value="Untitled map" maxlength="120" style="width:100%;box-sizing:border-box;padding:8px 9px;border:1px solid rgba(255,255,255,.2);border-radius:7px;background:#1b2420;color:#f4f1ea;font:12px system-ui,sans-serif" />
        </label>
        <div style="display:flex;justify-content:flex-end;gap:6px">
          <button data-daena-name-cancel type="button" style="appearance:none;border:1px solid rgba(255,255,255,.2);border-radius:7px;padding:6px 10px;background:transparent;color:#d7ddd6;font:12px system-ui,sans-serif;cursor:pointer">Cancel</button>
          <button data-daena-name-confirm type="submit" style="appearance:none;border:0;border-radius:7px;padding:6px 10px;background:#d5ab6c;color:#2c4032;font:700 12px system-ui,sans-serif;cursor:pointer">Save map</button>
        </div>
      </form>
      <div data-daena-back-confirm role="alertdialog" aria-label="Unsaved map changes" style="display:none;align-items:center;gap:8px;padding-top:8px;border-top:1px solid rgba(255,255,255,.14)">
        <span style="flex:1;line-height:1.3">Leave?</span>
        <button data-daena-back-leave type="button" aria-label="Leave without saving" title="Leave without saving" style="display:grid;place-items:center;width:28px;height:28px;appearance:none;border:0;border-radius:7px;padding:0;background:#d5ab6c;color:#2c4032;font:700 16px/1 system-ui,sans-serif;cursor:pointer">✓</button>
        <button data-daena-back-stay type="button" aria-label="Stay and keep editing" title="Stay and keep editing" style="display:grid;place-items:center;width:28px;height:28px;appearance:none;border:1px solid rgba(255,255,255,.2);border-radius:7px;padding:0;background:transparent;color:#d7ddd6;font:700 16px/1 system-ui,sans-serif;cursor:pointer">✕</button>
      </div>
      <button data-daena-fullscreen type="button" aria-label="Full screen" aria-pressed="false" title="Full screen" style="position:fixed;left:14px;bottom:14px;z-index:1;display:grid;place-items:center;width:32px;height:32px;appearance:none;border:1px solid rgba(255,255,255,.28);border-radius:8px;padding:0;background:rgba(32,40,36,.92);color:#f4f1ea;font:700 18px/1 system-ui,sans-serif;cursor:pointer;box-shadow:0 4px 14px #0005">⛶</button>`;
    const button = saveChrome.querySelector("[data-daena-save-button]");
    const backButton = saveChrome.querySelector("[data-daena-back]");
    const backStay = saveChrome.querySelector("[data-daena-back-stay]");
    const backLeave = saveChrome.querySelector("[data-daena-back-leave]");
    const linkOpen = saveChrome.querySelector("[data-daena-link-open]");
    const fullscreenButton = saveChrome.querySelector("[data-daena-fullscreen]");
    const form = saveChrome.querySelector("[data-daena-name-form]");
    const input = saveChrome.querySelector("[data-daena-name-input]");
    const cancel = saveChrome.querySelector("[data-daena-name-cancel]");
    button.addEventListener("click", () => {
      void requestSave().catch((error) => {
        showDiagnostic(error);
      });
    });
    backButton.addEventListener("click", () => {
      requestBack();
    });
    backStay.addEventListener("click", hideBackConfirmation);
    backLeave.addEventListener("click", () => {
      hideBackConfirmation();
      fullscreen = false;
      publishUiState("back");
    });
    linkOpen.addEventListener("click", () => {
      void requestLinkFromToolbar().catch(showDiagnostic);
    });
    fullscreenButton.addEventListener("click", () => {
      fullscreen = !fullscreen;
      fullscreenButton.textContent = "⛶";
      fullscreenButton.setAttribute("aria-label", fullscreen ? "Exit full screen" : "Full screen");
      fullscreenButton.title = fullscreen ? "Exit full screen" : "Full screen";
      fullscreenButton.setAttribute("aria-pressed", String(fullscreen));
      publishUiState("fullscreen", { enabled: fullscreen });
    });
    cancel.addEventListener("click", () => {
      hideNameForm();
    });
    form.addEventListener("submit", (event) => {
      event.preventDefault();
      const name = String(input.value || "").trim();
      if (!name) {
        input.focus();
        return;
      }
      hideNameForm();
      void commitFirstSave(name).catch((error) => {
        showDiagnostic(error);
      });
    });
    document.body.appendChild(saveChrome);
    updateSaveChrome();
    return saveChrome;
  }
  function showNameForm() {
    ensureSaveChrome();
    const form = saveChrome.querySelector("[data-daena-name-form]");
    const input = saveChrome.querySelector("[data-daena-name-input]");
    const row = saveChrome.querySelector("[data-daena-save-row]");
    form.style.display = "grid";
    row.style.display = "none";
    input.value = "Untitled map";
    input.focus();
    input.select();
  }
  function hideNameForm() {
    if (!saveChrome) return;
    const form = saveChrome.querySelector("[data-daena-name-form]");
    const row = saveChrome.querySelector("[data-daena-save-row]");
    form.style.display = "none";
    row.style.display = "flex";
    updateSaveChrome();
  }
  async function commitFirstSave(name) {
    hideNameForm();
    // Create the typed map entity only. Descriptor linking happens inside
    // maps.asset.create.commit; inline object fields are rejected by the
    // declared text schema for maps.map.
    const created = await rpc("entity.create", {
      name,
      type: "daena.maps:map",
    });
    if (!created?.id) throw new Error("map entity create did not return an id");
    mapId = created.id;
    asset.mapId = created.id;
    asset.assetId = null;
    asset.revision = null;
    asset.contentHash = null;
    await saveAsset(asset);
    void subscribeCoreEvents();
    void refreshOverlay();
  }
  async function requestSave() {
    if (savingNow) return;
    if (asset.assetId === null && !asset.mapId) {
      // Native browser prompts are unavailable in the plugin webview.
      showNameForm();
      return;
    }
    await saveAsset(asset);
  }
  let linkChrome = null;
  let linkAnchor = null;
  let linkBaseAnchor = null;
  let selectedEntityId = null;
  let pickMode = false;
  let linkArming = false;
  const CREATE_TYPES = [
    { value: "place", label: "Place" },
    { value: "person", label: "Person" },
    { value: "faction", label: "Faction" },
    { value: "artifact", label: "Artifact" },
    { value: "culture", label: "Culture" },
  ];
  function anchorLabel(anchor) {
    if (!anchor) return "Nothing selected";
    if (anchor.kind === "provider-feature")
      return `${anchor.featureKind || "feature"} ${anchor.featureId || ""}`.trim();
    if (anchor.kind === "point" && Array.isArray(anchor.point))
      return `Point (${anchor.point[0].toFixed(3)}, ${anchor.point[1].toFixed(3)})`;
    return "Selection";
  }
  function anchorPoint(anchor) {
    if (!anchor) return null;
    if (anchor.kind === "point" && Array.isArray(anchor.point)) return anchor.point;
    if (Array.isArray(anchor.fallbackPoint)) return anchor.fallbackPoint;
    return null;
  }
  function anchorsMatch(left, right) {
    if (!left || !right) return false;
    if (left.kind === "provider-feature" && right.kind === "provider-feature") {
      return left.featureKind === right.featureKind && String(left.featureId) === String(right.featureId);
    }
    if (left.kind === "point" && right.kind === "point") {
      return (
        Math.hypot((left.point?.[0] ?? 0) - (right.point?.[0] ?? 0), (left.point?.[1] ?? 0) - (right.point?.[1] ?? 0)) <
        0.01
      );
    }
    return false;
  }
  function locationAnchor(row) {
    if (row.anchorKind === "provider-feature" && row.featureKind && row.featureId) {
      return {
        kind: "provider-feature",
        provider: row.provider || "azgaar-fmg",
        featureKind: row.featureKind,
        featureId: String(row.featureId),
        fallbackPoint: overlayPoint(row) || [0.5, 0.5],
      };
    }
    const point = overlayPoint(row);
    return point ? { kind: "point", point } : null;
  }
  function updateLinkOpenButton() {
    if (!saveChrome) return;
    const button = saveChrome.querySelector("[data-daena-link-open]");
    if (!button) return;
    button.disabled = !mapId;
    button.style.opacity = mapId ? "1" : ".45";
    button.style.cursor = mapId ? "pointer" : "not-allowed";
    button.setAttribute("aria-label", linkArming ? "Click map to choose a location" : "Link location");
    button.title = linkArming ? "Click map to choose a location" : "Link location";
    button.style.background = linkArming ? "rgba(213,171,108,.22)" : "transparent";
  }
  function hideLinkChrome() {
    if (!linkChrome) return;
    linkChrome.style.display = "none";
    linkAnchor = null;
    linkBaseAnchor = null;
    selectedEntityId = null;
    linkArming = false;
    updateLinkOpenButton();
    if (pickMode) {
      pickMode = false;
      endPickUi();
      publishState("pick-cancelled", {});
    }
  }
  function ensureLinkChrome() {
    if (linkChrome && linkChrome.isConnected) return linkChrome;
    linkChrome = document.createElement("div");
    linkChrome.id = "daena-link-chrome";
    linkChrome.style.cssText =
      "position:fixed;z-index:2147483000;top:62px;right:14px;display:none;flex-direction:column;gap:10px;width:300px;max-height:calc(100vh - 90px);overflow:auto;padding:12px;border:1px solid rgba(255,255,255,.18);border-radius:10px;background:rgba(32,40,36,.96);color:#f4f1ea;font:12px system-ui,sans-serif;box-shadow:0 8px 24px #0005;pointer-events:auto";
    linkChrome.innerHTML = `
      <div style="display:flex;align-items:flex-start;justify-content:space-between;gap:8px">
        <div style="min-width:0">
          <div style="opacity:.7;font-size:10px;letter-spacing:.08em;text-transform:uppercase">Link location</div>
          <strong data-daena-link-selection style="display:block;margin-top:3px;font:600 13px system-ui,sans-serif"></strong>
        </div>
        <button data-daena-link-close type="button" style="appearance:none;border:0;background:transparent;color:#d7ddd6;font:16px system-ui,sans-serif;cursor:pointer;line-height:1">×</button>
      </div>
      <div data-daena-link-coords style="display:grid;grid-template-columns:1fr 1fr;gap:8px">
        <label style="display:grid;gap:4px">
          <span style="opacity:.75;font-size:10px;letter-spacing:.08em;text-transform:uppercase">X</span>
          <input data-daena-link-x type="number" min="0" max="1" step="0.001" style="width:100%;box-sizing:border-box;padding:7px 8px;border:1px solid rgba(255,255,255,.2);border-radius:7px;background:#1b2420;color:#f4f1ea;font:12px system-ui,sans-serif" />
        </label>
        <label style="display:grid;gap:4px">
          <span style="opacity:.75;font-size:10px;letter-spacing:.08em;text-transform:uppercase">Y</span>
          <input data-daena-link-y type="number" min="0" max="1" step="0.001" style="width:100%;box-sizing:border-box;padding:7px 8px;border:1px solid rgba(255,255,255,.2);border-radius:7px;background:#1b2420;color:#f4f1ea;font:12px system-ui,sans-serif" />
        </label>
      </div>
      <div style="display:flex;gap:6px">
        <button data-daena-link-apply-point type="button" style="flex:1;appearance:none;border:1px solid rgba(255,255,255,.2);border-radius:7px;padding:6px 8px;background:transparent;color:#d7ddd6;font:12px system-ui,sans-serif;cursor:pointer">Apply coordinates</button>
        <button data-daena-link-resnap type="button" style="flex:1;appearance:none;border:1px solid rgba(255,255,255,.2);border-radius:7px;padding:6px 8px;background:transparent;color:#d7ddd6;font:12px system-ui,sans-serif;cursor:pointer">Use map click</button>
      </div>
      <div data-daena-link-existing style="display:grid;gap:6px"></div>
      <label style="display:grid;gap:4px">
        <span style="opacity:.75;font-size:10px;letter-spacing:.08em;text-transform:uppercase">Role</span>
        <input data-daena-link-role type="text" value="story-location" maxlength="64" style="width:100%;box-sizing:border-box;padding:7px 8px;border:1px solid rgba(255,255,255,.2);border-radius:7px;background:#1b2420;color:#f4f1ea;font:12px system-ui,sans-serif" />
      </label>
      <div data-daena-link-mode style="display:flex;gap:6px">
        <button data-daena-mode-existing type="button" style="flex:1;appearance:none;border:1px solid rgba(255,255,255,.2);border-radius:7px;padding:6px 8px;background:#d5ab6c;color:#2c4032;font:700 11px system-ui,sans-serif;cursor:pointer">Existing</button>
        <button data-daena-mode-create type="button" style="flex:1;appearance:none;border:1px solid rgba(255,255,255,.2);border-radius:7px;padding:6px 8px;background:transparent;color:#d7ddd6;font:700 11px system-ui,sans-serif;cursor:pointer">Create</button>
      </div>
      <div data-daena-link-existing-form style="display:grid;gap:6px">
        <input data-daena-link-search type="search" placeholder="Search entities…" style="width:100%;box-sizing:border-box;padding:7px 8px;border:1px solid rgba(255,255,255,.2);border-radius:7px;background:#1b2420;color:#f4f1ea;font:12px system-ui,sans-serif" />
        <div data-daena-link-entity role="listbox" aria-label="Entities" style="max-height:240px;min-height:160px;overflow:auto;border:1px solid rgba(255,255,255,.2);border-radius:7px;background:#1b2420;padding:4px"></div>
        <button data-daena-link-existing-submit type="button" style="appearance:none;border:0;border-radius:7px;padding:8px 10px;background:#d5ab6c;color:#2c4032;font:700 12px system-ui,sans-serif;cursor:pointer">Link entity</button>
      </div>
      <div data-daena-link-create-form style="display:none;gap:6px">
        <input data-daena-link-name type="text" placeholder="New entry name" maxlength="120" style="width:100%;box-sizing:border-box;padding:7px 8px;border:1px solid rgba(255,255,255,.2);border-radius:7px;background:#1b2420;color:#f4f1ea;font:12px system-ui,sans-serif" />
        <select data-daena-link-type style="width:100%;box-sizing:border-box;padding:7px 8px;border:1px solid rgba(255,255,255,.2);border-radius:7px;background:#1b2420;color:#f4f1ea;font:12px system-ui,sans-serif"></select>
        <button data-daena-link-create-submit type="button" style="appearance:none;border:0;border-radius:7px;padding:8px 10px;background:#d5ab6c;color:#2c4032;font:700 12px system-ui,sans-serif;cursor:pointer">Create and link</button>
      </div>
      <div data-daena-link-status style="min-height:14px;opacity:.8;font-size:11px"></div>`;
    const typeSelect = linkChrome.querySelector("[data-daena-link-type]");
    for (const option of CREATE_TYPES) {
      const node = document.createElement("option");
      node.value = option.value;
      node.textContent = option.label;
      typeSelect.appendChild(node);
    }
    linkChrome.querySelector("[data-daena-link-close]").addEventListener("click", () => hideLinkChrome());
    linkChrome.querySelector("[data-daena-mode-existing]").addEventListener("click", () => setLinkMode("existing"));
    linkChrome.querySelector("[data-daena-mode-create]").addEventListener("click", () => setLinkMode("create"));
    linkChrome.querySelector("[data-daena-link-search]").addEventListener("input", () => {
      void refreshEntityOptions();
    });
    linkChrome.querySelector("[data-daena-link-existing-submit]").addEventListener("click", () => {
      void submitLinkExisting().catch(showDiagnostic);
    });
    linkChrome.querySelector("[data-daena-link-create-submit]").addEventListener("click", () => {
      void submitLinkCreate().catch(showDiagnostic);
    });
    linkChrome.querySelector("[data-daena-link-apply-point]").addEventListener("click", () => {
      applyEditedCoordinates();
    });
    linkChrome.querySelector("[data-daena-link-resnap]").addEventListener("click", () => {
      linkArming = true;
      updateLinkOpenButton();
      setLinkStatus("Click the map to choose a location");
    });
    document.body.appendChild(linkChrome);
    return linkChrome;
  }
  function setLinkMode(mode) {
    ensureLinkChrome();
    const existingBtn = linkChrome.querySelector("[data-daena-mode-existing]");
    const createBtn = linkChrome.querySelector("[data-daena-mode-create]");
    const existingForm = linkChrome.querySelector("[data-daena-link-existing-form]");
    const createForm = linkChrome.querySelector("[data-daena-link-create-form]");
    const active =
      "flex:1;appearance:none;border:1px solid rgba(255,255,255,.2);border-radius:7px;padding:6px 8px;background:#d5ab6c;color:#2c4032;font:700 11px system-ui,sans-serif;cursor:pointer";
    const idle =
      "flex:1;appearance:none;border:1px solid rgba(255,255,255,.2);border-radius:7px;padding:6px 8px;background:transparent;color:#d7ddd6;font:700 11px system-ui,sans-serif;cursor:pointer";
    existingBtn.style.cssText = mode === "existing" ? active : idle;
    createBtn.style.cssText = mode === "create" ? active : idle;
    existingForm.style.display = mode === "existing" ? "grid" : "none";
    createForm.style.display = mode === "create" ? "grid" : "none";
  }
  function setLinkStatus(message) {
    ensureLinkChrome();
    linkChrome.querySelector("[data-daena-link-status]").textContent = message || "";
  }
  function fillCoordinateFields(anchor) {
    ensureLinkChrome();
    const point = anchorPoint(anchor) || [0.5, 0.5];
    linkChrome.querySelector("[data-daena-link-x]").value = Number(point[0]).toFixed(3);
    linkChrome.querySelector("[data-daena-link-y]").value = Number(point[1]).toFixed(3);
  }
  function readCoordinateFields() {
    ensureLinkChrome();
    const x = Number(linkChrome.querySelector("[data-daena-link-x]").value);
    const y = Number(linkChrome.querySelector("[data-daena-link-y]").value);
    if (![x, y].every((value) => Number.isFinite(value) && value >= 0 && value <= 1)) return null;
    return [x, y];
  }
  function applyEditedCoordinates() {
    const point = readCoordinateFields();
    if (!point) {
      setLinkStatus("Coordinates must be between 0 and 1");
      return;
    }
    linkAnchor = { kind: "point", point };
    linkChrome.querySelector("[data-daena-link-selection]").textContent = anchorLabel(linkAnchor);
    setLinkStatus("Using edited point");
    void renderExistingLinks();
  }
  function resolveLinkAnchor() {
    const point = readCoordinateFields();
    if (!point) return linkAnchor;
    const base = linkBaseAnchor || linkAnchor;
    const original = anchorPoint(base);
    const unchanged =
      original && Math.abs(original[0] - point[0]) < 0.0005 && Math.abs(original[1] - point[1]) < 0.0005;
    if (unchanged && base) return base;
    return { kind: "point", point };
  }
  async function refreshEntityOptions() {
    ensureLinkChrome();
    const query = String(linkChrome.querySelector("[data-daena-link-search]").value || "")
      .trim()
      .toLowerCase();
    const list = linkChrome.querySelector("[data-daena-link-entity]");
    const entities = await rpc("entity.list", {}).catch(() => []);
    const options = (Array.isArray(entities) ? entities : [])
      .filter((entity) => entity?.entity_type !== "daena.maps:map" && entity?.type !== "daena.maps:map")
      .filter(
        (entity) =>
          !query ||
          String(entity.name || "")
            .toLowerCase()
            .includes(query) ||
          String(entity.entity_type || entity.type || "")
            .toLowerCase()
            .includes(query),
      )
      .slice(0, 120);
    list.replaceChildren();
    if (options.length === 0) {
      const empty = document.createElement("div");
      empty.style.cssText = "padding:10px 8px;opacity:.7;font-size:11px";
      empty.textContent = query ? "No matching entities." : "No entities to link yet.";
      list.appendChild(empty);
      selectedEntityId = null;
      return;
    }
    if (!options.some((entity) => entity.id === selectedEntityId)) selectedEntityId = options[0].id;
    for (const entity of options) {
      const option = document.createElement("button");
      option.type = "button";
      option.role = "option";
      option.dataset.entityId = entity.id;
      option.ariaSelected = entity.id === selectedEntityId ? "true" : "false";
      const selected = entity.id === selectedEntityId;
      option.style.cssText = `display:block;width:100%;text-align:left;padding:8px 9px;margin:0 0 3px;border:0;border-radius:6px;background:${selected ? "rgba(213,171,108,.28)" : "transparent"};color:#f4f1ea;font:12px system-ui,sans-serif;cursor:pointer`;
      option.innerHTML = `<strong style="display:block;font-size:12px">${entity.name}</strong><small style="opacity:.7">${entity.entity_type || entity.type || "entry"}</small>`;
      option.addEventListener("click", () => {
        selectedEntityId = entity.id;
        void refreshEntityOptions();
      });
      list.appendChild(option);
    }
  }
  async function renderExistingLinks() {
    ensureLinkChrome();
    const box = linkChrome.querySelector("[data-daena-link-existing]");
    box.replaceChildren();
    if (!mapId || !linkAnchor) return;
    const rows = await rpc("maps.locations.list", { mapEntityId: mapId }).catch(() => []);
    const matches = (Array.isArray(rows) ? rows : []).filter((row) => {
      const anchor = locationAnchor(row);
      return anchorsMatch(anchor, linkAnchor);
    });
    if (matches.length === 0) {
      const empty = document.createElement("div");
      empty.style.cssText = "opacity:.75;font-size:11px";
      empty.textContent = "No entities linked here yet.";
      box.appendChild(empty);
      return;
    }
    for (const row of matches) {
      const item = document.createElement("div");
      item.style.cssText =
        "display:flex;align-items:center;justify-content:space-between;gap:6px;padding:6px 7px;border:1px solid rgba(255,255,255,.12);border-radius:7px;background:#1b2420";
      item.innerHTML = `<span style="min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap"><strong style="display:block;font-size:11px">${row.label || row.role}</strong><small style="opacity:.7">${row.role}${row.resolution === "unresolved" ? " · unresolved" : ""}</small></span>`;
      const actions = document.createElement("div");
      actions.style.cssText = "display:flex;gap:4px;flex:0 0 auto";
      const open = document.createElement("button");
      open.type = "button";
      open.textContent = "Open";
      open.style.cssText =
        "appearance:none;border:1px solid rgba(255,255,255,.2);border-radius:6px;padding:4px 6px;background:transparent;color:#f4f1ea;font:11px system-ui,sans-serif;cursor:pointer";
      open.addEventListener("click", () => {
        publishState("open-entity", { entityId: row.entityId, linkId: row.id });
      });
      const unlink = document.createElement("button");
      unlink.type = "button";
      unlink.textContent = "Unlink";
      unlink.style.cssText =
        "appearance:none;border:1px solid rgba(255,255,255,.2);border-radius:6px;padding:4px 6px;background:transparent;color:#f4c7c0;font:11px system-ui,sans-serif;cursor:pointer";
      unlink.addEventListener("click", () => {
        void rpc("maps.locations.unlink", { entityId: row.entityId, locationId: row.id })
          .then(() => refreshOverlay())
          .then(() => renderExistingLinks())
          .then(() => setLinkStatus("Unlinked"))
          .catch(showDiagnostic);
      });
      actions.append(open, unlink);
      item.appendChild(actions);
      box.appendChild(item);
    }
  }
  async function openLinkChrome(anchor) {
    if (!mapId || !anchor) return;
    linkArming = false;
    updateLinkOpenButton();
    linkAnchor = anchor;
    linkBaseAnchor = anchor;
    ensureLinkChrome();
    linkChrome.style.display = "flex";
    linkChrome.querySelector("[data-daena-link-selection]").textContent = anchorLabel(anchor);
    linkChrome.querySelector("[data-daena-link-role]").value = "story-location";
    linkChrome.querySelector("[data-daena-link-name]").value =
      anchor.kind === "provider-feature"
        ? `${anchor.featureKind || "Feature"} ${anchor.featureId || ""}`.trim()
        : "Untitled place";
    fillCoordinateFields(anchor);
    setLinkMode("existing");
    setLinkStatus("");
    await refreshEntityOptions();
    await renderExistingLinks();
  }
  async function requestLinkFromToolbar() {
    if (!mapId) {
      showDiagnostic(new Error("Save the map before linking locations"));
      return;
    }
    if (linkArming) {
      linkArming = false;
      updateLinkOpenButton();
      setLinkStatus("");
      return;
    }
    const anchor = await computeSelection().catch(() => null);
    if (anchor) {
      await openLinkChrome(anchor);
      return;
    }
    linkArming = true;
    updateLinkOpenButton();
    ensureLinkChrome();
    linkChrome.style.display = "flex";
    linkChrome.querySelector("[data-daena-link-selection]").textContent = "Click the map to choose a location";
    linkChrome.querySelector("[data-daena-link-existing]").replaceChildren();
    setLinkStatus("Waiting for a map click");
  }
  function buildLocation(entityName) {
    const anchor = resolveLinkAnchor();
    if (!anchor) throw new Error("Choose a map location first");
    linkAnchor = anchor;
    return {
      id: crypto.randomUUID(),
      mapEntityId: mapId,
      role:
        String(linkChrome.querySelector("[data-daena-link-role]").value || "story-location").trim() || "story-location",
      label: entityName || "Location",
      anchor,
      validity: { from: null, to: null },
    };
  }
  async function submitLinkExisting() {
    const entityId = selectedEntityId;
    if (!entityId) {
      setLinkStatus("Choose an entity");
      return;
    }
    const selected = linkChrome.querySelector(`[data-entity-id="${entityId}"] strong`);
    const name = selected?.textContent || "Location";
    await rpc("maps.locations.upsert", { entityId, location: buildLocation(name) });
    await refreshOverlay();
    await renderExistingLinks();
    setLinkStatus("Linked");
    publishState("linked", { entityId });
  }
  async function submitLinkCreate() {
    const name = String(linkChrome.querySelector("[data-daena-link-name]").value || "").trim();
    const entityType = linkChrome.querySelector("[data-daena-link-type]").value || "place";
    if (!name) {
      setLinkStatus("Enter a name");
      return;
    }
    const created = await rpc("maps.locations.create_and_link", {
      name,
      entityType,
      location: buildLocation(name),
    });
    await refreshOverlay();
    await renderExistingLinks();
    setLinkStatus(`Created ${created?.name || name}`);
    publishState("linked", { entityId: created?.id ?? null });
  }
  function startPick() {
    if (!mapId) return;
    pickMode = false;
    hideLinkChrome();
    pickMode = true;
    linkArming = false;
    updateLinkOpenButton();
    ensureLinkChrome();
    linkChrome.style.display = "flex";
    linkChrome.querySelector("[data-daena-link-selection]").textContent = "Click the map to place this link";
    linkChrome.querySelector("[data-daena-link-existing]").replaceChildren();
    linkChrome.querySelector("[data-daena-link-existing-form]").style.display = "none";
    linkChrome.querySelector("[data-daena-link-create-form]").style.display = "none";
    linkChrome.querySelector("[data-daena-link-mode]").style.display = "none";
    linkChrome.querySelector("[data-daena-link-role]").parentElement.style.display = "none";
    linkChrome.querySelector("[data-daena-link-coords]").style.display = "none";
    linkChrome.querySelector("[data-daena-link-apply-point]").parentElement.style.display = "none";
    setLinkStatus("Pick mode");
  }
  function endPickUi() {
    ensureLinkChrome();
    linkChrome.querySelector("[data-daena-link-mode]").style.display = "flex";
    linkChrome.querySelector("[data-daena-link-role]").parentElement.style.display = "grid";
    linkChrome.querySelector("[data-daena-link-coords]").style.display = "grid";
    linkChrome.querySelector("[data-daena-link-apply-point]").parentElement.style.display = "flex";
  }
  const startDirtyWatcher = () => {
    window.setInterval(async () => {
      if (savingNow) return;
      try {
        if (!packIsReadyForSerialize()) return;
        const hash = await sha256(new TextEncoder().encode(prepareMapData()));
        if (lastSavedHash === null) setDirty(true);
        else if (hash !== lastSavedHash) setDirty(true);
        else if (dirty && hash === lastSavedHash) setDirty(false);
        updateSaveChrome();
      } catch (_) {
        /* FMG is not ready or mid-operation; try again on the next tick */
      }
    }, 1500);
  };
  const waitForProvider = () =>
    new Promise((resolve) => {
      const poll = () =>
        typeof window.uploadMap === "function" && typeof window.prepareMapData === "function"
          ? resolve()
          : setTimeout(poll, 25);
      poll();
    });
  await waitForProvider();
  // No mapEntityId means a disposable draft: generate in memory and create the
  // entity only when the in-FMG Save overlay commits a name.
  const mapAsset = requestedMapEntityId ? await firstMapAsset() : null;
  let source = null;
  if (mapAsset) {
    asset = { mapId: mapAsset.mapId, assetId: mapAsset.assetId, revision: null, contentHash: null };
    if (asset.assetId !== null) {
      const metadata = await rpc("asset.read.begin", { assetId: asset.assetId, namespace: "maps" });
      asset = {
        mapId: asset.mapId,
        assetId: asset.assetId,
        revision: metadata.revision,
        contentHash: metadata.contentHash,
      };
      source = metadata.size === 0 ? null : await loadAsset(asset.assetId);
    }
    mapId = asset.mapId;
  } else if (requestedMapEntityId) {
    throw new Error(`requested map is unavailable: ${requestedMapEntityId}`);
  }
  const featureCollections = {
    burg: "burgs",
    state: "states",
    province: "provinces",
    river: "rivers",
    marker: "markers",
  };
  const pointFor = (feature) => [feature.x / graphWidth, feature.y / graphHeight];
  const listFeatures = async (query) =>
    Object.entries(featureCollections).flatMap(([kind, collectionName]) =>
      (pack[collectionName] || [])
        .filter(
          (feature) =>
            (!query?.kind || query.kind === kind) &&
            (!query?.text ||
              String(feature.name || feature.i)
                .toLowerCase()
                .includes(query.text.toLowerCase())),
        )
        .map((feature) => ({ kind, id: String(feature.i), label: feature.name, point: pointFor(feature) })),
    );
  const resolveAnchor = async (anchor) => {
    if (anchor.kind !== "provider-feature")
      return {
        resolved: true,
        point: anchor.point || anchor.fallbackPoint || anchor.points?.[0] || anchor.rings?.[0]?.[0] || null,
      };
    const feature = (pack[featureCollections[anchor.featureKind]] || []).find(
      (item) => String(item.i) === String(anchor.featureId),
    );
    return feature ? { resolved: true, point: pointFor(feature) } : { resolved: false, point: anchor.fallbackPoint };
  };
  // Capture support. FMG's selection state is not exposed by the vendored
  // bundle, so capture uses click-to-pick: the last pointer position inside
  // the map SVG, snapped to the nearest feature within ~2% of the graph.
  let lastPointer = null;
  window.addEventListener(
    "pointerdown",
    (event) => {
      const svg = event.target?.closest?.("svg");
      if (!svg) return;
      const rect = svg.getBoundingClientRect();
      if (!rect.width || !rect.height) return;
      lastPointer = [(event.clientX - rect.left) / rect.width, (event.clientY - rect.top) / rect.height];
      if (!mapId) return;
      void computeSelection()
        .then((anchor) => {
          if (!anchor) return;
          void rpc("event.publish", { type: "daena.maps/selection@1", payload: { mapEntityId: mapId, anchor } }).catch(
            () => undefined,
          );
          if (pickMode) {
            pickMode = false;
            publishState("pick-complete", { anchor });
            hideLinkChrome();
            endPickUi();
            return;
          }
          if (linkArming) {
            void openLinkChrome(anchor).catch(showDiagnostic);
          }
        })
        .catch(() => undefined);
    },
    true,
  );
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
      return {
        kind: "provider-feature",
        provider: "azgaar-fmg",
        featureKind: nearest.kind,
        featureId: String(nearest.feature.i),
        fallbackPoint: pointFor(nearest.feature),
      };
    }
    return { kind: "point", point };
  }
  async function publishSelection() {
    if (!mapId) return;
    const anchor = await computeSelection().catch(() => null);
    void rpc("event.publish", { type: "daena.maps/selection@1", payload: { mapEntityId: mapId, anchor } }).catch(
      () => undefined,
    );
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
    // Host on body, not #map. Forcing #map to position:relative pulled the SVG
    // into document flow and pushed FMG's #collapsible options chrome below the
    // viewport. Markers use normalized 0–1 graph coords, so a viewport layer matches
    // fitMapToScreen without changing FMG layout CSS.
    overlayRoot = document.createElement("div");
    overlayRoot.id = "daena-semantic-overlay";
    overlayRoot.style.cssText = "position:fixed;inset:0;pointer-events:none;overflow:hidden;z-index:2000;";
    document.body.appendChild(overlayRoot);
    return overlayRoot;
  }
  function overlayPoint(location) {
    if (location.anchorKind === "provider-feature" && location.featureKind && location.featureId) {
      const feature = (pack[featureCollections[location.featureKind]] || []).find(
        (item) => String(item.i) === String(location.featureId),
      );
      if (feature && Number.isFinite(feature.x) && Number.isFinite(feature.y)) return pointFor(feature);
    }
    const [minX, minY, maxX, maxY] = location.bounds || [null, null, null, null];
    if (Number.isFinite(minX) && Number.isFinite(minY) && Number.isFinite(maxX) && Number.isFinite(maxY))
      return [(minX + maxX) / 2, (minY + maxY) / 2];
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
      marker.style.cssText =
        "position:absolute;width:10px;height:10px;margin:-5px 0 0 -5px;border-radius:50%;background:#e11d48;border:1.5px solid #fff;box-shadow:0 1px 3px #0006;";
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
    const location = overlayFrame.find((item) => item.id === linkId);
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
    if (!mapId || subscribeCoreEvents.started) return;
    subscribeCoreEvents.started = true;
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
      void rpc("event.publish", { type: "daena.maps/selection@1", payload: { mapEntityId: mapId, anchor } }).catch(
        () => undefined,
      );
    }, 900);
  };
  function packIsReadyForSerialize() {
    // prepareMapData() reads pack.cells.routes; uploadMap parses asynchronously, so
    // callers must wait until that field exists or they crash the child webview.
    return Boolean(window.pack?.cells && "routes" in window.pack.cells && window.pack.cells.p);
  }
  async function waitForUploadedPack(timeoutMs = 120000) {
    const started = performance.now();
    while (performance.now() - started < timeoutMs) {
      if (packIsReadyForSerialize()) {
        try {
          prepareMapData();
          return;
        } catch (_) {
          // parseLoadedData is still mid-flight after assigning cells
        }
      }
      await new Promise((resolve) => setTimeout(resolve, 50));
    }
    throw new Error("map source load timed out");
  }
  async function loadMapSource(bytes) {
    if (!bytes.length) throw new Error("map source is empty");
    // uploadMap uses FileReader and does not return a Promise; wait for pack.
    uploadMap(new File([bytes], "daena.map", { type: "application/octet-stream" }));
    await waitForUploadedPack();
  }
  async function reloadSource() {
    if (!asset.assetId) {
      showDiagnostic(new Error("this map has no saved source yet"));
      return;
    }
    const bytes = await loadAsset(asset.assetId);
    await loadMapSource(bytes);
    lastSavedHash = await sha256(new TextEncoder().encode(prepareMapData()));
    setDirty(false);
    publishState("clean");
    await refreshOverlay().catch(() => undefined);
  }
  window.daenaMapProvider = {
    provider: "azgaar-fmg",
    capabilities: async () => ({
      provider: "azgaar-fmg",
      adapterVersion: 1,
      featureKinds: Object.keys(featureCollections),
      supportsEditing: true,
    }),
    load: loadMapSource,
    serialize: () => new TextEncoder().encode(prepareMapData()),
    listFeatures,
    resolveAnchor,
    captureSelection: publishSelection,
    startPick,
    focus: async (anchor) => {
      const result = await resolveAnchor(anchor);
      if (result.point) zoomTo(result.point[0] * graphWidth, result.point[1] * graphHeight, 8, 500);
    },
    focusByLink,
    setSemanticOverlay: (frame) => {
      overlayFrame = Array.isArray(frame?.locations) ? frame.locations : null;
      setOverlayDate(frame?.date ?? null);
    },
    setDate: setOverlayDate,
    save: () => requestSave(),
    exportDraft,
    reloadSource,
    dispose: () => {
      delete window.daenaMapProvider;
    },
  };
  const originalSave = window.saveMap;
  window.saveMap = (method) =>
    method === "machine" || method === "dropbox" || method === "storage"
      ? requestSave().catch(showDiagnosticUnlessConflict)
      : originalSave?.(method);
  window.addEventListener("keydown", (event) => {
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "s") {
      event.preventDefault();
      void requestSave().catch(showDiagnosticUnlessConflict);
    }
  });
  ensureSaveChrome();
  if (source === null) {
    if (typeof window.generateMapOnLoad !== "function")
      throw new Error("new map source is empty and FMG generation is unavailable");
    await window.generateMapOnLoad();
    setDirty(true);
  } else {
    await window.daenaMapProvider.load(source);
    lastSavedHash = await sha256(new TextEncoder().encode(prepareMapData()));
    publishState("clean");
  }
  updateSaveChrome();
  void subscribeCoreEvents();
  void refreshOverlay();
  startDirtyWatcher();
  startSelectionWatcher();
})().catch((error) => {
  console.error("Daena Maps provider startup failed:", error);
  window.daenaMapDiagnostic?.(error);
});
