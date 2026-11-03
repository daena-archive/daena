var xv = Object.defineProperty;
var Ev = (t, e, r) => e in t ? xv(t, e, { enumerable: !0, configurable: !0, writable: !0, value: r }) : t[e] = r;
var Ja = (t, e, r) => Ev(t, typeof e != "symbol" ? e + "" : e, r);
const Cv = "daena.lore", Tv = "Lore", Sv = "0.1.0", kv = "daena-archive", Pv = ">=1.0.0 <2.0.0", Dv = "sandboxed", Av = { ui: "dist/ui/index.html" }, Bv = ["entity.read", "entity.write", "entity.delete", "document.read", "document.write", "field.read:self", "field.write:self", "relationship.read", "relationship.write", "asset.read:self", "asset.write:self", "asset.register", "search.query", "schema.overlay", "record.read:self", "record.write:self", "ai.text.generate-structured", "service.call:daena.maps/navigation"], Rv = { "daena.language": { version: ">=0.1.0 <1.0.0", required: !1 } }, Mv = ["lore"], Iv = [{ namespace: "lore", entityTypes: [{ id: "person", name: "Person", icon: { kind: "catalog", id: "person" }, iconColor: { kind: "preset", id: "rose" } }, { id: "place", name: "Place", icon: { kind: "catalog", id: "place" }, iconColor: { kind: "preset", id: "moss" } }, { id: "faction", name: "Faction", icon: { kind: "catalog", id: "faction" }, iconColor: { kind: "preset", id: "ember" } }, { id: "artifact", name: "Artifact", icon: { kind: "catalog", id: "artifact" }, iconColor: { kind: "preset", id: "copper" } }, { id: "culture", name: "Culture", icon: { kind: "catalog", id: "culture" }, iconColor: { kind: "preset", id: "gold" } }, { id: "concept", name: "Concept", icon: { kind: "catalog", id: "concept" }, iconColor: { kind: "preset", id: "violet" } }], fields: [{ key: "summary", label: "Summary", type: "text" }, { key: "aliases", label: "Aliases", type: "text" }, { key: "occupation", label: "Occupation", type: "text", entityTypes: ["person"], shared: !0 }, { key: "languages", label: "Languages", type: "relationship", relationshipType: "speaks", targetEntityTypes: ["daena.language:language"], entityTypes: ["person"], metadataFields: [{ key: "fluency", label: "Fluency", type: "enum", options: ["native", "fluent", "conversational", "basic", "learning"] }, { key: "dialect", label: "Dialect", type: "text" }, { key: "since", label: "Since", type: "date" }] }, { key: "origin", label: "Origin", type: "relationship", relationshipType: "originates_from", targetEntityTypes: ["place"], entityTypes: ["person"], metadataFields: [{ key: "note", label: "Note", type: "text" }] }, { key: "affiliation", label: "Affiliation", type: "relationship", relationshipType: "affiliated_with", targetEntityTypes: ["faction"], entityTypes: ["person"], metadataFields: [{ key: "start", label: "Starts", type: "date" }, { key: "end", label: "Ends", type: "date" }, { key: "role", label: "Role", type: "text" }, { key: "rank", label: "Rank", type: "enum", options: ["leader", "officer", "member", "affiliate"] }, { key: "status", label: "Status", type: "enum", options: ["active", "former", "honorary"] }] }, { key: "birth", label: "Birth", type: "date", entityTypes: ["person"], shared: !0, timeline: { role: "start", group: "life", label: "Born", layer: "lifelines" } }, { key: "death", label: "Death", type: "date", entityTypes: ["person"], shared: !0, timeline: { role: "end", group: "life", label: "Died", layer: "lifelines" } }, { key: "bornIn", label: "Born in", type: "relationship", relationshipType: "born_in", targetEntityTypes: ["place"], entityTypes: ["person"], metadataFields: [{ key: "circumstance", label: "Circumstance", type: "text" }] }, { key: "diedIn", label: "Died In", type: "relationship", relationshipType: "died_in", targetEntityTypes: ["place"], entityTypes: ["person"], metadataFields: [{ key: "circumstance", label: "Circumstance", type: "text" }] }, { key: "region", label: "Region", type: "text", entityTypes: ["place"] }, { key: "climate", label: "Climate", type: "text", entityTypes: ["place"] }, { key: "population", label: "Population", type: "number", entityTypes: ["place"] }, { key: "material", label: "Material", type: "text", entityTypes: ["artifact"] }, { key: "creator", label: "Creator", type: "relationship", relationshipType: "created_by", targetEntityTypes: ["person"], entityTypes: ["artifact"], metadataFields: [{ key: "role", label: "Role", type: "enum", options: ["creator", "forger", "commissioner"] }, { key: "commissioned", label: "Commissioned", type: "boolean" }, { key: "date", label: "Date", type: "date" }] }, { key: "currentOwner", label: "Owners", type: "relationship", relationshipType: "owned_by", targetEntityTypes: ["person"], entityTypes: ["artifact"], metadataFields: [{ key: "start", label: "Starts", type: "date" }, { key: "end", label: "Ends", type: "date" }, { key: "manner", label: "Manner", type: "enum", options: ["created", "gifted", "purchased", "inherited", "stolen", "seized"] }, { key: "condition", label: "Condition", type: "enum", options: ["pristine", "worn", "damaged"] }] }, { key: "createdAt", label: "Created at", type: "date", entityTypes: ["artifact", "place", "faction", "culture", "concept"], shared: !0, timeline: { role: "start", group: "existence", label: "Created", layer: "dates" } }, { key: "endedAt", label: "Ended at", type: "date", entityTypes: ["artifact", "place", "faction", "culture"], shared: !0, timeline: { role: "end", group: "existence", label: "Ended", layer: "dates" } }, { key: "goal", label: "Goal", type: "text", entityTypes: ["faction"] }, { key: "leader", label: "Leaders", type: "relationship", relationshipType: "led_by", targetEntityTypes: ["person"], entityTypes: ["faction"], metadataFields: [{ key: "start", label: "Starts", type: "date" }, { key: "end", label: "Ends", type: "date" }, { key: "title", label: "Title", type: "text" }, { key: "manner", label: "Manner", type: "enum", options: ["elected", "inherited", "appointed", "seized"] }] }, { key: "headquarters", label: "Headquarters", type: "relationship", relationshipType: "based_in", targetEntityTypes: ["place"], entityTypes: ["faction"], metadataFields: [{ key: "start", label: "Starts", type: "date" }, { key: "end", label: "Ends", type: "date" }, { key: "function", label: "Function", type: "enum", options: ["capital", "seat", "outpost", "hidden"] }] }, { key: "values", label: "Values", type: "text", entityTypes: ["culture"] }, { key: "language", label: "Language", type: "text", entityTypes: ["culture"] }, { key: "homeland", label: "Homeland", type: "relationship", relationshipType: "rooted_in", targetEntityTypes: ["place"], entityTypes: ["culture"], metadataFields: [{ key: "start", label: "Starts", type: "date" }, { key: "end", label: "Ends", type: "date" }, { key: "significance", label: "Significance", type: "enum", options: ["ancestral", "sacred", "contested"] }] }] }], Lv = [{ id: "person", name: "Person", entityType: "person", description: "A character with a history, voice, and place in the world.", requiredFields: [], fields: { summary: "", aliases: "", birth: "", death: "", occupation: "", languages: [], origin: [], affiliation: [], bornIn: [], diedIn: [] } }, { id: "place", name: "Place", entityType: "place", description: "A location, landmark, or region worth remembering.", requiredFields: [], fields: { summary: "", aliases: "", region: "", climate: "", population: "", createdAt: "", endedAt: "" } }, { id: "artifact", name: "Artifact", entityType: "artifact", description: "An object with meaning, history, or a story of its own.", requiredFields: [], fields: { summary: "", aliases: "", material: "", creator: [], currentOwner: [], createdAt: "", endedAt: "" } }, { id: "faction", name: "Faction", entityType: "faction", description: "A group, institution, or power with shared interests.", requiredFields: [], fields: { summary: "", aliases: "", goal: "", leader: [], headquarters: [], createdAt: "", endedAt: "" } }, { id: "culture", name: "Culture", entityType: "culture", description: "A people, tradition, or way of life that shapes the world.", requiredFields: [], fields: { summary: "", aliases: "", values: "", language: "", homeland: [], createdAt: "", endedAt: "" } }, { id: "concept", name: "Concept", entityType: "concept", description: "An idea, phenomenon, tradition, or other subject worth remembering.", fields: { summary: "", aliases: "", createdAt: "" } }], Ov = [{ id: "profile", ownerScope: "effective-schema", ownerEntityTypes: [], uniquePerOwner: !0, schema: { type: "object", properties: { schemaVersion: { type: "number" }, presetOrigin: { type: "string" }, allocation: { type: "object" }, components: { type: "array" } }, required: ["schemaVersion", "components"], additionalProperties: !1 } }, { id: "profile-preset", ownerScope: "project", schema: { type: "object", properties: { schemaVersion: { type: "number" }, name: { type: "string" }, description: { type: "string" }, icon: { type: "string" }, entityTypes: { type: "array" }, genre: { type: "string" }, builtin: { type: "boolean" }, hidden: { type: "boolean" }, customized: { type: "boolean" }, allocation: { type: "object" }, components: { type: "array" } }, required: ["schemaVersion", "name", "components"], additionalProperties: !1 } }, { id: "profile-change", ownerScope: "effective-schema", ownerEntityTypes: [], schema: { type: "object", properties: { schemaVersion: { type: "number" }, date: { type: "object" }, eventId: { type: "string" }, reason: { type: "string" }, patches: { type: "array" } }, required: ["schemaVersion", "date", "patches"], additionalProperties: !1 } }], Nv = [], zv = [{ id: "show-on-map", title: "Show selected entity on map", action: { type: "refresh-view" }, exposure: ["view"] }], Fv = { provides: [], consumes: [{ name: "daena.maps/navigation", major: 1 }] }, qv = { publishes: [], subscribes: [] }, Vv = [{ id: "lore-v1", from: 0, to: 1, recovery: "backup", operations: [{ kind: "create-namespace", namespace: "lore" }] }], _v = {
  manifestVersion: 1,
  id: Cv,
  name: Tv,
  version: Sv,
  publisher: kv,
  hostApi: Pv,
  kind: Dv,
  entrypoints: Av,
  capabilities: Bv,
  dependencies: Rv,
  namespaces: Mv,
  schemas: Iv,
  templates: Lv,
  records: Ov,
  views: Nv,
  commands: zv,
  services: Fv,
  events: qv,
  migrations: Vv
}, Gv = ["agriculture", "anchor", "animal", "art", "artifact", "bird", "calendar", "camp", "castle", "collection", "compass", "concept", "craft", "crown", "culture", "danger", "encounter", "era", "event", "faction", "fire", "fish", "flower", "forest", "group", "heart", "home", "ice", "insect", "journey", "key", "language", "law", "library", "lock", "magic", "manuscript", "map", "mine", "moon", "mountain", "music", "object", "person", "place", "plant", "reference", "religion", "science", "scroll", "settlement", "ship", "spirit", "star", "storm", "sun", "theatre", "unknown", "vision", "wand", "water", "wealth", "world"], Hv = ["brass", "copper", "ember", "moss", "pine", "ocean", "sky", "frost", "amber", "gold", "sand", "rose", "plum", "violet", "slate", "ink"], yc = ["ink", "ink-soft", "ink-faint", "ink-muted", "line", "line-soft", "line-strong", "surface", "surface-muted", "surface-warm", "surface-subtle", "surface-quiet", "canvas", "accent", "accent-dark", "accent-soft", "accent-bg", "on-accent", "on-bright-accent", "brass-ink", "danger", "danger-bg", "danger-line", "success", "success-bg", "success-line", "warning", "warning-bg", "warning-line", "info", "info-bg", "info-line", "rail-bg", "rail-surface", "rail-surface-strong", "rail-popover", "rail-text", "rail-text-soft", "rail-text-muted", "rail-text-faint", "rail-accent", "rail-accent-hover", "rail-online", "rail-offline", "rail-border"];
new Set(yc);
const $v = /^#(?:[0-9A-Fa-f]{3}|[0-9A-Fa-f]{6}|[0-9A-Fa-f]{8})$/;
function ss(t, e) {
  (t.resolved === "light" || t.resolved === "dark") && (e.dataset.theme = t.resolved, e.style.colorScheme = t.resolved), (t.preference === "light" || t.preference === "dark" || t.preference === "system") && (e.dataset.themePreference = t.preference);
  for (const r of yc) {
    const a = t.tokens[r];
    typeof a == "string" && $v.test(a) ? e.style.setProperty(`--${r}`, a) : e.style.removeProperty(`--${r}`);
  }
}
class Wv extends Error {
  constructor(r) {
    super(r.message);
    Ja(this, "code");
    Ja(this, "retryable");
    Ja(this, "details");
    this.name = "PluginRpcException", this.code = r.code, this.retryable = r.retryable, this.details = r.details;
  }
}
function si(t, e) {
  return `${t}@${e}`;
}
async function qe(t, e, r, a) {
  try {
    return await t.call(e, r, a);
  } catch (n) {
    throw us(n) ? new Wv(n) : n;
  }
}
function us(t) {
  return typeof t == "object" && t !== null && typeof t.code == "string" && typeof t.message == "string" && typeof t.retryable == "boolean";
}
function mt(t, e, r = !1, a) {
  return { code: t, message: e, retryable: r, details: a };
}
function qs(t) {
  var e;
  if (t === "pluginId" && typeof document < "u") {
    const r = (e = document.body) == null ? void 0 : e.dataset.plugin;
    if (r) return r;
  }
  if (t === "projectId" && typeof location < "u") {
    const r = new URLSearchParams(location.search).get("project");
    if (r) return r;
  }
  throw new Error(`plugin ${t} is not available in the current runtime`);
}
function Vs(t) {
  return new TextEncoder().encode(t).byteLength;
}
let _s = !1;
function Uv() {
  _s || typeof window > "u" || (_s = !0, window.addEventListener("daena:appearance", (t) => {
    const e = t.detail;
    !Nt(e) || !Nt(e.tokens) || typeof document > "u" || ss(e, document.documentElement);
  }));
}
function Kv(t, e) {
  return Nt(t) && us(t.error) ? t.error : Nt(t) && typeof t.error == "string" ? mt("transport.host", t.error) : mt("transport.protocol", e);
}
function Xv(t = {}) {
  var d;
  const e = t.pluginId ?? qs("pluginId"), r = t.projectId ?? qs("projectId"), a = t.endpoint ?? "/__rpc", n = t.fetch ?? ((d = globalThis.fetch) == null ? void 0 : d.bind(globalThis)), i = t.maxRequestBytes ?? 256 * 1024, o = t.maxResponseBytes ?? 256 * 1024;
  if (!n) throw new Error("plugin RPC requires fetch");
  let s = 0, u, l;
  async function c(y) {
    const g = JSON.stringify(y);
    if (Vs(g) > i)
      throw mt("payload.invalid", "plugin RPC request exceeds payload limit");
    let p;
    try {
      p = await n(a, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: g
      });
    } catch (w) {
      throw mt("transport.unavailable", w instanceof Error ? w.message : String(w), !0);
    }
    const m = await p.text();
    if (Vs(m) > o)
      throw mt("transport.protocol", "plugin RPC response exceeds payload limit");
    let b;
    try {
      b = JSON.parse(m);
    } catch {
      throw mt("transport.protocol", "plugin RPC returned invalid JSON");
    }
    if (!p.ok) throw Kv(b, `plugin RPC HTTP request failed (${p.status})`);
    return b;
  }
  async function f() {
    const y = await c({ op: "bootstrap", pluginId: e, projectId: r });
    if (!Nt(y) || y.rpcVersion !== 1 || y.pluginId !== e || y.projectId !== r || typeof y.sessionId != "string" || !y.sessionId || typeof y.version != "string" || typeof y.hostApi != "string" || !Array.isArray(y.grantedCapabilities) || !Array.isArray(y.optionalFeatures))
      throw mt("transport.protocol", "plugin bootstrap response is invalid");
    return u = y.sessionId, typeof document < "u" && Nt(y.appearance) && Nt(y.appearance.tokens) && ss(y.appearance, document.documentElement), y;
  }
  async function v() {
    u || (l ?? (l = f().finally(() => {
      l = void 0;
    })), await l);
  }
  async function h(y, g, p) {
    var w, C;
    if (y === "plugin.bootstrap") return f();
    await v();
    const m = p ?? ((C = (w = globalThis.crypto) == null ? void 0 : w.randomUUID) == null ? void 0 : C.call(w)) ?? `${e}-${++s}`, b = await c({
      op: "rpc",
      request: { rpcVersion: 1, sessionId: u, requestId: m, method: y, payload: g }
    });
    if (!Nt(b) || b.rpcVersion !== 1 || b.requestId !== m || typeof b.ok != "boolean")
      throw mt("transport.protocol", "plugin RPC response does not match the request");
    if (!b.ok)
      throw us(b.error) ? b.error : mt("transport.protocol", "plugin RPC returned an invalid error");
    if (!("result" in b)) throw mt("transport.protocol", "plugin RPC success has no result");
    return b.result;
  }
  return Uv(), { call: h };
}
function Nt(t) {
  return typeof t == "object" && t !== null && !Array.isArray(t);
}
function Ur(t) {
  if (!Nt(t) || typeof t.id != "string" || typeof t.name != "string" || typeof t.deleted != "boolean" || typeof t.revision != "string")
    throw mt("transport.protocol", "broker returned an invalid entity record");
  const e = t.entityType !== void 0 ? t.entityType : t.entity_type, r = t.createdAt ?? t.created_at, a = t.updatedAt ?? t.updated_at;
  if (e !== null && typeof e != "string" || typeof r != "string" || typeof a != "string")
    throw mt("transport.protocol", "broker returned an invalid entity record");
  return {
    id: t.id,
    name: t.name,
    entityType: e,
    deleted: t.deleted,
    createdAt: r,
    updatedAt: a,
    revision: t.revision
  };
}
function Yv(t) {
  if (!Nt(t) || !Array.isArray(t.items) || !Array.isArray(t.typeCounts) || typeof t.total != "number" || typeof t.offset != "number" || typeof t.limit != "number" || typeof t.hasMore != "boolean")
    throw mt("transport.protocol", "broker returned an invalid entity page");
  const e = t.typeCounts.map((r) => {
    if (!Nt(r) || typeof r.count != "number")
      throw mt("transport.protocol", "broker returned an invalid entity type count");
    const a = r.entityType;
    if (a != null && typeof a != "string")
      throw mt("transport.protocol", "broker returned an invalid entity type count");
    return { entityType: a, count: r.count };
  });
  return {
    items: t.items.map(Ur),
    total: t.total,
    offset: t.offset,
    limit: t.limit,
    hasMore: t.hasMore,
    typeCounts: e
  };
}
new Set(Gv);
new Set(Hv);
function Zv(t) {
  return {
    call: (e, r, a) => qe(t, e, r, a),
    bootstrap: () => qe(t, "plugin.bootstrap", {}),
    listEntities: async (e) => (await qe(t, "entity.list", e ? { entityType: e } : {})).map(Ur),
    queryEntities: async (e = {}) => Yv(await qe(t, "entity.query", e)),
    getEntity: async (e) => {
      const r = await qe(t, "entity.get", { id: e });
      return r == null ? null : Ur(r);
    },
    getEntities: async (e) => (await qe(t, "entity.getMany", { ids: e })).map(Ur),
    createEntity: async (e, r, a) => Ur(
      await qe(
        t,
        "entity.create",
        { name: e, type: r ?? null },
        a == null ? void 0 : a.requestId
      )
    ),
    updateEntity: async (e, r, a, n) => Ur(
      await qe(
        t,
        "entity.update",
        { id: e, name: r ?? null, type: a ?? null, expectedRevision: n == null ? void 0 : n.expectedRevision },
        n == null ? void 0 : n.requestId
      )
    ),
    deleteEntity: (e, r) => qe(
      t,
      "entity.delete",
      { id: e, expectedRevision: r == null ? void 0 : r.expectedRevision },
      r == null ? void 0 : r.requestId
    ),
    publishEvent: (e, r, a) => qe(t, "event.publish", { type: si(e, r), payload: a }),
    subscribeEvent: (e, r) => qe(t, "event.subscribe", { type: si(e, r) }),
    pollEvents: (e, r) => qe(t, "event.poll", { type: si(e, r) }),
    callService: (e, r, a, n = 5e3) => qe(t, "service.call", { name: e, major: r, payload: a, deadlineMs: n }),
    getAppVersion: () => qe(t, "app.version", {}),
    getAppearance: () => qe(t, "appearance.get", {}),
    beginAssetRead: (e, r) => qe(t, "asset.read.begin", { assetId: e, namespace: r }),
    beginSharedAssetRead: (e) => qe(t, "asset.shared.read.begin", { path: e }),
    updateAssetMetadata: (e, r) => qe(t, "asset.update", e, r == null ? void 0 : r.requestId),
    deleteAsset: (e, r) => qe(t, "asset.delete", e, r == null ? void 0 : r.requestId),
    beginAssetReplace: (e, r) => qe(t, "asset.replace.begin", e, r == null ? void 0 : r.requestId),
    commitAssetReplace: (e, r, a) => qe(t, "asset.replace.commit", { handle: e, contentHash: r }, a == null ? void 0 : a.requestId),
    cancelAssetTransfer: (e) => qe(t, "asset.transfer.cancel", { handle: e }),
    startAiRequest: (e) => qe(t, "ai.request.start", e),
    pollAiRequest: (e) => qe(t, "ai.request.poll", { requestId: e }),
    cancelAiRequest: (e) => qe(t, "ai.request.cancel", { requestId: e }),
    getAiResult: (e) => qe(t, "ai.request.result", { requestId: e }),
    getAiCitations: (e) => qe(t, "ai.request.citations", { requestId: e })
  };
}
function Qv(t) {
  return t.startsWith("family_");
}
function Jv(t) {
  return new Set([...t].filter(Qv));
}
function ls(t) {
  return t === "family_parent_of" ? "Parent" : t === "family_partner_with" ? "Partner" : t === "family_member_of" ? "House member" : t.split("_").join(" ");
}
function Tn(t) {
  return (t == null ? void 0 : t.trim()) || "entity";
}
function ui(t, e, r) {
  const a = (1 - Math.abs(2 * r - 1)) * e, n = t / 60, i = a * (1 - Math.abs(n % 2 - 1)), [o, s, u] = n < 1 ? [a, i, 0] : n < 2 ? [i, a, 0] : n < 3 ? [0, a, i] : n < 4 ? [0, i, a] : n < 5 ? [i, 0, a] : [a, 0, i], l = r - a / 2;
  return `#${[o, s, u].map(
    (c) => Math.round((c + l) * 255).toString(16).padStart(2, "0")
  ).join("")}`;
}
function jv(t) {
  return {
    fill: ui(t, 0.48, 0.87),
    border: ui(t, 0.46, 0.42),
    text: ui(t, 0.4, 0.22)
  };
}
function ed(t) {
  const e = [...new Set(t)].sort(), r = Math.max(e.length, 1);
  return new Map(e.map((a, n) => [a, jv((24 + n * 360 / r) % 360)]));
}
function td() {
  const t = document.createElement("style");
  return t.textContent = `
    .lore-graph-shell { display: grid; gap: 0; }
    .lore-graph-toolbar { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 10px 14px; border-bottom: 1px solid var(--line-soft); background: var(--surface); }
    .lore-graph-toolbar-actions { display: flex; gap: 7px; }
    .lore-graph-toolbar button { border: 1px solid var(--line-strong); border-radius: 7px; padding: 6px 9px; background: var(--surface); color: var(--ink-muted); font: 600 10px Inter, ui-sans-serif, system-ui, sans-serif; cursor: pointer; }
    .lore-graph-toolbar button:hover, .lore-graph-toolbar button:focus-visible { border-color: var(--accent); color: var(--theme-warning-text, #55351f); outline: none; }
    .lore-graph-legend { display: flex; flex-wrap: wrap; gap: 7px; min-width: 0; color: var(--ink-muted); font: 10px Inter, ui-sans-serif, system-ui, sans-serif; }
    .lore-graph-legend-item { display: inline-flex; align-items: center; gap: 5px; margin: 0; padding: 4px 8px; border: 1px solid var(--theme-warning-border, #e4d9c8); border-radius: 999px; background: var(--surface); color: var(--ink-muted); font: 600 10px Inter, ui-sans-serif, system-ui, sans-serif; cursor: pointer; }
    .lore-graph-legend-item:hover, .lore-graph-legend-item:focus-visible { border-color: var(--accent); color: var(--theme-warning-text, #55351f); outline: none; }
    .lore-graph-legend-item.is-hidden { opacity: 0.45; border-style: dashed; color: var(--ink-muted); }
    .lore-graph-legend-item.is-hidden .lore-graph-legend-swatch { background: transparent !important; }
    .lore-graph-legend-swatch { width: 8px; height: 8px; border: 1px solid currentColor; border-radius: 50%; }
    .lore-graph-relations { position: relative; }
    .lore-graph-relations summary { list-style: none; border: 1px solid var(--line-strong); border-radius: 7px; padding: 6px 9px; background: var(--surface); color: var(--ink-muted); font: 600 10px Inter, ui-sans-serif, system-ui, sans-serif; cursor: pointer; }
    .lore-graph-relations summary::-webkit-details-marker { display: none; }
    .lore-graph-relations summary:hover, .lore-graph-relations summary:focus-visible { border-color: var(--accent); color: var(--theme-warning-text, #55351f); outline: none; }
    .lore-graph-relations[open] summary { border-color: var(--accent); color: var(--theme-warning-text, #55351f); }
    .lore-graph-relations-menu { position: absolute; right: 0; top: calc(100% + 6px); z-index: 8; display: grid; gap: 2px; min-width: 220px; max-width: min(320px, 70vw); max-height: min(50vh, 320px); overflow: auto; padding: 8px; border: 1px solid var(--line-strong); border-radius: 10px; background: var(--surface); box-shadow: 0 10px 28px color-mix(in srgb, var(--ink, #302c26) 16%, transparent); }
    .lore-graph-relations-option { display: flex; align-items: center; gap: 8px; margin: 0; padding: 6px 8px; border-radius: 7px; color: var(--ink-muted); font: 600 11px Inter, ui-sans-serif, system-ui, sans-serif; cursor: pointer; }
    .lore-graph-relations-option:hover { background: var(--surface-warm); color: var(--theme-warning-text, #55351f); }
    .lore-graph-relations-option input { margin: 0; }
    .lore-graph-canvas { position: relative; height: min(58vh, 560px); min-height: 380px; background: radial-gradient(circle at 50% 42%, var(--theme-warning-bg, #fffdf7) 0, var(--theme-warning-bg, #fbf8f0) 52%, var(--surface-warm) 100%); }
    .lore-graph-details { display: grid; gap: 7px; min-height: 55px; padding: 12px 15px; border-top: 1px solid var(--line-soft); background: var(--surface); color: var(--ink-muted); font: 11px/1.45 Inter, ui-sans-serif, system-ui, sans-serif; }
    .lore-graph-details strong { color: var(--theme-neutral-text, #302c26); font: 500 16px/1.1 var(--font-display, Georgia, serif); }
    .lore-graph-details small { color: var(--ink-muted); }
    .lore-graph-details-list { display: flex; flex-wrap: wrap; gap: 5px 10px; margin: 0; padding: 0; list-style: none; }
    .lore-graph-details-list li { padding: 3px 7px; border-radius: 999px; background: var(--surface-warm); }
    .lore-graph-map-button { width: fit-content; padding: 5px 9px; border: 1px solid var(--line-strong); border-radius: 7px; background: var(--surface); color: var(--ink-muted); font: 600 10px Inter, ui-sans-serif, system-ui, sans-serif; cursor: pointer; }
    .lore-graph-map-button:hover, .lore-graph-map-button:focus-visible { border-color: var(--accent); color: var(--theme-warning-text, #55351f); outline: none; }
    .lore-graph-empty { margin: 0; padding: 28px 18px; color: var(--ink-muted); font: 12px/1.5 Inter, ui-sans-serif, system-ui, sans-serif; }
    @media (max-width: 760px) {
      .lore-graph-toolbar { align-items: flex-start; flex-direction: column; }
      .lore-graph-canvas { height: 420px; min-height: 320px; }
    }
  `, t;
}
function rd(t, e) {
  return t.name.localeCompare(e.name) || t.id.localeCompare(e.id);
}
function ad(t) {
  const e = Math.max(3, Math.ceil(Math.sqrt(t.length))), r = 360, a = 270, n = 220, i = 170;
  return new Map(
    [...t].sort(rd).map((o, s) => [
      o.id,
      {
        x: n + s % e * r,
        y: i + Math.floor(s / e) * a
      }
    ])
  );
}
function nd(t) {
  const e = t.nodes().toArray().sort((a, n) => a.id().localeCompare(n.id())), r = 36;
  for (let a = 0; a < 32; a += 1) {
    let n = !1;
    for (let i = 0; i < e.length; i += 1) {
      const o = e[i], s = o.boundingBox();
      for (let u = i + 1; u < e.length; u += 1) {
        const l = e[u], c = l.boundingBox(), f = Math.min(s.x2, c.x2) - Math.max(s.x1, c.x1) + r, v = Math.min(s.y2, c.y2) - Math.max(s.y1, c.y1) + r;
        if (f <= 0 || v <= 0) continue;
        const h = o.position(), d = l.position(), y = d.x > h.x || d.x === h.x && u % 2 === 0 ? 1 : -1, g = d.y > h.y || d.y === h.y && u % 2 === 0 ? 1 : -1;
        if (f <= v) {
          const p = f / 2;
          o.position({ x: h.x - y * p, y: h.y }), l.position({ x: d.x + y * p, y: d.y });
        } else {
          const p = v / 2;
          o.position({ x: h.x, y: h.y - g * p }), l.position({ x: d.x, y: d.y + g * p });
        }
        n = !0;
      }
    }
    if (!n) break;
  }
}
function id(t, e, r, a) {
  const n = new Set(t.map((i) => i.id));
  return [
    ...t.map((i) => ({
      group: "nodes",
      position: r.get(i.id),
      data: {
        id: i.id,
        label: i.name,
        type: i.type ?? "",
        typeLabel: Tn(i.type),
        ...a.get(Tn(i.type))
      }
    })),
    ...e.filter((i) => n.has(i.sourceId) && n.has(i.targetId)).map((i) => ({
      group: "edges",
      data: {
        id: i.id,
        source: i.sourceId,
        target: i.targetId,
        label: ls(i.type),
        relationType: i.type
      }
    }))
  ];
}
function od(t) {
  return typeof t.id == "function";
}
function Gs(t, e, r, a, n, i) {
  const o = od(e) ? r.get(e.id()) : e;
  if (!o) return;
  const s = a.filter(
    (v) => !i.has(v.type) && (v.sourceId === o.id || v.targetId === o.id)
  ).map((v) => {
    const h = v.sourceId === o.id ? v.targetId : v.sourceId, d = r.get(h);
    return d ? `${ls(v.type)} · ${d.name}` : null;
  }).filter((v) => !!v);
  t.replaceChildren();
  const u = document.createElement("strong");
  u.textContent = o.name;
  const l = document.createElement("small");
  l.textContent = `${Tn(o.type)} · ${s.length} connection${s.length === 1 ? "" : "s"}`;
  const c = document.createElement("div");
  if (c.className = "lore-graph-toolbar-actions", n.mapsNavigationEnabled()) {
    const v = document.createElement("button");
    v.type = "button", v.className = "lore-graph-map-button", v.textContent = "Show on map", v.onclick = () => void ud(n, o.id), c.append(v);
  }
  if (t.append(u, l, c), s.length === 0) return;
  const f = document.createElement("ul");
  f.className = "lore-graph-details-list";
  for (const v of s.slice(0, 8)) {
    const h = document.createElement("li");
    h.textContent = v, f.append(h);
  }
  t.append(f);
}
function zr(t, e, r = !1) {
  if (t.elements().removeClass("faded related hover-related"), !e || e.hasClass("type-hidden")) return;
  const a = e.closedNeighborhood().not(".type-hidden");
  t.elements().not(".type-hidden").difference(a).addClass("faded"), e.connectedEdges().not(".type-hidden").addClass("related"), r && e.connectedEdges().not(".type-hidden").addClass("hover-related");
}
function sd(t, e, r) {
  t.nodes().forEach((a) => {
    const n = e.has(String(a.data("typeLabel") ?? "entity"));
    a.toggleClass("type-hidden", n), n && a.removeClass("hovered selected");
  }), t.edges().forEach((a) => {
    const n = r.has(String(a.data("relationType") ?? "")) || a.source().hasClass("type-hidden") || a.target().hasClass("type-hidden");
    a.toggleClass("type-hidden", n), n && a.removeClass("related selected hover-related");
  });
}
async function ud(t, e) {
  try {
    const r = await t.focusEntity(e);
    if ((r == null ? void 0 : r.status) === "multiple-links" && Array.isArray(r.locations) && r.locations.length > 0) {
      const a = r.locations[0];
      a.mapEntityId && a.id && await t.openMap({ mapEntityId: a.mapEntityId, linkId: a.id });
    }
  } catch (r) {
    const a = r instanceof Error ? r.message : String(r);
    if (a.includes("not-on-map") || a.includes("link-unresolved") || a.includes("map-unavailable") || a.includes("forbidden") || a.includes("denied"))
      return;
    console.error("show on map failed", r);
  }
}
function ld(t, e) {
  let r = !1, a = null;
  const n = new AbortController(), i = td();
  return (async () => {
    var ae;
    const s = e.module.schemas.flatMap(
      (z) => z.entityTypes.map((L) => L.id)
    ), u = new Map(
      e.module.schemas.flatMap(
        (z) => z.entityTypes.map((L) => [L.id, L.name])
      )
    ), l = await e.listEntities(s), c = (await Promise.all(l.map((z) => e.listRelationships(z.id)))).flat().filter((z, L, V) => V.findIndex((W) => W.id === z.id) === L).filter(
      (z) => l.some((L) => L.id === z.sourceId) && l.some((L) => L.id === z.targetId)
    ).sort((z, L) => z.id.localeCompare(L.id));
    if (r) return;
    t.replaceChildren(), t.className = "projection-graph";
    const f = document.createElement("div");
    if (f.className = "lore-graph-shell", (ae = e.reportSurfaceMeta) == null || ae.call(e, {
      subtitle: `${l.length} entities · ${c.length} links`
    }), f.append(i), l.length === 0) {
      const z = document.createElement("p");
      z.className = "lore-graph-empty", z.textContent = "No entities yet.", f.append(z), t.append(f);
      return;
    }
    const v = document.createElement("div");
    v.className = "lore-graph-toolbar";
    const h = document.createElement("div");
    h.className = "lore-graph-legend";
    const d = [...new Set(l.map((z) => Tn(z.type)))].sort(), y = ed(d), g = /* @__PURE__ */ new Set(), p = [...new Set(c.map((z) => z.type))].sort(), m = Jv(p), b = /* @__PURE__ */ new Map();
    for (const z of d) {
      const L = document.createElement("button");
      L.type = "button", L.className = "lore-graph-legend-item", L.setAttribute("aria-pressed", "true"), L.title = z;
      const V = document.createElement("span");
      V.className = "lore-graph-legend-swatch";
      const W = y.get(z);
      V.style.color = W.border, V.style.background = W.fill;
      const K = document.createElement("span");
      K.textContent = u.get(z) ?? z, L.append(V, K), b.set(z, L), h.append(L);
    }
    const w = document.createElement("div");
    w.className = "lore-graph-toolbar-actions";
    const C = /* @__PURE__ */ new Map(), E = document.createElement("details"), x = document.createElement("summary"), T = () => {
      const z = m.size;
      x.textContent = z > 0 ? `Relations · ${z} hidden` : "Relations";
    };
    if (p.length > 0) {
      E.className = "lore-graph-relations", x.textContent = "Relations";
      const z = document.createElement("div");
      z.className = "lore-graph-relations-menu", z.setAttribute("role", "group"), z.setAttribute("aria-label", "Relations");
      for (const L of p) {
        const V = document.createElement("label");
        V.className = "lore-graph-relations-option", V.title = L;
        const W = document.createElement("input");
        W.type = "checkbox", W.checked = !m.has(L);
        const K = document.createElement("span");
        K.textContent = ls(L), V.append(W, K), C.set(L, W), z.append(V);
      }
      E.append(x, z), T(), document.addEventListener(
        "pointerdown",
        (L) => {
          E.open && (L.target instanceof Node && E.contains(L.target) || (E.open = !1));
        },
        { signal: n.signal }
      ), w.append(E);
    }
    const k = document.createElement("button");
    k.type = "button", k.textContent = "−", k.setAttribute("aria-label", "Zoom out"), k.title = "Zoom out";
    const D = document.createElement("button");
    D.type = "button", D.textContent = "+", D.setAttribute("aria-label", "Zoom in"), D.title = "Zoom in";
    const P = document.createElement("button");
    P.type = "button", P.textContent = "Fit graph";
    const A = document.createElement("button");
    A.type = "button", A.textContent = "Rearrange", w.append(k, D, P, A), v.append(h, w), f.append(v);
    const B = document.createElement("div");
    B.className = "lore-graph-canvas", f.append(B);
    const R = document.createElement("div");
    R.className = "lore-graph-details";
    const I = document.createElement("small");
    I.textContent = "Hover or select an entity to inspect its connections. Click a type to show or hide it. Use Relations to filter connection types.", R.append(I), f.append(R), t.append(f);
    const O = new Map(l.map((z) => [z.id, z])), M = ad(l), { default: N } = await Promise.resolve().then(() => Y0);
    if (r) return;
    const H = {
      animate: !1,
      fit: !1,
      padding: 72,
      randomize: !1,
      avoidOverlap: !0,
      nodeDimensionsIncludeLabels: !0,
      componentSpacing: 88,
      nodeRepulsion: 18e3,
      nodeOverlap: 56,
      idealEdgeLength: 340,
      edgeElasticity: 0.18,
      gravity: 0.55,
      numIter: 3e3
    };
    a = N({
      container: B,
      elements: id(l, c, M, y),
      minZoom: 0.25,
      maxZoom: 3,
      wheelSensitivity: 0.18,
      style: [
        {
          selector: "node",
          style: {
            "background-color": "data(fill)",
            "border-color": "data(border)",
            "border-width": 2,
            color: "data(text)",
            label: "data(label)",
            "font-family": "Inter, ui-sans-serif, system-ui, sans-serif",
            "font-size": 11,
            "font-weight": 600,
            "text-wrap": "ellipsis",
            "text-max-width": "112px",
            "text-valign": "center",
            "text-halign": "center",
            width: "label",
            height: 34,
            padding: "10px",
            shape: "round-rectangle",
            "overlay-opacity": 0
          }
        },
        {
          selector: "edge",
          style: {
            width: 1.6,
            "line-color": "#c6b9a7",
            "target-arrow-color": "#c6b9a7",
            "target-arrow-shape": "triangle",
            "curve-style": "bezier",
            label: "",
            color: "#8d8273",
            "font-family": "Inter, ui-sans-serif, system-ui, sans-serif",
            "font-size": 10,
            "text-wrap": "ellipsis",
            "text-max-width": "130px",
            "text-background-color": "#fffefa",
            "text-background-opacity": 0.9,
            "text-background-padding": "2px",
            "text-rotation": "autorotate",
            "overlay-opacity": 0
          }
        },
        {
          selector: "edge.related, edge.selected",
          style: {
            label: "",
            width: 2.4,
            "line-color": "#a87945",
            "target-arrow-color": "#a87945",
            color: "#6f4d2f"
          }
        },
        {
          selector: "edge.hover-related",
          style: { label: "data(label)" }
        },
        { selector: ".faded", style: { opacity: 0.16 } },
        { selector: ".hovered", style: { "border-width": 3, "border-color": "#8b5c2e" } },
        { selector: "node.selected", style: { "border-width": 4, "border-color": "#8b5c2e" } },
        { selector: ".type-hidden", style: { display: "none" } }
      ]
    });
    const G = () => a.elements().not(".type-hidden"), F = () => {
      if (!a) return;
      const z = a.edges().not(".type-hidden").length > 0;
      a.layout({ ...H, name: z ? "cose" : "grid" }).run(), nd(a);
      const L = G();
      L.length > 0 && a.fit(L, 72);
    }, _ = (z = !1) => {
      if (!a) return;
      sd(a, g, m);
      const L = a.nodes("node.selected").not(".type-hidden");
      L.length === 0 ? (a.elements().removeClass("selected"), zr(a, null), R.replaceChildren(I)) : (zr(a, L[0]), Gs(R, L[0], O, c, e, m)), z && F();
    };
    _(!0);
    let X = !1;
    const J = () => {
      X = !0;
    }, Y = () => {
      if (!a) return;
      const z = G();
      z.length > 0 && a.fit(z, 72);
    }, j = new ResizeObserver(() => {
      a && (a.resize(), !X && a.width() > 0 && a.height() > 0 && Y());
    });
    j.observe(B), n.signal.addEventListener("abort", () => j.disconnect()), B.addEventListener(
      "pointermove",
      (z) => {
        z.buttons !== 0 && J();
      },
      { signal: n.signal }
    ), B.addEventListener("wheel", J, { passive: !0, signal: n.signal });
    for (const [z, L] of b)
      L.onclick = () => {
        g.has(z) ? (g.delete(z), L.classList.remove("is-hidden"), L.setAttribute("aria-pressed", "true"), L.title = z) : (g.add(z), L.classList.add("is-hidden"), L.setAttribute("aria-pressed", "false"), L.title = z), _();
      };
    for (const [z, L] of C)
      L.onchange = () => {
        L.checked ? m.delete(z) : m.add(z), T(), _(!0);
      };
    const te = (z) => {
      a && (J(), a.zoom({
        level: Math.min(a.maxZoom(), Math.max(a.minZoom(), a.zoom() * z)),
        renderedPosition: { x: a.width() / 2, y: a.height() / 2 }
      }));
    };
    D.onclick = () => te(1.4), k.onclick = () => te(1 / 1.4), P.onclick = Y, A.onclick = () => {
      a && (a.nodes().forEach((z) => {
        const L = M.get(z.id());
        L && z.position(L);
      }), F());
    }, a.on("mouseover", "node", (z) => {
      const L = z.target;
      L.hasClass("type-hidden") || (L.addClass("hovered"), zr(a, L, !0));
    }), a.on("mouseout", "node", (z) => {
      z.target.removeClass("hovered");
      const L = a.nodes("node.selected").not(".type-hidden");
      zr(a, L.length > 0 ? L[0] : null);
    }), a.on("tap", "node", (z) => {
      const L = z.target;
      L.hasClass("type-hidden") || (a.elements().removeClass("selected"), L.addClass("selected"), L.connectedEdges().not(".type-hidden").addClass("selected"), Gs(R, L, O, c, e, m), zr(a, L));
    }), a.on("tap", (z) => {
      z.target === a && (a.elements().removeClass("selected"), zr(a, null), R.replaceChildren(I));
    });
  })().catch((s) => {
    if (r) return;
    const u = s instanceof Error ? s.message : String(s), l = document.createElement("p");
    l.className = "lore-graph-empty", l.textContent = u, t.replaceChildren(i, l);
  }), () => {
    r = !0, n.abort(), a == null || a.destroy(), a = null, t.replaceChildren();
  };
}
const Vo = "daena.maps/navigation";
function cd(t) {
  return {
    id: t.id,
    name: t.name,
    type: t.entityType ?? t.entity_type ?? t.type ?? null
  };
}
function fd(t) {
  return {
    id: t.id,
    sourceId: t.sourceId ?? t.source_id ?? "",
    targetId: t.targetId ?? t.target_id ?? "",
    type: t.relationshipType ?? t.relationship_type ?? ""
  };
}
function vd(t) {
  return t.some(
    (e) => {
      var r;
      return e.id === "daena.maps" && (((r = e.services) == null ? void 0 : r.provides) ?? []).some((a) => a.name === Vo && a.major === 1);
    }
  );
}
async function dd(t, e) {
  const r = await t.call("schema.list", {}), a = vd(r);
  return {
    module: r.find((i) => i.id === e.id) ?? e,
    mapsNavigationEnabled: () => a,
    async listEntities(i) {
      const o = [];
      let s = 0;
      for (; ; ) {
        const u = await t.call("entity.query", {
          entityTypes: [...i],
          excludedEntityTypes: [],
          offset: s,
          limit: 200
        });
        if (o.push(...u.items.map(cd)), !u.hasMore || u.items.length === 0) return o;
        s += u.items.length;
      }
    },
    async listRelationships(i) {
      return (await t.call("relationship.list", { entityId: i })).map(fd);
    },
    async focusEntity(i) {
      return t.call("service.call", {
        name: Vo,
        major: 1,
        payload: { operation: "focusEntity", entityId: i }
      });
    },
    async openMap(i) {
      await t.call("service.call", {
        name: Vo,
        major: 1,
        payload: { operation: "openMap", mapEntityId: i.mapEntityId, linkId: i.linkId }
      });
    }
  };
}
const hd = _v, Sn = document.querySelector("#lore-graph-root"), kn = document.querySelector("#lore-status");
async function gd() {
  if (!Sn) throw new Error("World graph root is missing");
  const t = Zv(Xv()), e = await t.bootstrap();
  ss(e.appearance, document.documentElement);
  const r = await dd(t, hd);
  r.reportSurfaceMeta = (a) => {
    kn && (kn.textContent = a.subtitle);
  }, ld(Sn, r);
}
gd().catch((t) => {
  const e = t instanceof Error ? t.message : String(t);
  kn ? kn.textContent = e : Sn && (Sn.textContent = e);
});
function _o(t, e) {
  (e == null || e > t.length) && (e = t.length);
  for (var r = 0, a = Array(e); r < e; r++) a[r] = t[r];
  return a;
}
function pd(t) {
  if (Array.isArray(t)) return t;
}
function yd(t) {
  if (Array.isArray(t)) return _o(t);
}
function pr(t, e) {
  if (!(t instanceof e)) throw new TypeError("Cannot call a class as a function");
}
function md(t, e) {
  for (var r = 0; r < e.length; r++) {
    var a = e[r];
    a.enumerable = a.enumerable || !1, a.configurable = !0, "value" in a && (a.writable = !0), Object.defineProperty(t, bc(a.key), a);
  }
}
function yr(t, e, r) {
  return e && md(t.prototype, e), Object.defineProperty(t, "prototype", {
    writable: !1
  }), t;
}
function St(t, e) {
  var r = typeof Symbol < "u" && t[Symbol.iterator] || t["@@iterator"];
  if (!r) {
    if (Array.isArray(t) || (r = cs(t)) || e) {
      r && (t = r);
      var a = 0, n = function() {
      };
      return {
        s: n,
        n: function() {
          return a >= t.length ? {
            done: !0
          } : {
            done: !1,
            value: t[a++]
          };
        },
        e: function(u) {
          throw u;
        },
        f: n
      };
    }
    throw new TypeError(`Invalid attempt to iterate non-iterable instance.
In order to be iterable, non-array objects must have a [Symbol.iterator]() method.`);
  }
  var i, o = !0, s = !1;
  return {
    s: function() {
      r = r.call(t);
    },
    n: function() {
      var u = r.next();
      return o = u.done, u;
    },
    e: function(u) {
      s = !0, i = u;
    },
    f: function() {
      try {
        o || r.return == null || r.return();
      } finally {
        if (s) throw i;
      }
    }
  };
}
function mc(t, e, r) {
  return (e = bc(e)) in t ? Object.defineProperty(t, e, {
    value: r,
    enumerable: !0,
    configurable: !0,
    writable: !0
  }) : t[e] = r, t;
}
function bd(t) {
  if (typeof Symbol < "u" && t[Symbol.iterator] != null || t["@@iterator"] != null) return Array.from(t);
}
function wd(t, e) {
  var r = t == null ? null : typeof Symbol < "u" && t[Symbol.iterator] || t["@@iterator"];
  if (r != null) {
    var a, n, i, o, s = [], u = !0, l = !1;
    try {
      if (i = (r = r.call(t)).next, e === 0) {
        if (Object(r) !== r) return;
        u = !1;
      } else for (; !(u = (a = i.call(r)).done) && (s.push(a.value), s.length !== e); u = !0) ;
    } catch (c) {
      l = !0, n = c;
    } finally {
      try {
        if (!u && r.return != null && (o = r.return(), Object(o) !== o)) return;
      } finally {
        if (l) throw n;
      }
    }
    return s;
  }
}
function xd() {
  throw new TypeError(`Invalid attempt to destructure non-iterable instance.
In order to be iterable, non-array objects must have a [Symbol.iterator]() method.`);
}
function Ed() {
  throw new TypeError(`Invalid attempt to spread non-iterable instance.
In order to be iterable, non-array objects must have a [Symbol.iterator]() method.`);
}
function Je(t, e) {
  return pd(t) || wd(t, e) || cs(t, e) || xd();
}
function Pn(t) {
  return yd(t) || bd(t) || cs(t) || Ed();
}
function Cd(t, e) {
  if (typeof t != "object" || !t) return t;
  var r = t[Symbol.toPrimitive];
  if (r !== void 0) {
    var a = r.call(t, e);
    if (typeof a != "object") return a;
    throw new TypeError("@@toPrimitive must return a primitive value.");
  }
  return String(t);
}
function bc(t) {
  var e = Cd(t, "string");
  return typeof e == "symbol" ? e : e + "";
}
function rt(t) {
  "@babel/helpers - typeof";
  return rt = typeof Symbol == "function" && typeof Symbol.iterator == "symbol" ? function(e) {
    return typeof e;
  } : function(e) {
    return e && typeof Symbol == "function" && e.constructor === Symbol && e !== Symbol.prototype ? "symbol" : typeof e;
  }, rt(t);
}
function cs(t, e) {
  if (t) {
    if (typeof t == "string") return _o(t, e);
    var r = {}.toString.call(t).slice(8, -1);
    return r === "Object" && t.constructor && (r = t.constructor.name), r === "Map" || r === "Set" ? Array.from(t) : r === "Arguments" || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(r) ? _o(t, e) : void 0;
  }
}
var et = typeof window > "u" ? null : window, Hs = et ? et.navigator : null;
et && et.document;
var Td = rt(""), wc = rt({}), Sd = rt(function() {
}), kd = typeof HTMLElement > "u" ? "undefined" : rt(HTMLElement), Fa = function(e) {
  return e && e.instanceString && Ue(e.instanceString) ? e.instanceString() : null;
}, he = function(e) {
  return e != null && rt(e) == Td;
}, Ue = function(e) {
  return e != null && rt(e) === Sd;
}, Ve = function(e) {
  return !kt(e) && (Array.isArray ? Array.isArray(e) : e != null && e instanceof Array);
}, Me = function(e) {
  return e != null && rt(e) === wc && !Ve(e) && e.constructor === Object;
}, Pd = function(e) {
  return e != null && rt(e) === wc;
}, ne = function(e) {
  return e != null && rt(e) === rt(1) && !isNaN(e);
}, Dd = function(e) {
  return ne(e) && Math.floor(e) === e;
}, Dn = function(e) {
  if (kd !== "undefined")
    return e != null && e instanceof HTMLElement;
}, kt = function(e) {
  return qa(e) || xc(e);
}, qa = function(e) {
  return Fa(e) === "collection" && e._private.single;
}, xc = function(e) {
  return Fa(e) === "collection" && !e._private.single;
}, fs = function(e) {
  return Fa(e) === "core";
}, Ec = function(e) {
  return Fa(e) === "stylesheet";
}, Ad = function(e) {
  return Fa(e) === "event";
}, cr = function(e) {
  return e == null ? !0 : !!(e === "" || e.match(/^\s+$/));
}, Bd = function(e) {
  return typeof HTMLElement > "u" ? !1 : e instanceof HTMLElement;
}, Rd = function(e) {
  return Me(e) && ne(e.x1) && ne(e.x2) && ne(e.y1) && ne(e.y2);
}, Md = function(e) {
  return Pd(e) && Ue(e.then);
}, Id = function() {
  return Hs && Hs.userAgent.match(/msie|trident|edge/i);
}, ta = function(e, r) {
  r || (r = function() {
    if (arguments.length === 1)
      return arguments[0];
    if (arguments.length === 0)
      return "undefined";
    for (var i = [], o = 0; o < arguments.length; o++)
      i.push(arguments[o]);
    return i.join("$");
  });
  var a = function() {
    var i = this, o = arguments, s, u = r.apply(i, o), l = a.cache;
    return (s = l[u]) || (s = l[u] = e.apply(i, o)), s;
  };
  return a.cache = {}, a;
}, vs = ta(function(t) {
  return t.replace(/([A-Z])/g, function(e) {
    return "-" + e.toLowerCase();
  });
}), _n = ta(function(t) {
  return t.replace(/(-\w)/g, function(e) {
    return e[1].toUpperCase();
  });
}), Cc = ta(function(t, e) {
  return t + e[0].toUpperCase() + e.substring(1);
}, function(t, e) {
  return t + "$" + e;
}), $s = function(e) {
  return cr(e) ? e : e.charAt(0).toUpperCase() + e.substring(1);
}, ir = function(e, r) {
  return e.slice(-1 * r.length) === r;
}, tt = "(?:[-+]?(?:(?:\\d+|\\d*\\.\\d+)(?:[Ee][+-]?\\d+)?))", Ld = "rgb[a]?\\((" + tt + "[%]?)\\s*,\\s*(" + tt + "[%]?)\\s*,\\s*(" + tt + "[%]?)(?:\\s*,\\s*(" + tt + "))?\\)", Od = "rgb[a]?\\((?:" + tt + "[%]?)\\s*,\\s*(?:" + tt + "[%]?)\\s*,\\s*(?:" + tt + "[%]?)(?:\\s*,\\s*(?:" + tt + "))?\\)", Nd = "hsl[a]?\\((" + tt + ")\\s*,\\s*(" + tt + "[%])\\s*,\\s*(" + tt + "[%])(?:\\s*,\\s*(" + tt + "))?\\)", zd = "hsl[a]?\\((?:" + tt + ")\\s*,\\s*(?:" + tt + "[%])\\s*,\\s*(?:" + tt + "[%])(?:\\s*,\\s*(?:" + tt + "))?\\)", Fd = "\\#[0-9a-fA-F]{3}", qd = "\\#[0-9a-fA-F]{6}", Tc = function(e, r) {
  return e < r ? -1 : e > r ? 1 : 0;
}, Vd = function(e, r) {
  return -1 * Tc(e, r);
}, ye = Object.assign != null ? Object.assign.bind(Object) : function(t) {
  for (var e = arguments, r = 1; r < e.length; r++) {
    var a = e[r];
    if (a != null)
      for (var n = Object.keys(a), i = 0; i < n.length; i++) {
        var o = n[i];
        t[o] = a[o];
      }
  }
  return t;
}, _d = function(e) {
  if (!(!(e.length === 4 || e.length === 7) || e[0] !== "#")) {
    var r = e.length === 4, a, n, i, o = 16;
    return r ? (a = parseInt(e[1] + e[1], o), n = parseInt(e[2] + e[2], o), i = parseInt(e[3] + e[3], o)) : (a = parseInt(e[1] + e[2], o), n = parseInt(e[3] + e[4], o), i = parseInt(e[5] + e[6], o)), [a, n, i];
  }
}, Gd = function(e) {
  var r, a, n, i, o, s, u, l;
  function c(d, y, g) {
    return g < 0 && (g += 1), g > 1 && (g -= 1), g < 1 / 6 ? d + (y - d) * 6 * g : g < 1 / 2 ? y : g < 2 / 3 ? d + (y - d) * (2 / 3 - g) * 6 : d;
  }
  var f = new RegExp("^" + Nd + "$").exec(e);
  if (f) {
    if (a = parseInt(f[1]), a < 0 ? a = (360 - -1 * a % 360) % 360 : a > 360 && (a = a % 360), a /= 360, n = parseFloat(f[2]), n < 0 || n > 100 || (n = n / 100, i = parseFloat(f[3]), i < 0 || i > 100) || (i = i / 100, o = f[4], o !== void 0 && (o = parseFloat(o), o < 0 || o > 1)))
      return;
    if (n === 0)
      s = u = l = Math.round(i * 255);
    else {
      var v = i < 0.5 ? i * (1 + n) : i + n - i * n, h = 2 * i - v;
      s = Math.round(255 * c(h, v, a + 1 / 3)), u = Math.round(255 * c(h, v, a)), l = Math.round(255 * c(h, v, a - 1 / 3));
    }
    r = [s, u, l, o];
  }
  return r;
}, Hd = function(e) {
  var r, a = new RegExp("^" + Ld + "$").exec(e);
  if (a) {
    r = [];
    for (var n = [], i = 1; i <= 3; i++) {
      var o = a[i];
      if (o[o.length - 1] === "%" && (n[i] = !0), o = parseFloat(o), n[i] && (o = o / 100 * 255), o < 0 || o > 255)
        return;
      r.push(Math.floor(o));
    }
    var s = n[1] || n[2] || n[3], u = n[1] && n[2] && n[3];
    if (s && !u)
      return;
    var l = a[4];
    if (l !== void 0) {
      if (l = parseFloat(l), l < 0 || l > 1)
        return;
      r.push(l);
    }
  }
  return r;
}, $d = function(e) {
  return Wd[e.toLowerCase()];
}, Sc = function(e) {
  return (Ve(e) ? e : null) || $d(e) || _d(e) || Hd(e) || Gd(e);
}, Wd = {
  // special colour names
  transparent: [0, 0, 0, 0],
  // NB alpha === 0
  // regular colours
  aliceblue: [240, 248, 255],
  antiquewhite: [250, 235, 215],
  aqua: [0, 255, 255],
  aquamarine: [127, 255, 212],
  azure: [240, 255, 255],
  beige: [245, 245, 220],
  bisque: [255, 228, 196],
  black: [0, 0, 0],
  blanchedalmond: [255, 235, 205],
  blue: [0, 0, 255],
  blueviolet: [138, 43, 226],
  brown: [165, 42, 42],
  burlywood: [222, 184, 135],
  cadetblue: [95, 158, 160],
  chartreuse: [127, 255, 0],
  chocolate: [210, 105, 30],
  coral: [255, 127, 80],
  cornflowerblue: [100, 149, 237],
  cornsilk: [255, 248, 220],
  crimson: [220, 20, 60],
  cyan: [0, 255, 255],
  darkblue: [0, 0, 139],
  darkcyan: [0, 139, 139],
  darkgoldenrod: [184, 134, 11],
  darkgray: [169, 169, 169],
  darkgreen: [0, 100, 0],
  darkgrey: [169, 169, 169],
  darkkhaki: [189, 183, 107],
  darkmagenta: [139, 0, 139],
  darkolivegreen: [85, 107, 47],
  darkorange: [255, 140, 0],
  darkorchid: [153, 50, 204],
  darkred: [139, 0, 0],
  darksalmon: [233, 150, 122],
  darkseagreen: [143, 188, 143],
  darkslateblue: [72, 61, 139],
  darkslategray: [47, 79, 79],
  darkslategrey: [47, 79, 79],
  darkturquoise: [0, 206, 209],
  darkviolet: [148, 0, 211],
  deeppink: [255, 20, 147],
  deepskyblue: [0, 191, 255],
  dimgray: [105, 105, 105],
  dimgrey: [105, 105, 105],
  dodgerblue: [30, 144, 255],
  firebrick: [178, 34, 34],
  floralwhite: [255, 250, 240],
  forestgreen: [34, 139, 34],
  fuchsia: [255, 0, 255],
  gainsboro: [220, 220, 220],
  ghostwhite: [248, 248, 255],
  gold: [255, 215, 0],
  goldenrod: [218, 165, 32],
  gray: [128, 128, 128],
  grey: [128, 128, 128],
  green: [0, 128, 0],
  greenyellow: [173, 255, 47],
  honeydew: [240, 255, 240],
  hotpink: [255, 105, 180],
  indianred: [205, 92, 92],
  indigo: [75, 0, 130],
  ivory: [255, 255, 240],
  khaki: [240, 230, 140],
  lavender: [230, 230, 250],
  lavenderblush: [255, 240, 245],
  lawngreen: [124, 252, 0],
  lemonchiffon: [255, 250, 205],
  lightblue: [173, 216, 230],
  lightcoral: [240, 128, 128],
  lightcyan: [224, 255, 255],
  lightgoldenrodyellow: [250, 250, 210],
  lightgray: [211, 211, 211],
  lightgreen: [144, 238, 144],
  lightgrey: [211, 211, 211],
  lightpink: [255, 182, 193],
  lightsalmon: [255, 160, 122],
  lightseagreen: [32, 178, 170],
  lightskyblue: [135, 206, 250],
  lightslategray: [119, 136, 153],
  lightslategrey: [119, 136, 153],
  lightsteelblue: [176, 196, 222],
  lightyellow: [255, 255, 224],
  lime: [0, 255, 0],
  limegreen: [50, 205, 50],
  linen: [250, 240, 230],
  magenta: [255, 0, 255],
  maroon: [128, 0, 0],
  mediumaquamarine: [102, 205, 170],
  mediumblue: [0, 0, 205],
  mediumorchid: [186, 85, 211],
  mediumpurple: [147, 112, 219],
  mediumseagreen: [60, 179, 113],
  mediumslateblue: [123, 104, 238],
  mediumspringgreen: [0, 250, 154],
  mediumturquoise: [72, 209, 204],
  mediumvioletred: [199, 21, 133],
  midnightblue: [25, 25, 112],
  mintcream: [245, 255, 250],
  mistyrose: [255, 228, 225],
  moccasin: [255, 228, 181],
  navajowhite: [255, 222, 173],
  navy: [0, 0, 128],
  oldlace: [253, 245, 230],
  olive: [128, 128, 0],
  olivedrab: [107, 142, 35],
  orange: [255, 165, 0],
  orangered: [255, 69, 0],
  orchid: [218, 112, 214],
  palegoldenrod: [238, 232, 170],
  palegreen: [152, 251, 152],
  paleturquoise: [175, 238, 238],
  palevioletred: [219, 112, 147],
  papayawhip: [255, 239, 213],
  peachpuff: [255, 218, 185],
  peru: [205, 133, 63],
  pink: [255, 192, 203],
  plum: [221, 160, 221],
  powderblue: [176, 224, 230],
  purple: [128, 0, 128],
  red: [255, 0, 0],
  rosybrown: [188, 143, 143],
  royalblue: [65, 105, 225],
  saddlebrown: [139, 69, 19],
  salmon: [250, 128, 114],
  sandybrown: [244, 164, 96],
  seagreen: [46, 139, 87],
  seashell: [255, 245, 238],
  sienna: [160, 82, 45],
  silver: [192, 192, 192],
  skyblue: [135, 206, 235],
  slateblue: [106, 90, 205],
  slategray: [112, 128, 144],
  slategrey: [112, 128, 144],
  snow: [255, 250, 250],
  springgreen: [0, 255, 127],
  steelblue: [70, 130, 180],
  tan: [210, 180, 140],
  teal: [0, 128, 128],
  thistle: [216, 191, 216],
  tomato: [255, 99, 71],
  turquoise: [64, 224, 208],
  violet: [238, 130, 238],
  wheat: [245, 222, 179],
  white: [255, 255, 255],
  whitesmoke: [245, 245, 245],
  yellow: [255, 255, 0],
  yellowgreen: [154, 205, 50]
}, kc = function(e) {
  for (var r = e.map, a = e.keys, n = a.length, i = 0; i < n; i++) {
    var o = a[i];
    if (Me(o))
      throw Error("Tried to set map with object key");
    i < a.length - 1 ? (r[o] == null && (r[o] = {}), r = r[o]) : r[o] = e.value;
  }
}, Pc = function(e) {
  for (var r = e.map, a = e.keys, n = a.length, i = 0; i < n; i++) {
    var o = a[i];
    if (Me(o))
      throw Error("Tried to get map with object key");
    if (r = r[o], r == null)
      return r;
  }
  return r;
}, ja = typeof globalThis < "u" ? globalThis : typeof window < "u" ? window : typeof global < "u" ? global : typeof self < "u" ? self : {};
function Va(t) {
  return t && t.__esModule && Object.prototype.hasOwnProperty.call(t, "default") ? t.default : t;
}
var li, Ws;
function _a() {
  if (Ws) return li;
  Ws = 1;
  function t(e) {
    var r = typeof e;
    return e != null && (r == "object" || r == "function");
  }
  return li = t, li;
}
var ci, Us;
function Ud() {
  if (Us) return ci;
  Us = 1;
  var t = typeof ja == "object" && ja && ja.Object === Object && ja;
  return ci = t, ci;
}
var fi, Ks;
function Gn() {
  if (Ks) return fi;
  Ks = 1;
  var t = Ud(), e = typeof self == "object" && self && self.Object === Object && self, r = t || e || Function("return this")();
  return fi = r, fi;
}
var vi, Xs;
function Kd() {
  if (Xs) return vi;
  Xs = 1;
  var t = Gn(), e = function() {
    return t.Date.now();
  };
  return vi = e, vi;
}
var di, Ys;
function Xd() {
  if (Ys) return di;
  Ys = 1;
  var t = /\s/;
  function e(r) {
    for (var a = r.length; a-- && t.test(r.charAt(a)); )
      ;
    return a;
  }
  return di = e, di;
}
var hi, Zs;
function Yd() {
  if (Zs) return hi;
  Zs = 1;
  var t = Xd(), e = /^\s+/;
  function r(a) {
    return a && a.slice(0, t(a) + 1).replace(e, "");
  }
  return hi = r, hi;
}
var gi, Qs;
function ds() {
  if (Qs) return gi;
  Qs = 1;
  var t = Gn(), e = t.Symbol;
  return gi = e, gi;
}
var pi, Js;
function Zd() {
  if (Js) return pi;
  Js = 1;
  var t = ds(), e = Object.prototype, r = e.hasOwnProperty, a = e.toString, n = t ? t.toStringTag : void 0;
  function i(o) {
    var s = r.call(o, n), u = o[n];
    try {
      o[n] = void 0;
      var l = !0;
    } catch {
    }
    var c = a.call(o);
    return l && (s ? o[n] = u : delete o[n]), c;
  }
  return pi = i, pi;
}
var yi, js;
function Qd() {
  if (js) return yi;
  js = 1;
  var t = Object.prototype, e = t.toString;
  function r(a) {
    return e.call(a);
  }
  return yi = r, yi;
}
var mi, eu;
function Dc() {
  if (eu) return mi;
  eu = 1;
  var t = ds(), e = Zd(), r = Qd(), a = "[object Null]", n = "[object Undefined]", i = t ? t.toStringTag : void 0;
  function o(s) {
    return s == null ? s === void 0 ? n : a : i && i in Object(s) ? e(s) : r(s);
  }
  return mi = o, mi;
}
var bi, tu;
function Jd() {
  if (tu) return bi;
  tu = 1;
  function t(e) {
    return e != null && typeof e == "object";
  }
  return bi = t, bi;
}
var wi, ru;
function Ga() {
  if (ru) return wi;
  ru = 1;
  var t = Dc(), e = Jd(), r = "[object Symbol]";
  function a(n) {
    return typeof n == "symbol" || e(n) && t(n) == r;
  }
  return wi = a, wi;
}
var xi, au;
function jd() {
  if (au) return xi;
  au = 1;
  var t = Yd(), e = _a(), r = Ga(), a = NaN, n = /^[-+]0x[0-9a-f]+$/i, i = /^0b[01]+$/i, o = /^0o[0-7]+$/i, s = parseInt;
  function u(l) {
    if (typeof l == "number")
      return l;
    if (r(l))
      return a;
    if (e(l)) {
      var c = typeof l.valueOf == "function" ? l.valueOf() : l;
      l = e(c) ? c + "" : c;
    }
    if (typeof l != "string")
      return l === 0 ? l : +l;
    l = t(l);
    var f = i.test(l);
    return f || o.test(l) ? s(l.slice(2), f ? 2 : 8) : n.test(l) ? a : +l;
  }
  return xi = u, xi;
}
var Ei, nu;
function eh() {
  if (nu) return Ei;
  nu = 1;
  var t = _a(), e = Kd(), r = jd(), a = "Expected a function", n = Math.max, i = Math.min;
  function o(s, u, l) {
    var c, f, v, h, d, y, g = 0, p = !1, m = !1, b = !0;
    if (typeof s != "function")
      throw new TypeError(a);
    u = r(u) || 0, t(l) && (p = !!l.leading, m = "maxWait" in l, v = m ? n(r(l.maxWait) || 0, u) : v, b = "trailing" in l ? !!l.trailing : b);
    function w(B) {
      var R = c, I = f;
      return c = f = void 0, g = B, h = s.apply(I, R), h;
    }
    function C(B) {
      return g = B, d = setTimeout(T, u), p ? w(B) : h;
    }
    function E(B) {
      var R = B - y, I = B - g, O = u - R;
      return m ? i(O, v - I) : O;
    }
    function x(B) {
      var R = B - y, I = B - g;
      return y === void 0 || R >= u || R < 0 || m && I >= v;
    }
    function T() {
      var B = e();
      if (x(B))
        return k(B);
      d = setTimeout(T, E(B));
    }
    function k(B) {
      return d = void 0, b && c ? w(B) : (c = f = void 0, h);
    }
    function D() {
      d !== void 0 && clearTimeout(d), g = 0, c = y = f = d = void 0;
    }
    function P() {
      return d === void 0 ? h : k(e());
    }
    function A() {
      var B = e(), R = x(B);
      if (c = arguments, f = this, y = B, R) {
        if (d === void 0)
          return C(y);
        if (m)
          return clearTimeout(d), d = setTimeout(T, u), w(y);
      }
      return d === void 0 && (d = setTimeout(T, u)), h;
    }
    return A.cancel = D, A.flush = P, A;
  }
  return Ei = o, Ei;
}
var th = eh(), Ha = /* @__PURE__ */ Va(th), Ci = et ? et.performance : null, Ac = Ci && Ci.now ? function() {
  return Ci.now();
} : function() {
  return Date.now();
}, rh = (function() {
  if (et) {
    if (et.requestAnimationFrame)
      return function(t) {
        et.requestAnimationFrame(t);
      };
    if (et.mozRequestAnimationFrame)
      return function(t) {
        et.mozRequestAnimationFrame(t);
      };
    if (et.webkitRequestAnimationFrame)
      return function(t) {
        et.webkitRequestAnimationFrame(t);
      };
    if (et.msRequestAnimationFrame)
      return function(t) {
        et.msRequestAnimationFrame(t);
      };
  }
  return function(t) {
    t && setTimeout(function() {
      t(Ac());
    }, 1e3 / 60);
  };
})(), An = function(e) {
  return rh(e);
}, Qt = Ac, kr = 9261, Bc = 65599, Kr = 5381, Rc = function(e) {
  for (var r = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : kr, a = r, n; n = e.next(), !n.done; )
    a = a * Bc + n.value | 0;
  return a;
}, Da = function(e) {
  var r = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : kr;
  return r * Bc + e | 0;
}, Aa = function(e) {
  var r = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : Kr;
  return (r << 5) + r + e | 0;
}, ah = function(e, r) {
  return e * 2097152 + r;
}, rr = function(e) {
  return e[0] * 2097152 + e[1];
}, en = function(e, r) {
  return [Da(e[0], r[0]), Aa(e[1], r[1])];
}, iu = function(e, r) {
  var a = {
    value: 0,
    done: !1
  }, n = 0, i = e.length, o = {
    next: function() {
      return n < i ? a.value = e[n++] : a.done = !0, a;
    }
  };
  return Rc(o, r);
}, Ar = function(e, r) {
  var a = {
    value: 0,
    done: !1
  }, n = 0, i = e.length, o = {
    next: function() {
      return n < i ? a.value = e.charCodeAt(n++) : a.done = !0, a;
    }
  };
  return Rc(o, r);
}, Mc = function() {
  return nh(arguments);
}, nh = function(e) {
  for (var r, a = 0; a < e.length; a++) {
    var n = e[a];
    a === 0 ? r = Ar(n) : r = Ar(n, r);
  }
  return r;
};
function ih(t, e, r, a, n) {
  var i = n * Math.PI / 180, o = Math.cos(i) * (t - r) - Math.sin(i) * (e - a) + r, s = Math.sin(i) * (t - r) + Math.cos(i) * (e - a) + a;
  return {
    x: o,
    y: s
  };
}
var oh = function(e, r, a, n, i, o) {
  return {
    x: (e - a) * i + a,
    y: (r - n) * o + n
  };
};
function sh(t, e, r) {
  if (r === 0) return t;
  var a = (e.x1 + e.x2) / 2, n = (e.y1 + e.y2) / 2, i = e.w / e.h, o = 1 / i, s = ih(t.x, t.y, a, n, r), u = oh(s.x, s.y, a, n, i, o);
  return {
    x: u.x,
    y: u.y
  };
}
var ou = !0, uh = console.warn != null, lh = console.trace != null, hs = Number.MAX_SAFE_INTEGER || 9007199254740991, Ic = function() {
  return !0;
}, Bn = function() {
  return !1;
}, su = function() {
  return 0;
}, gs = function() {
}, $e = function(e) {
  throw new Error(e);
}, Lc = function(e) {
  if (e !== void 0)
    ou = !!e;
  else
    return ou;
}, ze = function(e) {
  Lc() && (uh ? console.warn(e) : (console.log(e), lh && console.trace()));
}, ch = function(e) {
  return ye({}, e);
}, $t = function(e) {
  return e == null ? e : Ve(e) ? e.slice() : Me(e) ? ch(e) : e;
}, fh = function(e) {
  return e.slice();
}, Oc = function(e, r) {
  for (
    // loop :)
    r = e = "";
    // b - result , a - numeric letiable
    e++ < 36;
    //
    r += e * 51 & 52 ? (
      //  return a random number or 4
      (e ^ 15 ? (
        // generate a random number from 0 to 15
        8 ^ Math.random() * (e ^ 20 ? 16 : 4)
      ) : 4).toString(16)
    ) : "-"
  ) ;
  return r;
}, vh = {}, Nc = function() {
  return vh;
}, ft = function(e) {
  var r = Object.keys(e);
  return function(a) {
    for (var n = {}, i = 0; i < r.length; i++) {
      var o = r[i], s = a == null ? void 0 : a[o];
      n[o] = s === void 0 ? e[o] : s;
    }
    return n;
  };
}, fr = function(e, r, a) {
  for (var n = e.length - 1; n >= 0; n--)
    e[n] === r && e.splice(n, 1);
}, ps = function(e) {
  e.splice(0, e.length);
}, dh = function(e, r) {
  for (var a = 0; a < r.length; a++) {
    var n = r[a];
    e.push(n);
  }
}, bt = function(e, r, a) {
  return a && (r = Cc(a, r)), e[r];
}, Ht = function(e, r, a, n) {
  a && (r = Cc(a, r)), e[r] = n;
}, hh = /* @__PURE__ */ (function() {
  function t() {
    pr(this, t), this._obj = {};
  }
  return yr(t, [{
    key: "set",
    value: function(r, a) {
      return this._obj[r] = a, this;
    }
  }, {
    key: "delete",
    value: function(r) {
      return this._obj[r] = void 0, this;
    }
  }, {
    key: "clear",
    value: function() {
      this._obj = {};
    }
  }, {
    key: "has",
    value: function(r) {
      return this._obj[r] !== void 0;
    }
  }, {
    key: "get",
    value: function(r) {
      return this._obj[r];
    }
  }]);
})(), Zt = typeof Map < "u" ? Map : hh, gh = "undefined", ph = /* @__PURE__ */ (function() {
  function t(e) {
    if (pr(this, t), this._obj = /* @__PURE__ */ Object.create(null), this.size = 0, e != null) {
      var r;
      e.instanceString != null && e.instanceString() === this.instanceString() ? r = e.toArray() : r = e;
      for (var a = 0; a < r.length; a++)
        this.add(r[a]);
    }
  }
  return yr(t, [{
    key: "instanceString",
    value: function() {
      return "set";
    }
  }, {
    key: "add",
    value: function(r) {
      var a = this._obj;
      a[r] !== 1 && (a[r] = 1, this.size++);
    }
  }, {
    key: "delete",
    value: function(r) {
      var a = this._obj;
      a[r] === 1 && (a[r] = 0, this.size--);
    }
  }, {
    key: "clear",
    value: function() {
      this._obj = /* @__PURE__ */ Object.create(null);
    }
  }, {
    key: "has",
    value: function(r) {
      return this._obj[r] === 1;
    }
  }, {
    key: "toArray",
    value: function() {
      var r = this;
      return Object.keys(this._obj).filter(function(a) {
        return r.has(a);
      });
    }
  }, {
    key: "forEach",
    value: function(r, a) {
      return this.toArray().forEach(r, a);
    }
  }]);
})(), oa = (typeof Set > "u" ? "undefined" : rt(Set)) !== gh ? Set : ph, Hn = function(e, r) {
  var a = arguments.length > 2 && arguments[2] !== void 0 ? arguments[2] : !0;
  if (e === void 0 || r === void 0 || !fs(e)) {
    $e("An element must have a core reference and parameters set");
    return;
  }
  var n = r.group;
  if (n == null && (r.data && r.data.source != null && r.data.target != null ? n = "edges" : n = "nodes"), n !== "nodes" && n !== "edges") {
    $e("An element must be of type `nodes` or `edges`; you specified `" + n + "`");
    return;
  }
  this.length = 1, this[0] = this;
  var i = this._private = {
    cy: e,
    single: !0,
    // indicates this is an element
    data: r.data || {},
    // data object
    position: r.position || {
      x: 0,
      y: 0
    },
    // (x, y) position pair
    autoWidth: void 0,
    // width and height of nodes calculated by the renderer when set to special 'auto' value
    autoHeight: void 0,
    autoPadding: void 0,
    compoundBoundsClean: !1,
    // whether the compound dimensions need to be recalculated the next time dimensions are read
    listeners: [],
    // array of bound listeners
    group: n,
    // string; 'nodes' or 'edges'
    style: {},
    // properties as set by the style
    rstyle: {},
    // properties for style sent from the renderer to the core
    styleCxts: [],
    // applied style contexts from the styler
    styleKeys: {},
    // per-group keys of style property values
    removed: !0,
    // whether it's inside the vis; true if removed (set true here since we call restore)
    selected: !!r.selected,
    // whether it's selected
    selectable: r.selectable === void 0 ? !0 : !!r.selectable,
    // whether it's selectable
    locked: !!r.locked,
    // whether the element is locked (cannot be moved)
    grabbed: !1,
    // whether the element is grabbed by the mouse; renderer sets this privately
    grabbable: r.grabbable === void 0 ? !0 : !!r.grabbable,
    // whether the element can be grabbed
    pannable: r.pannable === void 0 ? n === "edges" : !!r.pannable,
    // whether the element has passthrough panning enabled
    active: !1,
    // whether the element is active from user interaction
    classes: new oa(),
    // map ( className => true )
    animation: {
      // object for currently-running animations
      current: [],
      queue: []
    },
    rscratch: {},
    // object in which the renderer can store information
    scratch: r.scratch || {},
    // scratch objects
    edges: [],
    // array of connected edges
    children: [],
    // array of children
    parent: r.parent && r.parent.isNode() ? r.parent : null,
    // parent ref
    traversalCache: {},
    // cache of output of traversal functions
    backgrounding: !1,
    // whether background images are loading
    bbCache: null,
    // cache of the current bounding box
    bbCacheShift: {
      x: 0,
      y: 0
    },
    // shift applied to cached bb to be applied on next get
    bodyBounds: null,
    // bounds cache of element body, w/o overlay
    overlayBounds: null,
    // bounds cache of element body, including overlay
    labelBounds: {
      // bounds cache of labels
      all: null,
      source: null,
      target: null,
      main: null
    },
    arrowBounds: {
      // bounds cache of edge arrows
      source: null,
      target: null,
      "mid-source": null,
      "mid-target": null
    }
  };
  if (i.position.x == null && (i.position.x = 0), i.position.y == null && (i.position.y = 0), r.renderedPosition) {
    var o = r.renderedPosition, s = e.pan(), u = e.zoom();
    i.position = {
      x: (o.x - s.x) / u,
      y: (o.y - s.y) / u
    };
  }
  var l = [];
  Ve(r.classes) ? l = r.classes : he(r.classes) && (l = r.classes.split(/\s+/));
  for (var c = 0, f = l.length; c < f; c++) {
    var v = l[c];
    !v || v === "" || i.classes.add(v);
  }
  this.createEmitter(), (a === void 0 || a) && this.restore();
  var h = r.style || r.css;
  h && (ze("Setting a `style` bypass at element creation should be done only when absolutely necessary.  Try to use the stylesheet instead."), this.style(h));
}, uu = function(e) {
  return e = {
    bfs: e.bfs || !e.dfs,
    dfs: e.dfs || !e.bfs
  }, function(a, n, i) {
    var o;
    Me(a) && !kt(a) && (o = a, a = o.roots || o.root, n = o.visit, i = o.directed), i = arguments.length === 2 && !Ue(n) ? n : i, n = Ue(n) ? n : function() {
    };
    for (var s = this._private.cy, u = a = he(a) ? this.filter(a) : a, l = [], c = [], f = {}, v = {}, h = {}, d = 0, y, g = this.byGroup(), p = g.nodes, m = g.edges, b = 0; b < u.length; b++) {
      var w = u[b], C = w.id();
      w.isNode() && (l.unshift(w), e.bfs && (h[C] = !0, c.push(w)), v[C] = 0);
    }
    for (var E = function() {
      var B = e.bfs ? l.shift() : l.pop(), R = B.id();
      if (e.dfs) {
        if (h[R])
          return 0;
        h[R] = !0, c.push(B);
      }
      var I = v[R], O = f[R], M = O != null ? O.source() : null, N = O != null ? O.target() : null, H = O == null ? void 0 : B.same(M) ? N[0] : M[0], G;
      if (G = n(B, O, H, d++, I), G === !0)
        return y = B, 1;
      if (G === !1)
        return 1;
      for (var F = B.connectedEdges().filter(function(j) {
        return (!i || j.source().same(B)) && m.has(j);
      }), _ = 0; _ < F.length; _++) {
        var X = F[_], J = X.connectedNodes().filter(function(j) {
          return !j.same(B) && p.has(j);
        }), Y = J.id();
        J.length !== 0 && !h[Y] && (J = J[0], l.push(J), e.bfs && (h[Y] = !0, c.push(J)), f[Y] = X, v[Y] = v[R] + 1);
      }
    }, x; l.length !== 0 && (x = E(), !(x !== 0 && x === 1)); )
      ;
    for (var T = s.collection(), k = 0; k < c.length; k++) {
      var D = c[k], P = f[D.id()];
      P != null && T.push(P), T.push(D);
    }
    return {
      path: s.collection(T),
      found: s.collection(y)
    };
  };
}, Ba = {
  breadthFirstSearch: uu({
    bfs: !0
  }),
  depthFirstSearch: uu({
    dfs: !0
  })
};
Ba.bfs = Ba.breadthFirstSearch;
Ba.dfs = Ba.depthFirstSearch;
var dn = { exports: {} }, yh = dn.exports, lu;
function mh() {
  return lu || (lu = 1, (function(t, e) {
    (function() {
      var r, a, n, i, o, s, u, l, c, f, v, h, d, y, g;
      n = Math.floor, f = Math.min, a = function(p, m) {
        return p < m ? -1 : p > m ? 1 : 0;
      }, c = function(p, m, b, w, C) {
        var E;
        if (b == null && (b = 0), C == null && (C = a), b < 0)
          throw new Error("lo must be non-negative");
        for (w == null && (w = p.length); b < w; )
          E = n((b + w) / 2), C(m, p[E]) < 0 ? w = E : b = E + 1;
        return [].splice.apply(p, [b, b - b].concat(m)), m;
      }, s = function(p, m, b) {
        return b == null && (b = a), p.push(m), y(p, 0, p.length - 1, b);
      }, o = function(p, m) {
        var b, w;
        return m == null && (m = a), b = p.pop(), p.length ? (w = p[0], p[0] = b, g(p, 0, m)) : w = b, w;
      }, l = function(p, m, b) {
        var w;
        return b == null && (b = a), w = p[0], p[0] = m, g(p, 0, b), w;
      }, u = function(p, m, b) {
        var w;
        return b == null && (b = a), p.length && b(p[0], m) < 0 && (w = [p[0], m], m = w[0], p[0] = w[1], g(p, 0, b)), m;
      }, i = function(p, m) {
        var b, w, C, E, x, T;
        for (m == null && (m = a), E = (function() {
          T = [];
          for (var k = 0, D = n(p.length / 2); 0 <= D ? k < D : k > D; 0 <= D ? k++ : k--)
            T.push(k);
          return T;
        }).apply(this).reverse(), x = [], w = 0, C = E.length; w < C; w++)
          b = E[w], x.push(g(p, b, m));
        return x;
      }, d = function(p, m, b) {
        var w;
        if (b == null && (b = a), w = p.indexOf(m), w !== -1)
          return y(p, 0, w, b), g(p, w, b);
      }, v = function(p, m, b) {
        var w, C, E, x, T;
        if (b == null && (b = a), C = p.slice(0, m), !C.length)
          return C;
        for (i(C, b), T = p.slice(m), E = 0, x = T.length; E < x; E++)
          w = T[E], u(C, w, b);
        return C.sort(b).reverse();
      }, h = function(p, m, b) {
        var w, C, E, x, T, k, D, P, A;
        if (b == null && (b = a), m * 10 <= p.length) {
          if (E = p.slice(0, m).sort(b), !E.length)
            return E;
          for (C = E[E.length - 1], D = p.slice(m), x = 0, k = D.length; x < k; x++)
            w = D[x], b(w, C) < 0 && (c(E, w, 0, null, b), E.pop(), C = E[E.length - 1]);
          return E;
        }
        for (i(p, b), A = [], T = 0, P = f(m, p.length); 0 <= P ? T < P : T > P; 0 <= P ? ++T : --T)
          A.push(o(p, b));
        return A;
      }, y = function(p, m, b, w) {
        var C, E, x;
        for (w == null && (w = a), C = p[b]; b > m; ) {
          if (x = b - 1 >> 1, E = p[x], w(C, E) < 0) {
            p[b] = E, b = x;
            continue;
          }
          break;
        }
        return p[b] = C;
      }, g = function(p, m, b) {
        var w, C, E, x, T;
        for (b == null && (b = a), C = p.length, T = m, E = p[m], w = 2 * m + 1; w < C; )
          x = w + 1, x < C && !(b(p[w], p[x]) < 0) && (w = x), p[m] = p[w], m = w, w = 2 * m + 1;
        return p[m] = E, y(p, T, m, b);
      }, r = (function() {
        p.push = s, p.pop = o, p.replace = l, p.pushpop = u, p.heapify = i, p.updateItem = d, p.nlargest = v, p.nsmallest = h;
        function p(m) {
          this.cmp = m ?? a, this.nodes = [];
        }
        return p.prototype.push = function(m) {
          return s(this.nodes, m, this.cmp);
        }, p.prototype.pop = function() {
          return o(this.nodes, this.cmp);
        }, p.prototype.peek = function() {
          return this.nodes[0];
        }, p.prototype.contains = function(m) {
          return this.nodes.indexOf(m) !== -1;
        }, p.prototype.replace = function(m) {
          return l(this.nodes, m, this.cmp);
        }, p.prototype.pushpop = function(m) {
          return u(this.nodes, m, this.cmp);
        }, p.prototype.heapify = function() {
          return i(this.nodes, this.cmp);
        }, p.prototype.updateItem = function(m) {
          return d(this.nodes, m, this.cmp);
        }, p.prototype.clear = function() {
          return this.nodes = [];
        }, p.prototype.empty = function() {
          return this.nodes.length === 0;
        }, p.prototype.size = function() {
          return this.nodes.length;
        }, p.prototype.clone = function() {
          var m;
          return m = new p(), m.nodes = this.nodes.slice(0), m;
        }, p.prototype.toArray = function() {
          return this.nodes.slice(0);
        }, p.prototype.insert = p.prototype.push, p.prototype.top = p.prototype.peek, p.prototype.front = p.prototype.peek, p.prototype.has = p.prototype.contains, p.prototype.copy = p.prototype.clone, p;
      })(), (function(p, m) {
        return t.exports = m();
      })(this, function() {
        return r;
      });
    }).call(yh);
  })(dn)), dn.exports;
}
var Ti, cu;
function bh() {
  return cu || (cu = 1, Ti = mh()), Ti;
}
var wh = bh(), $a = /* @__PURE__ */ Va(wh), xh = ft({
  root: null,
  weight: function(e) {
    return 1;
  },
  directed: !1
}), Eh = {
  dijkstra: function(e) {
    if (!Me(e)) {
      var r = arguments;
      e = {
        root: r[0],
        weight: r[1],
        directed: r[2]
      };
    }
    var a = xh(e), n = a.root, i = a.weight, o = a.directed, s = this, u = i, l = he(n) ? this.filter(n)[0] : n[0], c = {}, f = {}, v = {}, h = this.byGroup(), d = h.nodes, y = h.edges;
    y.unmergeBy(function(I) {
      return I.isLoop();
    });
    for (var g = function(O) {
      return c[O.id()];
    }, p = function(O, M) {
      c[O.id()] = M, m.updateItem(O);
    }, m = new $a(function(I, O) {
      return g(I) - g(O);
    }), b = 0; b < d.length; b++) {
      var w = d[b];
      c[w.id()] = w.same(l) ? 0 : 1 / 0, m.push(w);
    }
    for (var C = function(O, M) {
      for (var N = (o ? O.edgesTo(M) : O.edgesWith(M)).intersect(y), H = 1 / 0, G, F = 0; F < N.length; F++) {
        var _ = N[F], X = u(_);
        (X < H || !G) && (H = X, G = _);
      }
      return {
        edge: G,
        dist: H
      };
    }; m.size() > 0; ) {
      var E = m.pop(), x = g(E), T = E.id();
      if (v[T] = x, x !== 1 / 0)
        for (var k = E.neighborhood().intersect(d), D = 0; D < k.length; D++) {
          var P = k[D], A = P.id(), B = C(E, P), R = x + B.dist;
          R < g(P) && (p(P, R), f[A] = {
            node: E,
            edge: B.edge
          });
        }
    }
    return {
      distanceTo: function(O) {
        var M = he(O) ? d.filter(O)[0] : O[0];
        return v[M.id()];
      },
      pathTo: function(O) {
        var M = he(O) ? d.filter(O)[0] : O[0], N = [], H = M, G = H.id();
        if (M.length > 0)
          for (N.unshift(M); f[G]; ) {
            var F = f[G];
            N.unshift(F.edge), N.unshift(F.node), H = F.node, G = H.id();
          }
        return s.spawn(N);
      }
    };
  }
}, Ch = {
  // kruskal's algorithm (finds min spanning tree, assuming undirected graph)
  // implemented from pseudocode from wikipedia
  kruskal: function(e) {
    e = e || function(b) {
      return 1;
    };
    for (var r = this.byGroup(), a = r.nodes, n = r.edges, i = a.length, o = new Array(i), s = a, u = function(w) {
      for (var C = 0; C < o.length; C++) {
        var E = o[C];
        if (E.has(w))
          return C;
      }
    }, l = 0; l < i; l++)
      o[l] = this.spawn(a[l]);
    for (var c = n.sort(function(b, w) {
      return e(b) - e(w);
    }), f = 0; f < c.length; f++) {
      var v = c[f], h = v.source()[0], d = v.target()[0], y = u(h), g = u(d), p = o[y], m = o[g];
      y !== g && (s.merge(v), p.merge(m), o.splice(g, 1));
    }
    return s;
  }
}, Th = ft({
  root: null,
  goal: null,
  weight: function(e) {
    return 1;
  },
  heuristic: function(e) {
    return 0;
  },
  directed: !1
}), Sh = {
  // Implemented from pseudocode from wikipedia
  aStar: function(e) {
    var r = this.cy(), a = Th(e), n = a.root, i = a.goal, o = a.heuristic, s = a.directed, u = a.weight;
    n = r.collection(n)[0], i = r.collection(i)[0];
    var l = n.id(), c = i.id(), f = {}, v = {}, h = {}, d = new $a(function(G, F) {
      return v[G.id()] - v[F.id()];
    }), y = new oa(), g = {}, p = {}, m = function(F, _) {
      d.push(F), y.add(_);
    }, b, w, C = function() {
      b = d.pop(), w = b.id(), y.delete(w);
    }, E = function(F) {
      return y.has(F);
    };
    m(n, l), f[l] = 0, v[l] = o(n);
    for (var x = 0; d.size() > 0; ) {
      if (C(), x++, w === c) {
        for (var T = [], k = i, D = c, P = p[D]; T.unshift(k), P != null && T.unshift(P), k = g[D], k != null; )
          D = k.id(), P = p[D];
        return {
          found: !0,
          distance: f[w],
          path: this.spawn(T),
          steps: x
        };
      }
      h[w] = !0;
      for (var A = b._private.edges, B = 0; B < A.length; B++) {
        var R = A[B];
        if (this.hasElementWithId(R.id()) && !(s && R.data("source") !== w)) {
          var I = R.source(), O = R.target(), M = I.id() !== w ? I : O, N = M.id();
          if (this.hasElementWithId(N) && !h[N]) {
            var H = f[w] + u(R);
            if (!E(N)) {
              f[N] = H, v[N] = H + o(M), m(M, N), g[N] = b, p[N] = R;
              continue;
            }
            H < f[N] && (f[N] = H, v[N] = H + o(M), g[N] = b, p[N] = R);
          }
        }
      }
    }
    return {
      found: !1,
      distance: void 0,
      path: void 0,
      steps: x
    };
  }
}, kh = ft({
  weight: function(e) {
    return 1;
  },
  directed: !1
}), Ph = {
  // Implemented from pseudocode from wikipedia
  floydWarshall: function(e) {
    for (var r = this.cy(), a = kh(e), n = a.weight, i = a.directed, o = n, s = this.byGroup(), u = s.nodes, l = s.edges, c = u.length, f = c * c, v = function(X) {
      return u.indexOf(X);
    }, h = function(X) {
      return u[X];
    }, d = new Array(f), y = 0; y < f; y++) {
      var g = y % c, p = (y - g) / c;
      p === g ? d[y] = 0 : d[y] = 1 / 0;
    }
    for (var m = new Array(f), b = new Array(f), w = 0; w < l.length; w++) {
      var C = l[w], E = C.source()[0], x = C.target()[0];
      if (E !== x) {
        var T = v(E), k = v(x), D = T * c + k, P = o(C);
        if (d[D] > P && (d[D] = P, m[D] = k, b[D] = C), !i) {
          var A = k * c + T;
          !i && d[A] > P && (d[A] = P, m[A] = T, b[A] = C);
        }
      }
    }
    for (var B = 0; B < c; B++)
      for (var R = 0; R < c; R++)
        for (var I = R * c + B, O = 0; O < c; O++) {
          var M = R * c + O, N = B * c + O;
          d[I] + d[N] < d[M] && (d[M] = d[I] + d[N], m[M] = m[I]);
        }
    var H = function(X) {
      return (he(X) ? r.filter(X) : X)[0];
    }, G = function(X) {
      return v(H(X));
    }, F = {
      distance: function(X, J) {
        var Y = G(X), j = G(J);
        return d[Y * c + j];
      },
      path: function(X, J) {
        var Y = G(X), j = G(J), te = h(Y);
        if (Y === j)
          return te.collection();
        if (m[Y * c + j] == null)
          return r.collection();
        var ae = r.collection(), z = Y, L;
        for (ae.merge(te); Y !== j; )
          z = Y, Y = m[Y * c + j], L = b[z * c + Y], ae.merge(L), ae.merge(h(Y));
        return ae;
      }
    };
    return F;
  }
  // floydWarshall
}, Dh = ft({
  weight: function(e) {
    return 1;
  },
  directed: !1,
  root: null
}), Ah = {
  // Implemented from pseudocode from wikipedia
  bellmanFord: function(e) {
    var r = this, a = Dh(e), n = a.weight, i = a.directed, o = a.root, s = n, u = this, l = this.cy(), c = this.byGroup(), f = c.edges, v = c.nodes, h = v.length, d = new Zt(), y = !1, g = [];
    o = l.collection(o)[0], f.unmergeBy(function(we) {
      return we.isLoop();
    });
    for (var p = f.length, m = function(me) {
      var ge = d.get(me.id());
      return ge || (ge = {}, d.set(me.id(), ge)), ge;
    }, b = function(me) {
      return (he(me) ? l.$(me) : me)[0];
    }, w = function(me) {
      return m(b(me)).dist;
    }, C = function(me) {
      for (var ge = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : o, oe = b(me), de = [], ve = oe; ; ) {
        if (ve == null)
          return r.spawn();
        var xe = m(ve), be = xe.edge, Se = xe.pred;
        if (de.unshift(ve[0]), ve.same(ge) && de.length > 0)
          break;
        be != null && de.unshift(be), ve = Se;
      }
      return u.spawn(de);
    }, E = 0; E < h; E++) {
      var x = v[E], T = m(x);
      x.same(o) ? T.dist = 0 : T.dist = 1 / 0, T.pred = null, T.edge = null;
    }
    for (var k = !1, D = function(me, ge, oe, de, ve, xe) {
      var be = de.dist + xe;
      be < ve.dist && !oe.same(de.edge) && (ve.dist = be, ve.pred = me, ve.edge = oe, k = !0);
    }, P = 1; P < h; P++) {
      k = !1;
      for (var A = 0; A < p; A++) {
        var B = f[A], R = B.source(), I = B.target(), O = s(B), M = m(R), N = m(I);
        D(R, I, B, M, N, O), i || D(I, R, B, N, M, O);
      }
      if (!k)
        break;
    }
    if (k)
      for (var H = [], G = 0; G < p; G++) {
        var F = f[G], _ = F.source(), X = F.target(), J = s(F), Y = m(_).dist, j = m(X).dist;
        if (Y + J < j || !i && j + J < Y)
          if (y || (ze("Graph contains a negative weight cycle for Bellman-Ford"), y = !0), e.findNegativeWeightCycles !== !1) {
            var te = [];
            Y + J < j && te.push(_), !i && j + J < Y && te.push(X);
            for (var ae = te.length, z = 0; z < ae; z++) {
              var L = te[z], V = [L];
              V.push(m(L).edge);
              for (var W = m(L).pred; V.indexOf(W) === -1; )
                V.push(W), V.push(m(W).edge), W = m(W).pred;
              V = V.slice(V.indexOf(W));
              for (var K = V[0].id(), re = 0, le = 2; le < V.length; le += 2)
                V[le].id() < K && (K = V[le].id(), re = le);
              V = V.slice(re).concat(V.slice(0, re)), V.push(V[0]);
              var Pe = V.map(function(we) {
                return we.id();
              }).join(",");
              H.indexOf(Pe) === -1 && (g.push(u.spawn(V)), H.push(Pe));
            }
          } else
            break;
      }
    return {
      distanceTo: w,
      pathTo: C,
      hasNegativeWeightCycle: y,
      negativeWeightCycles: g
    };
  }
  // bellmanFord
}, Bh = Math.sqrt(2), Rh = function(e, r, a) {
  a.length === 0 && $e("Karger-Stein must be run on a connected (sub)graph");
  for (var n = a[e], i = n[1], o = n[2], s = r[i], u = r[o], l = a, c = l.length - 1; c >= 0; c--) {
    var f = l[c], v = f[1], h = f[2];
    (r[v] === s && r[h] === u || r[v] === u && r[h] === s) && l.splice(c, 1);
  }
  for (var d = 0; d < l.length; d++) {
    var y = l[d];
    y[1] === u ? (l[d] = y.slice(), l[d][1] = s) : y[2] === u && (l[d] = y.slice(), l[d][2] = s);
  }
  for (var g = 0; g < r.length; g++)
    r[g] === u && (r[g] = s);
  return l;
}, Si = function(e, r, a, n) {
  for (; a > n; ) {
    var i = Math.floor(Math.random() * r.length);
    r = Rh(i, e, r), a--;
  }
  return r;
}, Mh = {
  // Computes the minimum cut of an undirected graph
  // Returns the correct answer with high probability
  kargerStein: function() {
    var e = this, r = this.byGroup(), a = r.nodes, n = r.edges;
    n.unmergeBy(function(N) {
      return N.isLoop();
    });
    var i = a.length, o = n.length, s = Math.ceil(Math.pow(Math.log(i) / Math.LN2, 2)), u = Math.floor(i / Bh);
    if (i < 2) {
      $e("At least 2 nodes are required for Karger-Stein algorithm");
      return;
    }
    for (var l = [], c = 0; c < o; c++) {
      var f = n[c];
      l.push([c, a.indexOf(f.source()), a.indexOf(f.target())]);
    }
    for (var v = 1 / 0, h = [], d = new Array(i), y = new Array(i), g = new Array(i), p = function(H, G) {
      for (var F = 0; F < i; F++)
        G[F] = H[F];
    }, m = 0; m <= s; m++) {
      for (var b = 0; b < i; b++)
        y[b] = b;
      var w = Si(y, l.slice(), i, u), C = w.slice();
      p(y, g);
      var E = Si(y, w, u, 2), x = Si(g, C, u, 2);
      E.length <= x.length && E.length < v ? (v = E.length, h = E, p(y, d)) : x.length <= E.length && x.length < v && (v = x.length, h = x, p(g, d));
    }
    for (var T = this.spawn(h.map(function(N) {
      return n[N[0]];
    })), k = this.spawn(), D = this.spawn(), P = d[0], A = 0; A < d.length; A++) {
      var B = d[A], R = a[A];
      B === P ? k.merge(R) : D.merge(R);
    }
    var I = function(H) {
      var G = e.spawn();
      return H.forEach(function(F) {
        G.merge(F), F.connectedEdges().forEach(function(_) {
          e.contains(_) && !T.contains(_) && G.merge(_);
        });
      }), G;
    }, O = [I(k), I(D)], M = {
      cut: T,
      components: O,
      // n.b. partitions are included to be compatible with the old api spec
      // (could be removed in a future major version)
      partition1: k,
      partition2: D
    };
    return M;
  }
}, ki, Ih = function(e) {
  return {
    x: e.x,
    y: e.y
  };
}, $n = function(e, r, a) {
  return {
    x: e.x * r + a.x,
    y: e.y * r + a.y
  };
}, zc = function(e, r, a) {
  return {
    x: (e.x - a.x) / r,
    y: (e.y - a.y) / r
  };
}, Xr = function(e) {
  return {
    x: e[0],
    y: e[1]
  };
}, Lh = function(e) {
  for (var r = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : 0, a = arguments.length > 2 && arguments[2] !== void 0 ? arguments[2] : e.length, n = 1 / 0, i = r; i < a; i++) {
    var o = e[i];
    isFinite(o) && (n = Math.min(o, n));
  }
  return n;
}, Oh = function(e) {
  for (var r = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : 0, a = arguments.length > 2 && arguments[2] !== void 0 ? arguments[2] : e.length, n = -1 / 0, i = r; i < a; i++) {
    var o = e[i];
    isFinite(o) && (n = Math.max(o, n));
  }
  return n;
}, Nh = function(e) {
  for (var r = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : 0, a = arguments.length > 2 && arguments[2] !== void 0 ? arguments[2] : e.length, n = 0, i = 0, o = r; o < a; o++) {
    var s = e[o];
    isFinite(s) && (n += s, i++);
  }
  return n / i;
}, zh = function(e) {
  var r = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : 0, a = arguments.length > 2 && arguments[2] !== void 0 ? arguments[2] : e.length, n = arguments.length > 3 && arguments[3] !== void 0 ? arguments[3] : !0, i = arguments.length > 4 && arguments[4] !== void 0 ? arguments[4] : !0, o = arguments.length > 5 && arguments[5] !== void 0 ? arguments[5] : !0;
  n ? e = e.slice(r, a) : (a < e.length && e.splice(a, e.length - a), r > 0 && e.splice(0, r));
  for (var s = 0, u = e.length - 1; u >= 0; u--) {
    var l = e[u];
    o ? isFinite(l) || (e[u] = -1 / 0, s++) : e.splice(u, 1);
  }
  i && e.sort(function(v, h) {
    return v - h;
  });
  var c = e.length, f = Math.floor(c / 2);
  return c % 2 !== 0 ? e[f + 1 + s] : (e[f - 1 + s] + e[f + s]) / 2;
}, Fh = function(e) {
  return Math.PI * e / 180;
}, tn = function(e, r) {
  return Math.atan2(r, e) - Math.PI / 2;
}, ys = Math.log2 || function(t) {
  return Math.log(t) / Math.log(2);
}, ms = function(e) {
  return e > 0 ? 1 : e < 0 ? -1 : 0;
}, Br = function(e, r) {
  return Math.sqrt(Tr(e, r));
}, Tr = function(e, r) {
  var a = r.x - e.x, n = r.y - e.y;
  return a * a + n * n;
}, qh = function(e) {
  for (var r = e.length, a = 0, n = 0; n < r; n++)
    a += e[n];
  for (var i = 0; i < r; i++)
    e[i] = e[i] / a;
  return e;
}, it = function(e, r, a, n) {
  return (1 - n) * (1 - n) * e + 2 * (1 - n) * n * r + n * n * a;
}, Qr = function(e, r, a, n) {
  return {
    x: it(e.x, r.x, a.x, n),
    y: it(e.y, r.y, a.y, n)
  };
}, Vh = function(e, r, a, n) {
  var i = {
    x: r.x - e.x,
    y: r.y - e.y
  }, o = Br(e, r), s = {
    x: i.x / o,
    y: i.y / o
  };
  return a = a ?? 0, n = n ?? a * o, {
    x: e.x + s.x * n,
    y: e.y + s.y * n
  };
}, Ra = function(e, r, a) {
  return Math.max(e, Math.min(a, r));
}, wt = function(e) {
  if (e == null)
    return {
      x1: 1 / 0,
      y1: 1 / 0,
      x2: -1 / 0,
      y2: -1 / 0,
      w: 0,
      h: 0
    };
  if (e.x1 != null && e.y1 != null) {
    if (e.x2 != null && e.y2 != null && e.x2 >= e.x1 && e.y2 >= e.y1)
      return {
        x1: e.x1,
        y1: e.y1,
        x2: e.x2,
        y2: e.y2,
        w: e.x2 - e.x1,
        h: e.y2 - e.y1
      };
    if (e.w != null && e.h != null && e.w >= 0 && e.h >= 0)
      return {
        x1: e.x1,
        y1: e.y1,
        x2: e.x1 + e.w,
        y2: e.y1 + e.h,
        w: e.w,
        h: e.h
      };
  }
}, _h = function(e) {
  return {
    x1: e.x1,
    x2: e.x2,
    w: e.w,
    y1: e.y1,
    y2: e.y2,
    h: e.h
  };
}, Gh = function(e) {
  e.x1 = 1 / 0, e.y1 = 1 / 0, e.x2 = -1 / 0, e.y2 = -1 / 0, e.w = 0, e.h = 0;
}, Hh = function(e, r) {
  e.x1 = Math.min(e.x1, r.x1), e.x2 = Math.max(e.x2, r.x2), e.w = e.x2 - e.x1, e.y1 = Math.min(e.y1, r.y1), e.y2 = Math.max(e.y2, r.y2), e.h = e.y2 - e.y1;
}, Fc = function(e, r, a) {
  e.x1 = Math.min(e.x1, r), e.x2 = Math.max(e.x2, r), e.w = e.x2 - e.x1, e.y1 = Math.min(e.y1, a), e.y2 = Math.max(e.y2, a), e.h = e.y2 - e.y1;
}, hn = function(e) {
  var r = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : 0;
  return e.x1 -= r, e.x2 += r, e.y1 -= r, e.y2 += r, e.w = e.x2 - e.x1, e.h = e.y2 - e.y1, e;
}, gn = function(e) {
  var r = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : [0], a, n, i, o;
  if (r.length === 1)
    a = n = i = o = r[0];
  else if (r.length === 2)
    a = i = r[0], o = n = r[1];
  else if (r.length === 4) {
    var s = Je(r, 4);
    a = s[0], n = s[1], i = s[2], o = s[3];
  }
  return e.x1 -= o, e.x2 += n, e.y1 -= a, e.y2 += i, e.w = e.x2 - e.x1, e.h = e.y2 - e.y1, e;
}, fu = function(e, r) {
  e.x1 = r.x1, e.y1 = r.y1, e.x2 = r.x2, e.y2 = r.y2, e.w = e.x2 - e.x1, e.h = e.y2 - e.y1;
}, bs = function(e, r) {
  return !(e.x1 > r.x2 || r.x1 > e.x2 || e.x2 < r.x1 || r.x2 < e.x1 || e.y2 < r.y1 || r.y2 < e.y1 || e.y1 > r.y2 || r.y1 > e.y2);
}, or = function(e, r, a) {
  return e.x1 <= r && r <= e.x2 && e.y1 <= a && a <= e.y2;
}, vu = function(e, r) {
  return or(e, r.x, r.y);
}, qc = function(e, r) {
  return or(e, r.x1, r.y1) && or(e, r.x2, r.y2);
}, $h = (ki = Math.hypot) !== null && ki !== void 0 ? ki : function(t, e) {
  return Math.sqrt(t * t + e * e);
};
function Wh(t, e) {
  if (t.length < 3)
    throw new Error("Need at least 3 vertices");
  var r = function(T, k) {
    return {
      x: T.x + k.x,
      y: T.y + k.y
    };
  }, a = function(T, k) {
    return {
      x: T.x - k.x,
      y: T.y - k.y
    };
  }, n = function(T, k) {
    return {
      x: T.x * k,
      y: T.y * k
    };
  }, i = function(T, k) {
    return T.x * k.y - T.y * k.x;
  }, o = function(T) {
    var k = $h(T.x, T.y);
    return k === 0 ? {
      x: 0,
      y: 0
    } : {
      x: T.x / k,
      y: T.y / k
    };
  }, s = function(T) {
    for (var k = 0, D = 0; D < T.length; D++) {
      var P = T[D], A = T[(D + 1) % T.length];
      k += P.x * A.y - A.x * P.y;
    }
    return k / 2;
  }, u = function(T, k, D, P) {
    var A = a(k, T), B = a(P, D), R = i(A, B);
    if (Math.abs(R) < 1e-9)
      return r(T, n(A, 0.5));
    var I = i(a(D, T), B) / R;
    return r(T, n(A, I));
  }, l = t.map(function(x) {
    return {
      x: x.x,
      y: x.y
    };
  });
  s(l) < 0 && l.reverse();
  for (var c = l.length, f = [], v = 0; v < c; v++) {
    var h = l[v], d = l[(v + 1) % c], y = a(d, h), g = o({
      x: y.y,
      y: -y.x
    });
    f.push(g);
  }
  for (var p = f.map(function(x, T) {
    var k = r(l[T], n(x, e)), D = r(l[(T + 1) % c], n(x, e));
    return {
      p1: k,
      p2: D
    };
  }), m = [], b = 0; b < c; b++) {
    var w = p[(b - 1 + c) % c], C = p[b], E = u(w.p1, w.p2, C.p1, C.p2);
    m.push(E);
  }
  return m;
}
function Uh(t, e, r, a, n, i) {
  var o = tg(t, e, r, a, n), s = Wh(o, i), u = wt();
  return s.forEach(function(l) {
    return Fc(u, l.x, l.y);
  }), u;
}
var Vc = function(e, r, a, n, i, o, s) {
  var u = arguments.length > 7 && arguments[7] !== void 0 ? arguments[7] : "auto", l = u === "auto" ? vr(i, o) : u, c = i / 2, f = o / 2;
  l = Math.min(l, c, f);
  var v = l !== c, h = l !== f, d;
  if (v) {
    var y = a - c + l - s, g = n - f - s, p = a + c - l + s, m = g;
    if (d = sr(e, r, a, n, y, g, p, m, !1), d.length > 0)
      return d;
  }
  if (h) {
    var b = a + c + s, w = n - f + l - s, C = b, E = n + f - l + s;
    if (d = sr(e, r, a, n, b, w, C, E, !1), d.length > 0)
      return d;
  }
  if (v) {
    var x = a - c + l - s, T = n + f + s, k = a + c - l + s, D = T;
    if (d = sr(e, r, a, n, x, T, k, D, !1), d.length > 0)
      return d;
  }
  if (h) {
    var P = a - c - s, A = n - f + l - s, B = P, R = n + f - l + s;
    if (d = sr(e, r, a, n, P, A, B, R, !1), d.length > 0)
      return d;
  }
  var I;
  {
    var O = a - c + l, M = n - f + l;
    if (I = Ea(e, r, a, n, O, M, l + s), I.length > 0 && I[0] <= O && I[1] <= M)
      return [I[0], I[1]];
  }
  {
    var N = a + c - l, H = n - f + l;
    if (I = Ea(e, r, a, n, N, H, l + s), I.length > 0 && I[0] >= N && I[1] <= H)
      return [I[0], I[1]];
  }
  {
    var G = a + c - l, F = n + f - l;
    if (I = Ea(e, r, a, n, G, F, l + s), I.length > 0 && I[0] >= G && I[1] >= F)
      return [I[0], I[1]];
  }
  {
    var _ = a - c + l, X = n + f - l;
    if (I = Ea(e, r, a, n, _, X, l + s), I.length > 0 && I[0] <= _ && I[1] >= X)
      return [I[0], I[1]];
  }
  return [];
}, Kh = function(e, r, a, n, i, o, s) {
  var u = s, l = Math.min(a, i), c = Math.max(a, i), f = Math.min(n, o), v = Math.max(n, o);
  return l - u <= e && e <= c + u && f - u <= r && r <= v + u;
}, Xh = function(e, r, a, n, i, o, s, u, l) {
  var c = {
    x1: Math.min(a, s, i) - l,
    x2: Math.max(a, s, i) + l,
    y1: Math.min(n, u, o) - l,
    y2: Math.max(n, u, o) + l
  };
  return !(e < c.x1 || e > c.x2 || r < c.y1 || r > c.y2);
}, Yh = function(e, r, a, n) {
  a -= n;
  var i = r * r - 4 * e * a;
  if (i < 0)
    return [];
  var o = Math.sqrt(i), s = 2 * e, u = (-r + o) / s, l = (-r - o) / s;
  return [u, l];
}, Zh = function(e, r, a, n, i) {
  var o = 1e-5;
  e === 0 && (e = o), r /= e, a /= e, n /= e;
  var s, u, l, c, f, v, h, d;
  if (u = (3 * a - r * r) / 9, l = -(27 * n) + r * (9 * a - 2 * (r * r)), l /= 54, s = u * u * u + l * l, i[1] = 0, h = r / 3, s > 0) {
    f = l + Math.sqrt(s), f = f < 0 ? -Math.pow(-f, 1 / 3) : Math.pow(f, 1 / 3), v = l - Math.sqrt(s), v = v < 0 ? -Math.pow(-v, 1 / 3) : Math.pow(v, 1 / 3), i[0] = -h + f + v, h += (f + v) / 2, i[4] = i[2] = -h, h = Math.sqrt(3) * (-v + f) / 2, i[3] = h, i[5] = -h;
    return;
  }
  if (i[5] = i[3] = 0, s === 0) {
    d = l < 0 ? -Math.pow(-l, 1 / 3) : Math.pow(l, 1 / 3), i[0] = -h + 2 * d, i[4] = i[2] = -(d + h);
    return;
  }
  u = -u, c = u * u * u, c = Math.acos(l / Math.sqrt(c)), d = 2 * Math.sqrt(u), i[0] = -h + d * Math.cos(c / 3), i[2] = -h + d * Math.cos((c + 2 * Math.PI) / 3), i[4] = -h + d * Math.cos((c + 4 * Math.PI) / 3);
}, Qh = function(e, r, a, n, i, o, s, u) {
  var l = 1 * a * a - 4 * a * i + 2 * a * s + 4 * i * i - 4 * i * s + s * s + n * n - 4 * n * o + 2 * n * u + 4 * o * o - 4 * o * u + u * u, c = 9 * a * i - 3 * a * a - 3 * a * s - 6 * i * i + 3 * i * s + 9 * n * o - 3 * n * n - 3 * n * u - 6 * o * o + 3 * o * u, f = 3 * a * a - 6 * a * i + a * s - a * e + 2 * i * i + 2 * i * e - s * e + 3 * n * n - 6 * n * o + n * u - n * r + 2 * o * o + 2 * o * r - u * r, v = 1 * a * i - a * a + a * e - i * e + n * o - n * n + n * r - o * r, h = [];
  Zh(l, c, f, v, h);
  for (var d = 1e-7, y = [], g = 0; g < 6; g += 2)
    Math.abs(h[g + 1]) < d && h[g] >= 0 && h[g] <= 1 && y.push(h[g]);
  y.push(1), y.push(0);
  for (var p = -1, m, b, w, C = 0; C < y.length; C++)
    m = Math.pow(1 - y[C], 2) * a + 2 * (1 - y[C]) * y[C] * i + y[C] * y[C] * s, b = Math.pow(1 - y[C], 2) * n + 2 * (1 - y[C]) * y[C] * o + y[C] * y[C] * u, w = Math.pow(m - e, 2) + Math.pow(b - r, 2), p >= 0 ? w < p && (p = w) : p = w;
  return p;
}, Jh = function(e, r, a, n, i, o) {
  var s = [e - a, r - n], u = [i - a, o - n], l = u[0] * u[0] + u[1] * u[1], c = s[0] * s[0] + s[1] * s[1], f = s[0] * u[0] + s[1] * u[1], v = f * f / l;
  return f < 0 ? c : v > l ? (e - i) * (e - i) + (r - o) * (r - o) : c - v;
}, Tt = function(e, r, a) {
  for (var n, i, o, s, u, l = 0, c = 0; c < a.length / 2; c++)
    if (n = a[c * 2], i = a[c * 2 + 1], c + 1 < a.length / 2 ? (o = a[(c + 1) * 2], s = a[(c + 1) * 2 + 1]) : (o = a[(c + 1 - a.length / 2) * 2], s = a[(c + 1 - a.length / 2) * 2 + 1]), !(n == e && o == e)) if (n >= e && e >= o || n <= e && e <= o)
      u = (e - n) / (o - n) * (s - i) + i, u > r && l++;
    else
      continue;
  return l % 2 !== 0;
}, Jt = function(e, r, a, n, i, o, s, u, l) {
  var c = new Array(a.length), f;
  u[0] != null ? (f = Math.atan(u[1] / u[0]), u[0] < 0 ? f = f + Math.PI / 2 : f = -f - Math.PI / 2) : f = u;
  for (var v = Math.cos(-f), h = Math.sin(-f), d = 0; d < c.length / 2; d++)
    c[d * 2] = o / 2 * (a[d * 2] * v - a[d * 2 + 1] * h), c[d * 2 + 1] = s / 2 * (a[d * 2 + 1] * v + a[d * 2] * h), c[d * 2] += n, c[d * 2 + 1] += i;
  var y;
  if (l > 0) {
    var g = Mn(c, -l);
    y = Rn(g);
  } else
    y = c;
  return Tt(e, r, y);
}, jh = function(e, r, a, n, i, o, s, u) {
  for (var l = new Array(a.length * 2), c = 0; c < u.length; c++) {
    var f = u[c];
    l[c * 4 + 0] = f.startX, l[c * 4 + 1] = f.startY, l[c * 4 + 2] = f.stopX, l[c * 4 + 3] = f.stopY;
    var v = Math.pow(f.cx - e, 2) + Math.pow(f.cy - r, 2);
    if (v <= Math.pow(f.radius, 2))
      return !0;
  }
  return Tt(e, r, l);
}, Rn = function(e) {
  for (var r = new Array(e.length / 2), a, n, i, o, s, u, l, c, f = 0; f < e.length / 4; f++) {
    a = e[f * 4], n = e[f * 4 + 1], i = e[f * 4 + 2], o = e[f * 4 + 3], f < e.length / 4 - 1 ? (s = e[(f + 1) * 4], u = e[(f + 1) * 4 + 1], l = e[(f + 1) * 4 + 2], c = e[(f + 1) * 4 + 3]) : (s = e[0], u = e[1], l = e[2], c = e[3]);
    var v = sr(a, n, i, o, s, u, l, c, !0);
    r[f * 2] = v[0], r[f * 2 + 1] = v[1];
  }
  return r;
}, Mn = function(e, r) {
  for (var a = new Array(e.length * 2), n, i, o, s, u = 0; u < e.length / 2; u++) {
    n = e[u * 2], i = e[u * 2 + 1], u < e.length / 2 - 1 ? (o = e[(u + 1) * 2], s = e[(u + 1) * 2 + 1]) : (o = e[0], s = e[1]);
    var l = s - i, c = -(o - n), f = Math.sqrt(l * l + c * c), v = l / f, h = c / f;
    a[u * 4] = n + v * r, a[u * 4 + 1] = i + h * r, a[u * 4 + 2] = o + v * r, a[u * 4 + 3] = s + h * r;
  }
  return a;
}, eg = function(e, r, a, n, i, o) {
  var s = a - e, u = n - r;
  s /= i, u /= o;
  var l = Math.sqrt(s * s + u * u), c = l - 1;
  if (c < 0)
    return [];
  var f = c / l;
  return [(a - e) * f + e, (n - r) * f + r];
}, Dr = function(e, r, a, n, i, o, s) {
  return e -= i, r -= o, e /= a / 2 + s, r /= n / 2 + s, e * e + r * r <= 1;
}, Ea = function(e, r, a, n, i, o, s) {
  var u = [a - e, n - r], l = [e - i, r - o], c = u[0] * u[0] + u[1] * u[1], f = 2 * (l[0] * u[0] + l[1] * u[1]), v = l[0] * l[0] + l[1] * l[1] - s * s, h = f * f - 4 * c * v;
  if (h < 0)
    return [];
  var d = (-f + Math.sqrt(h)) / (2 * c), y = (-f - Math.sqrt(h)) / (2 * c), g = Math.min(d, y), p = Math.max(d, y), m = [];
  if (g >= 0 && g <= 1 && m.push(g), p >= 0 && p <= 1 && m.push(p), m.length === 0)
    return [];
  var b = m[0] * u[0] + e, w = m[0] * u[1] + r;
  if (m.length > 1) {
    if (m[0] == m[1])
      return [b, w];
    var C = m[1] * u[0] + e, E = m[1] * u[1] + r;
    return [b, w, C, E];
  } else
    return [b, w];
}, Pi = function(e, r, a) {
  return r <= e && e <= a || a <= e && e <= r ? e : e <= r && r <= a || a <= r && r <= e ? r : a;
}, sr = function(e, r, a, n, i, o, s, u, l) {
  var c = e - i, f = a - e, v = s - i, h = r - o, d = n - r, y = u - o, g = v * h - y * c, p = f * h - d * c, m = y * f - v * d;
  if (m !== 0) {
    var b = g / m, w = p / m, C = 1e-3, E = 0 - C, x = 1 + C;
    return E <= b && b <= x && E <= w && w <= x ? [e + b * f, r + b * d] : l ? [e + b * f, r + b * d] : [];
  } else
    return g === 0 || p === 0 ? Pi(e, a, s) === s ? [s, u] : Pi(e, a, i) === i ? [i, o] : Pi(i, s, a) === a ? [a, n] : [] : [];
}, tg = function(e, r, a, n, i) {
  var o = [], s = n / 2, u = i / 2, l = r, c = a;
  o.push({
    x: l + s * e[0],
    y: c + u * e[1]
  });
  for (var f = 1; f < e.length / 2; f++)
    o.push({
      x: l + s * e[f * 2],
      y: c + u * e[f * 2 + 1]
    });
  return o;
}, Ma = function(e, r, a, n, i, o, s, u) {
  var l = [], c, f = new Array(a.length), v = !0;
  o == null && (v = !1);
  var h;
  if (v) {
    for (var d = 0; d < f.length / 2; d++)
      f[d * 2] = a[d * 2] * o + n, f[d * 2 + 1] = a[d * 2 + 1] * s + i;
    if (u > 0) {
      var y = Mn(f, -u);
      h = Rn(y);
    } else
      h = f;
  } else
    h = a;
  for (var g, p, m, b, w = 0; w < h.length / 2; w++)
    g = h[w * 2], p = h[w * 2 + 1], w < h.length / 2 - 1 ? (m = h[(w + 1) * 2], b = h[(w + 1) * 2 + 1]) : (m = h[0], b = h[1]), c = sr(e, r, n, i, g, p, m, b), c.length !== 0 && l.push(c[0], c[1]);
  return l;
}, rg = function(e, r, a, n, i, o, s, u, l) {
  var c = [], f, v = new Array(a.length * 2);
  l.forEach(function(m, b) {
    b === 0 ? (v[v.length - 2] = m.startX, v[v.length - 1] = m.startY) : (v[b * 4 - 2] = m.startX, v[b * 4 - 1] = m.startY), v[b * 4] = m.stopX, v[b * 4 + 1] = m.stopY, f = Ea(e, r, n, i, m.cx, m.cy, m.radius), f.length !== 0 && c.push(f[0], f[1]);
  });
  for (var h = 0; h < v.length / 4; h++)
    f = sr(e, r, n, i, v[h * 4], v[h * 4 + 1], v[h * 4 + 2], v[h * 4 + 3], !1), f.length !== 0 && c.push(f[0], f[1]);
  if (c.length > 2) {
    for (var d = [c[0], c[1]], y = Math.pow(d[0] - e, 2) + Math.pow(d[1] - r, 2), g = 1; g < c.length / 2; g++) {
      var p = Math.pow(c[g * 2] - e, 2) + Math.pow(c[g * 2 + 1] - r, 2);
      p <= y && (d[0] = c[g * 2], d[1] = c[g * 2 + 1], y = p);
    }
    return d;
  }
  return c;
}, rn = function(e, r, a) {
  var n = [e[0] - r[0], e[1] - r[1]], i = Math.sqrt(n[0] * n[0] + n[1] * n[1]), o = (i - a) / i;
  return o < 0 && (o = 1e-5), [r[0] + o * n[0], r[1] + o * n[1]];
}, yt = function(e, r) {
  var a = Go(e, r);
  return a = _c(a), a;
}, _c = function(e) {
  for (var r, a, n = e.length / 2, i = 1 / 0, o = 1 / 0, s = -1 / 0, u = -1 / 0, l = 0; l < n; l++)
    r = e[2 * l], a = e[2 * l + 1], i = Math.min(i, r), s = Math.max(s, r), o = Math.min(o, a), u = Math.max(u, a);
  for (var c = 2 / (s - i), f = 2 / (u - o), v = 0; v < n; v++)
    r = e[2 * v] = e[2 * v] * c, a = e[2 * v + 1] = e[2 * v + 1] * f, i = Math.min(i, r), s = Math.max(s, r), o = Math.min(o, a), u = Math.max(u, a);
  if (o < -1)
    for (var h = 0; h < n; h++)
      a = e[2 * h + 1] = e[2 * h + 1] + (-1 - o);
  return e;
}, Go = function(e, r) {
  var a = 1 / e * 2 * Math.PI, n = e % 2 === 0 ? Math.PI / 2 + a / 2 : Math.PI / 2;
  n += r;
  for (var i = new Array(e * 2), o, s = 0; s < e; s++)
    o = s * a + n, i[2 * s] = Math.cos(o), i[2 * s + 1] = Math.sin(-o);
  return i;
}, vr = function(e, r) {
  return Math.min(e / 4, r / 4, 8);
}, Gc = function(e, r) {
  return Math.min(e / 10, r / 10, 8);
}, ws = function() {
  return 8;
}, ag = function(e, r, a) {
  return [e - 2 * r + a, 2 * (r - e), e];
}, Ho = function(e, r) {
  return {
    heightOffset: Math.min(15, 0.05 * r),
    widthOffset: Math.min(100, 0.25 * e),
    ctrlPtOffsetPct: 0.05
  };
};
function Di(t, e) {
  function r(f) {
    for (var v = [], h = 0; h < f.length; h++) {
      var d = f[h], y = f[(h + 1) % f.length], g = {
        x: y.x - d.x,
        y: y.y - d.y
      }, p = {
        x: -g.y,
        y: g.x
      }, m = Math.sqrt(p.x * p.x + p.y * p.y);
      v.push({
        x: p.x / m,
        y: p.y / m
      });
    }
    return v;
  }
  function a(f, v) {
    var h = 1 / 0, d = -1 / 0, y = St(f), g;
    try {
      for (y.s(); !(g = y.n()).done; ) {
        var p = g.value, m = p.x * v.x + p.y * v.y;
        h = Math.min(h, m), d = Math.max(d, m);
      }
    } catch (b) {
      y.e(b);
    } finally {
      y.f();
    }
    return {
      min: h,
      max: d
    };
  }
  function n(f, v) {
    return !(f.max < v.min || v.max < f.min);
  }
  var i = [].concat(Pn(r(t)), Pn(r(e))), o = St(i), s;
  try {
    for (o.s(); !(s = o.n()).done; ) {
      var u = s.value, l = a(t, u), c = a(e, u);
      if (!n(l, c))
        return !1;
    }
  } catch (f) {
    o.e(f);
  } finally {
    o.f();
  }
  return !0;
}
var ng = ft({
  dampingFactor: 0.8,
  precision: 1e-6,
  iterations: 200,
  weight: function(e) {
    return 1;
  }
}), ig = {
  pageRank: function(e) {
    for (var r = ng(e), a = r.dampingFactor, n = r.precision, i = r.iterations, o = r.weight, s = this._private.cy, u = this.byGroup(), l = u.nodes, c = u.edges, f = l.length, v = f * f, h = c.length, d = new Array(v), y = new Array(f), g = (1 - a) / f, p = 0; p < f; p++) {
      for (var m = 0; m < f; m++) {
        var b = p * f + m;
        d[b] = 0;
      }
      y[p] = 0;
    }
    for (var w = 0; w < h; w++) {
      var C = c[w], E = C.data("source"), x = C.data("target");
      if (E !== x) {
        var T = l.indexOfId(E), k = l.indexOfId(x), D = o(C), P = k * f + T;
        d[P] += D, y[T] += D;
      }
    }
    for (var A = 1 / f + g, B = 0; B < f; B++)
      if (y[B] === 0)
        for (var R = 0; R < f; R++) {
          var I = R * f + B;
          d[I] = A;
        }
      else
        for (var O = 0; O < f; O++) {
          var M = O * f + B;
          d[M] = d[M] / y[B] + g;
        }
    for (var N = new Array(f), H = new Array(f), G, F = 0; F < f; F++)
      N[F] = 1;
    for (var _ = 0; _ < i; _++) {
      for (var X = 0; X < f; X++)
        H[X] = 0;
      for (var J = 0; J < f; J++)
        for (var Y = 0; Y < f; Y++) {
          var j = J * f + Y;
          H[J] += d[j] * N[Y];
        }
      qh(H), G = N, N = H, H = G;
      for (var te = 0, ae = 0; ae < f; ae++) {
        var z = G[ae] - N[ae];
        te += z * z;
      }
      if (te < n)
        break;
    }
    var L = {
      rank: function(W) {
        return W = s.collection(W)[0], N[l.indexOf(W)];
      }
    };
    return L;
  }
  // pageRank
}, du = ft({
  root: null,
  weight: function(e) {
    return 1;
  },
  directed: !1,
  alpha: 0
}), Jr = {
  degreeCentralityNormalized: function(e) {
    e = du(e);
    var r = this.cy(), a = this.nodes(), n = a.length;
    if (e.directed) {
      for (var c = {}, f = {}, v = 0, h = 0, d = 0; d < n; d++) {
        var y = a[d], g = y.id();
        e.root = y;
        var p = this.degreeCentrality(e);
        v < p.indegree && (v = p.indegree), h < p.outdegree && (h = p.outdegree), c[g] = p.indegree, f[g] = p.outdegree;
      }
      return {
        indegree: function(b) {
          return v == 0 ? 0 : (he(b) && (b = r.filter(b)), c[b.id()] / v);
        },
        outdegree: function(b) {
          return h === 0 ? 0 : (he(b) && (b = r.filter(b)), f[b.id()] / h);
        }
      };
    } else {
      for (var i = {}, o = 0, s = 0; s < n; s++) {
        var u = a[s];
        e.root = u;
        var l = this.degreeCentrality(e);
        o < l.degree && (o = l.degree), i[u.id()] = l.degree;
      }
      return {
        degree: function(b) {
          return o === 0 ? 0 : (he(b) && (b = r.filter(b)), i[b.id()] / o);
        }
      };
    }
  },
  // degreeCentralityNormalized
  // Implemented from the algorithm in Opsahl's paper
  // "Node centrality in weighted networks: Generalizing degree and shortest paths"
  // check the heading 2 "Degree"
  degreeCentrality: function(e) {
    e = du(e);
    var r = this.cy(), a = this, n = e, i = n.root, o = n.weight, s = n.directed, u = n.alpha;
    if (i = r.collection(i)[0], s) {
      for (var h = i.connectedEdges(), d = h.filter(function(E) {
        return E.target().same(i) && a.has(E);
      }), y = h.filter(function(E) {
        return E.source().same(i) && a.has(E);
      }), g = d.length, p = y.length, m = 0, b = 0, w = 0; w < d.length; w++)
        m += o(d[w]);
      for (var C = 0; C < y.length; C++)
        b += o(y[C]);
      return {
        indegree: Math.pow(g, 1 - u) * Math.pow(m, u),
        outdegree: Math.pow(p, 1 - u) * Math.pow(b, u)
      };
    } else {
      for (var l = i.connectedEdges().intersection(a), c = l.length, f = 0, v = 0; v < l.length; v++)
        f += o(l[v]);
      return {
        degree: Math.pow(c, 1 - u) * Math.pow(f, u)
      };
    }
  }
  // degreeCentrality
};
Jr.dc = Jr.degreeCentrality;
Jr.dcn = Jr.degreeCentralityNormalised = Jr.degreeCentralityNormalized;
var hu = ft({
  harmonic: !0,
  weight: function() {
    return 1;
  },
  directed: !1,
  root: null
}), jr = {
  closenessCentralityNormalized: function(e) {
    for (var r = hu(e), a = r.harmonic, n = r.weight, i = r.directed, o = this.cy(), s = {}, u = 0, l = this.nodes(), c = this.floydWarshall({
      weight: n,
      directed: i
    }), f = 0; f < l.length; f++) {
      for (var v = 0, h = l[f], d = 0; d < l.length; d++)
        if (f !== d) {
          var y = c.distance(h, l[d]);
          a ? v += 1 / y : v += y;
        }
      a || (v = 1 / v), u < v && (u = v), s[h.id()] = v;
    }
    return {
      closeness: function(p) {
        return u == 0 ? 0 : (he(p) ? p = o.filter(p)[0].id() : p = p.id(), s[p] / u);
      }
    };
  },
  // Implemented from pseudocode from wikipedia
  closenessCentrality: function(e) {
    var r = hu(e), a = r.root, n = r.weight, i = r.directed, o = r.harmonic;
    a = this.filter(a)[0];
    for (var s = this.dijkstra({
      root: a,
      weight: n,
      directed: i
    }), u = 0, l = this.nodes(), c = 0; c < l.length; c++) {
      var f = l[c];
      if (!f.same(a)) {
        var v = s.distanceTo(f);
        o ? u += 1 / v : u += v;
      }
    }
    return o ? u : 1 / u;
  }
  // closenessCentrality
};
jr.cc = jr.closenessCentrality;
jr.ccn = jr.closenessCentralityNormalised = jr.closenessCentralityNormalized;
var og = ft({
  weight: null,
  directed: !1
}), $o = {
  // Implemented from the algorithm in the paper "On Variants of Shortest-Path Betweenness Centrality and their Generic Computation" by Ulrik Brandes
  betweennessCentrality: function(e) {
    for (var r = og(e), a = r.directed, n = r.weight, i = n != null, o = this.cy(), s = this.nodes(), u = {}, l = {}, c = 0, f = {
      set: function(b, w) {
        l[b] = w, w > c && (c = w);
      },
      get: function(b) {
        return l[b];
      }
    }, v = 0; v < s.length; v++) {
      var h = s[v], d = h.id();
      a ? u[d] = h.outgoers().nodes() : u[d] = h.openNeighborhood().nodes(), f.set(d, 0);
    }
    for (var y = function() {
      for (var b = s[g].id(), w = [], C = {}, E = {}, x = {}, T = new $a(function(J, Y) {
        return x[J] - x[Y];
      }), k = 0; k < s.length; k++) {
        var D = s[k].id();
        C[D] = [], E[D] = 0, x[D] = 1 / 0;
      }
      for (E[b] = 1, x[b] = 0, T.push(b); !T.empty(); ) {
        var P = T.pop();
        if (w.push(P), i)
          for (var A = 0; A < u[P].length; A++) {
            var B = u[P][A], R = o.getElementById(P), I = void 0;
            R.edgesTo(B).length > 0 ? I = R.edgesTo(B)[0] : I = B.edgesTo(R)[0];
            var O = n(I);
            B = B.id(), x[B] > x[P] + O && (x[B] = x[P] + O, T.nodes.indexOf(B) < 0 ? T.push(B) : T.updateItem(B), E[B] = 0, C[B] = []), x[B] == x[P] + O && (E[B] = E[B] + E[P], C[B].push(P));
          }
        else
          for (var M = 0; M < u[P].length; M++) {
            var N = u[P][M].id();
            x[N] == 1 / 0 && (T.push(N), x[N] = x[P] + 1), x[N] == x[P] + 1 && (E[N] = E[N] + E[P], C[N].push(P));
          }
      }
      for (var H = {}, G = 0; G < s.length; G++)
        H[s[G].id()] = 0;
      for (; w.length > 0; ) {
        for (var F = w.pop(), _ = 0; _ < C[F].length; _++) {
          var X = C[F][_];
          H[X] = H[X] + E[X] / E[F] * (1 + H[F]);
        }
        F != s[g].id() && f.set(F, f.get(F) + H[F]);
      }
    }, g = 0; g < s.length; g++)
      y();
    var p = {
      betweenness: function(b) {
        var w = o.collection(b).id();
        return f.get(w);
      },
      betweennessNormalized: function(b) {
        if (c == 0)
          return 0;
        var w = o.collection(b).id();
        return f.get(w) / c;
      }
    };
    return p.betweennessNormalised = p.betweennessNormalized, p;
  }
  // betweennessCentrality
};
$o.bc = $o.betweennessCentrality;
var sg = ft({
  expandFactor: 2,
  // affects time of computation and cluster granularity to some extent: M * M
  inflateFactor: 2,
  // affects cluster granularity (the greater the value, the more clusters): M(i,j) / E(j)
  multFactor: 1,
  // optional self loops for each node. Use a neutral value to improve cluster computations.
  maxIterations: 20,
  // maximum number of iterations of the MCL algorithm in a single run
  attributes: [
    // attributes/features used to group nodes, ie. similarity values between nodes
    function(t) {
      return 1;
    }
  ]
}), ug = function(e) {
  return sg(e);
}, lg = function(e, r) {
  for (var a = 0, n = 0; n < r.length; n++)
    a += r[n](e);
  return a;
}, cg = function(e, r, a) {
  for (var n = 0; n < r; n++)
    e[n * r + n] = a;
}, Hc = function(e, r) {
  for (var a, n = 0; n < r; n++) {
    a = 0;
    for (var i = 0; i < r; i++)
      a += e[i * r + n];
    for (var o = 0; o < r; o++)
      e[o * r + n] = e[o * r + n] / a;
  }
}, fg = function(e, r, a) {
  for (var n = new Array(a * a), i = 0; i < a; i++) {
    for (var o = 0; o < a; o++)
      n[i * a + o] = 0;
    for (var s = 0; s < a; s++)
      for (var u = 0; u < a; u++)
        n[i * a + u] += e[i * a + s] * r[s * a + u];
  }
  return n;
}, vg = function(e, r, a) {
  for (var n = e.slice(0), i = 1; i < a; i++)
    e = fg(e, n, r);
  return e;
}, dg = function(e, r, a) {
  for (var n = new Array(r * r), i = 0; i < r * r; i++)
    n[i] = Math.pow(e[i], a);
  return Hc(n, r), n;
}, hg = function(e, r, a, n) {
  for (var i = 0; i < a; i++) {
    var o = Math.round(e[i] * Math.pow(10, n)) / Math.pow(10, n), s = Math.round(r[i] * Math.pow(10, n)) / Math.pow(10, n);
    if (o !== s)
      return !1;
  }
  return !0;
}, gg = function(e, r, a, n) {
  for (var i = [], o = 0; o < r; o++) {
    for (var s = [], u = 0; u < r; u++)
      Math.round(e[o * r + u] * 1e3) / 1e3 > 0 && s.push(a[u]);
    s.length !== 0 && i.push(n.collection(s));
  }
  return i;
}, pg = function(e, r) {
  for (var a = 0; a < e.length; a++)
    if (!r[a] || e[a].id() !== r[a].id())
      return !1;
  return !0;
}, yg = function(e) {
  for (var r = 0; r < e.length; r++)
    for (var a = 0; a < e.length; a++)
      r != a && pg(e[r], e[a]) && e.splice(a, 1);
  return e;
}, gu = function(e) {
  for (var r = this.nodes(), a = this.edges(), n = this.cy(), i = ug(e), o = {}, s = 0; s < r.length; s++)
    o[r[s].id()] = s;
  for (var u = r.length, l = u * u, c = new Array(l), f, v = 0; v < l; v++)
    c[v] = 0;
  for (var h = 0; h < a.length; h++) {
    var d = a[h], y = o[d.source().id()], g = o[d.target().id()], p = lg(d, i.attributes);
    c[y * u + g] += p, c[g * u + y] += p;
  }
  cg(c, u, i.multFactor), Hc(c, u);
  for (var m = !0, b = 0; m && b < i.maxIterations; )
    m = !1, f = vg(c, u, i.expandFactor), c = dg(f, u, i.inflateFactor), hg(c, f, l, 4) || (m = !0), b++;
  var w = gg(c, u, r, n);
  return w = yg(w), w;
}, mg = {
  markovClustering: gu,
  mcl: gu
}, bg = function(e) {
  return e;
}, $c = function(e, r) {
  return Math.abs(r - e);
}, pu = function(e, r, a) {
  return e + $c(r, a);
}, yu = function(e, r, a) {
  return e + Math.pow(a - r, 2);
}, wg = function(e) {
  return Math.sqrt(e);
}, xg = function(e, r, a) {
  return Math.max(e, $c(r, a));
}, ga = function(e, r, a, n, i) {
  for (var o = arguments.length > 5 && arguments[5] !== void 0 ? arguments[5] : bg, s = n, u, l, c = 0; c < e; c++)
    u = r(c), l = a(c), s = i(s, u, l);
  return o(s);
}, ra = {
  euclidean: function(e, r, a) {
    return e >= 2 ? ga(e, r, a, 0, yu, wg) : ga(e, r, a, 0, pu);
  },
  squaredEuclidean: function(e, r, a) {
    return ga(e, r, a, 0, yu);
  },
  manhattan: function(e, r, a) {
    return ga(e, r, a, 0, pu);
  },
  max: function(e, r, a) {
    return ga(e, r, a, -1 / 0, xg);
  }
};
ra["squared-euclidean"] = ra.squaredEuclidean;
ra.squaredeuclidean = ra.squaredEuclidean;
function Wn(t, e, r, a, n, i) {
  var o;
  return Ue(t) ? o = t : o = ra[t] || ra.euclidean, e === 0 && Ue(t) ? o(n, i) : o(e, r, a, n, i);
}
var Eg = ft({
  k: 2,
  m: 2,
  sensitivityThreshold: 1e-4,
  distance: "euclidean",
  maxIterations: 10,
  attributes: [],
  testMode: !1,
  testCentroids: null
}), xs = function(e) {
  return Eg(e);
}, In = function(e, r, a, n, i) {
  var o = i !== "kMedoids", s = o ? function(f) {
    return a[f];
  } : function(f) {
    return n[f](a);
  }, u = function(v) {
    return n[v](r);
  }, l = a, c = r;
  return Wn(e, n.length, s, u, l, c);
}, Ai = function(e, r, a) {
  for (var n = a.length, i = new Array(n), o = new Array(n), s = new Array(r), u = null, l = 0; l < n; l++)
    i[l] = e.min(a[l]).value, o[l] = e.max(a[l]).value;
  for (var c = 0; c < r; c++) {
    u = [];
    for (var f = 0; f < n; f++)
      u[f] = Math.random() * (o[f] - i[f]) + i[f];
    s[c] = u;
  }
  return s;
}, Wc = function(e, r, a, n, i) {
  for (var o = 1 / 0, s = 0, u = 0; u < r.length; u++) {
    var l = In(a, e, r[u], n, i);
    l < o && (o = l, s = u);
  }
  return s;
}, Uc = function(e, r, a) {
  for (var n = [], i = null, o = 0; o < r.length; o++)
    i = r[o], a[i.id()] === e && n.push(i);
  return n;
}, Cg = function(e, r, a) {
  return Math.abs(r - e) <= a;
}, Tg = function(e, r, a) {
  for (var n = 0; n < e.length; n++)
    for (var i = 0; i < e[n].length; i++) {
      var o = Math.abs(e[n][i] - r[n][i]);
      if (o > a)
        return !1;
    }
  return !0;
}, Sg = function(e, r, a) {
  for (var n = 0; n < a; n++)
    if (e === r[n]) return !0;
  return !1;
}, mu = function(e, r) {
  var a = new Array(r);
  if (e.length < 50)
    for (var n = 0; n < r; n++) {
      for (var i = e[Math.floor(Math.random() * e.length)]; Sg(i, a, n); )
        i = e[Math.floor(Math.random() * e.length)];
      a[n] = i;
    }
  else
    for (var o = 0; o < r; o++)
      a[o] = e[Math.floor(Math.random() * e.length)];
  return a;
}, bu = function(e, r, a) {
  for (var n = 0, i = 0; i < r.length; i++)
    n += In("manhattan", r[i], e, a, "kMedoids");
  return n;
}, kg = function(e) {
  var r = this.cy(), a = this.nodes(), n = null, i = xs(e), o = new Array(i.k), s = {}, u;
  i.testMode ? typeof i.testCentroids == "number" ? (i.testCentroids, u = Ai(a, i.k, i.attributes)) : rt(i.testCentroids) === "object" ? u = i.testCentroids : u = Ai(a, i.k, i.attributes) : u = Ai(a, i.k, i.attributes);
  for (var l = !0, c = 0; l && c < i.maxIterations; ) {
    for (var f = 0; f < a.length; f++)
      n = a[f], s[n.id()] = Wc(n, u, i.distance, i.attributes, "kMeans");
    l = !1;
    for (var v = 0; v < i.k; v++) {
      var h = Uc(v, a, s);
      if (h.length !== 0) {
        for (var d = i.attributes.length, y = u[v], g = new Array(d), p = new Array(d), m = 0; m < d; m++) {
          p[m] = 0;
          for (var b = 0; b < h.length; b++)
            n = h[b], p[m] += i.attributes[m](n);
          g[m] = p[m] / h.length, Cg(g[m], y[m], i.sensitivityThreshold) || (l = !0);
        }
        u[v] = g, o[v] = r.collection(h);
      }
    }
    c++;
  }
  return o;
}, Pg = function(e) {
  var r = this.cy(), a = this.nodes(), n = null, i = xs(e), o = new Array(i.k), s, u = {}, l, c = new Array(i.k);
  i.testMode ? typeof i.testCentroids == "number" || (rt(i.testCentroids) === "object" ? s = i.testCentroids : s = mu(a, i.k)) : s = mu(a, i.k);
  for (var f = !0, v = 0; f && v < i.maxIterations; ) {
    for (var h = 0; h < a.length; h++)
      n = a[h], u[n.id()] = Wc(n, s, i.distance, i.attributes, "kMedoids");
    f = !1;
    for (var d = 0; d < s.length; d++) {
      var y = Uc(d, a, u);
      if (y.length !== 0) {
        c[d] = bu(s[d], y, i.attributes);
        for (var g = 0; g < y.length; g++)
          l = bu(y[g], y, i.attributes), l < c[d] && (c[d] = l, s[d] = y[g], f = !0);
        o[d] = r.collection(y);
      }
    }
    v++;
  }
  return o;
}, Dg = function(e, r, a, n, i) {
  for (var o, s, u = 0; u < r.length; u++)
    for (var l = 0; l < e.length; l++)
      n[u][l] = Math.pow(a[u][l], i.m);
  for (var c = 0; c < e.length; c++)
    for (var f = 0; f < i.attributes.length; f++) {
      o = 0, s = 0;
      for (var v = 0; v < r.length; v++)
        o += n[v][c] * i.attributes[f](r[v]), s += n[v][c];
      e[c][f] = o / s;
    }
}, Ag = function(e, r, a, n, i) {
  for (var o = 0; o < e.length; o++)
    r[o] = e[o].slice();
  for (var s, u, l, c = 2 / (i.m - 1), f = 0; f < a.length; f++)
    for (var v = 0; v < n.length; v++) {
      s = 0;
      for (var h = 0; h < a.length; h++)
        u = In(i.distance, n[v], a[f], i.attributes, "cmeans"), l = In(i.distance, n[v], a[h], i.attributes, "cmeans"), s += Math.pow(u / l, c);
      e[v][f] = 1 / s;
    }
}, Bg = function(e, r, a, n) {
  for (var i = new Array(a.k), o = 0; o < i.length; o++)
    i[o] = [];
  for (var s, u, l = 0; l < r.length; l++) {
    s = -1 / 0, u = -1;
    for (var c = 0; c < r[0].length; c++)
      r[l][c] > s && (s = r[l][c], u = c);
    i[u].push(e[l]);
  }
  for (var f = 0; f < i.length; f++)
    i[f] = n.collection(i[f]);
  return i;
}, wu = function(e) {
  var r = this.cy(), a = this.nodes(), n = xs(e), i, o, s, u, l;
  u = new Array(a.length);
  for (var c = 0; c < a.length; c++)
    u[c] = new Array(n.k);
  s = new Array(a.length);
  for (var f = 0; f < a.length; f++)
    s[f] = new Array(n.k);
  for (var v = 0; v < a.length; v++) {
    for (var h = 0, d = 0; d < n.k; d++)
      s[v][d] = Math.random(), h += s[v][d];
    for (var y = 0; y < n.k; y++)
      s[v][y] = s[v][y] / h;
  }
  o = new Array(n.k);
  for (var g = 0; g < n.k; g++)
    o[g] = new Array(n.attributes.length);
  l = new Array(a.length);
  for (var p = 0; p < a.length; p++)
    l[p] = new Array(n.k);
  for (var m = !0, b = 0; m && b < n.maxIterations; )
    m = !1, Dg(o, a, s, l, n), Ag(s, u, o, a, n), Tg(s, u, n.sensitivityThreshold) || (m = !0), b++;
  return i = Bg(a, s, n, r), {
    clusters: i,
    degreeOfMembership: s
  };
}, Rg = {
  kMeans: kg,
  kMedoids: Pg,
  fuzzyCMeans: wu,
  fcm: wu
}, Mg = ft({
  distance: "euclidean",
  // distance metric to compare nodes
  linkage: "min",
  // linkage criterion : how to determine the distance between clusters of nodes
  mode: "threshold",
  // mode:'threshold' => clusters must be threshold distance apart
  threshold: 1 / 0,
  // the distance threshold
  // mode:'dendrogram' => the nodes are organised as leaves in a tree (siblings are close), merging makes clusters
  addDendrogram: !1,
  // whether to add the dendrogram to the graph for viz
  dendrogramDepth: 0,
  // depth at which dendrogram branches are merged into the returned clusters
  attributes: []
  // array of attr functions
}), Ig = {
  single: "min",
  complete: "max"
}, Lg = function(e) {
  var r = Mg(e), a = Ig[r.linkage];
  return a != null && (r.linkage = a), r;
}, xu = function(e, r, a, n, i) {
  for (var o = 0, s = 1 / 0, u, l = i.attributes, c = function(k, D) {
    return Wn(i.distance, l.length, function(P) {
      return l[P](k);
    }, function(P) {
      return l[P](D);
    }, k, D);
  }, f = 0; f < e.length; f++) {
    var v = e[f].key, h = a[v][n[v]];
    h < s && (o = v, s = h);
  }
  if (i.mode === "threshold" && s >= i.threshold || i.mode === "dendrogram" && e.length === 1)
    return !1;
  var d = r[o], y = r[n[o]], g;
  i.mode === "dendrogram" ? g = {
    left: d,
    right: y,
    key: d.key
  } : g = {
    value: d.value.concat(y.value),
    key: d.key
  }, e[d.index] = g, e.splice(y.index, 1), r[d.key] = g;
  for (var p = 0; p < e.length; p++) {
    var m = e[p];
    d.key === m.key ? u = 1 / 0 : i.linkage === "min" ? (u = a[d.key][m.key], a[d.key][m.key] > a[y.key][m.key] && (u = a[y.key][m.key])) : i.linkage === "max" ? (u = a[d.key][m.key], a[d.key][m.key] < a[y.key][m.key] && (u = a[y.key][m.key])) : i.linkage === "mean" ? u = (a[d.key][m.key] * d.size + a[y.key][m.key] * y.size) / (d.size + y.size) : i.mode === "dendrogram" ? u = c(m.value, d.value) : u = c(m.value[0], d.value[0]), a[d.key][m.key] = a[m.key][d.key] = u;
  }
  for (var b = 0; b < e.length; b++) {
    var w = e[b].key;
    if (n[w] === d.key || n[w] === y.key) {
      for (var C = w, E = 0; E < e.length; E++) {
        var x = e[E].key;
        a[w][x] < a[w][C] && (C = x);
      }
      n[w] = C;
    }
    e[b].index = b;
  }
  return d.key = y.key = d.index = y.index = null, !0;
}, Yr = function(e, r, a) {
  e && (e.value ? r.push(e.value) : (e.left && Yr(e.left, r), e.right && Yr(e.right, r)));
}, Wo = function(e, r) {
  if (!e) return "";
  if (e.left && e.right) {
    var a = Wo(e.left, r), n = Wo(e.right, r), i = r.add({
      group: "nodes",
      data: {
        id: a + "," + n
      }
    });
    return r.add({
      group: "edges",
      data: {
        source: a,
        target: i.id()
      }
    }), r.add({
      group: "edges",
      data: {
        source: n,
        target: i.id()
      }
    }), i.id();
  } else if (e.value)
    return e.value.id();
}, Uo = function(e, r, a) {
  if (!e) return [];
  var n = [], i = [], o = [];
  return r === 0 ? (e.left && Yr(e.left, n), e.right && Yr(e.right, i), o = n.concat(i), [a.collection(o)]) : r === 1 ? e.value ? [a.collection(e.value)] : (e.left && Yr(e.left, n), e.right && Yr(e.right, i), [a.collection(n), a.collection(i)]) : e.value ? [a.collection(e.value)] : (e.left && (n = Uo(e.left, r - 1, a)), e.right && (i = Uo(e.right, r - 1, a)), n.concat(i));
}, Eu = function(e) {
  for (var r = this.cy(), a = this.nodes(), n = Lg(e), i = n.attributes, o = function(b, w) {
    return Wn(n.distance, i.length, function(C) {
      return i[C](b);
    }, function(C) {
      return i[C](w);
    }, b, w);
  }, s = [], u = [], l = [], c = [], f = 0; f < a.length; f++) {
    var v = {
      value: n.mode === "dendrogram" ? a[f] : [a[f]],
      key: f,
      index: f
    };
    s[f] = v, c[f] = v, u[f] = [], l[f] = 0;
  }
  for (var h = 0; h < s.length; h++)
    for (var d = 0; d <= h; d++) {
      var y = void 0;
      n.mode === "dendrogram" ? y = h === d ? 1 / 0 : o(s[h].value, s[d].value) : y = h === d ? 1 / 0 : o(s[h].value[0], s[d].value[0]), u[h][d] = y, u[d][h] = y, y < u[h][l[h]] && (l[h] = d);
    }
  for (var g = xu(s, c, u, l, n); g; )
    g = xu(s, c, u, l, n);
  var p;
  return n.mode === "dendrogram" ? (p = Uo(s[0], n.dendrogramDepth, r), n.addDendrogram && Wo(s[0], r)) : (p = new Array(s.length), s.forEach(function(m, b) {
    m.key = m.index = null, p[b] = r.collection(m.value);
  })), p;
}, Og = {
  hierarchicalClustering: Eu,
  hca: Eu
}, Ng = ft({
  distance: "euclidean",
  // distance metric to compare attributes between two nodes
  preference: "median",
  // suitability of a data point to serve as an exemplar
  damping: 0.8,
  // damping factor between [0.5, 1)
  maxIterations: 1e3,
  // max number of iterations to run
  minIterations: 100,
  // min number of iterations to run in order for clustering to stop
  attributes: [
    // functions to quantify the similarity between any two points
    // e.g. node => node.data('weight')
  ]
}), zg = function(e) {
  var r = e.damping, a = e.preference;
  0.5 <= r && r < 1 || $e("Damping must range on [0.5, 1).  Got: ".concat(r));
  var n = ["median", "mean", "min", "max"];
  return n.some(function(i) {
    return i === a;
  }) || ne(a) || $e("Preference must be one of [".concat(n.map(function(i) {
    return "'".concat(i, "'");
  }).join(", "), "] or a number.  Got: ").concat(a)), Ng(e);
}, Fg = function(e, r, a, n) {
  var i = function(s, u) {
    return n[u](s);
  };
  return -Wn(e, n.length, function(o) {
    return i(r, o);
  }, function(o) {
    return i(a, o);
  }, r, a);
}, qg = function(e, r) {
  var a = null;
  return r === "median" ? a = zh(e) : r === "mean" ? a = Nh(e) : r === "min" ? a = Lh(e) : r === "max" ? a = Oh(e) : a = r, a;
}, Vg = function(e, r, a) {
  for (var n = [], i = 0; i < e; i++)
    r[i * e + i] + a[i * e + i] > 0 && n.push(i);
  return n;
}, Cu = function(e, r, a) {
  for (var n = [], i = 0; i < e; i++) {
    for (var o = -1, s = -1 / 0, u = 0; u < a.length; u++) {
      var l = a[u];
      r[i * e + l] > s && (o = l, s = r[i * e + l]);
    }
    o > 0 && n.push(o);
  }
  for (var c = 0; c < a.length; c++)
    n[a[c]] = a[c];
  return n;
}, _g = function(e, r, a) {
  for (var n = Cu(e, r, a), i = 0; i < a.length; i++) {
    for (var o = [], s = 0; s < n.length; s++)
      n[s] === a[i] && o.push(s);
    for (var u = -1, l = -1 / 0, c = 0; c < o.length; c++) {
      for (var f = 0, v = 0; v < o.length; v++)
        f += r[o[v] * e + o[c]];
      f > l && (u = c, l = f);
    }
    a[i] = o[u];
  }
  return n = Cu(e, r, a), n;
}, Tu = function(e) {
  for (var r = this.cy(), a = this.nodes(), n = zg(e), i = {}, o = 0; o < a.length; o++)
    i[a[o].id()] = o;
  var s, u, l, c, f, v;
  s = a.length, u = s * s, l = new Array(u);
  for (var h = 0; h < u; h++)
    l[h] = -1 / 0;
  for (var d = 0; d < s; d++)
    for (var y = 0; y < s; y++)
      d !== y && (l[d * s + y] = Fg(n.distance, a[d], a[y], n.attributes));
  c = qg(l, n.preference);
  for (var g = 0; g < s; g++)
    l[g * s + g] = c;
  f = new Array(u);
  for (var p = 0; p < u; p++)
    f[p] = 0;
  v = new Array(u);
  for (var m = 0; m < u; m++)
    v[m] = 0;
  for (var b = new Array(s), w = new Array(s), C = new Array(s), E = 0; E < s; E++)
    b[E] = 0, w[E] = 0, C[E] = 0;
  for (var x = new Array(s * n.minIterations), T = 0; T < x.length; T++)
    x[T] = 0;
  var k;
  for (k = 0; k < n.maxIterations; k++) {
    for (var D = 0; D < s; D++) {
      for (var P = -1 / 0, A = -1 / 0, B = -1, R = 0, I = 0; I < s; I++)
        b[I] = f[D * s + I], R = v[D * s + I] + l[D * s + I], R >= P ? (A = P, P = R, B = I) : R > A && (A = R);
      for (var O = 0; O < s; O++)
        f[D * s + O] = (1 - n.damping) * (l[D * s + O] - P) + n.damping * b[O];
      f[D * s + B] = (1 - n.damping) * (l[D * s + B] - A) + n.damping * b[B];
    }
    for (var M = 0; M < s; M++) {
      for (var N = 0, H = 0; H < s; H++)
        b[H] = v[H * s + M], w[H] = Math.max(0, f[H * s + M]), N += w[H];
      N -= w[M], w[M] = f[M * s + M], N += w[M];
      for (var G = 0; G < s; G++)
        v[G * s + M] = (1 - n.damping) * Math.min(0, N - w[G]) + n.damping * b[G];
      v[M * s + M] = (1 - n.damping) * (N - w[M]) + n.damping * b[M];
    }
    for (var F = 0, _ = 0; _ < s; _++) {
      var X = v[_ * s + _] + f[_ * s + _] > 0 ? 1 : 0;
      x[k % n.minIterations * s + _] = X, F += X;
    }
    if (F > 0 && (k >= n.minIterations - 1 || k == n.maxIterations - 1)) {
      for (var J = 0, Y = 0; Y < s; Y++) {
        C[Y] = 0;
        for (var j = 0; j < n.minIterations; j++)
          C[Y] += x[j * s + Y];
        (C[Y] === 0 || C[Y] === n.minIterations) && J++;
      }
      if (J === s)
        break;
    }
  }
  for (var te = Vg(s, f, v), ae = _g(s, l, te), z = {}, L = 0; L < te.length; L++)
    z[te[L]] = [];
  for (var V = 0; V < a.length; V++) {
    var W = i[a[V].id()], K = ae[W];
    K != null && z[K].push(a[V]);
  }
  for (var re = new Array(te.length), le = 0; le < te.length; le++)
    re[le] = r.collection(z[te[le]]);
  return re;
}, Gg = {
  affinityPropagation: Tu,
  ap: Tu
}, Hg = ft({
  root: void 0,
  directed: !1
}), $g = {
  hierholzer: function(e) {
    if (!Me(e)) {
      var r = arguments;
      e = {
        root: r[0],
        directed: r[1]
      };
    }
    var a = Hg(e), n = a.root, i = a.directed, o = this, s = !1, u, l, c;
    n && (c = he(n) ? this.filter(n)[0].id() : n[0].id());
    var f = {}, v = {};
    i ? o.forEach(function(m) {
      var b = m.id();
      if (m.isNode()) {
        var w = m.indegree(!0), C = m.outdegree(!0), E = w - C, x = C - w;
        E == 1 ? u ? s = !0 : u = b : x == 1 ? l ? s = !0 : l = b : (x > 1 || E > 1) && (s = !0), f[b] = [], m.outgoers().forEach(function(T) {
          T.isEdge() && f[b].push(T.id());
        });
      } else
        v[b] = [void 0, m.target().id()];
    }) : o.forEach(function(m) {
      var b = m.id();
      if (m.isNode()) {
        var w = m.degree(!0);
        w % 2 && (u ? l ? s = !0 : l = b : u = b), f[b] = [], m.connectedEdges().forEach(function(C) {
          return f[b].push(C.id());
        });
      } else
        v[b] = [m.source().id(), m.target().id()];
    });
    var h = {
      found: !1,
      trail: void 0
    };
    if (s) return h;
    if (l && u)
      if (i) {
        if (c && l != c)
          return h;
        c = l;
      } else {
        if (c && l != c && u != c)
          return h;
        c || (c = l);
      }
    else
      c || (c = o[0].id());
    var d = function(b) {
      for (var w = b, C = [b], E, x, T; f[w].length; )
        E = f[w].shift(), x = v[E][0], T = v[E][1], w != T ? (f[T] = f[T].filter(function(k) {
          return k != E;
        }), w = T) : !i && w != x && (f[x] = f[x].filter(function(k) {
          return k != E;
        }), w = x), C.unshift(E), C.unshift(w);
      return C;
    }, y = [], g = [];
    for (g = d(c); g.length != 1; )
      f[g[0]].length == 0 ? (y.unshift(o.getElementById(g.shift())), y.unshift(o.getElementById(g.shift()))) : g = d(g.shift()).concat(g);
    y.unshift(o.getElementById(g.shift()));
    for (var p in f)
      if (f[p].length)
        return h;
    return h.found = !0, h.trail = this.spawn(y, !0), h;
  }
}, an = function() {
  var e = this, r = {}, a = 0, n = 0, i = [], o = [], s = {}, u = function(v, h) {
    for (var d = o.length - 1, y = [], g = e.spawn(); o[d].x != v || o[d].y != h; )
      y.push(o.pop().edge), d--;
    y.push(o.pop().edge), y.forEach(function(p) {
      var m = p.connectedNodes().intersection(e);
      g.merge(p), m.forEach(function(b) {
        var w = b.id(), C = b.connectedEdges().intersection(e);
        g.merge(b), r[w].cutVertex ? g.merge(C.filter(function(E) {
          return E.isLoop();
        })) : g.merge(C);
      });
    }), i.push(g);
  }, l = function(v, h, d) {
    v === d && (n += 1), r[h] = {
      id: a,
      low: a++,
      cutVertex: !1
    };
    var y = e.getElementById(h).connectedEdges().intersection(e);
    if (y.size() === 0)
      i.push(e.spawn(e.getElementById(h)));
    else {
      var g, p, m, b;
      y.forEach(function(w) {
        g = w.source().id(), p = w.target().id(), m = g === h ? p : g, m !== d && (b = w.id(), s[b] || (s[b] = !0, o.push({
          x: h,
          y: m,
          edge: w
        })), m in r ? r[h].low = Math.min(r[h].low, r[m].id) : (l(v, m, h), r[h].low = Math.min(r[h].low, r[m].low), r[h].id <= r[m].low && (r[h].cutVertex = !0, u(h, m))));
      });
    }
  };
  e.forEach(function(f) {
    if (f.isNode()) {
      var v = f.id();
      v in r || (n = 0, l(v, v), r[v].cutVertex = n > 1);
    }
  });
  var c = Object.keys(r).filter(function(f) {
    return r[f].cutVertex;
  }).map(function(f) {
    return e.getElementById(f);
  });
  return {
    cut: e.spawn(c),
    components: i
  };
}, Wg = {
  hopcroftTarjanBiconnected: an,
  htbc: an,
  htb: an,
  hopcroftTarjanBiconnectedComponents: an
}, nn = function() {
  var e = this, r = {}, a = 0, n = [], i = [], o = e.spawn(e), s = function(l) {
    i.push(l), r[l] = {
      index: a,
      low: a++,
      explored: !1
    };
    var c = e.getElementById(l).connectedEdges().intersection(e);
    if (c.forEach(function(y) {
      var g = y.target().id();
      g !== l && (g in r || s(g), r[g].explored || (r[l].low = Math.min(r[l].low, r[g].low)));
    }), r[l].index === r[l].low) {
      for (var f = e.spawn(); ; ) {
        var v = i.pop();
        if (f.merge(e.getElementById(v)), r[v].low = r[l].index, r[v].explored = !0, v === l)
          break;
      }
      var h = f.edgesWith(f), d = f.merge(h);
      n.push(d), o = o.difference(d);
    }
  };
  return e.forEach(function(u) {
    if (u.isNode()) {
      var l = u.id();
      l in r || s(l);
    }
  }), {
    cut: o,
    components: n
  };
}, Ug = {
  tarjanStronglyConnected: nn,
  tsc: nn,
  tscc: nn,
  tarjanStronglyConnectedComponents: nn
}, Kc = {};
[Ba, Eh, Ch, Sh, Ph, Ah, Mh, ig, Jr, jr, $o, mg, Rg, Og, Gg, $g, Wg, Ug].forEach(function(t) {
  ye(Kc, t);
});
/*!
Embeddable Minimum Strictly-Compliant Promises/A+ 1.1.1 Thenable
Copyright (c) 2013-2014 Ralf S. Engelschall (http://engelschall.com)
Licensed under The MIT License (http://opensource.org/licenses/MIT)
*/
var Xc = 0, Yc = 1, Zc = 2, Ft = function(e) {
  if (!(this instanceof Ft)) return new Ft(e);
  this.id = "Thenable/1.0.7", this.state = Xc, this.fulfillValue = void 0, this.rejectReason = void 0, this.onFulfilled = [], this.onRejected = [], this.proxy = {
    then: this.then.bind(this)
  }, typeof e == "function" && e.call(this, this.fulfill.bind(this), this.reject.bind(this));
};
Ft.prototype = {
  /*  promise resolving methods  */
  fulfill: function(e) {
    return Su(this, Yc, "fulfillValue", e);
  },
  reject: function(e) {
    return Su(this, Zc, "rejectReason", e);
  },
  /*  "The then Method" [Promises/A+ 1.1, 1.2, 2.2]  */
  then: function(e, r) {
    var a = this, n = new Ft();
    return a.onFulfilled.push(Pu(e, n, "fulfill")), a.onRejected.push(Pu(r, n, "reject")), Qc(a), n.proxy;
  }
};
var Su = function(e, r, a, n) {
  return e.state === Xc && (e.state = r, e[a] = n, Qc(e)), e;
}, Qc = function(e) {
  e.state === Yc ? ku(e, "onFulfilled", e.fulfillValue) : e.state === Zc && ku(e, "onRejected", e.rejectReason);
}, ku = function(e, r, a) {
  if (e[r].length !== 0) {
    var n = e[r];
    e[r] = [];
    var i = function() {
      for (var s = 0; s < n.length; s++) n[s](a);
    };
    typeof setImmediate == "function" ? setImmediate(i) : setTimeout(i, 0);
  }
}, Pu = function(e, r, a) {
  return function(n) {
    if (typeof e != "function")
      r[a].call(r, n);
    else {
      var i;
      try {
        i = e(n);
      } catch (o) {
        r.reject(o);
        return;
      }
      Jc(r, i);
    }
  };
}, Jc = function(e, r) {
  if (e === r || e.proxy === r) {
    e.reject(new TypeError("cannot resolve promise with itself"));
    return;
  }
  var a;
  if (rt(r) === "object" && r !== null || typeof r == "function")
    try {
      a = r.then;
    } catch (i) {
      e.reject(i);
      return;
    }
  if (typeof a == "function") {
    var n = !1;
    try {
      a.call(
        r,
        /*  resolvePromise  */
        /*  [Promises/A+ 2.3.3.3.1]  */
        function(i) {
          n || (n = !0, i === r ? e.reject(new TypeError("circular thenable chain")) : Jc(e, i));
        },
        /*  rejectPromise  */
        /*  [Promises/A+ 2.3.3.3.2]  */
        function(i) {
          n || (n = !0, e.reject(i));
        }
      );
    } catch (i) {
      n || e.reject(i);
    }
    return;
  }
  e.fulfill(r);
};
Ft.all = function(t) {
  return new Ft(function(e, r) {
    for (var a = new Array(t.length), n = 0, i = function(u, l) {
      a[u] = l, n++, n === t.length && e(a);
    }, o = 0; o < t.length; o++)
      (function(s) {
        var u = t[s], l = u != null && u.then != null;
        if (l)
          u.then(function(f) {
            i(s, f);
          }, function(f) {
            r(f);
          });
        else {
          var c = u;
          i(s, c);
        }
      })(o);
  });
};
Ft.resolve = function(t) {
  return new Ft(function(e, r) {
    e(t);
  });
};
Ft.reject = function(t) {
  return new Ft(function(e, r) {
    r(t);
  });
};
var sa = typeof Promise < "u" ? Promise : Ft, Ko = function(e, r, a) {
  var n = fs(e), i = !n, o = this._private = ye({
    duration: 1e3
  }, r, a);
  if (o.target = e, o.style = o.style || o.css, o.started = !1, o.playing = !1, o.hooked = !1, o.applying = !1, o.progress = 0, o.completes = [], o.frames = [], o.complete && Ue(o.complete) && o.completes.push(o.complete), i) {
    var s = e.position();
    o.startPosition = o.startPosition || {
      x: s.x,
      y: s.y
    }, o.startStyle = o.startStyle || e.cy().style().getAnimationStartStyle(e, o.style);
  }
  if (n) {
    var u = e.pan();
    o.startPan = {
      x: u.x,
      y: u.y
    }, o.startZoom = e.zoom();
  }
  this.length = 1, this[0] = this;
}, Rr = Ko.prototype;
ye(Rr, {
  instanceString: function() {
    return "animation";
  },
  hook: function() {
    var e = this._private;
    if (!e.hooked) {
      var r, a = e.target._private.animation;
      e.queue ? r = a.queue : r = a.current, r.push(this), kt(e.target) && e.target.cy().addToAnimationPool(e.target), e.hooked = !0;
    }
    return this;
  },
  play: function() {
    var e = this._private;
    return e.progress === 1 && (e.progress = 0), e.playing = !0, e.started = !1, e.stopped = !1, this.hook(), this;
  },
  playing: function() {
    return this._private.playing;
  },
  apply: function() {
    var e = this._private;
    return e.applying = !0, e.started = !1, e.stopped = !1, this.hook(), this;
  },
  applying: function() {
    return this._private.applying;
  },
  pause: function() {
    var e = this._private;
    return e.playing = !1, e.started = !1, this;
  },
  stop: function() {
    var e = this._private;
    return e.playing = !1, e.started = !1, e.stopped = !0, this;
  },
  rewind: function() {
    return this.progress(0);
  },
  fastforward: function() {
    return this.progress(1);
  },
  time: function(e) {
    var r = this._private;
    return e === void 0 ? r.progress * r.duration : this.progress(e / r.duration);
  },
  progress: function(e) {
    var r = this._private, a = r.playing;
    return e === void 0 ? r.progress : (a && this.pause(), r.progress = e, r.started = !1, a && this.play(), this);
  },
  completed: function() {
    return this._private.progress === 1;
  },
  reverse: function() {
    var e = this._private, r = e.playing;
    r && this.pause(), e.progress = 1 - e.progress, e.started = !1;
    var a = function(l, c) {
      var f = e[l];
      f != null && (e[l] = e[c], e[c] = f);
    };
    if (a("zoom", "startZoom"), a("pan", "startPan"), a("position", "startPosition"), e.style)
      for (var n = 0; n < e.style.length; n++) {
        var i = e.style[n], o = i.name, s = e.startStyle[o];
        e.startStyle[o] = i, e.style[n] = s;
      }
    return r && this.play(), this;
  },
  promise: function(e) {
    var r = this._private, a;
    switch (e) {
      case "frame":
        a = r.frames;
        break;
      default:
      case "complete":
      case "completed":
        a = r.completes;
    }
    return new sa(function(n, i) {
      a.push(function() {
        n();
      });
    });
  }
});
Rr.complete = Rr.completed;
Rr.run = Rr.play;
Rr.running = Rr.playing;
var Kg = {
  animated: function() {
    return function() {
      var r = this, a = r.length !== void 0, n = a ? r : [r], i = this._private.cy || this;
      if (!i.styleEnabled())
        return !1;
      var o = n[0];
      if (o)
        return o._private.animation.current.length > 0;
    };
  },
  // animated
  clearQueue: function() {
    return function() {
      var r = this, a = r.length !== void 0, n = a ? r : [r], i = this._private.cy || this;
      if (!i.styleEnabled())
        return this;
      for (var o = 0; o < n.length; o++) {
        var s = n[o];
        s._private.animation.queue = [];
      }
      return this;
    };
  },
  // clearQueue
  delay: function() {
    return function(r, a) {
      var n = this._private.cy || this;
      return n.styleEnabled() ? this.animate({
        delay: r,
        duration: r,
        complete: a
      }) : this;
    };
  },
  // delay
  delayAnimation: function() {
    return function(r, a) {
      var n = this._private.cy || this;
      return n.styleEnabled() ? this.animation({
        delay: r,
        duration: r,
        complete: a
      }) : this;
    };
  },
  // delay
  animation: function() {
    return function(r, a) {
      var n = this, i = n.length !== void 0, o = i ? n : [n], s = this._private.cy || this, u = !i, l = !u;
      if (!s.styleEnabled())
        return this;
      var c = s.style();
      r = ye({}, r, a);
      var f = Object.keys(r).length === 0;
      if (f)
        return new Ko(o[0], r);
      switch (r.duration === void 0 && (r.duration = 400), r.duration) {
        case "slow":
          r.duration = 600;
          break;
        case "fast":
          r.duration = 200;
          break;
      }
      if (l && (r.style = c.getPropsList(r.style || r.css), r.css = void 0), l && r.renderedPosition != null) {
        var v = r.renderedPosition, h = s.pan(), d = s.zoom();
        r.position = zc(v, d, h);
      }
      if (u && r.panBy != null) {
        var y = r.panBy, g = s.pan();
        r.pan = {
          x: g.x + y.x,
          y: g.y + y.y
        };
      }
      var p = r.center || r.centre;
      if (u && p != null) {
        var m = s.getCenterPan(p.eles, r.zoom);
        m != null && (r.pan = m);
      }
      if (u && r.fit != null) {
        var b = r.fit, w = s.getFitViewport(b.eles || b.boundingBox, b.padding);
        w != null && (r.pan = w.pan, r.zoom = w.zoom);
      }
      if (u && Me(r.zoom)) {
        var C = s.getZoomedViewport(r.zoom);
        C != null ? (C.zoomed && (r.zoom = C.zoom), C.panned && (r.pan = C.pan)) : r.zoom = null;
      }
      return new Ko(o[0], r);
    };
  },
  // animate
  animate: function() {
    return function(r, a) {
      var n = this, i = n.length !== void 0, o = i ? n : [n], s = this._private.cy || this;
      if (!s.styleEnabled())
        return this;
      a && (r = ye({}, r, a));
      for (var u = 0; u < o.length; u++) {
        var l = o[u], c = l.animated() && (r.queue === void 0 || r.queue), f = l.animation(r, c ? {
          queue: !0
        } : void 0);
        f.play();
      }
      return this;
    };
  },
  // animate
  stop: function() {
    return function(r, a) {
      var n = this, i = n.length !== void 0, o = i ? n : [n], s = this._private.cy || this;
      if (!s.styleEnabled())
        return this;
      for (var u = 0; u < o.length; u++) {
        for (var l = o[u], c = l._private, f = c.animation.current, v = 0; v < f.length; v++) {
          var h = f[v], d = h._private;
          a && (d.duration = 0);
        }
        r && (c.animation.queue = []), a || (c.animation.current = []);
      }
      return s.notify("draw"), this;
    };
  }
  // stop
}, Bi, Du;
function Un() {
  if (Du) return Bi;
  Du = 1;
  var t = Array.isArray;
  return Bi = t, Bi;
}
var Ri, Au;
function Xg() {
  if (Au) return Ri;
  Au = 1;
  var t = Un(), e = Ga(), r = /\.|\[(?:[^[\]]*|(["'])(?:(?!\1)[^\\]|\\.)*?\1)\]/, a = /^\w*$/;
  function n(i, o) {
    if (t(i))
      return !1;
    var s = typeof i;
    return s == "number" || s == "symbol" || s == "boolean" || i == null || e(i) ? !0 : a.test(i) || !r.test(i) || o != null && i in Object(o);
  }
  return Ri = n, Ri;
}
var Mi, Bu;
function Yg() {
  if (Bu) return Mi;
  Bu = 1;
  var t = Dc(), e = _a(), r = "[object AsyncFunction]", a = "[object Function]", n = "[object GeneratorFunction]", i = "[object Proxy]";
  function o(s) {
    if (!e(s))
      return !1;
    var u = t(s);
    return u == a || u == n || u == r || u == i;
  }
  return Mi = o, Mi;
}
var Ii, Ru;
function Zg() {
  if (Ru) return Ii;
  Ru = 1;
  var t = Gn(), e = t["__core-js_shared__"];
  return Ii = e, Ii;
}
var Li, Mu;
function Qg() {
  if (Mu) return Li;
  Mu = 1;
  var t = Zg(), e = (function() {
    var a = /[^.]+$/.exec(t && t.keys && t.keys.IE_PROTO || "");
    return a ? "Symbol(src)_1." + a : "";
  })();
  function r(a) {
    return !!e && e in a;
  }
  return Li = r, Li;
}
var Oi, Iu;
function Jg() {
  if (Iu) return Oi;
  Iu = 1;
  var t = Function.prototype, e = t.toString;
  function r(a) {
    if (a != null) {
      try {
        return e.call(a);
      } catch {
      }
      try {
        return a + "";
      } catch {
      }
    }
    return "";
  }
  return Oi = r, Oi;
}
var Ni, Lu;
function jg() {
  if (Lu) return Ni;
  Lu = 1;
  var t = Yg(), e = Qg(), r = _a(), a = Jg(), n = /[\\^$.*+?()[\]{}|]/g, i = /^\[object .+?Constructor\]$/, o = Function.prototype, s = Object.prototype, u = o.toString, l = s.hasOwnProperty, c = RegExp(
    "^" + u.call(l).replace(n, "\\$&").replace(/hasOwnProperty|(function).*?(?=\\\()| for .+?(?=\\\])/g, "$1.*?") + "$"
  );
  function f(v) {
    if (!r(v) || e(v))
      return !1;
    var h = t(v) ? c : i;
    return h.test(a(v));
  }
  return Ni = f, Ni;
}
var zi, Ou;
function ep() {
  if (Ou) return zi;
  Ou = 1;
  function t(e, r) {
    return e == null ? void 0 : e[r];
  }
  return zi = t, zi;
}
var Fi, Nu;
function Es() {
  if (Nu) return Fi;
  Nu = 1;
  var t = jg(), e = ep();
  function r(a, n) {
    var i = e(a, n);
    return t(i) ? i : void 0;
  }
  return Fi = r, Fi;
}
var qi, zu;
function Kn() {
  if (zu) return qi;
  zu = 1;
  var t = Es(), e = t(Object, "create");
  return qi = e, qi;
}
var Vi, Fu;
function tp() {
  if (Fu) return Vi;
  Fu = 1;
  var t = Kn();
  function e() {
    this.__data__ = t ? t(null) : {}, this.size = 0;
  }
  return Vi = e, Vi;
}
var _i, qu;
function rp() {
  if (qu) return _i;
  qu = 1;
  function t(e) {
    var r = this.has(e) && delete this.__data__[e];
    return this.size -= r ? 1 : 0, r;
  }
  return _i = t, _i;
}
var Gi, Vu;
function ap() {
  if (Vu) return Gi;
  Vu = 1;
  var t = Kn(), e = "__lodash_hash_undefined__", r = Object.prototype, a = r.hasOwnProperty;
  function n(i) {
    var o = this.__data__;
    if (t) {
      var s = o[i];
      return s === e ? void 0 : s;
    }
    return a.call(o, i) ? o[i] : void 0;
  }
  return Gi = n, Gi;
}
var Hi, _u;
function np() {
  if (_u) return Hi;
  _u = 1;
  var t = Kn(), e = Object.prototype, r = e.hasOwnProperty;
  function a(n) {
    var i = this.__data__;
    return t ? i[n] !== void 0 : r.call(i, n);
  }
  return Hi = a, Hi;
}
var $i, Gu;
function ip() {
  if (Gu) return $i;
  Gu = 1;
  var t = Kn(), e = "__lodash_hash_undefined__";
  function r(a, n) {
    var i = this.__data__;
    return this.size += this.has(a) ? 0 : 1, i[a] = t && n === void 0 ? e : n, this;
  }
  return $i = r, $i;
}
var Wi, Hu;
function op() {
  if (Hu) return Wi;
  Hu = 1;
  var t = tp(), e = rp(), r = ap(), a = np(), n = ip();
  function i(o) {
    var s = -1, u = o == null ? 0 : o.length;
    for (this.clear(); ++s < u; ) {
      var l = o[s];
      this.set(l[0], l[1]);
    }
  }
  return i.prototype.clear = t, i.prototype.delete = e, i.prototype.get = r, i.prototype.has = a, i.prototype.set = n, Wi = i, Wi;
}
var Ui, $u;
function sp() {
  if ($u) return Ui;
  $u = 1;
  function t() {
    this.__data__ = [], this.size = 0;
  }
  return Ui = t, Ui;
}
var Ki, Wu;
function jc() {
  if (Wu) return Ki;
  Wu = 1;
  function t(e, r) {
    return e === r || e !== e && r !== r;
  }
  return Ki = t, Ki;
}
var Xi, Uu;
function Xn() {
  if (Uu) return Xi;
  Uu = 1;
  var t = jc();
  function e(r, a) {
    for (var n = r.length; n--; )
      if (t(r[n][0], a))
        return n;
    return -1;
  }
  return Xi = e, Xi;
}
var Yi, Ku;
function up() {
  if (Ku) return Yi;
  Ku = 1;
  var t = Xn(), e = Array.prototype, r = e.splice;
  function a(n) {
    var i = this.__data__, o = t(i, n);
    if (o < 0)
      return !1;
    var s = i.length - 1;
    return o == s ? i.pop() : r.call(i, o, 1), --this.size, !0;
  }
  return Yi = a, Yi;
}
var Zi, Xu;
function lp() {
  if (Xu) return Zi;
  Xu = 1;
  var t = Xn();
  function e(r) {
    var a = this.__data__, n = t(a, r);
    return n < 0 ? void 0 : a[n][1];
  }
  return Zi = e, Zi;
}
var Qi, Yu;
function cp() {
  if (Yu) return Qi;
  Yu = 1;
  var t = Xn();
  function e(r) {
    return t(this.__data__, r) > -1;
  }
  return Qi = e, Qi;
}
var Ji, Zu;
function fp() {
  if (Zu) return Ji;
  Zu = 1;
  var t = Xn();
  function e(r, a) {
    var n = this.__data__, i = t(n, r);
    return i < 0 ? (++this.size, n.push([r, a])) : n[i][1] = a, this;
  }
  return Ji = e, Ji;
}
var ji, Qu;
function vp() {
  if (Qu) return ji;
  Qu = 1;
  var t = sp(), e = up(), r = lp(), a = cp(), n = fp();
  function i(o) {
    var s = -1, u = o == null ? 0 : o.length;
    for (this.clear(); ++s < u; ) {
      var l = o[s];
      this.set(l[0], l[1]);
    }
  }
  return i.prototype.clear = t, i.prototype.delete = e, i.prototype.get = r, i.prototype.has = a, i.prototype.set = n, ji = i, ji;
}
var eo, Ju;
function dp() {
  if (Ju) return eo;
  Ju = 1;
  var t = Es(), e = Gn(), r = t(e, "Map");
  return eo = r, eo;
}
var to, ju;
function hp() {
  if (ju) return to;
  ju = 1;
  var t = op(), e = vp(), r = dp();
  function a() {
    this.size = 0, this.__data__ = {
      hash: new t(),
      map: new (r || e)(),
      string: new t()
    };
  }
  return to = a, to;
}
var ro, el;
function gp() {
  if (el) return ro;
  el = 1;
  function t(e) {
    var r = typeof e;
    return r == "string" || r == "number" || r == "symbol" || r == "boolean" ? e !== "__proto__" : e === null;
  }
  return ro = t, ro;
}
var ao, tl;
function Yn() {
  if (tl) return ao;
  tl = 1;
  var t = gp();
  function e(r, a) {
    var n = r.__data__;
    return t(a) ? n[typeof a == "string" ? "string" : "hash"] : n.map;
  }
  return ao = e, ao;
}
var no, rl;
function pp() {
  if (rl) return no;
  rl = 1;
  var t = Yn();
  function e(r) {
    var a = t(this, r).delete(r);
    return this.size -= a ? 1 : 0, a;
  }
  return no = e, no;
}
var io, al;
function yp() {
  if (al) return io;
  al = 1;
  var t = Yn();
  function e(r) {
    return t(this, r).get(r);
  }
  return io = e, io;
}
var oo, nl;
function mp() {
  if (nl) return oo;
  nl = 1;
  var t = Yn();
  function e(r) {
    return t(this, r).has(r);
  }
  return oo = e, oo;
}
var so, il;
function bp() {
  if (il) return so;
  il = 1;
  var t = Yn();
  function e(r, a) {
    var n = t(this, r), i = n.size;
    return n.set(r, a), this.size += n.size == i ? 0 : 1, this;
  }
  return so = e, so;
}
var uo, ol;
function wp() {
  if (ol) return uo;
  ol = 1;
  var t = hp(), e = pp(), r = yp(), a = mp(), n = bp();
  function i(o) {
    var s = -1, u = o == null ? 0 : o.length;
    for (this.clear(); ++s < u; ) {
      var l = o[s];
      this.set(l[0], l[1]);
    }
  }
  return i.prototype.clear = t, i.prototype.delete = e, i.prototype.get = r, i.prototype.has = a, i.prototype.set = n, uo = i, uo;
}
var lo, sl;
function xp() {
  if (sl) return lo;
  sl = 1;
  var t = wp(), e = "Expected a function";
  function r(a, n) {
    if (typeof a != "function" || n != null && typeof n != "function")
      throw new TypeError(e);
    var i = function() {
      var o = arguments, s = n ? n.apply(this, o) : o[0], u = i.cache;
      if (u.has(s))
        return u.get(s);
      var l = a.apply(this, o);
      return i.cache = u.set(s, l) || u, l;
    };
    return i.cache = new (r.Cache || t)(), i;
  }
  return r.Cache = t, lo = r, lo;
}
var co, ul;
function Ep() {
  if (ul) return co;
  ul = 1;
  var t = xp(), e = 500;
  function r(a) {
    var n = t(a, function(o) {
      return i.size === e && i.clear(), o;
    }), i = n.cache;
    return n;
  }
  return co = r, co;
}
var fo, ll;
function ef() {
  if (ll) return fo;
  ll = 1;
  var t = Ep(), e = /[^.[\]]+|\[(?:(-?\d+(?:\.\d+)?)|(["'])((?:(?!\2)[^\\]|\\.)*?)\2)\]|(?=(?:\.|\[\])(?:\.|\[\]|$))/g, r = /\\(\\)?/g, a = t(function(n) {
    var i = [];
    return n.charCodeAt(0) === 46 && i.push(""), n.replace(e, function(o, s, u, l) {
      i.push(u ? l.replace(r, "$1") : s || o);
    }), i;
  });
  return fo = a, fo;
}
var vo, cl;
function tf() {
  if (cl) return vo;
  cl = 1;
  function t(e, r) {
    for (var a = -1, n = e == null ? 0 : e.length, i = Array(n); ++a < n; )
      i[a] = r(e[a], a, e);
    return i;
  }
  return vo = t, vo;
}
var ho, fl;
function Cp() {
  if (fl) return ho;
  fl = 1;
  var t = ds(), e = tf(), r = Un(), a = Ga(), n = t ? t.prototype : void 0, i = n ? n.toString : void 0;
  function o(s) {
    if (typeof s == "string")
      return s;
    if (r(s))
      return e(s, o) + "";
    if (a(s))
      return i ? i.call(s) : "";
    var u = s + "";
    return u == "0" && 1 / s == -1 / 0 ? "-0" : u;
  }
  return ho = o, ho;
}
var go, vl;
function rf() {
  if (vl) return go;
  vl = 1;
  var t = Cp();
  function e(r) {
    return r == null ? "" : t(r);
  }
  return go = e, go;
}
var po, dl;
function af() {
  if (dl) return po;
  dl = 1;
  var t = Un(), e = Xg(), r = ef(), a = rf();
  function n(i, o) {
    return t(i) ? i : e(i, o) ? [i] : r(a(i));
  }
  return po = n, po;
}
var yo, hl;
function Cs() {
  if (hl) return yo;
  hl = 1;
  var t = Ga();
  function e(r) {
    if (typeof r == "string" || t(r))
      return r;
    var a = r + "";
    return a == "0" && 1 / r == -1 / 0 ? "-0" : a;
  }
  return yo = e, yo;
}
var mo, gl;
function Tp() {
  if (gl) return mo;
  gl = 1;
  var t = af(), e = Cs();
  function r(a, n) {
    n = t(n, a);
    for (var i = 0, o = n.length; a != null && i < o; )
      a = a[e(n[i++])];
    return i && i == o ? a : void 0;
  }
  return mo = r, mo;
}
var bo, pl;
function Sp() {
  if (pl) return bo;
  pl = 1;
  var t = Tp();
  function e(r, a, n) {
    var i = r == null ? void 0 : t(r, a);
    return i === void 0 ? n : i;
  }
  return bo = e, bo;
}
var kp = Sp(), Pp = /* @__PURE__ */ Va(kp), wo, yl;
function Dp() {
  if (yl) return wo;
  yl = 1;
  var t = Es(), e = (function() {
    try {
      var r = t(Object, "defineProperty");
      return r({}, "", {}), r;
    } catch {
    }
  })();
  return wo = e, wo;
}
var xo, ml;
function Ap() {
  if (ml) return xo;
  ml = 1;
  var t = Dp();
  function e(r, a, n) {
    a == "__proto__" && t ? t(r, a, {
      configurable: !0,
      enumerable: !0,
      value: n,
      writable: !0
    }) : r[a] = n;
  }
  return xo = e, xo;
}
var Eo, bl;
function Bp() {
  if (bl) return Eo;
  bl = 1;
  var t = Ap(), e = jc(), r = Object.prototype, a = r.hasOwnProperty;
  function n(i, o, s) {
    var u = i[o];
    (!(a.call(i, o) && e(u, s)) || s === void 0 && !(o in i)) && t(i, o, s);
  }
  return Eo = n, Eo;
}
var Co, wl;
function Rp() {
  if (wl) return Co;
  wl = 1;
  var t = 9007199254740991, e = /^(?:0|[1-9]\d*)$/;
  function r(a, n) {
    var i = typeof a;
    return n = n ?? t, !!n && (i == "number" || i != "symbol" && e.test(a)) && a > -1 && a % 1 == 0 && a < n;
  }
  return Co = r, Co;
}
var To, xl;
function Mp() {
  if (xl) return To;
  xl = 1;
  var t = Bp(), e = af(), r = Rp(), a = _a(), n = Cs();
  function i(o, s, u, l) {
    if (!a(o))
      return o;
    s = e(s, o);
    for (var c = -1, f = s.length, v = f - 1, h = o; h != null && ++c < f; ) {
      var d = n(s[c]), y = u;
      if (d === "__proto__" || d === "constructor" || d === "prototype")
        return o;
      if (c != v) {
        var g = h[d];
        y = l ? l(g, d, h) : void 0, y === void 0 && (y = a(g) ? g : r(s[c + 1]) ? [] : {});
      }
      t(h, d, y), h = h[d];
    }
    return o;
  }
  return To = i, To;
}
var So, El;
function Ip() {
  if (El) return So;
  El = 1;
  var t = Mp();
  function e(r, a, n) {
    return r == null ? r : t(r, a, n);
  }
  return So = e, So;
}
var Lp = Ip(), Op = /* @__PURE__ */ Va(Lp), ko, Cl;
function Np() {
  if (Cl) return ko;
  Cl = 1;
  function t(e, r) {
    var a = -1, n = e.length;
    for (r || (r = Array(n)); ++a < n; )
      r[a] = e[a];
    return r;
  }
  return ko = t, ko;
}
var Po, Tl;
function zp() {
  if (Tl) return Po;
  Tl = 1;
  var t = tf(), e = Np(), r = Un(), a = Ga(), n = ef(), i = Cs(), o = rf();
  function s(u) {
    return r(u) ? t(u, i) : a(u) ? [u] : e(n(o(u)));
  }
  return Po = s, Po;
}
var Fp = zp(), qp = /* @__PURE__ */ Va(Fp), Vp = {
  // access data field
  data: function(e) {
    var r = {
      field: "data",
      bindingEvent: "data",
      allowBinding: !1,
      allowSetting: !1,
      allowGetting: !1,
      settingEvent: "data",
      settingTriggersEvent: !1,
      triggerFnName: "trigger",
      immutableKeys: {},
      // key => true if immutable
      updateStyle: !1,
      beforeGet: function(n) {
      },
      beforeSet: function(n, i) {
      },
      onSet: function(n) {
      },
      canSet: function(n) {
        return !0;
      }
    };
    return e = ye({}, r, e), function(n, i) {
      var o = e, s = this, u = s.length !== void 0, l = u ? s : [s], c = u ? s[0] : s;
      if (he(n)) {
        var f = n.indexOf(".") !== -1, v = f && qp(n);
        if (o.allowGetting && i === void 0) {
          var h;
          return c && (o.beforeGet(c), v && c._private[o.field][n] === void 0 ? h = Pp(c._private[o.field], v) : h = c._private[o.field][n]), h;
        } else if (o.allowSetting && i !== void 0) {
          var d = !o.immutableKeys[n];
          if (d) {
            var y = mc({}, n, i);
            o.beforeSet(s, y);
            for (var g = 0, p = l.length; g < p; g++) {
              var m = l[g];
              o.canSet(m) && (v && c._private[o.field][n] === void 0 ? Op(m._private[o.field], v, i) : m._private[o.field][n] = i);
            }
            o.updateStyle && s.updateStyle(), o.onSet(s), o.settingTriggersEvent && s[o.triggerFnName](o.settingEvent);
          }
        }
      } else if (o.allowSetting && Me(n)) {
        var b = n, w, C, E = Object.keys(b);
        o.beforeSet(s, b);
        for (var x = 0; x < E.length; x++) {
          w = E[x], C = b[w];
          var T = !o.immutableKeys[w];
          if (T)
            for (var k = 0; k < l.length; k++) {
              var D = l[k];
              o.canSet(D) && (D._private[o.field][w] = C);
            }
        }
        o.updateStyle && s.updateStyle(), o.onSet(s), o.settingTriggersEvent && s[o.triggerFnName](o.settingEvent);
      } else if (o.allowBinding && Ue(n)) {
        var P = n;
        s.on(o.bindingEvent, P);
      } else if (o.allowGetting && n === void 0) {
        var A;
        return c && (o.beforeGet(c), A = c._private[o.field]), A;
      }
      return s;
    };
  },
  // data
  // remove data field
  removeData: function(e) {
    var r = {
      field: "data",
      event: "data",
      triggerFnName: "trigger",
      triggerEvent: !1,
      immutableKeys: {}
      // key => true if immutable
    };
    return e = ye({}, r, e), function(n) {
      var i = e, o = this, s = o.length !== void 0, u = s ? o : [o];
      if (he(n)) {
        for (var l = n.split(/\s+/), c = l.length, f = 0; f < c; f++) {
          var v = l[f];
          if (!cr(v)) {
            var h = !i.immutableKeys[v];
            if (h)
              for (var d = 0, y = u.length; d < y; d++)
                u[d]._private[i.field][v] = void 0;
          }
        }
        i.triggerEvent && o[i.triggerFnName](i.event);
      } else if (n === void 0) {
        for (var g = 0, p = u.length; g < p; g++)
          for (var m = u[g]._private[i.field], b = Object.keys(m), w = 0; w < b.length; w++) {
            var C = b[w], E = !i.immutableKeys[C];
            E && (m[C] = void 0);
          }
        i.triggerEvent && o[i.triggerFnName](i.event);
      }
      return o;
    };
  }
  // removeData
}, _p = {
  eventAliasesOn: function(e) {
    var r = e;
    r.addListener = r.listen = r.bind = r.on, r.unlisten = r.unbind = r.off = r.removeListener, r.trigger = r.emit, r.pon = r.promiseOn = function(a, n) {
      var i = this, o = Array.prototype.slice.call(arguments, 0);
      return new sa(function(s, u) {
        var l = function(h) {
          i.off.apply(i, f), s(h);
        }, c = o.concat([l]), f = c.concat([]);
        i.on.apply(i, c);
      });
    };
  }
}, Ne = {};
[Kg, Vp, _p].forEach(function(t) {
  ye(Ne, t);
});
var Gp = {
  animate: Ne.animate(),
  animation: Ne.animation(),
  animated: Ne.animated(),
  clearQueue: Ne.clearQueue(),
  delay: Ne.delay(),
  delayAnimation: Ne.delayAnimation(),
  stop: Ne.stop()
}, pn = {
  classes: function(e) {
    var r = this;
    if (e === void 0) {
      var a = [];
      return r[0]._private.classes.forEach(function(d) {
        return a.push(d);
      }), a;
    } else Ve(e) || (e = (e || "").match(/\S+/g) || []);
    for (var n = [], i = new oa(e), o = 0; o < r.length; o++) {
      for (var s = r[o], u = s._private, l = u.classes, c = !1, f = 0; f < e.length; f++) {
        var v = e[f], h = l.has(v);
        if (!h) {
          c = !0;
          break;
        }
      }
      c || (c = l.size !== e.length), c && (u.classes = i, n.push(s));
    }
    return n.length > 0 && this.spawn(n).updateStyle().emit("class"), r;
  },
  addClass: function(e) {
    return this.toggleClass(e, !0);
  },
  hasClass: function(e) {
    var r = this[0];
    return r != null && r._private.classes.has(e);
  },
  toggleClass: function(e, r) {
    Ve(e) || (e = e.match(/\S+/g) || []);
    for (var a = this, n = r === void 0, i = [], o = 0, s = a.length; o < s; o++)
      for (var u = a[o], l = u._private.classes, c = !1, f = 0; f < e.length; f++) {
        var v = e[f], h = l.has(v), d = !1;
        r || n && !h ? (l.add(v), d = !0) : (!r || n && h) && (l.delete(v), d = !0), !c && d && (i.push(u), c = !0);
      }
    return i.length > 0 && this.spawn(i).updateStyle().emit("class"), a;
  },
  removeClass: function(e) {
    return this.toggleClass(e, !1);
  },
  flashClass: function(e, r) {
    var a = this;
    if (r == null)
      r = 250;
    else if (r === 0)
      return a;
    return a.addClass(e), setTimeout(function() {
      a.removeClass(e);
    }, r), a;
  }
};
pn.className = pn.classNames = pn.classes;
var Re = {
  metaChar: "[\\!\\\"\\#\\$\\%\\&\\'\\(\\)\\*\\+\\,\\.\\/\\:\\;\\<\\=\\>\\?\\@\\[\\]\\^\\`\\{\\|\\}\\~]",
  // chars we need to escape in let names, etc
  comparatorOp: "=|\\!=|>|>=|<|<=|\\$=|\\^=|\\*=",
  // binary comparison op (used in data selectors)
  boolOp: "\\?|\\!|\\^",
  // boolean (unary) operators (used in data selectors)
  string: `"(?:\\\\"|[^"])*"|'(?:\\\\'|[^'])*'`,
  // string literals (used in data selectors) -- doublequotes | singlequotes
  number: tt,
  // number literal (used in data selectors) --- e.g. 0.1234, 1234, 12e123
  meta: "degree|indegree|outdegree",
  // allowed metadata fields (i.e. allowed functions to use from Collection)
  separator: "\\s*,\\s*",
  // queries are separated by commas, e.g. edge[foo = 'bar'], node.someClass
  descendant: "\\s+",
  child: "\\s+>\\s+",
  subject: "\\$",
  group: "node|edge|\\*",
  directedEdge: "\\s+->\\s+",
  undirectedEdge: "\\s+<->\\s+"
};
Re.variable = "(?:[\\w-.]|(?:\\\\" + Re.metaChar + "))+";
Re.className = "(?:[\\w-]|(?:\\\\" + Re.metaChar + "))+";
Re.value = Re.string + "|" + Re.number;
Re.id = Re.variable;
(function() {
  var t, e, r;
  for (t = Re.comparatorOp.split("|"), r = 0; r < t.length; r++)
    e = t[r], Re.comparatorOp += "|@" + e;
  for (t = Re.comparatorOp.split("|"), r = 0; r < t.length; r++)
    e = t[r], !(e.indexOf("!") >= 0) && e !== "=" && (Re.comparatorOp += "|\\!" + e);
})();
var Fe = function() {
  return {
    checks: []
  };
}, ue = {
  /** E.g. node */
  GROUP: 0,
  /** A collection of elements */
  COLLECTION: 1,
  /** A filter(ele) function */
  FILTER: 2,
  /** E.g. [foo > 1] */
  DATA_COMPARE: 3,
  /** E.g. [foo] */
  DATA_EXIST: 4,
  /** E.g. [?foo] */
  DATA_BOOL: 5,
  /** E.g. [[degree > 2]] */
  META_COMPARE: 6,
  /** E.g. :selected */
  STATE: 7,
  /** E.g. #foo */
  ID: 8,
  /** E.g. .foo */
  CLASS: 9,
  /** E.g. #foo <-> #bar */
  UNDIRECTED_EDGE: 10,
  /** E.g. #foo -> #bar */
  DIRECTED_EDGE: 11,
  /** E.g. $#foo -> #bar */
  NODE_SOURCE: 12,
  /** E.g. #foo -> $#bar */
  NODE_TARGET: 13,
  /** E.g. $#foo <-> #bar */
  NODE_NEIGHBOR: 14,
  /** E.g. #foo > #bar */
  CHILD: 15,
  /** E.g. #foo #bar */
  DESCENDANT: 16,
  /** E.g. $#foo > #bar */
  PARENT: 17,
  /** E.g. $#foo #bar */
  ANCESTOR: 18,
  /** E.g. #foo > $bar > #baz */
  COMPOUND_SPLIT: 19,
  /** Always matches, useful placeholder for subject in `COMPOUND_SPLIT` */
  TRUE: 20
}, Xo = [{
  selector: ":selected",
  matches: function(e) {
    return e.selected();
  }
}, {
  selector: ":unselected",
  matches: function(e) {
    return !e.selected();
  }
}, {
  selector: ":selectable",
  matches: function(e) {
    return e.selectable();
  }
}, {
  selector: ":unselectable",
  matches: function(e) {
    return !e.selectable();
  }
}, {
  selector: ":locked",
  matches: function(e) {
    return e.locked();
  }
}, {
  selector: ":unlocked",
  matches: function(e) {
    return !e.locked();
  }
}, {
  selector: ":visible",
  matches: function(e) {
    return e.visible();
  }
}, {
  selector: ":hidden",
  matches: function(e) {
    return !e.visible();
  }
}, {
  selector: ":transparent",
  matches: function(e) {
    return e.transparent();
  }
}, {
  selector: ":grabbed",
  matches: function(e) {
    return e.grabbed();
  }
}, {
  selector: ":free",
  matches: function(e) {
    return !e.grabbed();
  }
}, {
  selector: ":removed",
  matches: function(e) {
    return e.removed();
  }
}, {
  selector: ":inside",
  matches: function(e) {
    return !e.removed();
  }
}, {
  selector: ":grabbable",
  matches: function(e) {
    return e.grabbable();
  }
}, {
  selector: ":ungrabbable",
  matches: function(e) {
    return !e.grabbable();
  }
}, {
  selector: ":animated",
  matches: function(e) {
    return e.animated();
  }
}, {
  selector: ":unanimated",
  matches: function(e) {
    return !e.animated();
  }
}, {
  selector: ":parent",
  matches: function(e) {
    return e.isParent();
  }
}, {
  selector: ":childless",
  matches: function(e) {
    return e.isChildless();
  }
}, {
  selector: ":child",
  matches: function(e) {
    return e.isChild();
  }
}, {
  selector: ":orphan",
  matches: function(e) {
    return e.isOrphan();
  }
}, {
  selector: ":nonorphan",
  matches: function(e) {
    return e.isChild();
  }
}, {
  selector: ":compound",
  matches: function(e) {
    return e.isNode() ? e.isParent() : e.source().isParent() || e.target().isParent();
  }
}, {
  selector: ":loop",
  matches: function(e) {
    return e.isLoop();
  }
}, {
  selector: ":simple",
  matches: function(e) {
    return e.isSimple();
  }
}, {
  selector: ":active",
  matches: function(e) {
    return e.active();
  }
}, {
  selector: ":inactive",
  matches: function(e) {
    return !e.active();
  }
}, {
  selector: ":backgrounding",
  matches: function(e) {
    return e.backgrounding();
  }
}, {
  selector: ":nonbackgrounding",
  matches: function(e) {
    return !e.backgrounding();
  }
}].sort(function(t, e) {
  return Vd(t.selector, e.selector);
}), Hp = (function() {
  for (var t = {}, e, r = 0; r < Xo.length; r++)
    e = Xo[r], t[e.selector] = e.matches;
  return t;
})(), $p = function(e, r) {
  return Hp[e](r);
}, Wp = "(" + Xo.map(function(t) {
  return t.selector;
}).join("|") + ")", Fr = function(e) {
  return e.replace(new RegExp("\\\\(" + Re.metaChar + ")", "g"), function(r, a) {
    return a;
  });
}, ar = function(e, r, a) {
  e[e.length - 1] = a;
}, Yo = [{
  name: "group",
  // just used for identifying when debugging
  query: !0,
  regex: "(" + Re.group + ")",
  populate: function(e, r, a) {
    var n = Je(a, 1), i = n[0];
    r.checks.push({
      type: ue.GROUP,
      value: i === "*" ? i : i + "s"
    });
  }
}, {
  name: "state",
  query: !0,
  regex: Wp,
  populate: function(e, r, a) {
    var n = Je(a, 1), i = n[0];
    r.checks.push({
      type: ue.STATE,
      value: i
    });
  }
}, {
  name: "id",
  query: !0,
  regex: "\\#(" + Re.id + ")",
  populate: function(e, r, a) {
    var n = Je(a, 1), i = n[0];
    r.checks.push({
      type: ue.ID,
      value: Fr(i)
    });
  }
}, {
  name: "className",
  query: !0,
  regex: "\\.(" + Re.className + ")",
  populate: function(e, r, a) {
    var n = Je(a, 1), i = n[0];
    r.checks.push({
      type: ue.CLASS,
      value: Fr(i)
    });
  }
}, {
  name: "dataExists",
  query: !0,
  regex: "\\[\\s*(" + Re.variable + ")\\s*\\]",
  populate: function(e, r, a) {
    var n = Je(a, 1), i = n[0];
    r.checks.push({
      type: ue.DATA_EXIST,
      field: Fr(i)
    });
  }
}, {
  name: "dataCompare",
  query: !0,
  regex: "\\[\\s*(" + Re.variable + ")\\s*(" + Re.comparatorOp + ")\\s*(" + Re.value + ")\\s*\\]",
  populate: function(e, r, a) {
    var n = Je(a, 3), i = n[0], o = n[1], s = n[2], u = new RegExp("^" + Re.string + "$").exec(s) != null;
    u ? s = s.substring(1, s.length - 1) : s = parseFloat(s), r.checks.push({
      type: ue.DATA_COMPARE,
      field: Fr(i),
      operator: o,
      value: s
    });
  }
}, {
  name: "dataBool",
  query: !0,
  regex: "\\[\\s*(" + Re.boolOp + ")\\s*(" + Re.variable + ")\\s*\\]",
  populate: function(e, r, a) {
    var n = Je(a, 2), i = n[0], o = n[1];
    r.checks.push({
      type: ue.DATA_BOOL,
      field: Fr(o),
      operator: i
    });
  }
}, {
  name: "metaCompare",
  query: !0,
  regex: "\\[\\[\\s*(" + Re.meta + ")\\s*(" + Re.comparatorOp + ")\\s*(" + Re.number + ")\\s*\\]\\]",
  populate: function(e, r, a) {
    var n = Je(a, 3), i = n[0], o = n[1], s = n[2];
    r.checks.push({
      type: ue.META_COMPARE,
      field: Fr(i),
      operator: o,
      value: parseFloat(s)
    });
  }
}, {
  name: "nextQuery",
  separator: !0,
  regex: Re.separator,
  populate: function(e, r) {
    var a = e.currentSubject, n = e.edgeCount, i = e.compoundCount, o = e[e.length - 1];
    a != null && (o.subject = a, e.currentSubject = null), o.edgeCount = n, o.compoundCount = i, e.edgeCount = 0, e.compoundCount = 0;
    var s = e[e.length++] = Fe();
    return s;
  }
}, {
  name: "directedEdge",
  separator: !0,
  regex: Re.directedEdge,
  populate: function(e, r) {
    if (e.currentSubject == null) {
      var a = Fe(), n = r, i = Fe();
      return a.checks.push({
        type: ue.DIRECTED_EDGE,
        source: n,
        target: i
      }), ar(e, r, a), e.edgeCount++, i;
    } else {
      var o = Fe(), s = r, u = Fe();
      return o.checks.push({
        type: ue.NODE_SOURCE,
        source: s,
        target: u
      }), ar(e, r, o), e.edgeCount++, u;
    }
  }
}, {
  name: "undirectedEdge",
  separator: !0,
  regex: Re.undirectedEdge,
  populate: function(e, r) {
    if (e.currentSubject == null) {
      var a = Fe(), n = r, i = Fe();
      return a.checks.push({
        type: ue.UNDIRECTED_EDGE,
        nodes: [n, i]
      }), ar(e, r, a), e.edgeCount++, i;
    } else {
      var o = Fe(), s = r, u = Fe();
      return o.checks.push({
        type: ue.NODE_NEIGHBOR,
        node: s,
        neighbor: u
      }), ar(e, r, o), u;
    }
  }
}, {
  name: "child",
  separator: !0,
  regex: Re.child,
  populate: function(e, r) {
    if (e.currentSubject == null) {
      var a = Fe(), n = Fe(), i = e[e.length - 1];
      return a.checks.push({
        type: ue.CHILD,
        parent: i,
        child: n
      }), ar(e, r, a), e.compoundCount++, n;
    } else if (e.currentSubject === r) {
      var o = Fe(), s = e[e.length - 1], u = Fe(), l = Fe(), c = Fe(), f = Fe();
      return o.checks.push({
        type: ue.COMPOUND_SPLIT,
        left: s,
        right: u,
        subject: l
      }), l.checks = r.checks, r.checks = [{
        type: ue.TRUE
      }], f.checks.push({
        type: ue.TRUE
      }), u.checks.push({
        type: ue.PARENT,
        // type is swapped on right side queries
        parent: f,
        child: c
        // empty for now
      }), ar(e, s, o), e.currentSubject = l, e.compoundCount++, c;
    } else {
      var v = Fe(), h = Fe(), d = [{
        type: ue.PARENT,
        parent: v,
        child: h
      }];
      return v.checks = r.checks, r.checks = d, e.compoundCount++, h;
    }
  }
}, {
  name: "descendant",
  separator: !0,
  regex: Re.descendant,
  populate: function(e, r) {
    if (e.currentSubject == null) {
      var a = Fe(), n = Fe(), i = e[e.length - 1];
      return a.checks.push({
        type: ue.DESCENDANT,
        ancestor: i,
        descendant: n
      }), ar(e, r, a), e.compoundCount++, n;
    } else if (e.currentSubject === r) {
      var o = Fe(), s = e[e.length - 1], u = Fe(), l = Fe(), c = Fe(), f = Fe();
      return o.checks.push({
        type: ue.COMPOUND_SPLIT,
        left: s,
        right: u,
        subject: l
      }), l.checks = r.checks, r.checks = [{
        type: ue.TRUE
      }], f.checks.push({
        type: ue.TRUE
      }), u.checks.push({
        type: ue.ANCESTOR,
        // type is swapped on right side queries
        ancestor: f,
        descendant: c
        // empty for now
      }), ar(e, s, o), e.currentSubject = l, e.compoundCount++, c;
    } else {
      var v = Fe(), h = Fe(), d = [{
        type: ue.ANCESTOR,
        ancestor: v,
        descendant: h
      }];
      return v.checks = r.checks, r.checks = d, e.compoundCount++, h;
    }
  }
}, {
  name: "subject",
  modifier: !0,
  regex: Re.subject,
  populate: function(e, r) {
    if (e.currentSubject != null && e.currentSubject !== r)
      return ze("Redefinition of subject in selector `" + e.toString() + "`"), !1;
    e.currentSubject = r;
    var a = e[e.length - 1], n = a.checks[0], i = n == null ? null : n.type;
    i === ue.DIRECTED_EDGE ? n.type = ue.NODE_TARGET : i === ue.UNDIRECTED_EDGE && (n.type = ue.NODE_NEIGHBOR, n.node = n.nodes[1], n.neighbor = n.nodes[0], n.nodes = null);
  }
}];
Yo.forEach(function(t) {
  return t.regexObj = new RegExp("^" + t.regex);
});
var Up = function(e) {
  for (var r, a, n, i = 0; i < Yo.length; i++) {
    var o = Yo[i], s = o.name, u = e.match(o.regexObj);
    if (u != null) {
      a = u, r = o, n = s;
      var l = u[0];
      e = e.substring(l.length);
      break;
    }
  }
  return {
    expr: r,
    match: a,
    name: n,
    remaining: e
  };
}, Kp = function(e) {
  var r = e.match(/^\s+/);
  if (r) {
    var a = r[0];
    e = e.substring(a.length);
  }
  return e;
}, Xp = function(e) {
  var r = this, a = r.inputText = e, n = r[0] = Fe();
  for (r.length = 1, a = Kp(a); ; ) {
    var i = Up(a);
    if (i.expr == null)
      return ze("The selector `" + e + "`is invalid"), !1;
    var o = i.match.slice(1), s = i.expr.populate(r, n, o);
    if (s === !1)
      return !1;
    if (s != null && (n = s), a = i.remaining, a.match(/^\s*$/))
      break;
  }
  var u = r[r.length - 1];
  r.currentSubject != null && (u.subject = r.currentSubject), u.edgeCount = r.edgeCount, u.compoundCount = r.compoundCount;
  for (var l = 0; l < r.length; l++) {
    var c = r[l];
    if (c.compoundCount > 0 && c.edgeCount > 0)
      return ze("The selector `" + e + "` is invalid because it uses both a compound selector and an edge selector"), !1;
    if (c.edgeCount > 1)
      return ze("The selector `" + e + "` is invalid because it uses multiple edge selectors"), !1;
    c.edgeCount === 1 && ze("The selector `" + e + "` is deprecated.  Edge selectors do not take effect on changes to source and target nodes after an edge is added, for performance reasons.  Use a class or data selector on edges instead, updating the class or data of an edge when your app detects a change in source or target nodes.");
  }
  return !0;
}, Yp = function() {
  if (this.toStringCache != null)
    return this.toStringCache;
  for (var e = function(c) {
    return c ?? "";
  }, r = function(c) {
    return he(c) ? '"' + c + '"' : e(c);
  }, a = function(c) {
    return " " + c + " ";
  }, n = function(c, f) {
    var v = c.type, h = c.value;
    switch (v) {
      case ue.GROUP: {
        var d = e(h);
        return d.substring(0, d.length - 1);
      }
      case ue.DATA_COMPARE: {
        var y = c.field, g = c.operator;
        return "[" + y + a(e(g)) + r(h) + "]";
      }
      case ue.DATA_BOOL: {
        var p = c.operator, m = c.field;
        return "[" + e(p) + m + "]";
      }
      case ue.DATA_EXIST: {
        var b = c.field;
        return "[" + b + "]";
      }
      case ue.META_COMPARE: {
        var w = c.operator, C = c.field;
        return "[[" + C + a(e(w)) + r(h) + "]]";
      }
      case ue.STATE:
        return h;
      case ue.ID:
        return "#" + h;
      case ue.CLASS:
        return "." + h;
      case ue.PARENT:
      case ue.CHILD:
        return i(c.parent, f) + a(">") + i(c.child, f);
      case ue.ANCESTOR:
      case ue.DESCENDANT:
        return i(c.ancestor, f) + " " + i(c.descendant, f);
      case ue.COMPOUND_SPLIT: {
        var E = i(c.left, f), x = i(c.subject, f), T = i(c.right, f);
        return E + (E.length > 0 ? " " : "") + x + T;
      }
      case ue.TRUE:
        return "";
    }
  }, i = function(c, f) {
    return c.checks.reduce(function(v, h, d) {
      return v + (f === c && d === 0 ? "$" : "") + n(h, f);
    }, "");
  }, o = "", s = 0; s < this.length; s++) {
    var u = this[s];
    o += i(u, u.subject), this.length > 1 && s < this.length - 1 && (o += ", ");
  }
  return this.toStringCache = o, o;
}, Zp = {
  parse: Xp,
  toString: Yp
}, nf = function(e, r, a) {
  var n, i = he(e), o = ne(e), s = he(a), u, l, c = !1, f = !1, v = !1;
  switch (r.indexOf("!") >= 0 && (r = r.replace("!", ""), f = !0), r.indexOf("@") >= 0 && (r = r.replace("@", ""), c = !0), (i || s || c) && (u = !i && !o ? "" : "" + e, l = "" + a), c && (e = u = u.toLowerCase(), a = l = l.toLowerCase()), r) {
    case "*=":
      n = u.indexOf(l) >= 0;
      break;
    case "$=":
      n = u.indexOf(l, u.length - l.length) >= 0;
      break;
    case "^=":
      n = u.indexOf(l) === 0;
      break;
    case "=":
      n = e === a;
      break;
    case ">":
      v = !0, n = e > a;
      break;
    case ">=":
      v = !0, n = e >= a;
      break;
    case "<":
      v = !0, n = e < a;
      break;
    case "<=":
      v = !0, n = e <= a;
      break;
    default:
      n = !1;
      break;
  }
  return f && (e != null || !v) && (n = !n), n;
}, Qp = function(e, r) {
  switch (r) {
    case "?":
      return !!e;
    case "!":
      return !e;
    case "^":
      return e === void 0;
  }
}, Jp = function(e) {
  return e !== void 0;
}, Ts = function(e, r) {
  return e.data(r);
}, jp = function(e, r) {
  return e[r]();
}, Ye = [], He = function(e, r) {
  return e.checks.every(function(a) {
    return Ye[a.type](a, r);
  });
};
Ye[ue.GROUP] = function(t, e) {
  var r = t.value;
  return r === "*" || r === e.group();
};
Ye[ue.STATE] = function(t, e) {
  var r = t.value;
  return $p(r, e);
};
Ye[ue.ID] = function(t, e) {
  var r = t.value;
  return e.id() === r;
};
Ye[ue.CLASS] = function(t, e) {
  var r = t.value;
  return e.hasClass(r);
};
Ye[ue.META_COMPARE] = function(t, e) {
  var r = t.field, a = t.operator, n = t.value;
  return nf(jp(e, r), a, n);
};
Ye[ue.DATA_COMPARE] = function(t, e) {
  var r = t.field, a = t.operator, n = t.value;
  return nf(Ts(e, r), a, n);
};
Ye[ue.DATA_BOOL] = function(t, e) {
  var r = t.field, a = t.operator;
  return Qp(Ts(e, r), a);
};
Ye[ue.DATA_EXIST] = function(t, e) {
  var r = t.field;
  return t.operator, Jp(Ts(e, r));
};
Ye[ue.UNDIRECTED_EDGE] = function(t, e) {
  var r = t.nodes[0], a = t.nodes[1], n = e.source(), i = e.target();
  return He(r, n) && He(a, i) || He(a, n) && He(r, i);
};
Ye[ue.NODE_NEIGHBOR] = function(t, e) {
  return He(t.node, e) && e.neighborhood().some(function(r) {
    return r.isNode() && He(t.neighbor, r);
  });
};
Ye[ue.DIRECTED_EDGE] = function(t, e) {
  return He(t.source, e.source()) && He(t.target, e.target());
};
Ye[ue.NODE_SOURCE] = function(t, e) {
  return He(t.source, e) && e.outgoers().some(function(r) {
    return r.isNode() && He(t.target, r);
  });
};
Ye[ue.NODE_TARGET] = function(t, e) {
  return He(t.target, e) && e.incomers().some(function(r) {
    return r.isNode() && He(t.source, r);
  });
};
Ye[ue.CHILD] = function(t, e) {
  return He(t.child, e) && He(t.parent, e.parent());
};
Ye[ue.PARENT] = function(t, e) {
  return He(t.parent, e) && e.children().some(function(r) {
    return He(t.child, r);
  });
};
Ye[ue.DESCENDANT] = function(t, e) {
  return He(t.descendant, e) && e.ancestors().some(function(r) {
    return He(t.ancestor, r);
  });
};
Ye[ue.ANCESTOR] = function(t, e) {
  return He(t.ancestor, e) && e.descendants().some(function(r) {
    return He(t.descendant, r);
  });
};
Ye[ue.COMPOUND_SPLIT] = function(t, e) {
  return He(t.subject, e) && He(t.left, e) && He(t.right, e);
};
Ye[ue.TRUE] = function() {
  return !0;
};
Ye[ue.COLLECTION] = function(t, e) {
  var r = t.value;
  return r.has(e);
};
Ye[ue.FILTER] = function(t, e) {
  var r = t.value;
  return r(e);
};
var ey = function(e) {
  var r = this;
  if (r.length === 1 && r[0].checks.length === 1 && r[0].checks[0].type === ue.ID)
    return e.getElementById(r[0].checks[0].value).collection();
  var a = function(i) {
    for (var o = 0; o < r.length; o++) {
      var s = r[o];
      if (He(s, i))
        return !0;
    }
    return !1;
  };
  return r.text() == null && (a = function() {
    return !0;
  }), e.filter(a);
}, ty = function(e) {
  for (var r = this, a = 0; a < r.length; a++) {
    var n = r[a];
    if (He(n, e))
      return !0;
  }
  return !1;
}, ry = {
  matches: ty,
  filter: ey
}, dr = function(e) {
  this.inputText = e, this.currentSubject = null, this.compoundCount = 0, this.edgeCount = 0, this.length = 0, e == null || he(e) && e.match(/^\s*$/) || (kt(e) ? this.addQuery({
    checks: [{
      type: ue.COLLECTION,
      value: e.collection()
    }]
  }) : Ue(e) ? this.addQuery({
    checks: [{
      type: ue.FILTER,
      value: e
    }]
  }) : he(e) ? this.parse(e) || (this.invalid = !0) : $e("A selector must be created from a string; found "));
}, hr = dr.prototype;
[Zp, ry].forEach(function(t) {
  return ye(hr, t);
});
hr.text = function() {
  return this.inputText;
};
hr.size = function() {
  return this.length;
};
hr.eq = function(t) {
  return this[t];
};
hr.sameText = function(t) {
  return !this.invalid && !t.invalid && this.text() === t.text();
};
hr.addQuery = function(t) {
  this[this.length++] = t;
};
hr.selector = hr.toString;
var ur = {
  allAre: function(e) {
    var r = new dr(e);
    return this.every(function(a) {
      return r.matches(a);
    });
  },
  is: function(e) {
    var r = new dr(e);
    return this.some(function(a) {
      return r.matches(a);
    });
  },
  some: function(e, r) {
    for (var a = 0; a < this.length; a++) {
      var n = r ? e.apply(r, [this[a], a, this]) : e(this[a], a, this);
      if (n)
        return !0;
    }
    return !1;
  },
  every: function(e, r) {
    for (var a = 0; a < this.length; a++) {
      var n = r ? e.apply(r, [this[a], a, this]) : e(this[a], a, this);
      if (!n)
        return !1;
    }
    return !0;
  },
  same: function(e) {
    if (this === e)
      return !0;
    e = this.cy().collection(e);
    var r = this.length, a = e.length;
    return r !== a ? !1 : r === 1 ? this[0] === e[0] : this.every(function(n) {
      return e.hasElementWithId(n.id());
    });
  },
  anySame: function(e) {
    return e = this.cy().collection(e), this.some(function(r) {
      return e.hasElementWithId(r.id());
    });
  },
  allAreNeighbors: function(e) {
    e = this.cy().collection(e);
    var r = this.neighborhood();
    return e.every(function(a) {
      return r.hasElementWithId(a.id());
    });
  },
  contains: function(e) {
    e = this.cy().collection(e);
    var r = this;
    return e.every(function(a) {
      return r.hasElementWithId(a.id());
    });
  }
};
ur.allAreNeighbours = ur.allAreNeighbors;
ur.has = ur.contains;
ur.equal = ur.equals = ur.same;
var Bt = function(e, r) {
  return function(n, i, o, s) {
    var u = n, l = this, c;
    if (u == null ? c = "" : kt(u) && u.length === 1 && (c = u.id()), l.length === 1 && c) {
      var f = l[0]._private, v = f.traversalCache = f.traversalCache || {}, h = v[r] = v[r] || [], d = Ar(c), y = h[d];
      return y || (h[d] = e.call(l, n, i, o, s));
    } else
      return e.call(l, n, i, o, s);
  };
}, aa = {
  parent: function(e) {
    var r = [];
    if (this.length === 1) {
      var a = this[0]._private.parent;
      if (a)
        return a;
    }
    for (var n = 0; n < this.length; n++) {
      var i = this[n], o = i._private.parent;
      o && r.push(o);
    }
    return this.spawn(r, !0).filter(e);
  },
  parents: function(e) {
    for (var r = [], a = this.parent(); a.nonempty(); ) {
      for (var n = 0; n < a.length; n++) {
        var i = a[n];
        r.push(i);
      }
      a = a.parent();
    }
    return this.spawn(r, !0).filter(e);
  },
  commonAncestors: function(e) {
    for (var r, a = 0; a < this.length; a++) {
      var n = this[a], i = n.parents();
      r = r || i, r = r.intersect(i);
    }
    return r.filter(e);
  },
  orphans: function(e) {
    return this.stdFilter(function(r) {
      return r.isOrphan();
    }).filter(e);
  },
  nonorphans: function(e) {
    return this.stdFilter(function(r) {
      return r.isChild();
    }).filter(e);
  },
  children: Bt(function(t) {
    for (var e = [], r = 0; r < this.length; r++)
      for (var a = this[r], n = a._private.children, i = 0; i < n.length; i++)
        e.push(n[i]);
    return this.spawn(e, !0).filter(t);
  }, "children"),
  siblings: function(e) {
    return this.parent().children().not(this).filter(e);
  },
  isParent: function() {
    var e = this[0];
    if (e)
      return e.isNode() && e._private.children.length !== 0;
  },
  isChildless: function() {
    var e = this[0];
    if (e)
      return e.isNode() && e._private.children.length === 0;
  },
  isChild: function() {
    var e = this[0];
    if (e)
      return e.isNode() && e._private.parent != null;
  },
  isOrphan: function() {
    var e = this[0];
    if (e)
      return e.isNode() && e._private.parent == null;
  },
  descendants: function(e) {
    var r = [];
    function a(n) {
      for (var i = 0; i < n.length; i++) {
        var o = n[i];
        r.push(o), o.children().nonempty() && a(o.children());
      }
    }
    return a(this.children()), this.spawn(r, !0).filter(e);
  }
};
function Ss(t, e, r, a) {
  for (var n = [], i = new oa(), o = t.cy(), s = o.hasCompoundNodes(), u = 0; u < t.length; u++) {
    var l = t[u];
    r ? n.push(l) : s && a(n, i, l);
  }
  for (; n.length > 0; ) {
    var c = n.shift();
    e(c), i.add(c.id()), s && a(n, i, c);
  }
  return t;
}
function of(t, e, r) {
  if (r.isParent())
    for (var a = r._private.children, n = 0; n < a.length; n++) {
      var i = a[n];
      e.has(i.id()) || t.push(i);
    }
}
aa.forEachDown = function(t) {
  var e = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : !0;
  return Ss(this, t, e, of);
};
function sf(t, e, r) {
  if (r.isChild()) {
    var a = r._private.parent;
    e.has(a.id()) || t.push(a);
  }
}
aa.forEachUp = function(t) {
  var e = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : !0;
  return Ss(this, t, e, sf);
};
function ay(t, e, r) {
  sf(t, e, r), of(t, e, r);
}
aa.forEachUpAndDown = function(t) {
  var e = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : !0;
  return Ss(this, t, e, ay);
};
aa.ancestors = aa.parents;
var Ia, uf;
Ia = uf = {
  data: Ne.data({
    field: "data",
    bindingEvent: "data",
    allowBinding: !0,
    allowSetting: !0,
    settingEvent: "data",
    settingTriggersEvent: !0,
    triggerFnName: "trigger",
    allowGetting: !0,
    immutableKeys: {
      id: !0,
      source: !0,
      target: !0,
      parent: !0
    },
    updateStyle: !0
  }),
  removeData: Ne.removeData({
    field: "data",
    event: "data",
    triggerFnName: "trigger",
    triggerEvent: !0,
    immutableKeys: {
      id: !0,
      source: !0,
      target: !0,
      parent: !0
    },
    updateStyle: !0
  }),
  scratch: Ne.data({
    field: "scratch",
    bindingEvent: "scratch",
    allowBinding: !0,
    allowSetting: !0,
    settingEvent: "scratch",
    settingTriggersEvent: !0,
    triggerFnName: "trigger",
    allowGetting: !0,
    updateStyle: !0
  }),
  removeScratch: Ne.removeData({
    field: "scratch",
    event: "scratch",
    triggerFnName: "trigger",
    triggerEvent: !0,
    updateStyle: !0
  }),
  rscratch: Ne.data({
    field: "rscratch",
    allowBinding: !1,
    allowSetting: !0,
    settingTriggersEvent: !1,
    allowGetting: !0
  }),
  removeRscratch: Ne.removeData({
    field: "rscratch",
    triggerEvent: !1
  }),
  id: function() {
    var e = this[0];
    if (e)
      return e._private.data.id;
  }
};
Ia.attr = Ia.data;
Ia.removeAttr = Ia.removeData;
var ny = uf, Zn = {};
function Do(t) {
  return function(e) {
    var r = this;
    if (e === void 0 && (e = !0), r.length !== 0)
      if (r.isNode() && !r.removed()) {
        for (var a = 0, n = r[0], i = n._private.edges, o = 0; o < i.length; o++) {
          var s = i[o];
          !e && s.isLoop() || (a += t(n, s));
        }
        return a;
      } else
        return;
  };
}
ye(Zn, {
  degree: Do(function(t, e) {
    return e.source().same(e.target()) ? 2 : 1;
  }),
  indegree: Do(function(t, e) {
    return e.target().same(t) ? 1 : 0;
  }),
  outdegree: Do(function(t, e) {
    return e.source().same(t) ? 1 : 0;
  })
});
function qr(t, e) {
  return function(r) {
    for (var a, n = this.nodes(), i = 0; i < n.length; i++) {
      var o = n[i], s = o[t](r);
      s !== void 0 && (a === void 0 || e(s, a)) && (a = s);
    }
    return a;
  };
}
ye(Zn, {
  minDegree: qr("degree", function(t, e) {
    return t < e;
  }),
  maxDegree: qr("degree", function(t, e) {
    return t > e;
  }),
  minIndegree: qr("indegree", function(t, e) {
    return t < e;
  }),
  maxIndegree: qr("indegree", function(t, e) {
    return t > e;
  }),
  minOutdegree: qr("outdegree", function(t, e) {
    return t < e;
  }),
  maxOutdegree: qr("outdegree", function(t, e) {
    return t > e;
  })
});
ye(Zn, {
  totalDegree: function(e) {
    for (var r = 0, a = this.nodes(), n = 0; n < a.length; n++)
      r += a[n].degree(e);
    return r;
  }
});
var zt, lf, cf = function(e, r, a) {
  for (var n = 0; n < e.length; n++) {
    var i = e[n];
    if (!i.locked()) {
      var o = i._private.position, s = {
        x: r.x != null ? r.x - o.x : 0,
        y: r.y != null ? r.y - o.y : 0
      };
      i.isParent() && !(s.x === 0 && s.y === 0) && i.children().shift(s, a), i.dirtyBoundingBoxCache();
    }
  }
}, Sl = {
  field: "position",
  bindingEvent: "position",
  allowBinding: !0,
  allowSetting: !0,
  settingEvent: "position",
  settingTriggersEvent: !0,
  triggerFnName: "emitAndNotify",
  allowGetting: !0,
  validKeys: ["x", "y"],
  beforeGet: function(e) {
    e.updateCompoundBounds();
  },
  beforeSet: function(e, r) {
    cf(e, r, !1);
  },
  onSet: function(e) {
    e.dirtyCompoundBoundsCache();
  },
  canSet: function(e) {
    return !e.locked();
  }
};
zt = lf = {
  position: Ne.data(Sl),
  // position but no notification to renderer
  silentPosition: Ne.data(ye({}, Sl, {
    allowBinding: !1,
    allowSetting: !0,
    settingTriggersEvent: !1,
    allowGetting: !1,
    beforeSet: function(e, r) {
      cf(e, r, !0);
    },
    onSet: function(e) {
      e.dirtyCompoundBoundsCache();
    }
  })),
  positions: function(e, r) {
    if (Me(e))
      r ? this.silentPosition(e) : this.position(e);
    else if (Ue(e)) {
      var a = e, n = this.cy();
      n.startBatch();
      for (var i = 0; i < this.length; i++) {
        var o = this[i], s = void 0;
        (s = a(o, i)) && (r ? o.silentPosition(s) : o.position(s));
      }
      n.endBatch();
    }
    return this;
  },
  silentPositions: function(e) {
    return this.positions(e, !0);
  },
  shift: function(e, r, a) {
    var n;
    if (Me(e) ? (n = {
      x: ne(e.x) ? e.x : 0,
      y: ne(e.y) ? e.y : 0
    }, a = r) : he(e) && ne(r) && (n = {
      x: 0,
      y: 0
    }, n[e] = r), n != null) {
      var i = this.cy();
      i.startBatch();
      for (var o = 0; o < this.length; o++) {
        var s = this[o];
        if (!(i.hasCompoundNodes() && s.isChild() && s.ancestors().anySame(this))) {
          var u = s.position(), l = {
            x: u.x + n.x,
            y: u.y + n.y
          };
          a ? s.silentPosition(l) : s.position(l);
        }
      }
      i.endBatch();
    }
    return this;
  },
  silentShift: function(e, r) {
    return Me(e) ? this.shift(e, !0) : he(e) && ne(r) && this.shift(e, r, !0), this;
  },
  // get/set the rendered (i.e. on screen) positon of the element
  renderedPosition: function(e, r) {
    var a = this[0], n = this.cy(), i = n.zoom(), o = n.pan(), s = Me(e) ? e : void 0, u = s !== void 0 || r !== void 0 && he(e);
    if (a && a.isNode())
      if (u)
        for (var l = 0; l < this.length; l++) {
          var c = this[l];
          r !== void 0 ? c.position(e, (r - o[e]) / i) : s !== void 0 && c.position(zc(s, i, o));
        }
      else {
        var f = a.position();
        return s = $n(f, i, o), e === void 0 ? s : s[e];
      }
    else if (!u)
      return;
    return this;
  },
  // get/set the position relative to the parent
  relativePosition: function(e, r) {
    var a = this[0], n = this.cy(), i = Me(e) ? e : void 0, o = i !== void 0 || r !== void 0 && he(e), s = n.hasCompoundNodes();
    if (a && a.isNode())
      if (o)
        for (var u = 0; u < this.length; u++) {
          var l = this[u], c = s ? l.parent() : null, f = c && c.length > 0, v = f;
          f && (c = c[0]);
          var h = v ? c.position() : {
            x: 0,
            y: 0
          };
          r !== void 0 ? l.position(e, r + h[e]) : i !== void 0 && l.position({
            x: i.x + h.x,
            y: i.y + h.y
          });
        }
      else {
        var d = a.position(), y = s ? a.parent() : null, g = y && y.length > 0, p = g;
        g && (y = y[0]);
        var m = p ? y.position() : {
          x: 0,
          y: 0
        };
        return i = {
          x: d.x - m.x,
          y: d.y - m.y
        }, e === void 0 ? i : i[e];
      }
    else if (!o)
      return;
    return this;
  }
};
zt.modelPosition = zt.point = zt.position;
zt.modelPositions = zt.points = zt.positions;
zt.renderedPoint = zt.renderedPosition;
zt.relativePoint = zt.relativePosition;
var iy = lf, na = function(e) {
  switch (e) {
    case "left":
    case "right-inside":
      return "left";
    case "right":
    case "left-inside":
      return "right";
    default:
      return "center";
  }
}, ia = function(e) {
  switch (e) {
    case "top":
    case "bottom-inside":
      return "top";
    case "bottom":
    case "top-inside":
      return "bottom";
    default:
      return "center";
  }
}, oy = function(e) {
  switch (e) {
    case "left":
      return "right";
    case "right":
      return "left";
    case "left-inside":
      return "left";
    case "right-inside":
      return "right";
    default:
      return "center";
  }
}, ea, mr;
ea = mr = {};
mr.renderedBoundingBox = function(t) {
  var e = this.boundingBox(t), r = this.cy(), a = r.zoom(), n = r.pan(), i = e.x1 * a + n.x, o = e.x2 * a + n.x, s = e.y1 * a + n.y, u = e.y2 * a + n.y;
  return {
    x1: i,
    x2: o,
    y1: s,
    y2: u,
    w: o - i,
    h: u - s
  };
};
mr.dirtyCompoundBoundsCache = function() {
  var t = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : !1, e = this.cy();
  return !e.styleEnabled() || !e.hasCompoundNodes() ? this : (this.forEachUp(function(r) {
    if (r.isParent()) {
      var a = r._private;
      a.compoundBoundsClean = !1, a.bbCache = null, t || r.emitAndNotify("bounds");
    }
  }), this);
};
mr.updateCompoundBounds = function() {
  var t = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : !1, e = this.cy();
  if (!e.styleEnabled() || !e.hasCompoundNodes())
    return this;
  if (!t && e.batching())
    return this;
  function r(o) {
    if (!o.isParent())
      return;
    var s = o._private, u = o.children(), l = o.pstyle("compound-sizing-wrt-labels").value === "include", c = {
      width: {
        val: o.pstyle("min-width").pfValue,
        left: o.pstyle("min-width-bias-left"),
        right: o.pstyle("min-width-bias-right")
      },
      height: {
        val: o.pstyle("min-height").pfValue,
        top: o.pstyle("min-height-bias-top"),
        bottom: o.pstyle("min-height-bias-bottom")
      }
    }, f = u.boundingBox({
      includeLabels: l,
      includeOverlays: !1,
      // updating the compound bounds happens outside of the regular
      // cache cycle (i.e. before fired events)
      useCache: !1
    }), v = s.position;
    (f.w === 0 || f.h === 0) && (f = {
      w: o.pstyle("width").pfValue,
      h: o.pstyle("height").pfValue
    }, f.x1 = v.x - f.w / 2, f.x2 = v.x + f.w / 2, f.y1 = v.y - f.h / 2, f.y2 = v.y + f.h / 2);
    function h(k, D, P) {
      var A = 0, B = 0, R = D + P;
      return k > 0 && R > 0 && (A = D / R * k, B = P / R * k), {
        biasDiff: A,
        biasComplementDiff: B
      };
    }
    function d(k, D, P, A) {
      if (P.units === "%")
        switch (A) {
          case "width":
            return k > 0 ? P.pfValue * k : 0;
          case "height":
            return D > 0 ? P.pfValue * D : 0;
          case "average":
            return k > 0 && D > 0 ? P.pfValue * (k + D) / 2 : 0;
          case "min":
            return k > 0 && D > 0 ? k > D ? P.pfValue * D : P.pfValue * k : 0;
          case "max":
            return k > 0 && D > 0 ? k > D ? P.pfValue * k : P.pfValue * D : 0;
          default:
            return 0;
        }
      else return P.units === "px" ? P.pfValue : 0;
    }
    var y = c.width.left.value;
    c.width.left.units === "px" && c.width.val > 0 && (y = y * 100 / c.width.val);
    var g = c.width.right.value;
    c.width.right.units === "px" && c.width.val > 0 && (g = g * 100 / c.width.val);
    var p = c.height.top.value;
    c.height.top.units === "px" && c.height.val > 0 && (p = p * 100 / c.height.val);
    var m = c.height.bottom.value;
    c.height.bottom.units === "px" && c.height.val > 0 && (m = m * 100 / c.height.val);
    var b = h(c.width.val - f.w, y, g), w = b.biasDiff, C = b.biasComplementDiff, E = h(c.height.val - f.h, p, m), x = E.biasDiff, T = E.biasComplementDiff;
    s.autoPadding = d(f.w, f.h, o.pstyle("padding"), o.pstyle("padding-relative-to").value), s.autoWidth = Math.max(f.w, c.width.val), v.x = (-w + f.x1 + f.x2 + C) / 2, s.autoHeight = Math.max(f.h, c.height.val), v.y = (-x + f.y1 + f.y2 + T) / 2;
  }
  for (var a = 0; a < this.length; a++) {
    var n = this[a], i = n._private;
    (!i.compoundBoundsClean || t) && (r(n), e.batching() || (i.compoundBoundsClean = !0));
  }
  return this;
};
var At = function(e) {
  return e === 1 / 0 || e === -1 / 0 ? 0 : e;
}, Ot = function(e, r, a, n, i) {
  n - r === 0 || i - a === 0 || r == null || a == null || n == null || i == null || (e.x1 = r < e.x1 ? r : e.x1, e.x2 = n > e.x2 ? n : e.x2, e.y1 = a < e.y1 ? a : e.y1, e.y2 = i > e.y2 ? i : e.y2, e.w = e.x2 - e.x1, e.h = e.y2 - e.y1);
}, nr = function(e, r) {
  return r == null ? e : Ot(e, r.x1, r.y1, r.x2, r.y2);
}, pa = function(e, r, a) {
  return bt(e, r, a);
}, on = function(e, r, a) {
  if (!r.cy().headless()) {
    var n = r._private, i = n.rstyle, o = i.arrowWidth / 2, s = r.pstyle(a + "-arrow-shape").value, u, l;
    if (s !== "none") {
      a === "source" ? (u = i.srcX, l = i.srcY) : a === "target" ? (u = i.tgtX, l = i.tgtY) : (u = i.midX, l = i.midY);
      var c = n.arrowBounds = n.arrowBounds || {}, f = c[a] = c[a] || {};
      f.x1 = u - o, f.y1 = l - o, f.x2 = u + o, f.y2 = l + o, f.w = f.x2 - f.x1, f.h = f.y2 - f.y1, hn(f, 1), Ot(e, f.x1, f.y1, f.x2, f.y2);
    }
  }
}, Ao = function(e, r, a) {
  if (!r.cy().headless()) {
    var n;
    a ? n = a + "-" : n = "";
    var i = r._private, o = i.rstyle, s = r.pstyle(n + "label").strValue;
    if (s) {
      var u = r.pstyle("text-halign"), l = r.pstyle("text-valign"), c = pa(o, "labelWidth", a), f = pa(o, "labelHeight", a), v = pa(o, "labelX", a), h = pa(o, "labelY", a), d = r.pstyle(n + "text-margin-x").pfValue, y = r.pstyle(n + "text-margin-y").pfValue, g = r.isEdge(), p = r.pstyle(n + "text-rotation"), m = r.pstyle("text-outline-width").pfValue, b = r.pstyle("text-border-width").pfValue, w = b / 2, C = r.pstyle("text-background-padding").pfValue, E = 2, x = f, T = c, k = T / 2, D = x / 2, P, A, B, R;
      if (g)
        P = v - k, A = v + k, B = h - D, R = h + D;
      else {
        switch (na(u.value)) {
          case "left":
            P = v - T, A = v;
            break;
          case "center":
            P = v - k, A = v + k;
            break;
          case "right":
            P = v, A = v + T;
            break;
        }
        switch (ia(l.value)) {
          case "top":
            B = h - x, R = h;
            break;
          case "center":
            B = h - D, R = h + D;
            break;
          case "bottom":
            B = h, R = h + x;
            break;
        }
      }
      var I = d - Math.max(m, w) - C - E, O = d + Math.max(m, w) + C + E, M = y - Math.max(m, w) - C - E, N = y + Math.max(m, w) + C + E;
      P += I, A += O, B += M, R += N;
      var H = a || "main", G = i.labelBounds, F = G[H] = G[H] || {};
      F.x1 = P, F.y1 = B, F.x2 = A, F.y2 = R, F.w = A - P, F.h = R - B, F.leftPad = I, F.rightPad = O, F.topPad = M, F.botPad = N;
      var _ = g && p.strValue === "autorotate", X = p.pfValue != null && p.pfValue !== 0;
      if (_ || X) {
        var J = _ ? pa(i.rstyle, "labelAngle", a) : p.pfValue, Y = Math.cos(J), j = Math.sin(J), te = (P + A) / 2, ae = (B + R) / 2;
        if (!g) {
          switch (na(u.value)) {
            case "left":
              te = A;
              break;
            case "right":
              te = P;
              break;
          }
          switch (ia(l.value)) {
            case "top":
              ae = R;
              break;
            case "bottom":
              ae = B;
              break;
          }
        }
        var z = function(we, me) {
          return we = we - te, me = me - ae, {
            x: we * Y - me * j + te,
            y: we * j + me * Y + ae
          };
        }, L = z(P, B), V = z(P, R), W = z(A, B), K = z(A, R);
        P = Math.min(L.x, V.x, W.x, K.x), A = Math.max(L.x, V.x, W.x, K.x), B = Math.min(L.y, V.y, W.y, K.y), R = Math.max(L.y, V.y, W.y, K.y);
      }
      var re = H + "Rot", le = G[re] = G[re] || {};
      le.x1 = P, le.y1 = B, le.x2 = A, le.y2 = R, le.w = A - P, le.h = R - B, Ot(e, P, B, A, R), Ot(i.labelBounds.all, P, B, A, R);
    }
    return e;
  }
}, kl = function(e, r) {
  if (!r.cy().headless()) {
    var a = r.pstyle("outline-opacity").value, n = r.pstyle("outline-width").value, i = r.pstyle("outline-offset").value, o = n + i;
    ff(e, r, a, o, "outside", o / 2);
  }
}, ff = function(e, r, a, n, i, o) {
  if (!(a === 0 || n <= 0 || i === "inside")) {
    var s = r.cy(), u = s.renderer(), l = u.nodeShapes[u.getNodeShape(r)];
    if (l) {
      var c = r.position(), f = c.x, v = c.y, h = r.width(), d = r.height();
      if (l.hasMiterBounds) {
        i === "center" && (n /= 2);
        var y = l.miterBounds(f, v, h, d, n);
        nr(e, y);
      } else o != null && o > 0 && gn(e, [o, o, o, o]);
    }
  }
}, sy = function(e, r) {
  if (!r.cy().headless()) {
    var a = r.pstyle("border-opacity").value, n = r.pstyle("border-width").pfValue, i = r.pstyle("border-position").value;
    ff(e, r, a, n, i);
  }
}, uy = function(e, r) {
  var a = e._private.cy, n = a.styleEnabled(), i = a.headless(), o = wt(), s = e._private, u = e.isNode(), l = e.isEdge(), c, f, v, h, d, y, g = s.rstyle, p = u && n ? e.pstyle("bounds-expansion").pfValue : [0], m = function(Pe) {
    return Pe.pstyle("display").value !== "none";
  }, b = !n || m(e) && (!l || m(e.source()) && m(e.target()));
  if (b) {
    var w = 0, C = 0;
    n && r.includeOverlays && (w = e.pstyle("overlay-opacity").value, w !== 0 && (C = e.pstyle("overlay-padding").value));
    var E = 0, x = 0;
    n && r.includeUnderlays && (E = e.pstyle("underlay-opacity").value, E !== 0 && (x = e.pstyle("underlay-padding").value));
    var T = Math.max(C, x), k = 0, D = 0;
    if (n && (k = e.pstyle("width").pfValue, D = k / 2), u && r.includeNodes) {
      var P = e.position();
      d = P.x, y = P.y;
      var A = e.outerWidth(), B = A / 2, R = e.outerHeight(), I = R / 2;
      c = d - B, f = d + B, v = y - I, h = y + I, Ot(o, c, v, f, h), n && kl(o, e), n && r.includeOutlines && !i && kl(o, e), n && sy(o, e);
    } else if (l && r.includeEdges)
      if (n && !i) {
        var O = e.pstyle("curve-style").strValue;
        if (c = Math.min(g.srcX, g.midX, g.tgtX), f = Math.max(g.srcX, g.midX, g.tgtX), v = Math.min(g.srcY, g.midY, g.tgtY), h = Math.max(g.srcY, g.midY, g.tgtY), c -= D, f += D, v -= D, h += D, Ot(o, c, v, f, h), O === "haystack") {
          var M = g.haystackPts;
          if (M && M.length === 2) {
            if (c = M[0].x, v = M[0].y, f = M[1].x, h = M[1].y, c > f) {
              var N = c;
              c = f, f = N;
            }
            if (v > h) {
              var H = v;
              v = h, h = H;
            }
            Ot(o, c - D, v - D, f + D, h + D);
          }
        } else if (O === "bezier" || O === "unbundled-bezier" || ir(O, "segments") || ir(O, "taxi")) {
          var G;
          switch (O) {
            case "bezier":
            case "unbundled-bezier":
              G = g.bezierPts;
              break;
            case "segments":
            case "taxi":
            case "round-segments":
            case "round-taxi":
              G = g.linePts;
              break;
          }
          if (G != null)
            for (var F = 0; F < G.length; F++) {
              var _ = G[F];
              c = _.x - D, f = _.x + D, v = _.y - D, h = _.y + D, Ot(o, c, v, f, h);
            }
        }
      } else {
        var X = e.source(), J = X.position(), Y = e.target(), j = Y.position();
        if (c = J.x, f = j.x, v = J.y, h = j.y, c > f) {
          var te = c;
          c = f, f = te;
        }
        if (v > h) {
          var ae = v;
          v = h, h = ae;
        }
        c -= D, f += D, v -= D, h += D, Ot(o, c, v, f, h);
      }
    if (n && r.includeEdges && l && (on(o, e, "mid-source"), on(o, e, "mid-target"), on(o, e, "source"), on(o, e, "target")), n) {
      var z = e.pstyle("ghost").value === "yes";
      if (z) {
        var L = e.pstyle("ghost-offset-x").pfValue, V = e.pstyle("ghost-offset-y").pfValue;
        Ot(o, o.x1 + L, o.y1 + V, o.x2 + L, o.y2 + V);
      }
    }
    var W = s.bodyBounds = s.bodyBounds || {};
    fu(W, o), gn(W, p), hn(W, 1), n && (c = o.x1, f = o.x2, v = o.y1, h = o.y2, Ot(o, c - T, v - T, f + T, h + T));
    var K = s.overlayBounds = s.overlayBounds || {};
    fu(K, o), gn(K, p), hn(K, 1);
    var re = s.labelBounds = s.labelBounds || {};
    re.all != null ? Gh(re.all) : re.all = wt(), n && r.includeLabels && (r.includeMainLabels && Ao(o, e, null), l && (r.includeSourceLabels && Ao(o, e, "source"), r.includeTargetLabels && Ao(o, e, "target")));
  }
  return o.x1 = At(o.x1), o.y1 = At(o.y1), o.x2 = At(o.x2), o.y2 = At(o.y2), o.w = At(o.x2 - o.x1), o.h = At(o.y2 - o.y1), o.w > 0 && o.h > 0 && b && (gn(o, p), hn(o, 1)), o;
}, vf = function(e) {
  var r = 0, a = function(o) {
    return (o ? 1 : 0) << r++;
  }, n = 0;
  return n += a(e.incudeNodes), n += a(e.includeEdges), n += a(e.includeLabels), n += a(e.includeMainLabels), n += a(e.includeSourceLabels), n += a(e.includeTargetLabels), n += a(e.includeOverlays), n += a(e.includeOutlines), n;
}, df = function(e) {
  var r = function(s) {
    return Math.round(s);
  };
  if (e.isEdge()) {
    var a = e.source().position(), n = e.target().position();
    return iu([r(a.x), r(a.y), r(n.x), r(n.y)]);
  } else {
    var i = e.position();
    return iu([r(i.x), r(i.y)]);
  }
}, Pl = function(e, r) {
  var a = e._private, n, i = e.isEdge(), o = r == null ? Dl : vf(r), s = o === Dl;
  if (a.bbCache == null ? (n = uy(e, La), a.bbCache = n, a.bbCachePosKey = df(e)) : n = a.bbCache, !s) {
    var u = e.isNode();
    n = wt(), (r.includeNodes && u || r.includeEdges && !u) && (r.includeOverlays ? nr(n, a.overlayBounds) : nr(n, a.bodyBounds)), r.includeLabels && (r.includeMainLabels && (!i || r.includeSourceLabels && r.includeTargetLabels) ? nr(n, a.labelBounds.all) : (r.includeMainLabels && nr(n, a.labelBounds.mainRot), r.includeSourceLabels && nr(n, a.labelBounds.sourceRot), r.includeTargetLabels && nr(n, a.labelBounds.targetRot))), n.w = n.x2 - n.x1, n.h = n.y2 - n.y1;
  }
  return n;
}, La = {
  includeNodes: !0,
  includeEdges: !0,
  includeLabels: !0,
  includeMainLabels: !0,
  includeSourceLabels: !0,
  includeTargetLabels: !0,
  includeOverlays: !0,
  includeUnderlays: !0,
  includeOutlines: !0,
  useCache: !0
}, Dl = vf(La), Al = ft(La);
mr.boundingBox = function(t) {
  var e, r = t === void 0 || t.useCache === void 0 || t.useCache === !0, a = ta(function(c) {
    var f = c._private;
    return f.bbCache == null || f.styleDirty || f.bbCachePosKey !== df(c);
  }, function(c) {
    return c.id();
  });
  if (r && this.length === 1 && !a(this[0]))
    t === void 0 ? t = La : t = Al(t), e = Pl(this[0], t);
  else {
    e = wt(), t = t || La;
    var n = Al(t), i = this, o = i.cy(), s = o.styleEnabled();
    this.edges().forEach(a), this.nodes().forEach(a), s && this.recalculateRenderedStyle(r), this.updateCompoundBounds(!r);
    for (var u = 0; u < i.length; u++) {
      var l = i[u];
      a(l) && l.dirtyBoundingBoxCache(), nr(e, Pl(l, n));
    }
  }
  return e.x1 = At(e.x1), e.y1 = At(e.y1), e.x2 = At(e.x2), e.y2 = At(e.y2), e.w = At(e.x2 - e.x1), e.h = At(e.y2 - e.y1), e;
};
mr.dirtyBoundingBoxCache = function() {
  for (var t = 0; t < this.length; t++) {
    var e = this[t]._private;
    e.bbCache = null, e.bbCachePosKey = null, e.bodyBounds = null, e.overlayBounds = null, e.labelBounds.all = null, e.labelBounds.source = null, e.labelBounds.target = null, e.labelBounds.main = null, e.labelBounds.sourceRot = null, e.labelBounds.targetRot = null, e.labelBounds.mainRot = null, e.arrowBounds.source = null, e.arrowBounds.target = null, e.arrowBounds["mid-source"] = null, e.arrowBounds["mid-target"] = null;
  }
  return this.emitAndNotify("bounds"), this;
};
mr.boundingBoxAt = function(t) {
  var e = this.nodes(), r = this.cy(), a = r.hasCompoundNodes(), n = r.collection();
  if (a && (n = e.filter(function(l) {
    return l.isParent();
  }), e = e.not(n)), Me(t)) {
    var i = t;
    t = function() {
      return i;
    };
  }
  var o = function(c, f) {
    return c._private.bbAtOldPos = t(c, f);
  }, s = function(c) {
    return c._private.bbAtOldPos;
  };
  r.startBatch(), e.forEach(o).silentPositions(t), a && (n.dirtyCompoundBoundsCache(), n.dirtyBoundingBoxCache(), n.updateCompoundBounds(!0));
  var u = _h(this.boundingBox({
    useCache: !1
  }));
  return e.silentPositions(s), a && (n.dirtyCompoundBoundsCache(), n.dirtyBoundingBoxCache(), n.updateCompoundBounds(!0)), r.endBatch(), u;
};
ea.boundingbox = ea.bb = ea.boundingBox;
ea.renderedBoundingbox = ea.renderedBoundingBox;
var ly = mr, Ca, Wa;
Ca = Wa = {};
var hf = function(e) {
  e.uppercaseName = $s(e.name), e.autoName = "auto" + e.uppercaseName, e.labelName = "label" + e.uppercaseName, e.outerName = "outer" + e.uppercaseName, e.uppercaseOuterName = $s(e.outerName), Ca[e.name] = function() {
    var a = this[0], n = a._private, i = n.cy, o = i._private.styleEnabled;
    if (a)
      if (o) {
        if (a.isParent())
          return a.updateCompoundBounds(), n[e.autoName] || 0;
        var s = a.pstyle(e.name);
        switch (s.strValue) {
          case "label":
            return a.recalculateRenderedStyle(), n.rstyle[e.labelName] || 0;
          default:
            return s.pfValue;
        }
      } else
        return 1;
  }, Ca["outer" + e.uppercaseName] = function() {
    var a = this[0], n = a._private, i = n.cy, o = i._private.styleEnabled;
    if (a)
      if (o) {
        var s = a[e.name](), u = a.pstyle("border-position").value, l;
        u === "center" ? l = a.pstyle("border-width").pfValue : u === "outside" ? l = 2 * a.pstyle("border-width").pfValue : l = 0;
        var c = 2 * a.padding();
        return s + l + c;
      } else
        return 1;
  }, Ca["rendered" + e.uppercaseName] = function() {
    var a = this[0];
    if (a) {
      var n = a[e.name]();
      return n * this.cy().zoom();
    }
  }, Ca["rendered" + e.uppercaseOuterName] = function() {
    var a = this[0];
    if (a) {
      var n = a[e.outerName]();
      return n * this.cy().zoom();
    }
  };
};
hf({
  name: "width"
});
hf({
  name: "height"
});
Wa.padding = function() {
  var t = this[0], e = t._private;
  return t.isParent() ? (t.updateCompoundBounds(), e.autoPadding !== void 0 ? e.autoPadding : t.pstyle("padding").pfValue) : t.pstyle("padding").pfValue;
};
Wa.paddedHeight = function() {
  var t = this[0];
  return t.height() + 2 * t.padding();
};
Wa.paddedWidth = function() {
  var t = this[0];
  return t.width() + 2 * t.padding();
};
var cy = Wa, fy = function(e, r) {
  if (e.isEdge() && e.takesUpSpace())
    return r(e);
}, vy = function(e, r) {
  if (e.isEdge() && e.takesUpSpace()) {
    var a = e.cy();
    return $n(r(e), a.zoom(), a.pan());
  }
}, dy = function(e, r) {
  if (e.isEdge() && e.takesUpSpace()) {
    var a = e.cy(), n = a.pan(), i = a.zoom();
    return r(e).map(function(o) {
      return $n(o, i, n);
    });
  }
}, hy = function(e) {
  return e.renderer().getControlPoints(e);
}, gy = function(e) {
  return e.renderer().getSegmentPoints(e);
}, py = function(e) {
  return e.renderer().getSourceEndpoint(e);
}, yy = function(e) {
  return e.renderer().getTargetEndpoint(e);
}, my = function(e) {
  return e.renderer().getEdgeMidpoint(e);
}, Bl = {
  controlPoints: {
    get: hy,
    mult: !0
  },
  segmentPoints: {
    get: gy,
    mult: !0
  },
  sourceEndpoint: {
    get: py
  },
  targetEndpoint: {
    get: yy
  },
  midpoint: {
    get: my
  }
}, by = function(e) {
  return "rendered" + e[0].toUpperCase() + e.substr(1);
}, wy = Object.keys(Bl).reduce(function(t, e) {
  var r = Bl[e], a = by(e);
  return t[e] = function() {
    return fy(this, r.get);
  }, r.mult ? t[a] = function() {
    return dy(this, r.get);
  } : t[a] = function() {
    return vy(this, r.get);
  }, t;
}, {}), xy = ye({}, iy, ly, cy, wy);
/*!
Event object based on jQuery events, MIT license

https://jquery.org/license/
https://tldrlegal.com/license/mit-license
https://github.com/jquery/jquery/blob/master/src/event.js
*/
var gf = function(e, r) {
  this.recycle(e, r);
};
function ya() {
  return !1;
}
function sn() {
  return !0;
}
gf.prototype = {
  instanceString: function() {
    return "event";
  },
  recycle: function(e, r) {
    if (this.isImmediatePropagationStopped = this.isPropagationStopped = this.isDefaultPrevented = ya, e != null && e.preventDefault ? (this.type = e.type, this.isDefaultPrevented = e.defaultPrevented ? sn : ya) : e != null && e.type ? r = e : this.type = e, r != null && (this.originalEvent = r.originalEvent, this.type = r.type != null ? r.type : this.type, this.cy = r.cy, this.target = r.target, this.position = r.position, this.renderedPosition = r.renderedPosition, this.namespace = r.namespace, this.layout = r.layout), this.cy != null && this.position != null && this.renderedPosition == null) {
      var a = this.position, n = this.cy.zoom(), i = this.cy.pan();
      this.renderedPosition = {
        x: a.x * n + i.x,
        y: a.y * n + i.y
      };
    }
    this.timeStamp = e && e.timeStamp || Date.now();
  },
  preventDefault: function() {
    this.isDefaultPrevented = sn;
    var e = this.originalEvent;
    e && e.preventDefault && e.preventDefault();
  },
  stopPropagation: function() {
    this.isPropagationStopped = sn;
    var e = this.originalEvent;
    e && e.stopPropagation && e.stopPropagation();
  },
  stopImmediatePropagation: function() {
    this.isImmediatePropagationStopped = sn, this.stopPropagation();
  },
  isDefaultPrevented: ya,
  isPropagationStopped: ya,
  isImmediatePropagationStopped: ya
};
var pf = /^([^.]+)(\.(?:[^.]+))?$/, Ey = ".*", yf = {
  qualifierCompare: function(e, r) {
    return e === r;
  },
  eventMatches: function() {
    return !0;
  },
  addEventFields: function() {
  },
  callbackContext: function(e) {
    return e;
  },
  beforeEmit: function() {
  },
  afterEmit: function() {
  },
  bubble: function() {
    return !1;
  },
  parent: function() {
    return null;
  },
  context: null
}, Rl = Object.keys(yf), Cy = {};
function Qn() {
  for (var t = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : Cy, e = arguments.length > 1 ? arguments[1] : void 0, r = 0; r < Rl.length; r++) {
    var a = Rl[r];
    this[a] = t[a] || yf[a];
  }
  this.context = e || this.context, this.listeners = [], this.emitting = 0;
}
var gr = Qn.prototype, mf = function(e, r, a, n, i, o, s) {
  Ue(n) && (i = n, n = null), s && (o == null ? o = s : o = ye({}, o, s));
  for (var u = Ve(a) ? a : a.split(/\s+/), l = 0; l < u.length; l++) {
    var c = u[l];
    if (!cr(c)) {
      var f = c.match(pf);
      if (f) {
        var v = f[1], h = f[2] ? f[2] : null, d = r(e, c, v, h, n, i, o);
        if (d === !1)
          break;
      }
    }
  }
}, Ml = function(e, r) {
  return e.addEventFields(e.context, r), new gf(r.type, r);
}, Ty = function(e, r, a) {
  if (Ad(a)) {
    r(e, a);
    return;
  } else if (Me(a)) {
    r(e, Ml(e, a));
    return;
  }
  for (var n = Ve(a) ? a : a.split(/\s+/), i = 0; i < n.length; i++) {
    var o = n[i];
    if (!cr(o)) {
      var s = o.match(pf);
      if (s) {
        var u = s[1], l = s[2] ? s[2] : null, c = Ml(e, {
          type: u,
          namespace: l,
          target: e.context
        });
        r(e, c);
      }
    }
  }
};
gr.on = gr.addListener = function(t, e, r, a, n) {
  return mf(this, function(i, o, s, u, l, c, f) {
    Ue(c) && i.listeners.push({
      event: o,
      // full event string
      callback: c,
      // callback to run
      type: s,
      // the event type (e.g. 'click')
      namespace: u,
      // the event namespace (e.g. ".foo")
      qualifier: l,
      // a restriction on whether to match this emitter
      conf: f
      // additional configuration
    });
  }, t, e, r, a, n), this;
};
gr.one = function(t, e, r, a) {
  return this.on(t, e, r, a, {
    one: !0
  });
};
gr.removeListener = gr.off = function(t, e, r, a) {
  var n = this;
  this.emitting !== 0 && (this.listeners = fh(this.listeners));
  for (var i = this.listeners, o = function(l) {
    var c = i[l];
    mf(n, function(f, v, h, d, y, g) {
      if ((c.type === h || t === "*") && (!d && c.namespace !== ".*" || c.namespace === d) && (!y || f.qualifierCompare(c.qualifier, y)) && (!g || c.callback === g))
        return i.splice(l, 1), !1;
    }, t, e, r, a);
  }, s = i.length - 1; s >= 0; s--)
    o(s);
  return this;
};
gr.removeAllListeners = function() {
  return this.removeListener("*");
};
gr.emit = gr.trigger = function(t, e, r) {
  var a = this.listeners, n = a.length;
  return this.emitting++, Ve(e) || (e = [e]), Ty(this, function(i, o) {
    r != null && (a = [{
      event: o.event,
      type: o.type,
      namespace: o.namespace,
      callback: r
    }], n = a.length);
    for (var s = function() {
      var c = a[u];
      if (c.type === o.type && (!c.namespace || c.namespace === o.namespace || c.namespace === Ey) && i.eventMatches(i.context, c, o)) {
        var f = [o];
        e != null && dh(f, e), i.beforeEmit(i.context, c, o), c.conf && c.conf.one && (i.listeners = i.listeners.filter(function(d) {
          return d !== c;
        }));
        var v = i.callbackContext(i.context, c, o), h = c.callback.apply(v, f);
        i.afterEmit(i.context, c, o), h === !1 && (o.stopPropagation(), o.preventDefault());
      }
    }, u = 0; u < n; u++)
      s();
    i.bubble(i.context) && !o.isPropagationStopped() && i.parent(i.context).emit(o, e);
  }, t), this.emitting--, this;
};
var Sy = {
  qualifierCompare: function(e, r) {
    return e == null || r == null ? e == null && r == null : e.sameText(r);
  },
  eventMatches: function(e, r, a) {
    var n = r.qualifier;
    return n != null ? e !== a.target && qa(a.target) && n.matches(a.target) : !0;
  },
  addEventFields: function(e, r) {
    r.cy = e.cy(), r.target = e;
  },
  callbackContext: function(e, r, a) {
    return r.qualifier != null ? a.target : e;
  },
  beforeEmit: function(e, r) {
    r.conf && r.conf.once && r.conf.onceCollection.removeListener(r.event, r.qualifier, r.callback);
  },
  bubble: function() {
    return !0;
  },
  parent: function(e) {
    return e.isChild() ? e.parent() : e.cy();
  }
}, un = function(e) {
  return he(e) ? new dr(e) : e;
}, bf = {
  createEmitter: function() {
    for (var e = 0; e < this.length; e++) {
      var r = this[e], a = r._private;
      a.emitter || (a.emitter = new Qn(Sy, r));
    }
    return this;
  },
  emitter: function() {
    return this._private.emitter;
  },
  on: function(e, r, a) {
    for (var n = un(r), i = 0; i < this.length; i++) {
      var o = this[i];
      o.emitter().on(e, n, a);
    }
    return this;
  },
  removeListener: function(e, r, a) {
    for (var n = un(r), i = 0; i < this.length; i++) {
      var o = this[i];
      o.emitter().removeListener(e, n, a);
    }
    return this;
  },
  removeAllListeners: function() {
    for (var e = 0; e < this.length; e++) {
      var r = this[e];
      r.emitter().removeAllListeners();
    }
    return this;
  },
  one: function(e, r, a) {
    for (var n = un(r), i = 0; i < this.length; i++) {
      var o = this[i];
      o.emitter().one(e, n, a);
    }
    return this;
  },
  once: function(e, r, a) {
    for (var n = un(r), i = 0; i < this.length; i++) {
      var o = this[i];
      o.emitter().on(e, n, a, {
        once: !0,
        onceCollection: this
      });
    }
  },
  emit: function(e, r) {
    for (var a = 0; a < this.length; a++) {
      var n = this[a];
      n.emitter().emit(e, r);
    }
    return this;
  },
  emitAndNotify: function(e, r) {
    if (this.length !== 0)
      return this.cy().notify(e, this), this.emit(e, r), this;
  }
};
Ne.eventAliasesOn(bf);
var wf = {
  nodes: function(e) {
    return this.filter(function(r) {
      return r.isNode();
    }).filter(e);
  },
  edges: function(e) {
    return this.filter(function(r) {
      return r.isEdge();
    }).filter(e);
  },
  // internal helper to get nodes and edges as separate collections with single iteration over elements
  byGroup: function() {
    for (var e = this.spawn(), r = this.spawn(), a = 0; a < this.length; a++) {
      var n = this[a];
      n.isNode() ? e.push(n) : r.push(n);
    }
    return {
      nodes: e,
      edges: r
    };
  },
  filter: function(e, r) {
    if (e === void 0)
      return this;
    if (he(e) || kt(e))
      return new dr(e).filter(this);
    if (Ue(e)) {
      for (var a = this.spawn(), n = this, i = 0; i < n.length; i++) {
        var o = n[i], s = r ? e.apply(r, [o, i, n]) : e(o, i, n);
        s && a.push(o);
      }
      return a;
    }
    return this.spawn();
  },
  not: function(e) {
    if (e) {
      he(e) && (e = this.filter(e));
      for (var r = this.spawn(), a = 0; a < this.length; a++) {
        var n = this[a], i = e.has(n);
        i || r.push(n);
      }
      return r;
    } else
      return this;
  },
  absoluteComplement: function() {
    var e = this.cy();
    return e.mutableElements().not(this);
  },
  intersect: function(e) {
    if (he(e)) {
      var r = e;
      return this.filter(r);
    }
    for (var a = this.spawn(), n = this, i = e, o = this.length < e.length, s = o ? n : i, u = o ? i : n, l = 0; l < s.length; l++) {
      var c = s[l];
      u.has(c) && a.push(c);
    }
    return a;
  },
  xor: function(e) {
    var r = this._private.cy;
    he(e) && (e = r.$(e));
    var a = this.spawn(), n = this, i = e, o = function(u, l) {
      for (var c = 0; c < u.length; c++) {
        var f = u[c], v = f._private.data.id, h = l.hasElementWithId(v);
        h || a.push(f);
      }
    };
    return o(n, i), o(i, n), a;
  },
  diff: function(e) {
    var r = this._private.cy;
    he(e) && (e = r.$(e));
    var a = this.spawn(), n = this.spawn(), i = this.spawn(), o = this, s = e, u = function(c, f, v) {
      for (var h = 0; h < c.length; h++) {
        var d = c[h], y = d._private.data.id, g = f.hasElementWithId(y);
        g ? i.merge(d) : v.push(d);
      }
    };
    return u(o, s, a), u(s, o, n), {
      left: a,
      right: n,
      both: i
    };
  },
  add: function(e) {
    var r = this._private.cy;
    if (!e)
      return this;
    if (he(e)) {
      var a = e;
      e = r.mutableElements().filter(a);
    }
    for (var n = this.spawnSelf(), i = 0; i < e.length; i++) {
      var o = e[i], s = !this.has(o);
      s && n.push(o);
    }
    return n;
  },
  // in place merge on calling collection
  merge: function(e) {
    var r = this._private, a = r.cy;
    if (!e)
      return this;
    if (e && he(e)) {
      var n = e;
      e = a.mutableElements().filter(n);
    }
    for (var i = r.map, o = 0; o < e.length; o++) {
      var s = e[o], u = s._private.data.id, l = !i.has(u);
      if (l) {
        var c = this.length++;
        this[c] = s, i.set(u, {
          ele: s,
          index: c
        });
      }
    }
    return this;
  },
  unmergeAt: function(e) {
    var r = this[e], a = r.id(), n = this._private, i = n.map;
    this[e] = void 0, i.delete(a);
    var o = e === this.length - 1;
    if (this.length > 1 && !o) {
      var s = this.length - 1, u = this[s], l = u._private.data.id;
      this[s] = void 0, this[e] = u, i.set(l, {
        ele: u,
        index: e
      });
    }
    return this.length--, this;
  },
  // remove single ele in place in calling collection
  unmergeOne: function(e) {
    e = e[0];
    var r = this._private, a = e._private.data.id, n = r.map, i = n.get(a);
    if (!i)
      return this;
    var o = i.index;
    return this.unmergeAt(o), this;
  },
  // remove eles in place on calling collection
  unmerge: function(e) {
    var r = this._private.cy;
    if (!e)
      return this;
    if (e && he(e)) {
      var a = e;
      e = r.mutableElements().filter(a);
    }
    for (var n = 0; n < e.length; n++)
      this.unmergeOne(e[n]);
    return this;
  },
  unmergeBy: function(e) {
    for (var r = this.length - 1; r >= 0; r--) {
      var a = this[r];
      e(a) && this.unmergeAt(r);
    }
    return this;
  },
  map: function(e, r) {
    for (var a = [], n = this, i = 0; i < n.length; i++) {
      var o = n[i], s = r ? e.apply(r, [o, i, n]) : e(o, i, n);
      a.push(s);
    }
    return a;
  },
  reduce: function(e, r) {
    for (var a = r, n = this, i = 0; i < n.length; i++)
      a = e(a, n[i], i, n);
    return a;
  },
  max: function(e, r) {
    for (var a = -1 / 0, n, i = this, o = 0; o < i.length; o++) {
      var s = i[o], u = r ? e.apply(r, [s, o, i]) : e(s, o, i);
      u > a && (a = u, n = s);
    }
    return {
      value: a,
      ele: n
    };
  },
  min: function(e, r) {
    for (var a = 1 / 0, n, i = this, o = 0; o < i.length; o++) {
      var s = i[o], u = r ? e.apply(r, [s, o, i]) : e(s, o, i);
      u < a && (a = u, n = s);
    }
    return {
      value: a,
      ele: n
    };
  }
}, Le = wf;
Le.u = Le["|"] = Le["+"] = Le.union = Le.or = Le.add;
Le["\\"] = Le["!"] = Le["-"] = Le.difference = Le.relativeComplement = Le.subtract = Le.not;
Le.n = Le["&"] = Le["."] = Le.and = Le.intersection = Le.intersect;
Le["^"] = Le["(+)"] = Le["(-)"] = Le.symmetricDifference = Le.symdiff = Le.xor;
Le.fnFilter = Le.filterFn = Le.stdFilter = Le.filter;
Le.complement = Le.abscomp = Le.absoluteComplement;
var ky = {
  isNode: function() {
    return this.group() === "nodes";
  },
  isEdge: function() {
    return this.group() === "edges";
  },
  isLoop: function() {
    return this.isEdge() && this.source()[0] === this.target()[0];
  },
  isSimple: function() {
    return this.isEdge() && this.source()[0] !== this.target()[0];
  },
  group: function() {
    var e = this[0];
    if (e)
      return e._private.group;
  }
}, xf = function(e, r) {
  var a = e.cy(), n = a.hasCompoundNodes();
  function i(c) {
    var f = c.pstyle("z-compound-depth");
    return f.value === "auto" ? n ? c.zDepth() : 0 : f.value === "bottom" ? -1 : f.value === "top" ? hs : 0;
  }
  var o = i(e) - i(r);
  if (o !== 0)
    return o;
  function s(c) {
    var f = c.pstyle("z-index-compare");
    return f.value === "auto" && c.isNode() ? 1 : 0;
  }
  var u = s(e) - s(r);
  if (u !== 0)
    return u;
  var l = e.pstyle("z-index").value - r.pstyle("z-index").value;
  return l !== 0 ? l : e.poolIndex() - r.poolIndex();
}, Ln = {
  forEach: function(e, r) {
    if (Ue(e))
      for (var a = this.length, n = 0; n < a; n++) {
        var i = this[n], o = r ? e.apply(r, [i, n, this]) : e(i, n, this);
        if (o === !1)
          break;
      }
    return this;
  },
  toArray: function() {
    for (var e = [], r = 0; r < this.length; r++)
      e.push(this[r]);
    return e;
  },
  slice: function(e, r) {
    var a = [], n = this.length;
    r == null && (r = n), e == null && (e = 0), e < 0 && (e = n + e), r < 0 && (r = n + r);
    for (var i = e; i >= 0 && i < r && i < n; i++)
      a.push(this[i]);
    return this.spawn(a);
  },
  size: function() {
    return this.length;
  },
  eq: function(e) {
    return this[e] || this.spawn();
  },
  first: function() {
    return this[0] || this.spawn();
  },
  last: function() {
    return this[this.length - 1] || this.spawn();
  },
  empty: function() {
    return this.length === 0;
  },
  nonempty: function() {
    return !this.empty();
  },
  sort: function(e) {
    if (!Ue(e))
      return this;
    var r = this.toArray().sort(e);
    return this.spawn(r);
  },
  sortByZIndex: function() {
    return this.sort(xf);
  },
  zDepth: function() {
    var e = this[0];
    if (e) {
      var r = e._private, a = r.group;
      if (a === "nodes") {
        var n = r.data.parent ? e.parents().size() : 0;
        return e.isParent() ? n : hs - 1;
      } else {
        var i = r.source, o = r.target, s = i.zDepth(), u = o.zDepth();
        return Math.max(s, u, 0);
      }
    }
  }
};
Ln.each = Ln.forEach;
var Py = function() {
  var e = "undefined", r = (typeof Symbol > "u" ? "undefined" : rt(Symbol)) != e && rt(Symbol.iterator) != e;
  r && (Ln[Symbol.iterator] = function() {
    var a = this, n = {
      value: void 0,
      done: !1
    }, i = 0, o = this.length;
    return mc({
      next: function() {
        return i < o ? n.value = a[i++] : (n.value = void 0, n.done = !0), n;
      }
    }, Symbol.iterator, function() {
      return this;
    });
  });
};
Py();
var Dy = ft({
  nodeDimensionsIncludeLabels: !1
}), yn = {
  // Calculates and returns node dimensions { x, y } based on options given
  layoutDimensions: function(e) {
    e = Dy(e);
    var r;
    if (!this.takesUpSpace())
      r = {
        w: 0,
        h: 0
      };
    else if (e.nodeDimensionsIncludeLabels) {
      var a = this.boundingBox();
      r = {
        w: a.w,
        h: a.h
      };
    } else
      r = {
        w: this.outerWidth(),
        h: this.outerHeight()
      };
    return (r.w === 0 || r.h === 0) && (r.w = r.h = 1), r;
  },
  // using standard layout options, apply position function (w/ or w/o animation)
  layoutPositions: function(e, r, a) {
    var n = this.nodes().filter(function(C) {
      return !C.isParent();
    }), i = this.cy(), o = r.eles, s = function(E) {
      return E.id();
    }, u = ta(a, s);
    e.emit({
      type: "layoutstart",
      layout: e
    }), e.animations = [];
    var l = function(E, x, T) {
      var k = {
        x: x.x1 + x.w / 2,
        y: x.y1 + x.h / 2
      }, D = {
        // scale from center of bounding box (not necessarily 0,0)
        x: (T.x - k.x) * E,
        y: (T.y - k.y) * E
      };
      return {
        x: k.x + D.x,
        y: k.y + D.y
      };
    }, c = r.spacingFactor && r.spacingFactor !== 1, f = function() {
      if (!c)
        return null;
      for (var E = wt(), x = 0; x < n.length; x++) {
        var T = n[x], k = u(T, x);
        Fc(E, k.x, k.y);
      }
      return E;
    }, v = f(), h = ta(function(C, E) {
      var x = u(C, E);
      if (c) {
        var T = Math.abs(r.spacingFactor);
        x = l(T, v, x);
      }
      return r.transform != null && (x = r.transform(C, x)), x;
    }, s);
    if (r.animate) {
      for (var d = 0; d < n.length; d++) {
        var y = n[d], g = h(y, d), p = r.animateFilter == null || r.animateFilter(y, d);
        if (p) {
          var m = y.animation({
            position: g,
            duration: r.animationDuration,
            easing: r.animationEasing
          });
          e.animations.push(m);
        } else
          y.position(g);
      }
      if (r.fit) {
        var b = i.animation({
          fit: {
            boundingBox: o.boundingBoxAt(h),
            padding: r.padding
          },
          duration: r.animationDuration,
          easing: r.animationEasing
        });
        e.animations.push(b);
      } else if (r.zoom !== void 0 && r.pan !== void 0) {
        var w = i.animation({
          zoom: r.zoom,
          pan: r.pan,
          duration: r.animationDuration,
          easing: r.animationEasing
        });
        e.animations.push(w);
      }
      e.animations.forEach(function(C) {
        return C.play();
      }), e.one("layoutready", r.ready), e.emit({
        type: "layoutready",
        layout: e
      }), sa.all(e.animations.map(function(C) {
        return C.promise();
      })).then(function() {
        e.one("layoutstop", r.stop), e.emit({
          type: "layoutstop",
          layout: e
        });
      });
    } else
      n.positions(h), r.fit && i.fit(r.eles, r.padding), r.zoom != null && i.zoom(r.zoom), r.pan && i.pan(r.pan), e.one("layoutready", r.ready), e.emit({
        type: "layoutready",
        layout: e
      }), e.one("layoutstop", r.stop), e.emit({
        type: "layoutstop",
        layout: e
      });
    return this;
  },
  layout: function(e) {
    var r = this.cy();
    return r.makeLayout(ye({}, e, {
      eles: this
    }));
  }
};
yn.createLayout = yn.makeLayout = yn.layout;
function Ef(t, e, r) {
  var a = r._private, n = a.styleCache = a.styleCache || [], i;
  return (i = n[t]) != null || (i = n[t] = e(r)), i;
}
function Jn(t, e) {
  return t = Ar(t), function(a) {
    return Ef(t, e, a);
  };
}
function jn(t, e) {
  t = Ar(t);
  var r = function(n) {
    return e.call(n);
  };
  return function() {
    var n = this[0];
    if (n)
      return Ef(t, r, n);
  };
}
var lt = {
  recalculateRenderedStyle: function(e) {
    var r = this.cy(), a = r.renderer(), n = r.styleEnabled();
    return a && n && a.recalculateRenderedStyle(this, e), this;
  },
  dirtyStyleCache: function() {
    var e = this.cy(), r = function(i) {
      return i._private.styleCache = null;
    };
    if (e.hasCompoundNodes()) {
      var a;
      a = this.spawnSelf().merge(this.descendants()).merge(this.parents()), a.merge(a.connectedEdges()), a.forEach(r);
    } else
      this.forEach(function(n) {
        r(n), n.connectedEdges().forEach(r);
      });
    return this;
  },
  // fully updates (recalculates) the style for the elements
  updateStyle: function(e) {
    var r = this._private.cy;
    if (!r.styleEnabled())
      return this;
    if (r.batching()) {
      var a = r._private.batchStyleEles;
      return a.merge(this), this;
    }
    var n = r.hasCompoundNodes(), i = this;
    e = !!(e || e === void 0), n && (i = this.spawnSelf().merge(this.descendants()).merge(this.parents()));
    var o = i;
    return e ? o.emitAndNotify("style") : o.emit("style"), i.forEach(function(s) {
      return s._private.styleDirty = !0;
    }), this;
  },
  // private: clears dirty flag and recalculates style
  cleanStyle: function() {
    var e = this.cy();
    if (e.styleEnabled())
      for (var r = 0; r < this.length; r++) {
        var a = this[r];
        a._private.styleDirty && (a._private.styleDirty = !1, e.style().apply(a));
      }
  },
  // get the internal parsed style object for the specified property
  parsedStyle: function(e) {
    var r = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : !0, a = this[0], n = a.cy();
    if (n.styleEnabled() && a) {
      a._private.styleDirty && (a._private.styleDirty = !1, n.style().apply(a));
      var i = a._private.style[e];
      return i ?? (r ? n.style().getDefaultProperty(e) : null);
    }
  },
  numericStyle: function(e) {
    var r = this[0];
    if (r.cy().styleEnabled() && r) {
      var a = r.pstyle(e);
      return a.pfValue !== void 0 ? a.pfValue : a.value;
    }
  },
  numericStyleUnits: function(e) {
    var r = this[0];
    if (r.cy().styleEnabled() && r)
      return r.pstyle(e).units;
  },
  // get the specified css property as a rendered value (i.e. on-screen value)
  // or get the whole rendered style if no property specified (NB doesn't allow setting)
  renderedStyle: function(e) {
    var r = this.cy();
    if (!r.styleEnabled())
      return this;
    var a = this[0];
    if (a)
      return r.style().getRenderedStyle(a, e);
  },
  // read the calculated css style of the element or override the style (via a bypass)
  style: function(e, r) {
    var a = this.cy();
    if (!a.styleEnabled())
      return this;
    var n = !1, i = a.style();
    if (Me(e)) {
      var o = e;
      i.applyBypass(this, o, n), this.emitAndNotify("style");
    } else if (he(e))
      if (r === void 0) {
        var s = this[0];
        return s ? i.getStylePropertyValue(s, e) : void 0;
      } else
        i.applyBypass(this, e, r, n), this.emitAndNotify("style");
    else if (e === void 0) {
      var u = this[0];
      return u ? i.getRawStyle(u) : void 0;
    }
    return this;
  },
  removeStyle: function(e) {
    var r = this.cy();
    if (!r.styleEnabled())
      return this;
    var a = !1, n = r.style(), i = this;
    if (e === void 0)
      for (var o = 0; o < i.length; o++) {
        var s = i[o];
        n.removeAllBypasses(s, a);
      }
    else {
      e = e.split(/\s+/);
      for (var u = 0; u < i.length; u++) {
        var l = i[u];
        n.removeBypasses(l, e, a);
      }
    }
    return this.emitAndNotify("style"), this;
  },
  show: function() {
    return this.css("display", "element"), this;
  },
  hide: function() {
    return this.css("display", "none"), this;
  },
  effectiveOpacity: function() {
    var e = this.cy();
    if (!e.styleEnabled())
      return 1;
    var r = e.hasCompoundNodes(), a = this[0];
    if (a) {
      var n = a._private, i = a.pstyle("opacity").value;
      if (!r)
        return i;
      var o = n.data.parent ? a.parents() : null;
      if (o)
        for (var s = 0; s < o.length; s++) {
          var u = o[s], l = u.pstyle("opacity").value;
          i = l * i;
        }
      return i;
    }
  },
  transparent: function() {
    var e = this.cy();
    if (!e.styleEnabled())
      return !1;
    var r = this[0], a = r.cy().hasCompoundNodes();
    if (r)
      return a ? r.effectiveOpacity() === 0 : r.pstyle("opacity").value === 0;
  },
  backgrounding: function() {
    var e = this.cy();
    if (!e.styleEnabled())
      return !1;
    var r = this[0];
    return !!r._private.backgrounding;
  }
};
function Bo(t, e) {
  var r = t._private, a = r.data.parent ? t.parents() : null;
  if (a)
    for (var n = 0; n < a.length; n++) {
      var i = a[n];
      if (!e(i))
        return !1;
    }
  return !0;
}
function ks(t) {
  var e = t.ok, r = t.edgeOkViaNode || t.ok, a = t.parentOk || t.ok;
  return function() {
    var n = this.cy();
    if (!n.styleEnabled())
      return !0;
    var i = this[0], o = n.hasCompoundNodes();
    if (i) {
      var s = i._private;
      if (!e(i))
        return !1;
      if (i.isNode())
        return !o || Bo(i, a);
      var u = s.source, l = s.target;
      return r(u) && (!o || Bo(u, r)) && (u === l || r(l) && (!o || Bo(l, r)));
    }
  };
}
var ua = Jn("eleTakesUpSpace", function(t) {
  return t.pstyle("display").value === "element" && t.width() !== 0 && (t.isNode() ? t.height() !== 0 : !0);
});
lt.takesUpSpace = jn("takesUpSpace", ks({
  ok: ua
}));
var Ay = Jn("eleInteractive", function(t) {
  return t.pstyle("events").value === "yes" && t.pstyle("visibility").value === "visible" && ua(t);
}), By = Jn("parentInteractive", function(t) {
  return t.pstyle("visibility").value === "visible" && ua(t);
});
lt.interactive = jn("interactive", ks({
  ok: Ay,
  parentOk: By,
  edgeOkViaNode: ua
}));
lt.noninteractive = function() {
  var t = this[0];
  if (t)
    return !t.interactive();
};
var Ry = Jn("eleVisible", function(t) {
  return t.pstyle("visibility").value === "visible" && t.pstyle("opacity").pfValue !== 0 && ua(t);
}), My = ua;
lt.visible = jn("visible", ks({
  ok: Ry,
  edgeOkViaNode: My
}));
lt.hidden = function() {
  var t = this[0];
  if (t)
    return !t.visible();
};
lt.isBundledBezier = jn("isBundledBezier", function() {
  return this.cy().styleEnabled() ? !this.removed() && this.pstyle("curve-style").value === "bezier" && this.takesUpSpace() : !1;
});
lt.bypass = lt.css = lt.style;
lt.renderedCss = lt.renderedStyle;
lt.removeBypass = lt.removeCss = lt.removeStyle;
lt.pstyle = lt.parsedStyle;
var lr = {};
function Il(t) {
  return function() {
    var e = arguments, r = [];
    if (e.length === 2) {
      var a = e[0], n = e[1];
      this.on(t.event, a, n);
    } else if (e.length === 1 && Ue(e[0])) {
      var i = e[0];
      this.on(t.event, i);
    } else if (e.length === 0 || e.length === 1 && Ve(e[0])) {
      for (var o = e.length === 1 ? e[0] : null, s = 0; s < this.length; s++) {
        var u = this[s], l = !t.ableField || u._private[t.ableField], c = u._private[t.field] != t.value;
        if (t.overrideAble) {
          var f = t.overrideAble(u);
          if (f !== void 0 && (l = f, !f))
            return this;
        }
        l && (u._private[t.field] = t.value, c && r.push(u));
      }
      var v = this.spawn(r);
      v.updateStyle(), v.emit(t.event), o && v.emit(o);
    }
    return this;
  };
}
function la(t) {
  lr[t.field] = function() {
    var e = this[0];
    if (e) {
      if (t.overrideField) {
        var r = t.overrideField(e);
        if (r !== void 0)
          return r;
      }
      return e._private[t.field];
    }
  }, lr[t.on] = Il({
    event: t.on,
    field: t.field,
    ableField: t.ableField,
    overrideAble: t.overrideAble,
    value: !0
  }), lr[t.off] = Il({
    event: t.off,
    field: t.field,
    ableField: t.ableField,
    overrideAble: t.overrideAble,
    value: !1
  });
}
la({
  field: "locked",
  overrideField: function(e) {
    return e.cy().autolock() ? !0 : void 0;
  },
  on: "lock",
  off: "unlock"
});
la({
  field: "grabbable",
  overrideField: function(e) {
    return e.cy().autoungrabify() || e.pannable() ? !1 : void 0;
  },
  on: "grabify",
  off: "ungrabify"
});
la({
  field: "selected",
  ableField: "selectable",
  overrideAble: function(e) {
    return e.cy().autounselectify() ? !1 : void 0;
  },
  on: "select",
  off: "unselect"
});
la({
  field: "selectable",
  overrideField: function(e) {
    return e.cy().autounselectify() ? !1 : void 0;
  },
  on: "selectify",
  off: "unselectify"
});
lr.deselect = lr.unselect;
lr.grabbed = function() {
  var t = this[0];
  if (t)
    return t._private.grabbed;
};
la({
  field: "active",
  on: "activate",
  off: "unactivate"
});
la({
  field: "pannable",
  on: "panify",
  off: "unpanify"
});
lr.inactive = function() {
  var t = this[0];
  if (t)
    return !t._private.active;
};
var ht = {}, Ll = function(e) {
  return function(a) {
    for (var n = this, i = [], o = 0; o < n.length; o++) {
      var s = n[o];
      if (s.isNode()) {
        for (var u = !1, l = s.connectedEdges(), c = 0; c < l.length; c++) {
          var f = l[c], v = f.source(), h = f.target();
          if (e.noIncomingEdges && h === s && v !== s || e.noOutgoingEdges && v === s && h !== s) {
            u = !0;
            break;
          }
        }
        u || i.push(s);
      }
    }
    return this.spawn(i, !0).filter(a);
  };
}, Ol = function(e) {
  return function(r) {
    for (var a = this, n = [], i = 0; i < a.length; i++) {
      var o = a[i];
      if (o.isNode())
        for (var s = o.connectedEdges(), u = 0; u < s.length; u++) {
          var l = s[u], c = l.source(), f = l.target();
          e.outgoing && c === o ? (n.push(l), n.push(f)) : e.incoming && f === o && (n.push(l), n.push(c));
        }
    }
    return this.spawn(n, !0).filter(r);
  };
}, Nl = function(e) {
  return function(r) {
    for (var a = this, n = [], i = {}; ; ) {
      var o = e.outgoing ? a.outgoers() : a.incomers();
      if (o.length === 0)
        break;
      for (var s = !1, u = 0; u < o.length; u++) {
        var l = o[u], c = l.id();
        i[c] || (i[c] = !0, n.push(l), s = !0);
      }
      if (!s)
        break;
      a = o;
    }
    return this.spawn(n, !0).filter(r);
  };
};
ht.clearTraversalCache = function() {
  for (var t = 0; t < this.length; t++)
    this[t]._private.traversalCache = null;
};
ye(ht, {
  // get the root nodes in the DAG
  roots: Ll({
    noIncomingEdges: !0
  }),
  // get the leaf nodes in the DAG
  leaves: Ll({
    noOutgoingEdges: !0
  }),
  // normally called children in graph theory
  // these nodes =edges=> outgoing nodes
  outgoers: Bt(Ol({
    outgoing: !0
  }), "outgoers"),
  // aka DAG descendants
  successors: Nl({
    outgoing: !0
  }),
  // normally called parents in graph theory
  // these nodes <=edges= incoming nodes
  incomers: Bt(Ol({
    incoming: !0
  }), "incomers"),
  // aka DAG ancestors
  predecessors: Nl({})
});
ye(ht, {
  neighborhood: Bt(function(t) {
    for (var e = [], r = this.nodes(), a = 0; a < r.length; a++)
      for (var n = r[a], i = n.connectedEdges(), o = 0; o < i.length; o++) {
        var s = i[o], u = s.source(), l = s.target(), c = n === u ? l : u;
        c.length > 0 && e.push(c[0]), e.push(s[0]);
      }
    return this.spawn(e, !0).filter(t);
  }, "neighborhood"),
  closedNeighborhood: function(e) {
    return this.neighborhood().add(this).filter(e);
  },
  openNeighborhood: function(e) {
    return this.neighborhood(e);
  }
});
ht.neighbourhood = ht.neighborhood;
ht.closedNeighbourhood = ht.closedNeighborhood;
ht.openNeighbourhood = ht.openNeighborhood;
ye(ht, {
  source: Bt(function(e) {
    var r = this[0], a;
    return r && (a = r._private.source || r.cy().collection()), a && e ? a.filter(e) : a;
  }, "source"),
  target: Bt(function(e) {
    var r = this[0], a;
    return r && (a = r._private.target || r.cy().collection()), a && e ? a.filter(e) : a;
  }, "target"),
  sources: zl({
    attr: "source"
  }),
  targets: zl({
    attr: "target"
  })
});
function zl(t) {
  return function(r) {
    for (var a = [], n = 0; n < this.length; n++) {
      var i = this[n], o = i._private[t.attr];
      o && a.push(o);
    }
    return this.spawn(a, !0).filter(r);
  };
}
ye(ht, {
  edgesWith: Bt(Fl(), "edgesWith"),
  edgesTo: Bt(Fl({
    thisIsSrc: !0
  }), "edgesTo")
});
function Fl(t) {
  return function(r) {
    var a = [], n = this._private.cy, i = t || {};
    he(r) && (r = n.$(r));
    for (var o = 0; o < r.length; o++)
      for (var s = r[o]._private.edges, u = 0; u < s.length; u++) {
        var l = s[u], c = l._private.data, f = this.hasElementWithId(c.source) && r.hasElementWithId(c.target), v = r.hasElementWithId(c.source) && this.hasElementWithId(c.target), h = f || v;
        h && ((i.thisIsSrc || i.thisIsTgt) && (i.thisIsSrc && !f || i.thisIsTgt && !v) || a.push(l));
      }
    return this.spawn(a, !0);
  };
}
ye(ht, {
  connectedEdges: Bt(function(t) {
    for (var e = [], r = this, a = 0; a < r.length; a++) {
      var n = r[a];
      if (n.isNode())
        for (var i = n._private.edges, o = 0; o < i.length; o++) {
          var s = i[o];
          e.push(s);
        }
    }
    return this.spawn(e, !0).filter(t);
  }, "connectedEdges"),
  connectedNodes: Bt(function(t) {
    for (var e = [], r = this, a = 0; a < r.length; a++) {
      var n = r[a];
      n.isEdge() && (e.push(n.source()[0]), e.push(n.target()[0]));
    }
    return this.spawn(e, !0).filter(t);
  }, "connectedNodes"),
  parallelEdges: Bt(ql(), "parallelEdges"),
  codirectedEdges: Bt(ql({
    codirected: !0
  }), "codirectedEdges")
});
function ql(t) {
  var e = {
    codirected: !1
  };
  return t = ye({}, e, t), function(a) {
    for (var n = [], i = this.edges(), o = t, s = 0; s < i.length; s++)
      for (var u = i[s], l = u._private, c = l.source, f = c._private.data.id, v = l.data.target, h = c._private.edges, d = 0; d < h.length; d++) {
        var y = h[d], g = y._private.data, p = g.target, m = g.source, b = p === v && m === f, w = f === p && v === m;
        (o.codirected && b || !o.codirected && (b || w)) && n.push(y);
      }
    return this.spawn(n, !0).filter(a);
  };
}
ye(ht, {
  components: function(e) {
    var r = this, a = r.cy(), n = a.collection(), i = e == null ? r.nodes() : e.nodes(), o = [];
    e != null && i.empty() && (i = e.sources());
    var s = function(c, f) {
      n.merge(c), i.unmerge(c), f.merge(c);
    };
    if (i.empty())
      return r.spawn();
    var u = function() {
      var c = a.collection();
      o.push(c);
      var f = i[0];
      s(f, c), r.bfs({
        directed: !1,
        roots: f,
        visit: function(h) {
          return s(h, c);
        }
      }), c.forEach(function(v) {
        v.connectedEdges().forEach(function(h) {
          r.has(h) && c.has(h.source()) && c.has(h.target()) && c.merge(h);
        });
      });
    };
    do
      u();
    while (i.length > 0);
    return o;
  },
  component: function() {
    var e = this[0];
    return e.cy().mutableElements().components(e)[0];
  }
});
ht.componentsOf = ht.components;
var ct = function(e, r) {
  var a = arguments.length > 2 && arguments[2] !== void 0 ? arguments[2] : !1, n = arguments.length > 3 && arguments[3] !== void 0 ? arguments[3] : !1;
  if (e === void 0) {
    $e("A collection must have a reference to the core");
    return;
  }
  var i = new Zt(), o = !1;
  if (!r)
    r = [];
  else if (r.length > 0 && Me(r[0]) && !qa(r[0])) {
    o = !0;
    for (var s = [], u = new oa(), l = 0, c = r.length; l < c; l++) {
      var f = r[l];
      f.data == null && (f.data = {});
      var v = f.data;
      if (v.id == null)
        v.id = Oc();
      else if (e.hasElementWithId(v.id) || u.has(v.id))
        continue;
      var h = new Hn(e, f, !1);
      s.push(h), u.add(v.id);
    }
    r = s;
  }
  this.length = 0;
  for (var d = 0, y = r.length; d < y; d++) {
    var g = r[d][0];
    if (g != null) {
      var p = g._private.data.id;
      (!a || !i.has(p)) && (a && i.set(p, {
        index: this.length,
        ele: g
      }), this[this.length] = g, this.length++);
    }
  }
  this._private = {
    eles: this,
    cy: e,
    get map() {
      return this.lazyMap == null && this.rebuildMap(), this.lazyMap;
    },
    set map(m) {
      this.lazyMap = m;
    },
    rebuildMap: function() {
      for (var b = this.lazyMap = new Zt(), w = this.eles, C = 0; C < w.length; C++) {
        var E = w[C];
        b.set(E.id(), {
          index: C,
          ele: E
        });
      }
    }
  }, a && (this._private.map = i), o && !n && this.restore();
}, Ge = Hn.prototype = ct.prototype = Object.create(Array.prototype);
Ge.instanceString = function() {
  return "collection";
};
Ge.spawn = function(t, e) {
  return new ct(this.cy(), t, e);
};
Ge.spawnSelf = function() {
  return this.spawn(this);
};
Ge.cy = function() {
  return this._private.cy;
};
Ge.renderer = function() {
  return this._private.cy.renderer();
};
Ge.element = function() {
  return this[0];
};
Ge.collection = function() {
  return xc(this) ? this : new ct(this._private.cy, [this]);
};
Ge.unique = function() {
  return new ct(this._private.cy, this, !0);
};
Ge.hasElementWithId = function(t) {
  return t = "" + t, this._private.map.has(t);
};
Ge.getElementById = function(t) {
  t = "" + t;
  var e = this._private.cy, r = this._private.map.get(t);
  return r ? r.ele : new ct(e);
};
Ge.$id = Ge.getElementById;
Ge.poolIndex = function() {
  var t = this._private.cy, e = t._private.elements, r = this[0]._private.data.id;
  return e._private.map.get(r).index;
};
Ge.indexOf = function(t) {
  var e = t[0]._private.data.id;
  return this._private.map.get(e).index;
};
Ge.indexOfId = function(t) {
  return t = "" + t, this._private.map.get(t).index;
};
Ge.json = function(t) {
  var e = this.element(), r = this.cy();
  if (e == null && t)
    return this;
  if (e != null) {
    var a = e._private;
    if (Me(t)) {
      if (r.startBatch(), t.data) {
        e.data(t.data);
        var n = a.data;
        if (e.isEdge()) {
          var i = !1, o = {}, s = t.data.source, u = t.data.target;
          s != null && s != n.source && (o.source = "" + s, i = !0), u != null && u != n.target && (o.target = "" + u, i = !0), i && (e = e.move(o));
        } else {
          var l = "parent" in t.data, c = t.data.parent;
          l && (c != null || n.parent != null) && c != n.parent && (c === void 0 && (c = null), c != null && (c = "" + c), e = e.move({
            parent: c
          }));
        }
      }
      t.position && e.position(t.position);
      var f = function(y, g, p) {
        var m = t[y];
        m != null && m !== a[y] && (m ? e[g]() : e[p]());
      };
      return f("removed", "remove", "restore"), f("selected", "select", "unselect"), f("selectable", "selectify", "unselectify"), f("locked", "lock", "unlock"), f("grabbable", "grabify", "ungrabify"), f("pannable", "panify", "unpanify"), t.classes != null && e.classes(t.classes), r.endBatch(), this;
    } else if (t === void 0) {
      var v = {
        data: $t(a.data),
        position: $t(a.position),
        group: a.group,
        removed: a.removed,
        selected: a.selected,
        selectable: a.selectable,
        locked: a.locked,
        grabbable: a.grabbable,
        pannable: a.pannable,
        classes: null
      };
      v.classes = "";
      var h = 0;
      return a.classes.forEach(function(d) {
        return v.classes += h++ === 0 ? d : " " + d;
      }), v;
    }
  }
};
Ge.jsons = function() {
  for (var t = [], e = 0; e < this.length; e++) {
    var r = this[e], a = r.json();
    t.push(a);
  }
  return t;
};
Ge.clone = function() {
  for (var t = this.cy(), e = [], r = 0; r < this.length; r++) {
    var a = this[r], n = a.json(), i = new Hn(t, n, !1);
    e.push(i);
  }
  return new ct(t, e);
};
Ge.copy = Ge.clone;
Ge.restore = function() {
  for (var t = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : !0, e = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : !0, r = this, a = r.cy(), n = a._private, i = [], o = [], s, u = 0, l = r.length; u < l; u++) {
    var c = r[u];
    e && !c.removed() || (c.isNode() ? i.push(c) : o.push(c));
  }
  s = i.concat(o);
  var f, v = function() {
    s.splice(f, 1), f--;
  };
  for (f = 0; f < s.length; f++) {
    var h = s[f], d = h._private, y = d.data;
    if (h.clearTraversalCache(), !(!e && !d.removed)) {
      if (y.id === void 0)
        y.id = Oc();
      else if (ne(y.id))
        y.id = "" + y.id;
      else if (cr(y.id) || !he(y.id)) {
        $e("Can not create element with invalid string ID `" + y.id + "`"), v();
        continue;
      } else if (a.hasElementWithId(y.id)) {
        $e("Can not create second element with ID `" + y.id + "`"), v();
        continue;
      }
    }
    var g = y.id;
    if (h.isNode()) {
      var p = d.position;
      p.x == null && (p.x = 0), p.y == null && (p.y = 0);
    }
    if (h.isEdge()) {
      for (var m = h, b = ["source", "target"], w = b.length, C = !1, E = 0; E < w; E++) {
        var x = b[E], T = y[x];
        ne(T) && (T = y[x] = "" + y[x]), T == null || T === "" ? ($e("Can not create edge `" + g + "` with unspecified " + x), C = !0) : a.hasElementWithId(T) || ($e("Can not create edge `" + g + "` with nonexistent " + x + " `" + T + "`"), C = !0);
      }
      if (C) {
        v();
        continue;
      }
      var k = a.getElementById(y.source), D = a.getElementById(y.target);
      k.same(D) ? k._private.edges.push(m) : (k._private.edges.push(m), D._private.edges.push(m)), m._private.source = k, m._private.target = D;
    }
    d.map = new Zt(), d.map.set(g, {
      ele: h,
      index: 0
    }), d.removed = !1, e && a.addToPool(h);
  }
  for (var P = 0; P < i.length; P++) {
    var A = i[P], B = A._private.data;
    ne(B.parent) && (B.parent = "" + B.parent);
    var R = B.parent, I = R != null;
    if (I || A._private.parent) {
      var O = A._private.parent ? a.collection().merge(A._private.parent) : a.getElementById(R);
      if (O.empty())
        B.parent = void 0;
      else if (O[0].removed())
        ze("Node added with missing parent, reference to parent removed"), B.parent = void 0, A._private.parent = null;
      else {
        for (var M = !1, N = O; !N.empty(); ) {
          if (A.same(N)) {
            M = !0, B.parent = void 0;
            break;
          }
          N = N.parent();
        }
        M || (O[0]._private.children.push(A), A._private.parent = O[0], n.hasCompoundNodes = !0);
      }
    }
  }
  if (s.length > 0) {
    for (var H = s.length === r.length ? r : new ct(a, s), G = 0; G < H.length; G++) {
      var F = H[G];
      F.isNode() || (F.parallelEdges().clearTraversalCache(), F.source().clearTraversalCache(), F.target().clearTraversalCache());
    }
    var _;
    n.hasCompoundNodes ? _ = a.collection().merge(H).merge(H.connectedNodes()).merge(H.parent()) : _ = H, _.dirtyCompoundBoundsCache().dirtyBoundingBoxCache().updateStyle(t), t ? H.emitAndNotify("add") : e && H.emit("add");
  }
  return r;
};
Ge.removed = function() {
  var t = this[0];
  return t && t._private.removed;
};
Ge.inside = function() {
  var t = this[0];
  return t && !t._private.removed;
};
Ge.remove = function() {
  var t = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : !0, e = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : !0, r = this, a = [], n = {}, i = r._private.cy;
  function o(R) {
    for (var I = R._private.edges, O = 0; O < I.length; O++)
      u(I[O]);
  }
  function s(R) {
    for (var I = R._private.children, O = 0; O < I.length; O++)
      u(I[O]);
  }
  function u(R) {
    var I = n[R.id()];
    e && R.removed() || I || (n[R.id()] = !0, R.isNode() ? (a.push(R), o(R), s(R)) : a.unshift(R));
  }
  for (var l = 0, c = r.length; l < c; l++) {
    var f = r[l];
    u(f);
  }
  function v(R, I) {
    var O = R._private.edges;
    fr(O, I), R.clearTraversalCache();
  }
  function h(R) {
    R.clearTraversalCache();
  }
  var d = [];
  d.ids = {};
  function y(R, I) {
    I = I[0], R = R[0];
    var O = R._private.children, M = R.id();
    fr(O, I), I._private.parent = null, d.ids[M] || (d.ids[M] = !0, d.push(R));
  }
  r.dirtyCompoundBoundsCache(), e && i.removeFromPool(a);
  for (var g = 0; g < a.length; g++) {
    var p = a[g];
    if (p.isEdge()) {
      var m = p.source()[0], b = p.target()[0];
      v(m, p), v(b, p);
      for (var w = p.parallelEdges(), C = 0; C < w.length; C++) {
        var E = w[C];
        h(E), E.isBundledBezier() && E.dirtyBoundingBoxCache();
      }
    } else {
      var x = p.parent();
      x.length !== 0 && y(x, p);
    }
    e && (p._private.removed = !0);
  }
  var T = i._private.elements;
  i._private.hasCompoundNodes = !1;
  for (var k = 0; k < T.length; k++) {
    var D = T[k];
    if (D.isParent()) {
      i._private.hasCompoundNodes = !0;
      break;
    }
  }
  var P = new ct(this.cy(), a);
  P.size() > 0 && (t ? P.emitAndNotify("remove") : e && P.emit("remove"));
  for (var A = 0; A < d.length; A++) {
    var B = d[A];
    (!e || !B.removed()) && B.updateStyle();
  }
  return P;
};
Ge.move = function(t) {
  var e = this._private.cy, r = this, a = !1, n = !1, i = function(d) {
    return d == null ? d : "" + d;
  };
  if (t.source !== void 0 || t.target !== void 0) {
    var o = i(t.source), s = i(t.target), u = o != null && e.hasElementWithId(o), l = s != null && e.hasElementWithId(s);
    (u || l) && (e.batch(function() {
      r.remove(a, n), r.emitAndNotify("moveout");
      for (var h = 0; h < r.length; h++) {
        var d = r[h], y = d._private.data;
        d.isEdge() && (u && (y.source = o), l && (y.target = s));
      }
      r.restore(a, n);
    }), r.emitAndNotify("move"));
  } else if (t.parent !== void 0) {
    var c = i(t.parent), f = c === null || e.hasElementWithId(c);
    if (f) {
      var v = c === null ? void 0 : c;
      e.batch(function() {
        var h = r.remove(a, n);
        h.emitAndNotify("moveout");
        for (var d = 0; d < r.length; d++) {
          var y = r[d], g = y._private.data;
          y.isNode() && (g.parent = v);
        }
        h.restore(a, n);
      }), r.emitAndNotify("move");
    }
  }
  return this;
};
[Kc, Gp, pn, ur, aa, ny, Zn, xy, bf, wf, ky, Ln, yn, lt, lr, ht].forEach(function(t) {
  ye(Ge, t);
});
var Iy = {
  add: function(e) {
    var r, a = this;
    if (kt(e)) {
      var n = e;
      if (n._private.cy === a)
        r = n.restore();
      else {
        for (var i = [], o = 0; o < n.length; o++) {
          var s = n[o];
          i.push(s.json());
        }
        r = new ct(a, i);
      }
    } else if (Ve(e)) {
      var u = e;
      r = new ct(a, u);
    } else if (Me(e) && (Ve(e.nodes) || Ve(e.edges))) {
      for (var l = e, c = [], f = ["nodes", "edges"], v = 0, h = f.length; v < h; v++) {
        var d = f[v], y = l[d];
        if (Ve(y))
          for (var g = 0, p = y.length; g < p; g++) {
            var m = ye({
              group: d
            }, y[g]);
            c.push(m);
          }
      }
      r = new ct(a, c);
    } else {
      var b = e;
      r = new Hn(a, b).collection();
    }
    return r;
  },
  remove: function(e) {
    if (!kt(e)) {
      if (he(e)) {
        var r = e;
        e = this.$(r);
      }
    }
    return e.remove();
  }
};
/*! Bezier curve function generator. Copyright Gaetan Renaudeau. MIT License: http://en.wikipedia.org/wiki/MIT_License */
function Ly(t, e, r, a) {
  var n = 4, i = 1e-3, o = 1e-7, s = 10, u = 11, l = 1 / (u - 1), c = typeof Float32Array < "u";
  if (arguments.length !== 4)
    return !1;
  for (var f = 0; f < 4; ++f)
    if (typeof arguments[f] != "number" || isNaN(arguments[f]) || !isFinite(arguments[f]))
      return !1;
  t = Math.min(t, 1), r = Math.min(r, 1), t = Math.max(t, 0), r = Math.max(r, 0);
  var v = c ? new Float32Array(u) : new Array(u);
  function h(D, P) {
    return 1 - 3 * P + 3 * D;
  }
  function d(D, P) {
    return 3 * P - 6 * D;
  }
  function y(D) {
    return 3 * D;
  }
  function g(D, P, A) {
    return ((h(P, A) * D + d(P, A)) * D + y(P)) * D;
  }
  function p(D, P, A) {
    return 3 * h(P, A) * D * D + 2 * d(P, A) * D + y(P);
  }
  function m(D, P) {
    for (var A = 0; A < n; ++A) {
      var B = p(P, t, r);
      if (B === 0)
        return P;
      var R = g(P, t, r) - D;
      P -= R / B;
    }
    return P;
  }
  function b() {
    for (var D = 0; D < u; ++D)
      v[D] = g(D * l, t, r);
  }
  function w(D, P, A) {
    var B, R, I = 0;
    do
      R = P + (A - P) / 2, B = g(R, t, r) - D, B > 0 ? A = R : P = R;
    while (Math.abs(B) > o && ++I < s);
    return R;
  }
  function C(D) {
    for (var P = 0, A = 1, B = u - 1; A !== B && v[A] <= D; ++A)
      P += l;
    --A;
    var R = (D - v[A]) / (v[A + 1] - v[A]), I = P + R * l, O = p(I, t, r);
    return O >= i ? m(D, I) : O === 0 ? I : w(D, P, P + l);
  }
  var E = !1;
  function x() {
    E = !0, (t !== e || r !== a) && b();
  }
  var T = function(P) {
    return E || x(), t === e && r === a ? P : P === 0 ? 0 : P === 1 ? 1 : g(C(P), e, a);
  };
  T.getControlPoints = function() {
    return [{
      x: t,
      y: e
    }, {
      x: r,
      y: a
    }];
  };
  var k = "generateBezier(" + [t, e, r, a] + ")";
  return T.toString = function() {
    return k;
  }, T;
}
/*! Runge-Kutta spring physics function generator. Adapted from Framer.js, copyright Koen Bok. MIT License: http://en.wikipedia.org/wiki/MIT_License */
var Oy = /* @__PURE__ */ (function() {
  function t(a) {
    return -a.tension * a.x - a.friction * a.v;
  }
  function e(a, n, i) {
    var o = {
      x: a.x + i.dx * n,
      v: a.v + i.dv * n,
      tension: a.tension,
      friction: a.friction
    };
    return {
      dx: o.v,
      dv: t(o)
    };
  }
  function r(a, n) {
    var i = {
      dx: a.v,
      dv: t(a)
    }, o = e(a, n * 0.5, i), s = e(a, n * 0.5, o), u = e(a, n, s), l = 1 / 6 * (i.dx + 2 * (o.dx + s.dx) + u.dx), c = 1 / 6 * (i.dv + 2 * (o.dv + s.dv) + u.dv);
    return a.x = a.x + l * n, a.v = a.v + c * n, a;
  }
  return function a(n, i, o) {
    var s = {
      x: -1,
      v: 0,
      tension: null,
      friction: null
    }, u = [0], l = 0, c = 1 / 1e4, f = 16 / 1e3, v, h, d;
    for (n = parseFloat(n) || 500, i = parseFloat(i) || 20, o = o || null, s.tension = n, s.friction = i, v = o !== null, v ? (l = a(n, i), h = l / o * f) : h = f; d = r(d || s, h), u.push(1 + d.x), l += 16, Math.abs(d.x) > c && Math.abs(d.v) > c; )
      ;
    return v ? function(y) {
      return u[y * (u.length - 1) | 0];
    } : l;
  };
})(), _e = function(e, r, a, n) {
  var i = Ly(e, r, a, n);
  return function(o, s, u) {
    return o + (s - o) * i(u);
  };
}, mn = {
  linear: function(e, r, a) {
    return e + (r - e) * a;
  },
  // default easings
  ease: _e(0.25, 0.1, 0.25, 1),
  "ease-in": _e(0.42, 0, 1, 1),
  "ease-out": _e(0, 0, 0.58, 1),
  "ease-in-out": _e(0.42, 0, 0.58, 1),
  // sine
  "ease-in-sine": _e(0.47, 0, 0.745, 0.715),
  "ease-out-sine": _e(0.39, 0.575, 0.565, 1),
  "ease-in-out-sine": _e(0.445, 0.05, 0.55, 0.95),
  // quad
  "ease-in-quad": _e(0.55, 0.085, 0.68, 0.53),
  "ease-out-quad": _e(0.25, 0.46, 0.45, 0.94),
  "ease-in-out-quad": _e(0.455, 0.03, 0.515, 0.955),
  // cubic
  "ease-in-cubic": _e(0.55, 0.055, 0.675, 0.19),
  "ease-out-cubic": _e(0.215, 0.61, 0.355, 1),
  "ease-in-out-cubic": _e(0.645, 0.045, 0.355, 1),
  // quart
  "ease-in-quart": _e(0.895, 0.03, 0.685, 0.22),
  "ease-out-quart": _e(0.165, 0.84, 0.44, 1),
  "ease-in-out-quart": _e(0.77, 0, 0.175, 1),
  // quint
  "ease-in-quint": _e(0.755, 0.05, 0.855, 0.06),
  "ease-out-quint": _e(0.23, 1, 0.32, 1),
  "ease-in-out-quint": _e(0.86, 0, 0.07, 1),
  // expo
  "ease-in-expo": _e(0.95, 0.05, 0.795, 0.035),
  "ease-out-expo": _e(0.19, 1, 0.22, 1),
  "ease-in-out-expo": _e(1, 0, 0, 1),
  // circ
  "ease-in-circ": _e(0.6, 0.04, 0.98, 0.335),
  "ease-out-circ": _e(0.075, 0.82, 0.165, 1),
  "ease-in-out-circ": _e(0.785, 0.135, 0.15, 0.86),
  // user param easings...
  spring: function(e, r, a) {
    if (a === 0)
      return mn.linear;
    var n = Oy(e, r, a);
    return function(i, o, s) {
      return i + (o - i) * n(s);
    };
  },
  "cubic-bezier": _e
};
function Vl(t, e, r, a, n) {
  if (a === 1 || e === r)
    return r;
  var i = n(e, r, a);
  return t == null || ((t.roundValue || t.color) && (i = Math.round(i)), t.min !== void 0 && (i = Math.max(i, t.min)), t.max !== void 0 && (i = Math.min(i, t.max))), i;
}
function _l(t, e) {
  return t.pfValue != null || t.value != null ? t.pfValue != null && (e == null || e.type.units !== "%") ? t.pfValue : t.value : t;
}
function Vr(t, e, r, a, n) {
  var i = n != null ? n.type : null;
  r < 0 ? r = 0 : r > 1 && (r = 1);
  var o = _l(t, n), s = _l(e, n);
  if (ne(o) && ne(s))
    return Vl(i, o, s, r, a);
  if (Ve(o) && Ve(s)) {
    for (var u = [], l = 0; l < s.length; l++) {
      var c = o[l], f = s[l];
      if (c != null && f != null) {
        var v = Vl(i, c, f, r, a);
        u.push(v);
      } else
        u.push(f);
    }
    return u;
  }
}
function Ny(t, e, r, a) {
  var n = !a, i = t._private, o = e._private, s = o.easing, u = o.startTime, l = a ? t : t.cy(), c = l.style();
  if (!o.easingImpl)
    if (s == null)
      o.easingImpl = mn.linear;
    else {
      var f;
      if (he(s)) {
        var v = c.parse("transition-timing-function", s);
        f = v.value;
      } else
        f = s;
      var h, d;
      he(f) ? (h = f, d = []) : (h = f[1], d = f.slice(2).map(function(H) {
        return +H;
      })), d.length > 0 ? (h === "spring" && d.push(o.duration), o.easingImpl = mn[h].apply(null, d)) : o.easingImpl = mn[h];
    }
  var y = o.easingImpl, g;
  if (o.duration === 0 ? g = 1 : g = (r - u) / o.duration, o.applying && (g = o.progress), g < 0 ? g = 0 : g > 1 && (g = 1), o.delay == null) {
    var p = o.startPosition, m = o.position;
    if (m && n && !t.locked()) {
      var b = {};
      ma(p.x, m.x) && (b.x = Vr(p.x, m.x, g, y)), ma(p.y, m.y) && (b.y = Vr(p.y, m.y, g, y)), t.position(b);
    }
    var w = o.startPan, C = o.pan, E = i.pan, x = C != null && a;
    x && (ma(w.x, C.x) && (E.x = Vr(w.x, C.x, g, y)), ma(w.y, C.y) && (E.y = Vr(w.y, C.y, g, y)), t.emit("pan"));
    var T = o.startZoom, k = o.zoom, D = k != null && a;
    D && (ma(T, k) && (i.zoom = Ra(i.minZoom, Vr(T, k, g, y), i.maxZoom)), t.emit("zoom")), (x || D) && t.emit("viewport");
    var P = o.style;
    if (P && P.length > 0 && n) {
      for (var A = 0; A < P.length; A++) {
        var B = P[A], R = B.name, I = B, O = o.startStyle[R], M = c.properties[O.name], N = Vr(O, I, g, y, M);
        c.overrideBypass(t, R, N);
      }
      t.emit("style");
    }
  }
  return o.progress = g, g;
}
function ma(t, e) {
  return t == null || e == null ? !1 : ne(t) && ne(e) ? !0 : !!(t && e);
}
function zy(t, e, r, a) {
  var n = e._private;
  n.started = !0, n.startTime = r - n.progress * n.duration;
}
function Gl(t, e) {
  var r = e._private.aniEles, a = [];
  function n(c, f) {
    var v = c._private, h = v.animation.current, d = v.animation.queue, y = !1;
    if (h.length === 0) {
      var g = d.shift();
      g && h.push(g);
    }
    for (var p = function(E) {
      for (var x = E.length - 1; x >= 0; x--) {
        var T = E[x];
        T();
      }
      E.splice(0, E.length);
    }, m = h.length - 1; m >= 0; m--) {
      var b = h[m], w = b._private;
      if (w.stopped) {
        h.splice(m, 1), w.hooked = !1, w.playing = !1, w.started = !1, p(w.frames);
        continue;
      }
      !w.playing && !w.applying || (w.playing && w.applying && (w.applying = !1), w.started || zy(c, b, t), Ny(c, b, t, f), w.applying && (w.applying = !1), p(w.frames), w.step != null && w.step(t), b.completed() && (h.splice(m, 1), w.hooked = !1, w.playing = !1, w.started = !1, p(w.completes)), y = !0);
    }
    return !f && h.length === 0 && d.length === 0 && a.push(c), y;
  }
  for (var i = !1, o = 0; o < r.length; o++) {
    var s = r[o], u = n(s);
    i = i || u;
  }
  var l = n(e, !0);
  (i || l) && (r.length > 0 ? e.notify("draw", r) : e.notify("draw")), r.unmerge(a), e.emit("step");
}
var Fy = {
  // pull in animation functions
  animate: Ne.animate(),
  animation: Ne.animation(),
  animated: Ne.animated(),
  clearQueue: Ne.clearQueue(),
  delay: Ne.delay(),
  delayAnimation: Ne.delayAnimation(),
  stop: Ne.stop(),
  addToAnimationPool: function(e) {
    var r = this;
    r.styleEnabled() && r._private.aniEles.merge(e);
  },
  stopAnimationLoop: function() {
    this._private.animationsRunning = !1;
  },
  startAnimationLoop: function() {
    var e = this;
    if (e._private.animationsRunning = !0, !e.styleEnabled())
      return;
    function r() {
      e._private.animationsRunning && An(function(i) {
        Gl(i, e), r();
      });
    }
    var a = e.renderer();
    a && a.beforeRender ? a.beforeRender(function(i, o) {
      Gl(o, e);
    }, a.beforeRenderPriorities.animations) : r();
  }
}, qy = {
  qualifierCompare: function(e, r) {
    return e == null || r == null ? e == null && r == null : e.sameText(r);
  },
  eventMatches: function(e, r, a) {
    var n = r.qualifier;
    return n != null ? e !== a.target && qa(a.target) && n.matches(a.target) : !0;
  },
  addEventFields: function(e, r) {
    r.cy = e, r.target = e;
  },
  callbackContext: function(e, r, a) {
    return r.qualifier != null ? a.target : e;
  }
}, ln = function(e) {
  return he(e) ? new dr(e) : e;
}, Cf = {
  createEmitter: function() {
    var e = this._private;
    return e.emitter || (e.emitter = new Qn(qy, this)), this;
  },
  emitter: function() {
    return this._private.emitter;
  },
  on: function(e, r, a) {
    return this.emitter().on(e, ln(r), a), this;
  },
  removeListener: function(e, r, a) {
    return this.emitter().removeListener(e, ln(r), a), this;
  },
  removeAllListeners: function() {
    return this.emitter().removeAllListeners(), this;
  },
  one: function(e, r, a) {
    return this.emitter().one(e, ln(r), a), this;
  },
  once: function(e, r, a) {
    return this.emitter().one(e, ln(r), a), this;
  },
  emit: function(e, r) {
    return this.emitter().emit(e, r), this;
  },
  emitAndNotify: function(e, r) {
    return this.emit(e), this.notify(e, r), this;
  }
};
Ne.eventAliasesOn(Cf);
var Zo = {
  png: function(e) {
    var r = this._private.renderer;
    return e = e || {}, r.png(e);
  },
  jpg: function(e) {
    var r = this._private.renderer;
    return e = e || {}, e.bg = e.bg || "#fff", r.jpg(e);
  }
};
Zo.jpeg = Zo.jpg;
var bn = {
  layout: function(e) {
    var r = this;
    if (e == null) {
      $e("Layout options must be specified to make a layout");
      return;
    }
    if (e.name == null) {
      $e("A `name` must be specified to make a layout");
      return;
    }
    var a = e.name, n = r.extension("layout", a);
    if (n == null) {
      $e("No such layout `" + a + "` found.  Did you forget to import it and `cytoscape.use()` it?");
      return;
    }
    var i;
    he(e.eles) ? i = r.$(e.eles) : i = e.eles != null ? e.eles : r.$();
    var o = new n(ye({}, e, {
      cy: r,
      eles: i
    }));
    return o;
  }
};
bn.createLayout = bn.makeLayout = bn.layout;
var Vy = {
  notify: function(e, r) {
    var a = this._private;
    if (this.batching()) {
      a.batchNotifications = a.batchNotifications || {};
      var n = a.batchNotifications[e] = a.batchNotifications[e] || this.collection();
      r != null && n.merge(r);
      return;
    }
    if (a.notificationsEnabled) {
      var i = this.renderer();
      this.destroyed() || !i || i.notify(e, r);
    }
  },
  notifications: function(e) {
    var r = this._private;
    return e === void 0 ? r.notificationsEnabled : (r.notificationsEnabled = !!e, this);
  },
  noNotifications: function(e) {
    this.notifications(!1), e(), this.notifications(!0);
  },
  batching: function() {
    return this._private.batchCount > 0;
  },
  startBatch: function() {
    var e = this._private;
    return e.batchCount == null && (e.batchCount = 0), e.batchCount === 0 && (e.batchStyleEles = this.collection(), e.batchNotifications = {}), e.batchCount++, this;
  },
  endBatch: function() {
    var e = this._private;
    if (e.batchCount === 0)
      return this;
    if (e.batchCount--, e.batchCount === 0) {
      e.batchStyleEles.updateStyle();
      var r = this.renderer();
      Object.keys(e.batchNotifications).forEach(function(a) {
        var n = e.batchNotifications[a];
        n.empty() ? r.notify(a) : r.notify(a, n);
      });
    }
    return this;
  },
  batch: function(e) {
    return this.startBatch(), e(), this.endBatch(), this;
  },
  // for backwards compatibility
  batchData: function(e) {
    var r = this;
    return this.batch(function() {
      for (var a = Object.keys(e), n = 0; n < a.length; n++) {
        var i = a[n], o = e[i], s = r.getElementById(i);
        s.data(o);
      }
    });
  }
}, _y = ft({
  hideEdgesOnViewport: !1,
  textureOnViewport: !1,
  motionBlur: !1,
  motionBlurOpacity: 0.05,
  pixelRatio: void 0,
  desktopTapThreshold: 4,
  touchTapThreshold: 8,
  wheelSensitivity: 1,
  debug: !1,
  showFps: !1,
  // webgl options
  webgl: !1,
  webglDebug: !1,
  webglDebugShowAtlases: !1,
  // defaults good for mobile
  webglTexSize: 2048,
  webglTexRows: 36,
  webglTexRowsNodes: 18,
  webglBatchSize: 2048,
  webglTexPerBatch: 14,
  webglBgColor: [255, 255, 255]
}), Qo = {
  renderTo: function(e, r, a, n) {
    var i = this._private.renderer;
    return i.renderTo(e, r, a, n), this;
  },
  renderer: function() {
    return this._private.renderer;
  },
  forceRender: function() {
    return this.notify("draw"), this;
  },
  resize: function() {
    return this.invalidateSize(), this.emitAndNotify("resize"), this;
  },
  initRenderer: function(e) {
    var r = this, a = r.extension("renderer", e.name);
    if (a == null) {
      $e("Can not initialise: No such renderer `".concat(e.name, "` found. Did you forget to import it and `cytoscape.use()` it?"));
      return;
    }
    e.wheelSensitivity !== void 0 && ze("You have set a custom wheel sensitivity.  This will make your app zoom unnaturally when using mainstream mice.  You should change this value from the default only if you can guarantee that all your users will use the same hardware and OS configuration as your current machine.");
    var n = _y(e);
    n.cy = r, r._private.renderer = new a(n), this.notify("init");
  },
  destroyRenderer: function() {
    var e = this;
    e.notify("destroy");
    var r = e.container();
    if (r)
      for (r._cyreg = null; r.childNodes.length > 0; )
        r.removeChild(r.childNodes[0]);
    e._private.renderer = null, e.mutableElements().forEach(function(a) {
      var n = a._private;
      n.rscratch = {}, n.rstyle = {}, n.animation.current = [], n.animation.queue = [];
    });
  },
  onRender: function(e) {
    return this.on("render", e);
  },
  offRender: function(e) {
    return this.off("render", e);
  }
};
Qo.invalidateDimensions = Qo.resize;
var wn = {
  // get a collection
  // - empty collection on no args
  // - collection of elements in the graph on selector arg
  // - guarantee a returned collection when elements or collection specified
  collection: function(e, r) {
    return he(e) ? this.$(e) : kt(e) ? e.collection() : Ve(e) ? (r || (r = {}), new ct(this, e, r.unique, r.removed)) : new ct(this);
  },
  nodes: function(e) {
    var r = this.$(function(a) {
      return a.isNode();
    });
    return e ? r.filter(e) : r;
  },
  edges: function(e) {
    var r = this.$(function(a) {
      return a.isEdge();
    });
    return e ? r.filter(e) : r;
  },
  // search the graph like jQuery
  $: function(e) {
    var r = this._private.elements;
    return e ? r.filter(e) : r.spawnSelf();
  },
  mutableElements: function() {
    return this._private.elements;
  }
};
wn.elements = wn.filter = wn.$;
var st = {}, Sa = "t", Gy = "f";
st.apply = function(t) {
  for (var e = this, r = e._private, a = r.cy, n = a.collection(), i = 0; i < t.length; i++) {
    var o = t[i], s = e.getContextMeta(o);
    if (!s.empty) {
      var u = e.getContextStyle(s), l = e.applyContextStyle(s, u, o);
      o._private.appliedInitStyle ? e.updateTransitions(o, l.diffProps) : o._private.appliedInitStyle = !0;
      var c = e.updateStyleHints(o);
      c && n.push(o);
    }
  }
  return n;
};
st.getPropertiesDiff = function(t, e) {
  var r = this, a = r._private.propDiffs = r._private.propDiffs || {}, n = t + "-" + e, i = a[n];
  if (i)
    return i;
  for (var o = [], s = {}, u = 0; u < r.length; u++) {
    var l = r[u], c = t[u] === Sa, f = e[u] === Sa, v = c !== f, h = l.mappedProperties.length > 0;
    if (v || f && h) {
      var d = void 0;
      v && h || v ? d = l.properties : h && (d = l.mappedProperties);
      for (var y = 0; y < d.length; y++) {
        for (var g = d[y], p = g.name, m = !1, b = u + 1; b < r.length; b++) {
          var w = r[b], C = e[b] === Sa;
          if (C && (m = w.properties[g.name] != null, m))
            break;
        }
        !s[p] && !m && (s[p] = !0, o.push(p));
      }
    }
  }
  return a[n] = o, o;
};
st.getContextMeta = function(t) {
  for (var e = this, r = "", a, n = t._private.styleCxtKey || "", i = 0; i < e.length; i++) {
    var o = e[i], s = o.selector && o.selector.matches(t);
    s ? r += Sa : r += Gy;
  }
  return a = e.getPropertiesDiff(n, r), t._private.styleCxtKey = r, {
    key: r,
    diffPropNames: a,
    empty: a.length === 0
  };
};
st.getContextStyle = function(t) {
  var e = t.key, r = this, a = this._private.contextStyles = this._private.contextStyles || {};
  if (a[e])
    return a[e];
  for (var n = {
    _private: {
      key: e
    }
  }, i = 0; i < r.length; i++) {
    var o = r[i], s = e[i] === Sa;
    if (s)
      for (var u = 0; u < o.properties.length; u++) {
        var l = o.properties[u];
        n[l.name] = l;
      }
  }
  return a[e] = n, n;
};
st.applyContextStyle = function(t, e, r) {
  for (var a = this, n = t.diffPropNames, i = {}, o = a.types, s = 0; s < n.length; s++) {
    var u = n[s], l = e[u], c = r.pstyle(u);
    if (!l)
      if (c)
        c.bypass ? l = {
          name: u,
          deleteBypassed: !0
        } : l = {
          name: u,
          delete: !0
        };
      else continue;
    if (c !== l) {
      if (l.mapped === o.fn && c != null && c.mapping != null && c.mapping.value === l.value) {
        var f = c.mapping, v = f.fnValue = l.value(r);
        if (v === f.prevFnValue)
          continue;
      }
      var h = i[u] = {
        prev: c
      };
      a.applyParsedProperty(r, l), h.next = r.pstyle(u), h.next && h.next.bypass && (h.next = h.next.bypassed);
    }
  }
  return {
    diffProps: i
  };
};
st.updateStyleHints = function(t) {
  var e = t._private, r = this, a = r.propertyGroupNames, n = r.propertyGroupKeys, i = function(W, K, re) {
    return r.getPropertiesHash(W, K, re);
  }, o = e.styleKey;
  if (t.removed())
    return !1;
  var s = e.group === "nodes", u = t._private.style;
  a = Object.keys(u);
  for (var l = 0; l < n.length; l++) {
    var c = n[l];
    e.styleKeys[c] = [kr, Kr];
  }
  for (var f = function(W, K) {
    return e.styleKeys[K][0] = Da(W, e.styleKeys[K][0]);
  }, v = function(W, K) {
    return e.styleKeys[K][1] = Aa(W, e.styleKeys[K][1]);
  }, h = function(W, K) {
    f(W, K), v(W, K);
  }, d = function(W, K) {
    for (var re = 0; re < W.length; re++) {
      var le = W.charCodeAt(re);
      f(le, K), v(le, K);
    }
  }, y = 2e9, g = function(W) {
    return -128 < W && W < 128 && Math.floor(W) !== W ? y - (W * 1024 | 0) : W;
  }, p = 0; p < a.length; p++) {
    var m = a[p], b = u[m];
    if (b != null) {
      var w = this.properties[m], C = w.type, E = w.groupKey, x = void 0;
      w.hashOverride != null ? x = w.hashOverride(t, b) : b.pfValue != null && (x = b.pfValue);
      var T = w.enums == null ? b.value : null, k = x != null, D = T != null, P = k || D, A = b.units;
      if (C.number && P && !C.multiple) {
        var B = k ? x : T;
        h(g(B), E), !k && A != null && d(A, E);
      } else
        d(b.strValue, E);
    }
  }
  for (var R = [kr, Kr], I = 0; I < n.length; I++) {
    var O = n[I], M = e.styleKeys[O];
    R[0] = Da(M[0], R[0]), R[1] = Aa(M[1], R[1]);
  }
  e.styleKey = ah(R[0], R[1]);
  var N = e.styleKeys;
  e.labelDimsKey = rr(N.labelDimensions);
  var H = i(t, ["label"], N.labelDimensions);
  if (e.labelKey = rr(H), e.labelStyleKey = rr(en(N.commonLabel, H)), !s) {
    var G = i(t, ["source-label"], N.labelDimensions);
    e.sourceLabelKey = rr(G), e.sourceLabelStyleKey = rr(en(N.commonLabel, G));
    var F = i(t, ["target-label"], N.labelDimensions);
    e.targetLabelKey = rr(F), e.targetLabelStyleKey = rr(en(N.commonLabel, F));
  }
  if (s) {
    var _ = e.styleKeys, X = _.nodeBody, J = _.nodeBorder, Y = _.nodeOutline, j = _.backgroundImage, te = _.compound, ae = _.pie, z = _.stripe, L = [X, J, Y, j, te, ae, z].filter(function(V) {
      return V != null;
    }).reduce(en, [kr, Kr]);
    e.nodeKey = rr(L), e.hasPie = ae != null && ae[0] !== kr && ae[1] !== Kr, e.hasStripe = z != null && z[0] !== kr && z[1] !== Kr;
  }
  return o !== e.styleKey;
};
st.clearStyleHints = function(t) {
  var e = t._private;
  e.styleCxtKey = "", e.styleKeys = {}, e.styleKey = null, e.labelKey = null, e.labelStyleKey = null, e.sourceLabelKey = null, e.sourceLabelStyleKey = null, e.targetLabelKey = null, e.targetLabelStyleKey = null, e.nodeKey = null, e.hasPie = null, e.hasStripe = null;
};
st.applyParsedProperty = function(t, e) {
  var r = this, a = e, n = t._private.style, i, o = r.types, s = r.properties[a.name].type, u = a.bypass, l = n[a.name], c = l && l.bypass, f = t._private, v = "mapping", h = function(X) {
    return X == null ? null : X.pfValue != null ? X.pfValue : X.value;
  }, d = function() {
    var X = h(l), J = h(a);
    r.checkTriggers(t, a.name, X, J);
  };
  if (e.name === "curve-style" && t.isEdge() && // loops must be bundled beziers
  (e.value !== "bezier" && t.isLoop() || // edges connected to compound nodes can not be haystacks
  e.value === "haystack" && (t.source().isParent() || t.target().isParent())) && (a = e = this.parse(e.name, "bezier", u)), a.delete)
    return n[a.name] = void 0, d(), !0;
  if (a.deleteBypassed)
    return l ? l.bypass ? (l.bypassed = void 0, d(), !0) : !1 : (d(), !0);
  if (a.deleteBypass)
    return l ? l.bypass ? (n[a.name] = l.bypassed, d(), !0) : !1 : (d(), !0);
  var y = function() {
    ze("Do not assign mappings to elements without corresponding data (i.e. ele `" + t.id() + "` has no mapping for property `" + a.name + "` with data field `" + a.field + "`); try a `[" + a.field + "]` selector to limit scope to elements with `" + a.field + "` defined");
  };
  switch (a.mapped) {
    // flatten the property if mapped
    case o.mapData: {
      for (var g = a.field.split("."), p = f.data, m = 0; m < g.length && p; m++) {
        var b = g[m];
        p = p[b];
      }
      if (p == null)
        return y(), !1;
      var w;
      if (ne(p)) {
        var C = a.fieldMax - a.fieldMin;
        C === 0 ? w = 0 : w = (p - a.fieldMin) / C;
      } else
        return ze("Do not use continuous mappers without specifying numeric data (i.e. `" + a.field + ": " + p + "` for `" + t.id() + "` is non-numeric)"), !1;
      if (w < 0 ? w = 0 : w > 1 && (w = 1), s.color) {
        var E = a.valueMin[0], x = a.valueMax[0], T = a.valueMin[1], k = a.valueMax[1], D = a.valueMin[2], P = a.valueMax[2], A = a.valueMin[3] == null ? 1 : a.valueMin[3], B = a.valueMax[3] == null ? 1 : a.valueMax[3], R = [Math.round(E + (x - E) * w), Math.round(T + (k - T) * w), Math.round(D + (P - D) * w), Math.round(A + (B - A) * w)];
        i = {
          // colours are simple, so just create the flat property instead of expensive string parsing
          bypass: a.bypass,
          // we're a bypass if the mapping property is a bypass
          name: a.name,
          value: R,
          strValue: "rgb(" + R[0] + ", " + R[1] + ", " + R[2] + ")"
        };
      } else if (s.number) {
        var I = a.valueMin + (a.valueMax - a.valueMin) * w;
        i = this.parse(a.name, I, a.bypass, v);
      } else
        return !1;
      if (!i)
        return y(), !1;
      i.mapping = a, a = i;
      break;
    }
    // direct mapping
    case o.data: {
      for (var O = a.field.split("."), M = f.data, N = 0; N < O.length && M; N++) {
        var H = O[N];
        M = M[H];
      }
      if (M != null && (i = this.parse(a.name, M, a.bypass, v)), !i)
        return y(), !1;
      i.mapping = a, a = i;
      break;
    }
    case o.fn: {
      var G = a.value, F = a.fnValue != null ? a.fnValue : G(t);
      if (a.prevFnValue = F, F == null)
        return ze("Custom function mappers may not return null (i.e. `" + a.name + "` for ele `" + t.id() + "` is null)"), !1;
      if (i = this.parse(a.name, F, a.bypass, v), !i)
        return ze("Custom function mappers may not return invalid values for the property type (i.e. `" + a.name + "` for ele `" + t.id() + "` is invalid)"), !1;
      i.mapping = $t(a), a = i;
      break;
    }
    case void 0:
      break;
    // just set the property
    default:
      return !1;
  }
  return u ? (c ? a.bypassed = l.bypassed : a.bypassed = l, n[a.name] = a) : c ? l.bypassed = a : n[a.name] = a, d(), !0;
};
st.cleanElements = function(t, e) {
  for (var r = 0; r < t.length; r++) {
    var a = t[r];
    if (this.clearStyleHints(a), a.dirtyCompoundBoundsCache(), a.dirtyBoundingBoxCache(), !e)
      a._private.style = {};
    else
      for (var n = a._private.style, i = Object.keys(n), o = 0; o < i.length; o++) {
        var s = i[o], u = n[s];
        u != null && (u.bypass ? u.bypassed = null : n[s] = null);
      }
  }
};
st.update = function() {
  var t = this._private.cy, e = t.mutableElements();
  e.updateStyle();
};
st.updateTransitions = function(t, e) {
  var r = this, a = t._private, n = t.pstyle("transition-property").value, i = t.pstyle("transition-duration").pfValue, o = t.pstyle("transition-delay").pfValue;
  if (n.length > 0 && i > 0) {
    for (var s = {}, u = !1, l = 0; l < n.length; l++) {
      var c = n[l], f = t.pstyle(c), v = e[c];
      if (v) {
        var h = v.prev, d = h, y = v.next != null ? v.next : f, g = !1, p = void 0, m = 1e-6;
        d && (ne(d.pfValue) && ne(y.pfValue) ? (g = y.pfValue - d.pfValue, p = d.pfValue + m * g) : ne(d.value) && ne(y.value) ? (g = y.value - d.value, p = d.value + m * g) : Ve(d.value) && Ve(y.value) && (g = d.value[0] !== y.value[0] || d.value[1] !== y.value[1] || d.value[2] !== y.value[2], p = d.strValue), g && (s[c] = y.strValue, this.applyBypass(t, c, p), u = !0));
      }
    }
    if (!u)
      return;
    a.transitioning = !0, new sa(function(b) {
      o > 0 ? t.delayAnimation(o).play().promise().then(b) : b();
    }).then(function() {
      return t.animation({
        style: s,
        duration: i,
        easing: t.pstyle("transition-timing-function").value,
        queue: !1
      }).play().promise();
    }).then(function() {
      r.removeBypasses(t, n), t.emitAndNotify("style"), a.transitioning = !1;
    });
  } else a.transitioning && (this.removeBypasses(t, n), t.emitAndNotify("style"), a.transitioning = !1);
};
st.checkTrigger = function(t, e, r, a, n, i) {
  var o = this.properties[e], s = n(o);
  t.removed() || s != null && s(r, a, t) && i(o);
};
st.checkZOrderTrigger = function(t, e, r, a) {
  var n = this;
  this.checkTrigger(t, e, r, a, function(i) {
    return i.triggersZOrder;
  }, function() {
    n._private.cy.notify("zorder", t);
  });
};
st.checkBoundsTrigger = function(t, e, r, a) {
  this.checkTrigger(t, e, r, a, function(n) {
    return n.triggersBounds;
  }, function(n) {
    t.dirtyCompoundBoundsCache(), t.dirtyBoundingBoxCache();
  });
};
st.checkConnectedEdgesBoundsTrigger = function(t, e, r, a) {
  this.checkTrigger(t, e, r, a, function(n) {
    return n.triggersBoundsOfConnectedEdges;
  }, function(n) {
    t.connectedEdges().forEach(function(i) {
      i.dirtyBoundingBoxCache();
    });
  });
};
st.checkParallelEdgesBoundsTrigger = function(t, e, r, a) {
  this.checkTrigger(t, e, r, a, function(n) {
    return n.triggersBoundsOfParallelEdges;
  }, function(n) {
    t.parallelEdges().forEach(function(i) {
      i.dirtyBoundingBoxCache();
    });
  });
};
st.checkTriggers = function(t, e, r, a) {
  t.dirtyStyleCache(), this.checkZOrderTrigger(t, e, r, a), this.checkBoundsTrigger(t, e, r, a), this.checkConnectedEdgesBoundsTrigger(t, e, r, a), this.checkParallelEdgesBoundsTrigger(t, e, r, a);
};
var Ua = {};
Ua.applyBypass = function(t, e, r, a) {
  var n = this, i = [], o = !0;
  if (e === "*" || e === "**") {
    if (r !== void 0)
      for (var s = 0; s < n.properties.length; s++) {
        var u = n.properties[s], l = u.name, c = this.parse(l, r, !0);
        c && i.push(c);
      }
  } else if (he(e)) {
    var f = this.parse(e, r, !0);
    f && i.push(f);
  } else if (Me(e)) {
    var v = e;
    a = r;
    for (var h = Object.keys(v), d = 0; d < h.length; d++) {
      var y = h[d], g = v[y];
      if (g === void 0 && (g = v[_n(y)]), g !== void 0) {
        var p = this.parse(y, g, !0);
        p && i.push(p);
      }
    }
  } else
    return !1;
  if (i.length === 0)
    return !1;
  for (var m = !1, b = 0; b < t.length; b++) {
    for (var w = t[b], C = {}, E = void 0, x = 0; x < i.length; x++) {
      var T = i[x];
      if (a) {
        var k = w.pstyle(T.name);
        E = C[T.name] = {
          prev: k
        };
      }
      m = this.applyParsedProperty(w, $t(T)) || m, a && (E.next = w.pstyle(T.name));
    }
    m && this.updateStyleHints(w), a && this.updateTransitions(w, C, o);
  }
  return m;
};
Ua.overrideBypass = function(t, e, r) {
  e = vs(e);
  for (var a = 0; a < t.length; a++) {
    var n = t[a], i = n._private.style[e], o = this.properties[e].type, s = o.color, u = o.mutiple, l = i ? i.pfValue != null ? i.pfValue : i.value : null;
    !i || !i.bypass ? this.applyBypass(n, e, r) : (i.value = r, i.pfValue != null && (i.pfValue = r), s ? i.strValue = "rgb(" + r.join(",") + ")" : u ? i.strValue = r.join(" ") : i.strValue = "" + r, this.updateStyleHints(n)), this.checkTriggers(n, e, l, r);
  }
};
Ua.removeAllBypasses = function(t, e) {
  return this.removeBypasses(t, this.propertyNames, e);
};
Ua.removeBypasses = function(t, e, r) {
  for (var a = !0, n = 0; n < t.length; n++) {
    for (var i = t[n], o = {}, s = 0; s < e.length; s++) {
      var u = e[s], l = this.properties[u], c = i.pstyle(l.name);
      if (!(!c || !c.bypass)) {
        var f = "", v = this.parse(u, f, !0), h = o[l.name] = {
          prev: c
        };
        this.applyParsedProperty(i, v), h.next = i.pstyle(l.name);
      }
    }
    this.updateStyleHints(i), r && this.updateTransitions(i, o, a);
  }
};
var Ps = {};
Ps.getEmSizeInPixels = function() {
  var t = this.containerCss("font-size");
  return t != null ? parseFloat(t) : 1;
};
Ps.containerCss = function(t) {
  var e = this._private.cy, r = e.container(), a = e.window();
  if (a && r && a.getComputedStyle)
    return a.getComputedStyle(r).getPropertyValue(t);
};
var Wt = {};
Wt.getRenderedStyle = function(t, e) {
  return e ? this.getStylePropertyValue(t, e, !0) : this.getRawStyle(t, !0);
};
Wt.getRawStyle = function(t, e) {
  var r = this;
  if (t = t[0], t) {
    for (var a = {}, n = 0; n < r.properties.length; n++) {
      var i = r.properties[n], o = r.getStylePropertyValue(t, i.name, e);
      o != null && (a[i.name] = o, a[_n(i.name)] = o);
    }
    return a;
  }
};
Wt.getIndexedStyle = function(t, e, r, a) {
  var n = t.pstyle(e)[r][a];
  return n ?? t.cy().style().getDefaultProperty(e)[r][0];
};
Wt.getStylePropertyValue = function(t, e, r) {
  var a = this;
  if (t = t[0], t) {
    var n = a.properties[e];
    n.alias && (n = n.pointsTo);
    var i = n.type, o = t.pstyle(n.name);
    if (o) {
      var s = o.value, u = o.units, l = o.strValue;
      if (r && i.number && s != null && ne(s)) {
        var c = t.cy().zoom(), f = function(g) {
          return g * c;
        }, v = function(g, p) {
          return f(g) + p;
        }, h = Ve(s), d = h ? u.every(function(y) {
          return y != null;
        }) : u != null;
        return d ? h ? s.map(function(y, g) {
          return v(y, u[g]);
        }).join(" ") : v(s, u) : h ? s.map(function(y) {
          return he(y) ? y : "" + f(y);
        }).join(" ") : "" + f(s);
      } else if (l != null)
        return l;
    }
    return null;
  }
};
Wt.getAnimationStartStyle = function(t, e) {
  for (var r = {}, a = 0; a < e.length; a++) {
    var n = e[a], i = n.name, o = t.pstyle(i);
    o !== void 0 && (Me(o) ? o = this.parse(i, o.strValue) : o = this.parse(i, o)), o && (r[i] = o);
  }
  return r;
};
Wt.getPropsList = function(t) {
  var e = this, r = [], a = t, n = e.properties;
  if (a)
    for (var i = Object.keys(a), o = 0; o < i.length; o++) {
      var s = i[o], u = a[s], l = n[s] || n[vs(s)], c = this.parse(l.name, u);
      c && r.push(c);
    }
  return r;
};
Wt.getNonDefaultPropertiesHash = function(t, e, r) {
  var a = r.slice(), n, i, o, s, u, l;
  for (u = 0; u < e.length; u++)
    if (n = e[u], i = t.pstyle(n, !1), i != null)
      if (i.pfValue != null)
        a[0] = Da(s, a[0]), a[1] = Aa(s, a[1]);
      else
        for (o = i.strValue, l = 0; l < o.length; l++)
          s = o.charCodeAt(l), a[0] = Da(s, a[0]), a[1] = Aa(s, a[1]);
  return a;
};
Wt.getPropertiesHash = Wt.getNonDefaultPropertiesHash;
var ei = {};
ei.appendFromJson = function(t) {
  for (var e = this, r = 0; r < t.length; r++) {
    var a = t[r], n = a.selector, i = a.style || a.css, o = Object.keys(i);
    e.selector(n);
    for (var s = 0; s < o.length; s++) {
      var u = o[s], l = i[u];
      e.css(u, l);
    }
  }
  return e;
};
ei.fromJson = function(t) {
  var e = this;
  return e.resetToDefault(), e.appendFromJson(t), e;
};
ei.json = function() {
  for (var t = [], e = this.defaultLength; e < this.length; e++) {
    for (var r = this[e], a = r.selector, n = r.properties, i = {}, o = 0; o < n.length; o++) {
      var s = n[o];
      i[s.name] = s.strValue;
    }
    t.push({
      selector: a ? a.toString() : "core",
      style: i
    });
  }
  return t;
};
var Ds = {};
Ds.appendFromString = function(t) {
  var e = this, r = this, a = "" + t, n, i, o;
  a = a.replace(/[/][*](\s|.)+?[*][/]/g, "");
  function s() {
    a.length > n.length ? a = a.substr(n.length) : a = "";
  }
  function u() {
    i.length > o.length ? i = i.substr(o.length) : i = "";
  }
  for (; ; ) {
    var l = a.match(/^\s*$/);
    if (l)
      break;
    var c = a.match(/^\s*((?:.|\s)+?)\s*\{((?:.|\s)+?)\}/);
    if (!c) {
      ze("Halting stylesheet parsing: String stylesheet contains more to parse but no selector and block found in: " + a);
      break;
    }
    n = c[0];
    var f = c[1];
    if (f !== "core") {
      var v = new dr(f);
      if (v.invalid) {
        ze("Skipping parsing of block: Invalid selector found in string stylesheet: " + f), s();
        continue;
      }
    }
    var h = c[2], d = !1;
    i = h;
    for (var y = []; ; ) {
      var g = i.match(/^\s*$/);
      if (g)
        break;
      var p = i.match(/^\s*(.+?)\s*:\s*(.+?)(?:\s*;|\s*$)/);
      if (!p) {
        ze("Skipping parsing of block: Invalid formatting of style property and value definitions found in:" + h), d = !0;
        break;
      }
      o = p[0];
      var m = p[1], b = p[2], w = e.properties[m];
      if (!w) {
        ze("Skipping property: Invalid property name in: " + o), u();
        continue;
      }
      var C = r.parse(m, b);
      if (!C) {
        ze("Skipping property: Invalid property definition in: " + o), u();
        continue;
      }
      y.push({
        name: m,
        val: b
      }), u();
    }
    if (d) {
      s();
      break;
    }
    r.selector(f);
    for (var E = 0; E < y.length; E++) {
      var x = y[E];
      r.css(x.name, x.val);
    }
    s();
  }
  return r;
};
Ds.fromString = function(t) {
  var e = this;
  return e.resetToDefault(), e.appendFromString(t), e;
};
var Qe = {};
(function() {
  var t = tt, e = Od, r = zd, a = Fd, n = qd, i = function(V) {
    return "^" + V + "\\s*\\(\\s*([\\w\\.]+)\\s*\\)$";
  }, o = function(V) {
    var W = t + "|\\w+|" + e + "|" + r + "|" + a + "|" + n;
    return "^" + V + "\\s*\\(([\\w\\.]+)\\s*\\,\\s*(" + t + ")\\s*\\,\\s*(" + t + ")\\s*,\\s*(" + W + ")\\s*\\,\\s*(" + W + ")\\)$";
  }, s = [`^url\\s*\\(\\s*['"]?(.+?)['"]?\\s*\\)$`, "^(none)$", "^(.+)$"];
  Qe.types = {
    time: {
      number: !0,
      min: 0,
      units: "s|ms",
      implicitUnits: "ms"
    },
    percent: {
      number: !0,
      min: 0,
      max: 100,
      units: "%",
      implicitUnits: "%"
    },
    percentages: {
      number: !0,
      min: 0,
      max: 100,
      units: "%",
      implicitUnits: "%",
      multiple: !0
    },
    zeroOneNumber: {
      number: !0,
      min: 0,
      max: 1,
      unitless: !0
    },
    zeroOneNumbers: {
      number: !0,
      min: 0,
      max: 1,
      unitless: !0,
      multiple: !0
    },
    nOneOneNumber: {
      number: !0,
      min: -1,
      max: 1,
      unitless: !0
    },
    nonNegativeInt: {
      number: !0,
      min: 0,
      integer: !0,
      unitless: !0
    },
    nonNegativeNumber: {
      number: !0,
      min: 0,
      unitless: !0
    },
    position: {
      enums: ["parent", "origin"]
    },
    nodeSize: {
      number: !0,
      min: 0,
      enums: ["label"]
    },
    number: {
      number: !0,
      unitless: !0
    },
    numbers: {
      number: !0,
      unitless: !0,
      multiple: !0
    },
    positiveNumber: {
      number: !0,
      unitless: !0,
      min: 0,
      strictMin: !0
    },
    size: {
      number: !0,
      min: 0
    },
    bidirectionalSize: {
      number: !0
    },
    // allows negative
    bidirectionalSizeMaybePercent: {
      number: !0,
      allowPercent: !0
    },
    // allows negative
    bidirectionalSizes: {
      number: !0,
      multiple: !0
    },
    // allows negative
    sizeMaybePercent: {
      number: !0,
      min: 0,
      allowPercent: !0
    },
    axisDirection: {
      enums: ["horizontal", "leftward", "rightward", "vertical", "upward", "downward", "auto"]
    },
    axisDirectionExplicit: {
      enums: ["leftward", "rightward", "upward", "downward"]
    },
    axisDirectionPrimary: {
      enums: ["horizontal", "vertical"]
    },
    paddingRelativeTo: {
      enums: ["width", "height", "average", "min", "max"]
    },
    bgWH: {
      number: !0,
      min: 0,
      allowPercent: !0,
      enums: ["auto"],
      multiple: !0
    },
    bgPos: {
      number: !0,
      allowPercent: !0,
      multiple: !0
    },
    bgRelativeTo: {
      enums: ["inner", "include-padding"],
      multiple: !0
    },
    bgRepeat: {
      enums: ["repeat", "repeat-x", "repeat-y", "no-repeat"],
      multiple: !0
    },
    bgFit: {
      enums: ["none", "contain", "cover"],
      multiple: !0
    },
    bgCrossOrigin: {
      enums: ["anonymous", "use-credentials", "null"],
      multiple: !0
    },
    bgClip: {
      enums: ["none", "node"],
      multiple: !0
    },
    bgContainment: {
      enums: ["inside", "over"],
      multiple: !0
    },
    boxSelection: {
      enums: ["contain", "overlap", "none"]
    },
    color: {
      color: !0
    },
    colors: {
      color: !0,
      multiple: !0
    },
    fill: {
      enums: ["solid", "linear-gradient", "radial-gradient"]
    },
    bool: {
      enums: ["yes", "no"]
    },
    bools: {
      enums: ["yes", "no"],
      multiple: !0
    },
    lineStyle: {
      enums: ["solid", "dotted", "dashed"]
    },
    lineCap: {
      enums: ["butt", "round", "square"]
    },
    linePosition: {
      enums: ["center", "inside", "outside"]
    },
    lineJoin: {
      enums: ["round", "bevel", "miter"]
    },
    borderStyle: {
      enums: ["solid", "dotted", "dashed", "double"]
    },
    curveStyle: {
      enums: ["bezier", "unbundled-bezier", "haystack", "segments", "straight", "straight-triangle", "taxi", "round-segments", "round-taxi"]
    },
    radiusType: {
      enums: ["arc-radius", "influence-radius"],
      multiple: !0
    },
    fontFamily: {
      regex: '^([\\w- \\"]+(?:\\s*,\\s*[\\w- \\"]+)*)$'
    },
    fontStyle: {
      enums: ["italic", "normal", "oblique"]
    },
    fontWeight: {
      enums: ["normal", "bold", "bolder", "lighter", "100", "200", "300", "400", "500", "600", "800", "900", 100, 200, 300, 400, 500, 600, 700, 800, 900]
    },
    textDecoration: {
      enums: ["none", "underline", "overline", "line-through"]
    },
    textTransform: {
      enums: ["none", "uppercase", "lowercase"]
    },
    textWrap: {
      enums: ["none", "wrap", "ellipsis"]
    },
    textOverflowWrap: {
      enums: ["whitespace", "anywhere"]
    },
    textBackgroundShape: {
      enums: ["rectangle", "roundrectangle", "round-rectangle", "circle"]
    },
    nodeShape: {
      enums: ["rectangle", "roundrectangle", "round-rectangle", "cutrectangle", "cut-rectangle", "bottomroundrectangle", "bottom-round-rectangle", "barrel", "ellipse", "triangle", "round-triangle", "square", "pentagon", "round-pentagon", "hexagon", "round-hexagon", "concavehexagon", "concave-hexagon", "heptagon", "round-heptagon", "octagon", "round-octagon", "tag", "round-tag", "star", "diamond", "round-diamond", "vee", "rhomboid", "right-rhomboid", "polygon"]
    },
    overlayShape: {
      enums: ["roundrectangle", "round-rectangle", "ellipse"]
    },
    cornerRadius: {
      number: !0,
      min: 0,
      units: "px|em",
      implicitUnits: "px",
      enums: ["auto"]
    },
    compoundIncludeLabels: {
      enums: ["include", "exclude"]
    },
    arrowShape: {
      enums: ["tee", "triangle", "triangle-tee", "circle-triangle", "triangle-cross", "triangle-backcurve", "vee", "square", "circle", "diamond", "chevron", "none"]
    },
    arrowFill: {
      enums: ["filled", "hollow"]
    },
    arrowWidth: {
      number: !0,
      units: "%|px|em",
      implicitUnits: "px",
      enums: ["match-line"]
    },
    display: {
      enums: ["element", "none"]
    },
    visibility: {
      enums: ["hidden", "visible"]
    },
    zCompoundDepth: {
      enums: ["bottom", "orphan", "auto", "top"]
    },
    zIndexCompare: {
      enums: ["auto", "manual"]
    },
    valign: {
      enums: ["top", "top-inside", "center", "bottom", "bottom-inside"]
    },
    halign: {
      enums: ["left", "left-inside", "center", "right", "right-inside"]
    },
    justification: {
      enums: ["left", "center", "right", "auto"]
    },
    textMetrics: {
      enums: ["font", "glyph"]
    },
    text: {
      string: !0
    },
    data: {
      mapping: !0,
      regex: i("data")
    },
    layoutData: {
      mapping: !0,
      regex: i("layoutData")
    },
    scratch: {
      mapping: !0,
      regex: i("scratch")
    },
    mapData: {
      mapping: !0,
      regex: o("mapData")
    },
    mapLayoutData: {
      mapping: !0,
      regex: o("mapLayoutData")
    },
    mapScratch: {
      mapping: !0,
      regex: o("mapScratch")
    },
    fn: {
      mapping: !0,
      fn: !0
    },
    url: {
      regexes: s,
      singleRegexMatchValue: !0
    },
    urls: {
      regexes: s,
      singleRegexMatchValue: !0,
      multiple: !0
    },
    propList: {
      propList: !0
    },
    angle: {
      number: !0,
      units: "deg|rad",
      implicitUnits: "rad"
    },
    textRotation: {
      number: !0,
      units: "deg|rad",
      implicitUnits: "rad",
      enums: ["none", "autorotate"]
    },
    polygonPointList: {
      number: !0,
      multiple: !0,
      evenMultiple: !0,
      min: -1,
      max: 1,
      unitless: !0
    },
    edgeDistances: {
      enums: ["intersection", "node-position", "endpoints"]
    },
    edgeEndpoint: {
      number: !0,
      multiple: !0,
      units: "%|px|em|deg|rad",
      implicitUnits: "px",
      enums: ["inside-to-node", "outside-to-node", "outside-to-node-or-label", "outside-to-line", "outside-to-line-or-label"],
      singleEnum: !0,
      validate: function(V, W) {
        switch (V.length) {
          case 2:
            return W[0] !== "deg" && W[0] !== "rad" && W[1] !== "deg" && W[1] !== "rad";
          case 1:
            return he(V[0]) || W[0] === "deg" || W[0] === "rad";
          default:
            return !1;
        }
      }
    },
    easing: {
      regexes: ["^(spring)\\s*\\(\\s*(" + t + ")\\s*,\\s*(" + t + ")\\s*\\)$", "^(cubic-bezier)\\s*\\(\\s*(" + t + ")\\s*,\\s*(" + t + ")\\s*,\\s*(" + t + ")\\s*,\\s*(" + t + ")\\s*\\)$"],
      enums: ["linear", "ease", "ease-in", "ease-out", "ease-in-out", "ease-in-sine", "ease-out-sine", "ease-in-out-sine", "ease-in-quad", "ease-out-quad", "ease-in-out-quad", "ease-in-cubic", "ease-out-cubic", "ease-in-out-cubic", "ease-in-quart", "ease-out-quart", "ease-in-out-quart", "ease-in-quint", "ease-out-quint", "ease-in-out-quint", "ease-in-expo", "ease-out-expo", "ease-in-out-expo", "ease-in-circ", "ease-out-circ", "ease-in-out-circ"]
    },
    gradientDirection: {
      enums: [
        "to-bottom",
        "to-top",
        "to-left",
        "to-right",
        "to-bottom-right",
        "to-bottom-left",
        "to-top-right",
        "to-top-left",
        "to-right-bottom",
        "to-left-bottom",
        "to-right-top",
        "to-left-top"
        // different order
      ]
    },
    boundsExpansion: {
      number: !0,
      multiple: !0,
      min: 0,
      validate: function(V) {
        var W = V.length;
        return W === 1 || W === 2 || W === 4;
      }
    }
  };
  var u = {
    zeroNonZero: function(V, W) {
      return (V == null || W == null) && V !== W || V == 0 && W != 0 ? !0 : V != 0 && W == 0;
    },
    any: function(V, W) {
      return V != W;
    },
    emptyNonEmpty: function(V, W) {
      var K = cr(V), re = cr(W);
      return K && !re || !K && re;
    }
  }, l = Qe.types, c = [{
    name: "label",
    type: l.text,
    triggersBounds: u.any,
    triggersZOrder: u.emptyNonEmpty
  }, {
    name: "text-rotation",
    type: l.textRotation,
    triggersBounds: u.any
  }, {
    name: "text-margin-x",
    type: l.bidirectionalSize,
    triggersBounds: u.any
  }, {
    name: "text-margin-y",
    type: l.bidirectionalSize,
    triggersBounds: u.any
  }], f = [{
    name: "source-label",
    type: l.text,
    triggersBounds: u.any
  }, {
    name: "source-text-rotation",
    type: l.textRotation,
    triggersBounds: u.any
  }, {
    name: "source-text-margin-x",
    type: l.bidirectionalSize,
    triggersBounds: u.any
  }, {
    name: "source-text-margin-y",
    type: l.bidirectionalSize,
    triggersBounds: u.any
  }, {
    name: "source-text-offset",
    type: l.size,
    triggersBounds: u.any
  }], v = [{
    name: "target-label",
    type: l.text,
    triggersBounds: u.any
  }, {
    name: "target-text-rotation",
    type: l.textRotation,
    triggersBounds: u.any
  }, {
    name: "target-text-margin-x",
    type: l.bidirectionalSize,
    triggersBounds: u.any
  }, {
    name: "target-text-margin-y",
    type: l.bidirectionalSize,
    triggersBounds: u.any
  }, {
    name: "target-text-offset",
    type: l.size,
    triggersBounds: u.any
  }], h = [{
    name: "font-family",
    type: l.fontFamily,
    triggersBounds: u.any
  }, {
    name: "font-style",
    type: l.fontStyle,
    triggersBounds: u.any
  }, {
    name: "font-weight",
    type: l.fontWeight,
    triggersBounds: u.any
  }, {
    name: "font-size",
    type: l.size,
    triggersBounds: u.any
  }, {
    name: "text-transform",
    type: l.textTransform,
    triggersBounds: u.any
  }, {
    name: "text-wrap",
    type: l.textWrap,
    triggersBounds: u.any
  }, {
    name: "text-overflow-wrap",
    type: l.textOverflowWrap,
    triggersBounds: u.any
  }, {
    name: "text-max-width",
    type: l.size,
    triggersBounds: u.any
  }, {
    name: "text-outline-width",
    type: l.size,
    triggersBounds: u.any
  }, {
    name: "line-height",
    type: l.positiveNumber,
    triggersBounds: u.any
  }], d = [{
    name: "text-valign",
    type: l.valign,
    triggersBounds: u.any
  }, {
    name: "text-halign",
    type: l.halign,
    triggersBounds: u.any
  }, {
    name: "color",
    type: l.color
  }, {
    name: "text-outline-color",
    type: l.color
  }, {
    name: "text-outline-opacity",
    type: l.zeroOneNumber
  }, {
    name: "text-background-color",
    type: l.color
  }, {
    name: "text-background-opacity",
    type: l.zeroOneNumber
  }, {
    name: "text-background-padding",
    type: l.size,
    triggersBounds: u.any
  }, {
    name: "text-border-opacity",
    type: l.zeroOneNumber
  }, {
    name: "text-border-color",
    type: l.color
  }, {
    name: "text-border-width",
    type: l.size,
    triggersBounds: u.any
  }, {
    name: "text-border-style",
    type: l.borderStyle,
    triggersBounds: u.any
  }, {
    name: "text-background-shape",
    type: l.textBackgroundShape,
    triggersBounds: u.any
  }, {
    name: "text-justification",
    type: l.justification
  }, {
    name: "text-metrics",
    type: l.textMetrics
  }, {
    name: "box-select-labels",
    type: l.bool,
    triggersBounds: u.any
  }], y = [{
    name: "events",
    type: l.bool,
    triggersZOrder: u.any
  }, {
    name: "text-events",
    type: l.bool,
    triggersZOrder: u.any
  }, {
    name: "box-selection",
    type: l.boxSelection,
    triggersZOrder: u.any
  }], g = [{
    name: "display",
    type: l.display,
    triggersZOrder: u.any,
    triggersBounds: u.any,
    triggersBoundsOfConnectedEdges: u.any,
    triggersBoundsOfParallelEdges: function(V, W, K) {
      return V === W ? !1 : K.pstyle("curve-style").value === "bezier";
    }
  }, {
    name: "visibility",
    type: l.visibility,
    triggersZOrder: u.any
  }, {
    name: "opacity",
    type: l.zeroOneNumber,
    triggersZOrder: u.zeroNonZero
  }, {
    name: "text-opacity",
    type: l.zeroOneNumber
  }, {
    name: "min-zoomed-font-size",
    type: l.size
  }, {
    name: "z-compound-depth",
    type: l.zCompoundDepth,
    triggersZOrder: u.any
  }, {
    name: "z-index-compare",
    type: l.zIndexCompare,
    triggersZOrder: u.any
  }, {
    name: "z-index",
    type: l.number,
    triggersZOrder: u.any
  }], p = [{
    name: "overlay-padding",
    type: l.size,
    triggersBounds: u.any
  }, {
    name: "overlay-color",
    type: l.color
  }, {
    name: "overlay-opacity",
    type: l.zeroOneNumber,
    triggersBounds: u.zeroNonZero
  }, {
    name: "overlay-shape",
    type: l.overlayShape,
    triggersBounds: u.any
  }, {
    name: "overlay-corner-radius",
    type: l.cornerRadius
  }], m = [{
    name: "underlay-padding",
    type: l.size,
    triggersBounds: u.any
  }, {
    name: "underlay-color",
    type: l.color
  }, {
    name: "underlay-opacity",
    type: l.zeroOneNumber,
    triggersBounds: u.zeroNonZero
  }, {
    name: "underlay-shape",
    type: l.overlayShape,
    triggersBounds: u.any
  }, {
    name: "underlay-corner-radius",
    type: l.cornerRadius
  }], b = [{
    name: "transition-property",
    type: l.propList
  }, {
    name: "transition-duration",
    type: l.time
  }, {
    name: "transition-delay",
    type: l.time
  }, {
    name: "transition-timing-function",
    type: l.easing
  }], w = function(V, W) {
    return W.value === "label" ? -V.poolIndex() : W.pfValue;
  }, C = [{
    name: "height",
    type: l.nodeSize,
    triggersBounds: u.any,
    hashOverride: w
  }, {
    name: "width",
    type: l.nodeSize,
    triggersBounds: u.any,
    hashOverride: w
  }, {
    name: "shape",
    type: l.nodeShape,
    triggersBounds: u.any
  }, {
    name: "shape-polygon-points",
    type: l.polygonPointList,
    triggersBounds: u.any
  }, {
    name: "corner-radius",
    type: l.cornerRadius
  }, {
    name: "background-color",
    type: l.color
  }, {
    name: "background-fill",
    type: l.fill
  }, {
    name: "background-opacity",
    type: l.zeroOneNumber
  }, {
    name: "background-blacken",
    type: l.nOneOneNumber
  }, {
    name: "background-gradient-stop-colors",
    type: l.colors
  }, {
    name: "background-gradient-stop-positions",
    type: l.percentages
  }, {
    name: "background-gradient-direction",
    type: l.gradientDirection
  }, {
    name: "padding",
    type: l.sizeMaybePercent,
    triggersBounds: u.any
  }, {
    name: "padding-relative-to",
    type: l.paddingRelativeTo,
    triggersBounds: u.any
  }, {
    name: "bounds-expansion",
    type: l.boundsExpansion,
    triggersBounds: u.any
  }], E = [{
    name: "border-color",
    type: l.color
  }, {
    name: "border-opacity",
    type: l.zeroOneNumber
  }, {
    name: "border-width",
    type: l.size,
    triggersBounds: u.any
  }, {
    name: "border-style",
    type: l.borderStyle
  }, {
    name: "border-cap",
    type: l.lineCap
  }, {
    name: "border-join",
    type: l.lineJoin
  }, {
    name: "border-dash-pattern",
    type: l.numbers
  }, {
    name: "border-dash-offset",
    type: l.number
  }, {
    name: "border-position",
    type: l.linePosition
  }], x = [{
    name: "outline-color",
    type: l.color
  }, {
    name: "outline-opacity",
    type: l.zeroOneNumber
  }, {
    name: "outline-width",
    type: l.size,
    triggersBounds: u.any
  }, {
    name: "outline-style",
    type: l.borderStyle
  }, {
    name: "outline-offset",
    type: l.size,
    triggersBounds: u.any
  }], T = [{
    name: "background-image",
    type: l.urls
  }, {
    name: "background-image-crossorigin",
    type: l.bgCrossOrigin
  }, {
    name: "background-image-opacity",
    type: l.zeroOneNumbers
  }, {
    name: "background-image-containment",
    type: l.bgContainment
  }, {
    name: "background-image-smoothing",
    type: l.bools
  }, {
    name: "background-position-x",
    type: l.bgPos
  }, {
    name: "background-position-y",
    type: l.bgPos
  }, {
    name: "background-width-relative-to",
    type: l.bgRelativeTo
  }, {
    name: "background-height-relative-to",
    type: l.bgRelativeTo
  }, {
    name: "background-repeat",
    type: l.bgRepeat
  }, {
    name: "background-fit",
    type: l.bgFit
  }, {
    name: "background-clip",
    type: l.bgClip
  }, {
    name: "background-width",
    type: l.bgWH
  }, {
    name: "background-height",
    type: l.bgWH
  }, {
    name: "background-offset-x",
    type: l.bgPos
  }, {
    name: "background-offset-y",
    type: l.bgPos
  }], k = [{
    name: "position",
    type: l.position,
    triggersBounds: u.any
  }, {
    name: "compound-sizing-wrt-labels",
    type: l.compoundIncludeLabels,
    triggersBounds: u.any
  }, {
    name: "min-width",
    type: l.size,
    triggersBounds: u.any
  }, {
    name: "min-width-bias-left",
    type: l.sizeMaybePercent,
    triggersBounds: u.any
  }, {
    name: "min-width-bias-right",
    type: l.sizeMaybePercent,
    triggersBounds: u.any
  }, {
    name: "min-height",
    type: l.size,
    triggersBounds: u.any
  }, {
    name: "min-height-bias-top",
    type: l.sizeMaybePercent,
    triggersBounds: u.any
  }, {
    name: "min-height-bias-bottom",
    type: l.sizeMaybePercent,
    triggersBounds: u.any
  }], D = [{
    name: "line-style",
    type: l.lineStyle
  }, {
    name: "line-color",
    type: l.color
  }, {
    name: "line-fill",
    type: l.fill
  }, {
    name: "line-cap",
    type: l.lineCap
  }, {
    name: "line-opacity",
    type: l.zeroOneNumber
  }, {
    name: "line-dash-pattern",
    type: l.numbers
  }, {
    name: "line-dash-offset",
    type: l.number
  }, {
    name: "line-outline-width",
    type: l.size
  }, {
    name: "line-outline-color",
    type: l.color
  }, {
    name: "line-gradient-stop-colors",
    type: l.colors
  }, {
    name: "line-gradient-stop-positions",
    type: l.percentages
  }, {
    name: "curve-style",
    type: l.curveStyle,
    triggersBounds: u.any,
    triggersBoundsOfParallelEdges: function(V, W) {
      return V === W ? !1 : V === "bezier" || // remove from bundle
      W === "bezier";
    }
  }, {
    name: "haystack-radius",
    type: l.zeroOneNumber,
    triggersBounds: u.any
  }, {
    name: "source-endpoint",
    type: l.edgeEndpoint,
    triggersBounds: u.any
  }, {
    name: "target-endpoint",
    type: l.edgeEndpoint,
    triggersBounds: u.any
  }, {
    name: "control-point-step-size",
    type: l.size,
    triggersBounds: u.any
  }, {
    name: "control-point-distances",
    type: l.bidirectionalSizes,
    triggersBounds: u.any
  }, {
    name: "control-point-weights",
    type: l.numbers,
    triggersBounds: u.any
  }, {
    name: "segment-distances",
    type: l.bidirectionalSizes,
    triggersBounds: u.any
  }, {
    name: "segment-weights",
    type: l.numbers,
    triggersBounds: u.any
  }, {
    name: "segment-radii",
    type: l.numbers,
    triggersBounds: u.any
  }, {
    name: "radius-type",
    type: l.radiusType,
    triggersBounds: u.any
  }, {
    name: "taxi-turn",
    type: l.bidirectionalSizeMaybePercent,
    triggersBounds: u.any
  }, {
    name: "taxi-turn-min-distance",
    type: l.size,
    triggersBounds: u.any
  }, {
    name: "taxi-direction",
    type: l.axisDirection,
    triggersBounds: u.any
  }, {
    name: "taxi-radius",
    type: l.number,
    triggersBounds: u.any
  }, {
    name: "edge-distances",
    type: l.edgeDistances,
    triggersBounds: u.any
  }, {
    name: "arrow-scale",
    type: l.positiveNumber,
    triggersBounds: u.any
  }, {
    name: "loop-direction",
    type: l.angle,
    triggersBounds: u.any
  }, {
    name: "loop-sweep",
    type: l.angle,
    triggersBounds: u.any
  }, {
    name: "source-distance-from-node",
    type: l.size,
    triggersBounds: u.any
  }, {
    name: "target-distance-from-node",
    type: l.size,
    triggersBounds: u.any
  }], P = [{
    name: "ghost",
    type: l.bool,
    triggersBounds: u.any
  }, {
    name: "ghost-offset-x",
    type: l.bidirectionalSize,
    triggersBounds: u.any
  }, {
    name: "ghost-offset-y",
    type: l.bidirectionalSize,
    triggersBounds: u.any
  }, {
    name: "ghost-opacity",
    type: l.zeroOneNumber
  }], A = [{
    name: "selection-box-color",
    type: l.color
  }, {
    name: "selection-box-opacity",
    type: l.zeroOneNumber
  }, {
    name: "selection-box-border-color",
    type: l.color
  }, {
    name: "selection-box-border-width",
    type: l.size
  }, {
    name: "active-bg-color",
    type: l.color
  }, {
    name: "active-bg-opacity",
    type: l.zeroOneNumber
  }, {
    name: "active-bg-size",
    type: l.size
  }, {
    name: "outside-texture-bg-color",
    type: l.color
  }, {
    name: "outside-texture-bg-opacity",
    type: l.zeroOneNumber
  }], B = [];
  Qe.pieBackgroundN = 16, B.push({
    name: "pie-size",
    type: l.sizeMaybePercent
  }), B.push({
    name: "pie-hole",
    type: l.sizeMaybePercent
  }), B.push({
    name: "pie-start-angle",
    type: l.angle
  });
  for (var R = 1; R <= Qe.pieBackgroundN; R++)
    B.push({
      name: "pie-" + R + "-background-color",
      type: l.color
    }), B.push({
      name: "pie-" + R + "-background-size",
      type: l.percent
    }), B.push({
      name: "pie-" + R + "-background-opacity",
      type: l.zeroOneNumber
    });
  var I = [];
  Qe.stripeBackgroundN = 16, I.push({
    name: "stripe-size",
    type: l.sizeMaybePercent
  }), I.push({
    name: "stripe-direction",
    type: l.axisDirectionPrimary
  });
  for (var O = 1; O <= Qe.stripeBackgroundN; O++)
    I.push({
      name: "stripe-" + O + "-background-color",
      type: l.color
    }), I.push({
      name: "stripe-" + O + "-background-size",
      type: l.percent
    }), I.push({
      name: "stripe-" + O + "-background-opacity",
      type: l.zeroOneNumber
    });
  var M = [], N = Qe.arrowPrefixes = ["source", "mid-source", "target", "mid-target"];
  [{
    name: "arrow-shape",
    type: l.arrowShape,
    triggersBounds: u.any
  }, {
    name: "arrow-color",
    type: l.color
  }, {
    name: "arrow-fill",
    type: l.arrowFill
  }, {
    name: "arrow-width",
    type: l.arrowWidth
  }].forEach(function(L) {
    N.forEach(function(V) {
      var W = V + "-" + L.name, K = L.type, re = L.triggersBounds;
      M.push({
        name: W,
        type: K,
        triggersBounds: re
      });
    });
  }, {});
  var H = Qe.properties = [].concat(y, b, g, p, m, P, d, h, c, f, v, C, E, x, T, B, I, k, D, M, A), G = Qe.propertyGroups = {
    // common to all eles
    behavior: y,
    transition: b,
    visibility: g,
    overlay: p,
    underlay: m,
    ghost: P,
    // labels
    commonLabel: d,
    labelDimensions: h,
    mainLabel: c,
    sourceLabel: f,
    targetLabel: v,
    // node props
    nodeBody: C,
    nodeBorder: E,
    nodeOutline: x,
    backgroundImage: T,
    pie: B,
    stripe: I,
    compound: k,
    // edge props
    edgeLine: D,
    edgeArrow: M,
    core: A
  }, F = Qe.propertyGroupNames = {}, _ = Qe.propertyGroupKeys = Object.keys(G);
  _.forEach(function(L) {
    F[L] = G[L].map(function(V) {
      return V.name;
    }), G[L].forEach(function(V) {
      return V.groupKey = L;
    });
  });
  var X = Qe.aliases = [{
    name: "content",
    pointsTo: "label"
  }, {
    name: "control-point-distance",
    pointsTo: "control-point-distances"
  }, {
    name: "control-point-weight",
    pointsTo: "control-point-weights"
  }, {
    name: "segment-distance",
    pointsTo: "segment-distances"
  }, {
    name: "segment-weight",
    pointsTo: "segment-weights"
  }, {
    name: "segment-radius",
    pointsTo: "segment-radii"
  }, {
    name: "edge-text-rotation",
    pointsTo: "text-rotation"
  }, {
    name: "padding-left",
    pointsTo: "padding"
  }, {
    name: "padding-right",
    pointsTo: "padding"
  }, {
    name: "padding-top",
    pointsTo: "padding"
  }, {
    name: "padding-bottom",
    pointsTo: "padding"
  }];
  Qe.propertyNames = H.map(function(L) {
    return L.name;
  });
  for (var J = 0; J < H.length; J++) {
    var Y = H[J];
    H[Y.name] = Y;
  }
  for (var j = 0; j < X.length; j++) {
    var te = X[j], ae = H[te.pointsTo], z = {
      name: te.name,
      alias: !0,
      pointsTo: ae
    };
    H.push(z), H[te.name] = z;
  }
})();
Qe.getDefaultProperty = function(t) {
  return this.getDefaultProperties()[t];
};
Qe.getDefaultProperties = function() {
  var t = this._private;
  if (t.defaultProperties != null)
    return t.defaultProperties;
  for (var e = ye({
    // core props
    "selection-box-color": "#ddd",
    "selection-box-opacity": 0.65,
    "selection-box-border-color": "#aaa",
    "selection-box-border-width": 1,
    "active-bg-color": "black",
    "active-bg-opacity": 0.15,
    "active-bg-size": 30,
    "outside-texture-bg-color": "#000",
    "outside-texture-bg-opacity": 0.125,
    // common node/edge props
    events: "yes",
    "text-events": "no",
    "text-valign": "top",
    "text-halign": "center",
    "text-justification": "auto",
    "line-height": 1,
    color: "#000",
    "box-selection": "contain",
    "text-outline-color": "#000",
    "text-outline-width": 0,
    "text-outline-opacity": 1,
    "text-opacity": 1,
    "text-decoration": "none",
    "text-transform": "none",
    "text-wrap": "none",
    "text-overflow-wrap": "whitespace",
    "text-max-width": 9999,
    "text-background-color": "#000",
    "text-background-opacity": 0,
    "text-background-shape": "rectangle",
    "text-background-padding": 0,
    "text-border-opacity": 0,
    "text-border-width": 0,
    "text-border-style": "solid",
    "text-border-color": "#000",
    "font-family": "Helvetica Neue, Helvetica, sans-serif",
    "font-style": "normal",
    "font-weight": "normal",
    "font-size": 16,
    "min-zoomed-font-size": 0,
    "text-rotation": "none",
    "source-text-rotation": "none",
    "target-text-rotation": "none",
    visibility: "visible",
    display: "element",
    opacity: 1,
    "z-compound-depth": "auto",
    "z-index-compare": "auto",
    "z-index": 0,
    label: "",
    "text-margin-x": 0,
    "text-margin-y": 0,
    "source-label": "",
    "source-text-offset": 0,
    "source-text-margin-x": 0,
    "source-text-margin-y": 0,
    "target-label": "",
    "target-text-offset": 0,
    "target-text-margin-x": 0,
    "target-text-margin-y": 0,
    "overlay-opacity": 0,
    "overlay-color": "#000",
    "overlay-padding": 10,
    "overlay-shape": "round-rectangle",
    "overlay-corner-radius": "auto",
    "underlay-opacity": 0,
    "underlay-color": "#000",
    "underlay-padding": 10,
    "underlay-shape": "round-rectangle",
    "underlay-corner-radius": "auto",
    "text-metrics": "font",
    "transition-property": "none",
    "transition-duration": 0,
    "transition-delay": 0,
    "transition-timing-function": "linear",
    "box-select-labels": "no",
    // node props
    "background-blacken": 0,
    "background-color": "#999",
    "background-fill": "solid",
    "background-opacity": 1,
    "background-image": "none",
    "background-image-crossorigin": "anonymous",
    "background-image-opacity": 1,
    "background-image-containment": "inside",
    "background-image-smoothing": "yes",
    "background-position-x": "50%",
    "background-position-y": "50%",
    "background-offset-x": 0,
    "background-offset-y": 0,
    "background-width-relative-to": "include-padding",
    "background-height-relative-to": "include-padding",
    "background-repeat": "no-repeat",
    "background-fit": "none",
    "background-clip": "node",
    "background-width": "auto",
    "background-height": "auto",
    "border-color": "#000",
    "border-opacity": 1,
    "border-width": 0,
    "border-style": "solid",
    "border-dash-pattern": [4, 2],
    "border-dash-offset": 0,
    "border-cap": "butt",
    "border-join": "miter",
    "border-position": "center",
    "outline-color": "#999",
    "outline-opacity": 1,
    "outline-width": 0,
    "outline-offset": 0,
    "outline-style": "solid",
    height: 30,
    width: 30,
    shape: "ellipse",
    "shape-polygon-points": "-1, -1,   1, -1,   1, 1,   -1, 1",
    "corner-radius": "auto",
    "bounds-expansion": 0,
    // node gradient
    "background-gradient-direction": "to-bottom",
    "background-gradient-stop-colors": "#999",
    "background-gradient-stop-positions": "0%",
    // ghost props
    ghost: "no",
    "ghost-offset-y": 0,
    "ghost-offset-x": 0,
    "ghost-opacity": 0,
    // compound props
    padding: 0,
    "padding-relative-to": "width",
    position: "origin",
    "compound-sizing-wrt-labels": "include",
    "min-width": 0,
    "min-width-bias-left": 0,
    "min-width-bias-right": 0,
    "min-height": 0,
    "min-height-bias-top": 0,
    "min-height-bias-bottom": 0
  }, {
    // node pie bg
    "pie-size": "100%",
    "pie-hole": 0,
    "pie-start-angle": "0deg"
  }, [{
    name: "pie-{{i}}-background-color",
    value: "black"
  }, {
    name: "pie-{{i}}-background-size",
    value: "0%"
  }, {
    name: "pie-{{i}}-background-opacity",
    value: 1
  }].reduce(function(u, l) {
    for (var c = 1; c <= Qe.pieBackgroundN; c++) {
      var f = l.name.replace("{{i}}", c), v = l.value;
      u[f] = v;
    }
    return u;
  }, {}), {
    // node stripes bg
    "stripe-size": "100%",
    "stripe-direction": "horizontal"
  }, [{
    name: "stripe-{{i}}-background-color",
    value: "black"
  }, {
    name: "stripe-{{i}}-background-size",
    value: "0%"
  }, {
    name: "stripe-{{i}}-background-opacity",
    value: 1
  }].reduce(function(u, l) {
    for (var c = 1; c <= Qe.stripeBackgroundN; c++) {
      var f = l.name.replace("{{i}}", c), v = l.value;
      u[f] = v;
    }
    return u;
  }, {}), {
    // edge props
    "line-style": "solid",
    "line-color": "#999",
    "line-fill": "solid",
    "line-cap": "butt",
    "line-opacity": 1,
    "line-outline-width": 0,
    "line-outline-color": "#000",
    "line-gradient-stop-colors": "#999",
    "line-gradient-stop-positions": "0%",
    "control-point-step-size": 40,
    "control-point-weights": 0.5,
    "segment-weights": 0.5,
    "segment-distances": 20,
    "segment-radii": 15,
    "radius-type": "arc-radius",
    "taxi-turn": "50%",
    "taxi-radius": 15,
    "taxi-turn-min-distance": 10,
    "taxi-direction": "auto",
    "edge-distances": "intersection",
    "curve-style": "haystack",
    "haystack-radius": 0,
    "arrow-scale": 1,
    "loop-direction": "-45deg",
    "loop-sweep": "-90deg",
    "source-distance-from-node": 0,
    "target-distance-from-node": 0,
    "source-endpoint": "outside-to-node",
    "target-endpoint": "outside-to-node",
    "line-dash-pattern": [6, 3],
    "line-dash-offset": 0
  }, [{
    name: "arrow-shape",
    value: "none"
  }, {
    name: "arrow-color",
    value: "#999"
  }, {
    name: "arrow-fill",
    value: "filled"
  }, {
    name: "arrow-width",
    value: 1
  }].reduce(function(u, l) {
    return Qe.arrowPrefixes.forEach(function(c) {
      var f = c + "-" + l.name, v = l.value;
      u[f] = v;
    }), u;
  }, {})), r = {}, a = 0; a < this.properties.length; a++) {
    var n = this.properties[a];
    if (!n.pointsTo) {
      var i = n.name, o = e[i], s = this.parse(i, o);
      r[i] = s;
    }
  }
  return t.defaultProperties = r, t.defaultProperties;
};
Qe.addDefaultStylesheet = function() {
  this.selector(":parent").css({
    shape: "rectangle",
    padding: 10,
    "background-color": "#eee",
    "border-color": "#ccc",
    "border-width": 1
  }).selector("edge").css({
    width: 3
  }).selector(":loop").css({
    "curve-style": "bezier"
  }).selector("edge:compound").css({
    "curve-style": "bezier",
    "source-endpoint": "outside-to-line",
    "target-endpoint": "outside-to-line"
  }).selector(":selected").css({
    "background-color": "#0169D9",
    "line-color": "#0169D9",
    "source-arrow-color": "#0169D9",
    "target-arrow-color": "#0169D9",
    "mid-source-arrow-color": "#0169D9",
    "mid-target-arrow-color": "#0169D9"
  }).selector(":parent:selected").css({
    "background-color": "#CCE1F9",
    "border-color": "#aec8e5"
  }).selector(":active").css({
    "overlay-color": "black",
    "overlay-padding": 10,
    "overlay-opacity": 0.25
  }), this.defaultLength = this.length;
};
var ti = {};
ti.parse = function(t, e, r, a) {
  var n = this;
  if (Ue(e))
    return n.parseImplWarn(t, e, r, a);
  var i = a === "mapping" || a === !0 || a === !1 || a == null ? "dontcare" : a, o = r ? "t" : "f", s = "" + e, u = Mc(t, s, o, i), l = n.propCache = n.propCache || [], c;
  return (c = l[u]) || (c = l[u] = n.parseImplWarn(t, e, r, a)), (r || a === "mapping") && (c = $t(c), c && (c.value = $t(c.value))), c;
};
ti.parseImplWarn = function(t, e, r, a) {
  var n = this.parseImpl(t, e, r, a);
  return !n && e != null && ze("The style property `".concat(t, ": ").concat(e, "` is invalid")), n && (n.name === "width" || n.name === "height") && e === "label" && ze("The style value of `label` is deprecated for `" + n.name + "`"), n;
};
ti.parseImpl = function(t, e, r, a) {
  var n = this;
  t = vs(t);
  var i = n.properties[t], o = e, s = n.types;
  if (!i || e === void 0)
    return null;
  i.alias && (i = i.pointsTo, t = i.name);
  var u = he(e);
  u && (e = e.trim());
  var l = i.type;
  if (!l)
    return null;
  if (r && (e === "" || e === null))
    return {
      name: t,
      value: e,
      bypass: !0,
      deleteBypass: !0
    };
  if (Ue(e))
    return {
      name: t,
      value: e,
      strValue: "fn",
      mapped: s.fn,
      bypass: r
    };
  var c, f;
  if (!(!u || a || e.length < 7 || e[1] !== "a")) {
    if (e.length >= 7 && e[0] === "d" && (c = new RegExp(s.data.regex).exec(e))) {
      if (r)
        return !1;
      var v = s.data;
      return {
        name: t,
        value: c,
        strValue: "" + e,
        mapped: v,
        field: c[1],
        bypass: r
      };
    } else if (e.length >= 10 && e[0] === "m" && (f = new RegExp(s.mapData.regex).exec(e))) {
      if (r || l.multiple)
        return !1;
      var h = s.mapData;
      if (!(l.color || l.number))
        return !1;
      var d = this.parse(t, f[4]);
      if (!d || d.mapped)
        return !1;
      var y = this.parse(t, f[5]);
      if (!y || y.mapped)
        return !1;
      if (d.pfValue === y.pfValue || d.strValue === y.strValue)
        return ze("`" + t + ": " + e + "` is not a valid mapper because the output range is zero; converting to `" + t + ": " + d.strValue + "`"), this.parse(t, d.strValue);
      if (l.color) {
        var g = d.value, p = y.value, m = g[0] === p[0] && g[1] === p[1] && g[2] === p[2] && // optional alpha
        (g[3] === p[3] || (g[3] == null || g[3] === 1) && (p[3] == null || p[3] === 1));
        if (m)
          return !1;
      }
      return {
        name: t,
        value: f,
        strValue: "" + e,
        mapped: h,
        field: f[1],
        fieldMin: parseFloat(f[2]),
        // min & max are numeric
        fieldMax: parseFloat(f[3]),
        valueMin: d.value,
        valueMax: y.value,
        bypass: r
      };
    }
  }
  if (l.multiple && a !== "multiple") {
    var b;
    if (u ? b = e.split(/\s+/) : Ve(e) ? b = e : b = [e], l.evenMultiple && b.length % 2 !== 0)
      return null;
    for (var w = [], C = [], E = [], x = "", T = !1, k = 0; k < b.length; k++) {
      var D = n.parse(t, b[k], r, "multiple");
      T = T || he(D.value), w.push(D.value), E.push(D.pfValue != null ? D.pfValue : D.value), C.push(D.units), x += (k > 0 ? " " : "") + D.strValue;
    }
    return l.validate && !l.validate(w, C) ? null : l.singleEnum && T ? w.length === 1 && he(w[0]) ? {
      name: t,
      value: w[0],
      strValue: w[0],
      bypass: r
    } : null : {
      name: t,
      value: w,
      pfValue: E,
      strValue: x,
      bypass: r,
      units: C
    };
  }
  var P = function() {
    for (var z = 0; z < l.enums.length; z++) {
      var L = l.enums[z];
      if (L === e)
        return {
          name: t,
          value: e,
          strValue: "" + e,
          bypass: r
        };
    }
    return null;
  };
  if (l.number) {
    var A, B = "px";
    if (l.units && (A = l.units), l.implicitUnits && (B = l.implicitUnits), !l.unitless)
      if (u) {
        var R = "px|em" + (l.allowPercent ? "|\\%" : "");
        A && (R = A);
        var I = e.match("^(" + tt + ")(" + R + ")?$");
        I && (e = I[1], A = I[2] || B);
      } else (!A || l.implicitUnits) && (A = B);
    if (e = parseFloat(e), isNaN(e) && l.enums === void 0)
      return null;
    if (isNaN(e) && l.enums !== void 0)
      return e = o, P();
    if (l.integer && !Dd(e) || l.min !== void 0 && (e < l.min || l.strictMin && e === l.min) || l.max !== void 0 && (e > l.max || l.strictMax && e === l.max))
      return null;
    var O = {
      name: t,
      value: e,
      strValue: "" + e + (A || ""),
      units: A,
      bypass: r
    };
    return l.unitless || A !== "px" && A !== "em" ? O.pfValue = e : O.pfValue = A === "px" || !A ? e : this.getEmSizeInPixels() * e, (A === "ms" || A === "s") && (O.pfValue = A === "ms" ? e : 1e3 * e), (A === "deg" || A === "rad") && (O.pfValue = A === "rad" ? e : Fh(e)), A === "%" && (O.pfValue = e / 100), O;
  } else if (l.propList) {
    var M = [], N = "" + e;
    if (N !== "none") {
      for (var H = N.split(/\s*,\s*|\s+/), G = 0; G < H.length; G++) {
        var F = H[G].trim();
        n.properties[F] ? M.push(F) : ze("`" + F + "` is not a valid property name");
      }
      if (M.length === 0)
        return null;
    }
    return {
      name: t,
      value: M,
      strValue: M.length === 0 ? "none" : M.join(" "),
      bypass: r
    };
  } else if (l.color) {
    var _ = Sc(e);
    return _ ? {
      name: t,
      value: _,
      pfValue: _,
      strValue: "rgb(" + _[0] + "," + _[1] + "," + _[2] + ")",
      // n.b. no spaces b/c of multiple support
      bypass: r
    } : null;
  } else if (l.regex || l.regexes) {
    if (l.enums) {
      var X = P();
      if (X)
        return X;
    }
    for (var J = l.regexes ? l.regexes : [l.regex], Y = 0; Y < J.length; Y++) {
      var j = new RegExp(J[Y]), te = j.exec(e);
      if (te)
        return {
          name: t,
          value: l.singleRegexMatchValue ? te[1] : te,
          strValue: "" + e,
          bypass: r
        };
    }
    return null;
  } else return l.string ? {
    name: t,
    value: "" + e,
    strValue: "" + e,
    bypass: r
  } : l.enums ? P() : null;
};
var ot = function(e) {
  if (!(this instanceof ot))
    return new ot(e);
  if (!fs(e)) {
    $e("A style must have a core reference");
    return;
  }
  this._private = {
    cy: e,
    coreStyle: {}
  }, this.length = 0, this.resetToDefault();
}, gt = ot.prototype;
gt.instanceString = function() {
  return "style";
};
gt.clear = function() {
  for (var t = this._private, e = t.cy, r = e.elements(), a = 0; a < this.length; a++)
    this[a] = void 0;
  return this.length = 0, t.contextStyles = {}, t.propDiffs = {}, this.cleanElements(r, !0), r.forEach(function(n) {
    var i = n[0]._private;
    i.styleDirty = !0, i.appliedInitStyle = !1;
  }), this;
};
gt.resetToDefault = function() {
  return this.clear(), this.addDefaultStylesheet(), this;
};
gt.core = function(t) {
  return this._private.coreStyle[t] || this.getDefaultProperty(t);
};
gt.selector = function(t) {
  var e = t === "core" ? null : new dr(t), r = this.length++;
  return this[r] = {
    selector: e,
    properties: [],
    mappedProperties: [],
    index: r
  }, this;
};
gt.css = function() {
  var t = this, e = arguments;
  if (e.length === 1)
    for (var r = e[0], a = 0; a < t.properties.length; a++) {
      var n = t.properties[a], i = r[n.name];
      i === void 0 && (i = r[_n(n.name)]), i !== void 0 && this.cssRule(n.name, i);
    }
  else e.length === 2 && this.cssRule(e[0], e[1]);
  return this;
};
gt.style = gt.css;
gt.cssRule = function(t, e) {
  var r = this.parse(t, e);
  if (r) {
    var a = this.length - 1;
    this[a].properties.push(r), this[a].properties[r.name] = r, r.name.match(/pie-(\d+)-background-size/) && r.value && (this._private.hasPie = !0), r.name.match(/stripe-(\d+)-background-size/) && r.value && (this._private.hasStripe = !0), r.mapped && this[a].mappedProperties.push(r);
    var n = !this[a].selector;
    n && (this._private.coreStyle[r.name] = r);
  }
  return this;
};
gt.append = function(t) {
  return Ec(t) ? t.appendToStyle(this) : Ve(t) ? this.appendFromJson(t) : he(t) && this.appendFromString(t), this;
};
ot.fromJson = function(t, e) {
  var r = new ot(t);
  return r.fromJson(e), r;
};
ot.fromString = function(t, e) {
  return new ot(t).fromString(e);
};
[st, Ua, Ps, Wt, ei, Ds, Qe, ti].forEach(function(t) {
  ye(gt, t);
});
ot.types = gt.types;
ot.properties = gt.properties;
ot.propertyGroups = gt.propertyGroups;
ot.propertyGroupNames = gt.propertyGroupNames;
ot.propertyGroupKeys = gt.propertyGroupKeys;
var Hy = {
  style: function(e) {
    if (e) {
      var r = this.setStyle(e);
      r.update();
    }
    return this._private.style;
  },
  setStyle: function(e) {
    var r = this._private;
    return Ec(e) ? r.style = e.generateStyle(this) : Ve(e) ? r.style = ot.fromJson(this, e) : he(e) ? r.style = ot.fromString(this, e) : r.style = ot(this), r.style;
  },
  // e.g. cy.data() changed => recalc ele mappers
  updateStyle: function() {
    this.mutableElements().updateStyle();
  }
}, $y = "single", Mr = {
  autolock: function(e) {
    if (e !== void 0)
      this._private.autolock = !!e;
    else
      return this._private.autolock;
    return this;
  },
  autoungrabify: function(e) {
    if (e !== void 0)
      this._private.autoungrabify = !!e;
    else
      return this._private.autoungrabify;
    return this;
  },
  autounselectify: function(e) {
    if (e !== void 0)
      this._private.autounselectify = !!e;
    else
      return this._private.autounselectify;
    return this;
  },
  selectionType: function(e) {
    var r = this._private;
    if (r.selectionType == null && (r.selectionType = $y), e !== void 0)
      (e === "additive" || e === "single") && (r.selectionType = e);
    else
      return r.selectionType;
    return this;
  },
  panningEnabled: function(e) {
    if (e !== void 0)
      this._private.panningEnabled = !!e;
    else
      return this._private.panningEnabled;
    return this;
  },
  userPanningEnabled: function(e) {
    if (e !== void 0)
      this._private.userPanningEnabled = !!e;
    else
      return this._private.userPanningEnabled;
    return this;
  },
  zoomingEnabled: function(e) {
    if (e !== void 0)
      this._private.zoomingEnabled = !!e;
    else
      return this._private.zoomingEnabled;
    return this;
  },
  userZoomingEnabled: function(e) {
    if (e !== void 0)
      this._private.userZoomingEnabled = !!e;
    else
      return this._private.userZoomingEnabled;
    return this;
  },
  boxSelectionEnabled: function(e) {
    if (e !== void 0)
      this._private.boxSelectionEnabled = !!e;
    else
      return this._private.boxSelectionEnabled;
    return this;
  },
  pan: function() {
    var e = arguments, r = this._private.pan, a, n, i, o, s;
    switch (e.length) {
      case 0:
        return r;
      case 1:
        if (he(e[0]))
          return a = e[0], r[a];
        if (Me(e[0])) {
          if (!this._private.panningEnabled)
            return this;
          i = e[0], o = i.x, s = i.y, ne(o) && (r.x = o), ne(s) && (r.y = s), this.emit("pan viewport");
        }
        break;
      case 2:
        if (!this._private.panningEnabled)
          return this;
        a = e[0], n = e[1], (a === "x" || a === "y") && ne(n) && (r[a] = n), this.emit("pan viewport");
        break;
    }
    return this.notify("viewport"), this;
  },
  panBy: function(e, r) {
    var a = arguments, n = this._private.pan, i, o, s, u, l;
    if (!this._private.panningEnabled)
      return this;
    switch (a.length) {
      case 1:
        Me(e) && (s = a[0], u = s.x, l = s.y, ne(u) && (n.x += u), ne(l) && (n.y += l), this.emit("pan viewport"));
        break;
      case 2:
        i = e, o = r, (i === "x" || i === "y") && ne(o) && (n[i] += o), this.emit("pan viewport");
        break;
    }
    return this.notify("viewport"), this;
  },
  gc: function() {
    this.notify("gc");
  },
  fit: function(e, r) {
    var a = this.getFitViewport(e, r);
    if (a) {
      var n = this._private;
      n.zoom = a.zoom, n.pan = a.pan, this.emit("pan zoom viewport"), this.notify("viewport");
    }
    return this;
  },
  getFitViewport: function(e, r) {
    if (ne(e) && r === void 0 && (r = e, e = void 0), !(!this._private.panningEnabled || !this._private.zoomingEnabled)) {
      var a;
      if (he(e)) {
        var n = e;
        e = this.$(n);
      } else if (Rd(e)) {
        var i = e;
        a = {
          x1: i.x1,
          y1: i.y1,
          x2: i.x2,
          y2: i.y2
        }, a.w = a.x2 - a.x1, a.h = a.y2 - a.y1;
      } else kt(e) || (e = this.mutableElements());
      if (!(kt(e) && e.empty())) {
        a = a || e.boundingBox();
        var o = this.width(), s = this.height(), u;
        if (r = ne(r) ? r : 0, !isNaN(o) && !isNaN(s) && o > 0 && s > 0 && !isNaN(a.w) && !isNaN(a.h) && a.w > 0 && a.h > 0) {
          u = Math.min((o - 2 * r) / a.w, (s - 2 * r) / a.h), u = u > this._private.maxZoom ? this._private.maxZoom : u, u = u < this._private.minZoom ? this._private.minZoom : u;
          var l = {
            // now pan to middle
            x: (o - u * (a.x1 + a.x2)) / 2,
            y: (s - u * (a.y1 + a.y2)) / 2
          };
          return {
            zoom: u,
            pan: l
          };
        }
      }
    }
  },
  zoomRange: function(e, r) {
    var a = this._private;
    if (r == null) {
      var n = e;
      e = n.min, r = n.max;
    }
    return ne(e) && ne(r) && e <= r ? (a.minZoom = e, a.maxZoom = r) : ne(e) && r === void 0 && e <= a.maxZoom ? a.minZoom = e : ne(r) && e === void 0 && r >= a.minZoom && (a.maxZoom = r), this;
  },
  minZoom: function(e) {
    return e === void 0 ? this._private.minZoom : this.zoomRange({
      min: e
    });
  },
  maxZoom: function(e) {
    return e === void 0 ? this._private.maxZoom : this.zoomRange({
      max: e
    });
  },
  getZoomedViewport: function(e) {
    var r = this._private, a = r.pan, n = r.zoom, i, o, s = !1;
    if (r.zoomingEnabled || (s = !0), ne(e) ? o = e : Me(e) && (o = e.level, e.position != null ? i = $n(e.position, n, a) : e.renderedPosition != null && (i = e.renderedPosition), i != null && !r.panningEnabled && (s = !0)), o = o > r.maxZoom ? r.maxZoom : o, o = o < r.minZoom ? r.minZoom : o, s || !ne(o) || o === n || i != null && (!ne(i.x) || !ne(i.y)))
      return null;
    if (i != null) {
      var u = a, l = n, c = o, f = {
        x: -c / l * (i.x - u.x) + i.x,
        y: -c / l * (i.y - u.y) + i.y
      };
      return {
        zoomed: !0,
        panned: !0,
        zoom: c,
        pan: f
      };
    } else
      return {
        zoomed: !0,
        panned: !1,
        zoom: o,
        pan: a
      };
  },
  zoom: function(e) {
    if (e === void 0)
      return this._private.zoom;
    var r = this.getZoomedViewport(e), a = this._private;
    return r == null || !r.zoomed ? this : (a.zoom = r.zoom, r.panned && (a.pan.x = r.pan.x, a.pan.y = r.pan.y), this.emit("zoom" + (r.panned ? " pan" : "") + " viewport"), this.notify("viewport"), this);
  },
  viewport: function(e) {
    var r = this._private, a = !0, n = !0, i = [], o = !1, s = !1;
    if (!e)
      return this;
    if (ne(e.zoom) || (a = !1), Me(e.pan) || (n = !1), !a && !n)
      return this;
    if (a) {
      var u = e.zoom;
      u < r.minZoom || u > r.maxZoom || !r.zoomingEnabled ? o = !0 : (r.zoom = u, i.push("zoom"));
    }
    if (n && (!o || !e.cancelOnFailedZoom) && r.panningEnabled) {
      var l = e.pan;
      ne(l.x) && (r.pan.x = l.x, s = !1), ne(l.y) && (r.pan.y = l.y, s = !1), s || i.push("pan");
    }
    return i.length > 0 && (i.push("viewport"), this.emit(i.join(" ")), this.notify("viewport")), this;
  },
  center: function(e) {
    var r = this.getCenterPan(e);
    return r && (this._private.pan = r, this.emit("pan viewport"), this.notify("viewport")), this;
  },
  getCenterPan: function(e, r) {
    if (this._private.panningEnabled) {
      if (he(e)) {
        var a = e;
        e = this.mutableElements().filter(a);
      } else kt(e) || (e = this.mutableElements());
      if (e.length !== 0) {
        var n = e.boundingBox(), i = this.width(), o = this.height();
        r = r === void 0 ? this._private.zoom : r;
        var s = {
          // middle
          x: (i - r * (n.x1 + n.x2)) / 2,
          y: (o - r * (n.y1 + n.y2)) / 2
        };
        return s;
      }
    }
  },
  reset: function() {
    return !this._private.panningEnabled || !this._private.zoomingEnabled ? this : (this.viewport({
      pan: {
        x: 0,
        y: 0
      },
      zoom: 1
    }), this);
  },
  invalidateSize: function() {
    this._private.sizeCache = null;
  },
  size: function() {
    var e = this._private, r = e.container, a = this;
    return e.sizeCache = e.sizeCache || (r ? (function() {
      var n = a.window().getComputedStyle(r), i = function(s) {
        return parseFloat(n.getPropertyValue(s));
      };
      return {
        width: r.clientWidth - i("padding-left") - i("padding-right"),
        height: r.clientHeight - i("padding-top") - i("padding-bottom")
      };
    })() : {
      // fallback if no container (not 0 b/c can be used for dividing etc)
      width: 1,
      height: 1
    });
  },
  width: function() {
    return this.size().width;
  },
  height: function() {
    return this.size().height;
  },
  extent: function() {
    var e = this._private.pan, r = this._private.zoom, a = this.renderedExtent(), n = {
      x1: (a.x1 - e.x) / r,
      x2: (a.x2 - e.x) / r,
      y1: (a.y1 - e.y) / r,
      y2: (a.y2 - e.y) / r
    };
    return n.w = n.x2 - n.x1, n.h = n.y2 - n.y1, n;
  },
  renderedExtent: function() {
    var e = this.width(), r = this.height();
    return {
      x1: 0,
      y1: 0,
      x2: e,
      y2: r,
      w: e,
      h: r
    };
  },
  multiClickDebounceTime: function(e) {
    if (e) this._private.multiClickDebounceTime = e;
    else return this._private.multiClickDebounceTime;
    return this;
  }
};
Mr.centre = Mr.center;
Mr.autolockNodes = Mr.autolock;
Mr.autoungrabifyNodes = Mr.autoungrabify;
var Oa = {
  data: Ne.data({
    field: "data",
    bindingEvent: "data",
    allowBinding: !0,
    allowSetting: !0,
    settingEvent: "data",
    settingTriggersEvent: !0,
    triggerFnName: "trigger",
    allowGetting: !0,
    updateStyle: !0
  }),
  removeData: Ne.removeData({
    field: "data",
    event: "data",
    triggerFnName: "trigger",
    triggerEvent: !0,
    updateStyle: !0
  }),
  scratch: Ne.data({
    field: "scratch",
    bindingEvent: "scratch",
    allowBinding: !0,
    allowSetting: !0,
    settingEvent: "scratch",
    settingTriggersEvent: !0,
    triggerFnName: "trigger",
    allowGetting: !0,
    updateStyle: !0
  }),
  removeScratch: Ne.removeData({
    field: "scratch",
    event: "scratch",
    triggerFnName: "trigger",
    triggerEvent: !0,
    updateStyle: !0
  })
};
Oa.attr = Oa.data;
Oa.removeAttr = Oa.removeData;
var Na = function(e) {
  var r = this;
  e = ye({}, e);
  var a = e.container;
  a && !Dn(a) && Dn(a[0]) && (a = a[0]);
  var n = a ? a._cyreg : null;
  n = n || {}, n && n.cy && (n.cy.destroy(), n = {});
  var i = n.readies = n.readies || [];
  a && (a._cyreg = n), n.cy = r;
  var o = et !== void 0 && a !== void 0 && !e.headless, s = e;
  s.layout = ye({
    name: o ? "grid" : "null"
  }, s.layout), s.renderer = ye({
    name: o ? "canvas" : "null"
  }, s.renderer);
  var u = function(d, y, g) {
    return y !== void 0 ? y : g !== void 0 ? g : d;
  }, l = this._private = {
    container: a,
    // html dom ele container
    ready: !1,
    // whether ready has been triggered
    options: s,
    // cached options
    elements: new ct(this),
    // elements in the graph
    listeners: [],
    // list of listeners
    aniEles: new ct(this),
    // elements being animated
    data: s.data || {},
    // data for the core
    scratch: {},
    // scratch object for core
    layout: null,
    renderer: null,
    destroyed: !1,
    // whether destroy was called
    notificationsEnabled: !0,
    // whether notifications are sent to the renderer
    minZoom: 1e-50,
    maxZoom: 1e50,
    zoomingEnabled: u(!0, s.zoomingEnabled),
    userZoomingEnabled: u(!0, s.userZoomingEnabled),
    panningEnabled: u(!0, s.panningEnabled),
    userPanningEnabled: u(!0, s.userPanningEnabled),
    boxSelectionEnabled: u(!0, s.boxSelectionEnabled),
    autolock: u(!1, s.autolock, s.autolockNodes),
    autoungrabify: u(!1, s.autoungrabify, s.autoungrabifyNodes),
    autounselectify: u(!1, s.autounselectify),
    styleEnabled: s.styleEnabled === void 0 ? o : s.styleEnabled,
    zoom: ne(s.zoom) ? s.zoom : 1,
    pan: {
      x: Me(s.pan) && ne(s.pan.x) ? s.pan.x : 0,
      y: Me(s.pan) && ne(s.pan.y) ? s.pan.y : 0
    },
    animation: {
      // object for currently-running animations
      current: [],
      queue: []
    },
    hasCompoundNodes: !1,
    multiClickDebounceTime: u(250, s.multiClickDebounceTime)
  };
  this.createEmitter(), this.selectionType(s.selectionType), this.zoomRange({
    min: s.minZoom,
    max: s.maxZoom
  });
  var c = function(d, y) {
    var g = d.some(Md);
    if (g)
      return sa.all(d).then(y);
    y(d);
  };
  l.styleEnabled && r.setStyle([]);
  var f = ye({}, s, s.renderer);
  r.initRenderer(f);
  var v = function(d, y, g) {
    r.notifications(!1);
    var p = r.mutableElements();
    p.length > 0 && p.remove(), d != null && (Me(d) || Ve(d)) && r.add(d), r.one("layoutready", function(b) {
      r.notifications(!0), r.emit(b), r.one("load", y), r.emitAndNotify("load");
    }).one("layoutstop", function() {
      r.one("done", g), r.emit("done");
    });
    var m = ye({}, r._private.options.layout);
    m.eles = r.elements(), r.layout(m).run();
  };
  c([s.style, s.elements], function(h) {
    var d = h[0], y = h[1];
    l.styleEnabled && r.style().append(d), v(y, function() {
      r.startAnimationLoop(), l.ready = !0, Ue(s.ready) && r.on("ready", s.ready);
      for (var g = 0; g < i.length; g++) {
        var p = i[g];
        r.on("ready", p);
      }
      n && (n.readies = []), r.emit("ready");
    }, s.done);
  });
}, On = Na.prototype;
ye(On, {
  instanceString: function() {
    return "core";
  },
  isReady: function() {
    return this._private.ready;
  },
  destroyed: function() {
    return this._private.destroyed;
  },
  ready: function(e) {
    return this.isReady() ? this.emitter().emit("ready", [], e) : this.on("ready", e), this;
  },
  destroy: function() {
    var e = this;
    if (!e.destroyed())
      return e.stopAnimationLoop(), e.destroyRenderer(), this.emit("destroy"), e._private.destroyed = !0, e;
  },
  hasElementWithId: function(e) {
    return this._private.elements.hasElementWithId(e);
  },
  getElementById: function(e) {
    return this._private.elements.getElementById(e);
  },
  hasCompoundNodes: function() {
    return this._private.hasCompoundNodes;
  },
  headless: function() {
    return this._private.renderer.isHeadless();
  },
  styleEnabled: function() {
    return this._private.styleEnabled;
  },
  addToPool: function(e) {
    return this._private.elements.merge(e), this;
  },
  removeFromPool: function(e) {
    return this._private.elements.unmerge(e), this;
  },
  container: function() {
    return this._private.container || null;
  },
  window: function() {
    var e = this._private.container;
    if (e == null) return et;
    var r = this._private.container.ownerDocument;
    return r === void 0 || r == null ? et : r.defaultView || et;
  },
  mount: function(e) {
    if (e != null) {
      var r = this, a = r._private, n = a.options;
      return !Dn(e) && Dn(e[0]) && (e = e[0]), r.stopAnimationLoop(), r.destroyRenderer(), a.container = e, a.styleEnabled = !0, r.invalidateSize(), r.initRenderer(ye({}, n, n.renderer, {
        // allow custom renderer name to be re-used, otherwise use canvas
        name: n.renderer.name === "null" ? "canvas" : n.renderer.name
      })), r.startAnimationLoop(), r.style(n.style), r.emit("mount"), r;
    }
  },
  unmount: function() {
    var e = this;
    return e.stopAnimationLoop(), e.destroyRenderer(), e.initRenderer({
      name: "null"
    }), e.emit("unmount"), e;
  },
  options: function() {
    return $t(this._private.options);
  },
  json: function(e) {
    var r = this, a = r._private, n = r.mutableElements(), i = function(w) {
      return r.getElementById(w.id());
    };
    if (Me(e)) {
      if (r.startBatch(), e.elements) {
        var o = {}, s = function(w, C) {
          for (var E = [], x = [], T = 0; T < w.length; T++) {
            var k = w[T];
            if (!k.data.id) {
              ze("cy.json() cannot handle elements without an ID attribute");
              continue;
            }
            var D = "" + k.data.id, P = r.getElementById(D);
            o[D] = !0, P.length !== 0 ? x.push({
              ele: P,
              json: k
            }) : (C && (k.group = C), E.push(k));
          }
          r.add(E);
          for (var A = 0; A < x.length; A++) {
            var B = x[A], R = B.ele, I = B.json;
            R.json(I);
          }
        };
        if (Ve(e.elements))
          s(e.elements);
        else
          for (var u = ["nodes", "edges"], l = 0; l < u.length; l++) {
            var c = u[l], f = e.elements[c];
            Ve(f) && s(f, c);
          }
        var v = r.collection();
        n.filter(function(b) {
          return !o[b.id()];
        }).forEach(function(b) {
          b.isParent() ? v.merge(b) : b.remove();
        }), v.forEach(function(b) {
          return b.children().move({
            parent: null
          });
        }), v.forEach(function(b) {
          return i(b).remove();
        });
      }
      e.style && r.style(e.style), e.zoom != null && e.zoom !== a.zoom && r.zoom(e.zoom), e.pan && (e.pan.x !== a.pan.x || e.pan.y !== a.pan.y) && r.pan(e.pan), e.data && r.data(e.data);
      for (var h = ["minZoom", "maxZoom", "zoomingEnabled", "userZoomingEnabled", "panningEnabled", "userPanningEnabled", "boxSelectionEnabled", "autolock", "autoungrabify", "autounselectify", "multiClickDebounceTime"], d = 0; d < h.length; d++) {
        var y = h[d];
        e[y] != null && r[y](e[y]);
      }
      return r.endBatch(), this;
    } else {
      var g = !!e, p = {};
      g ? p.elements = this.elements().map(function(b) {
        return b.json();
      }) : (p.elements = {}, n.forEach(function(b) {
        var w = b.group();
        p.elements[w] || (p.elements[w] = []), p.elements[w].push(b.json());
      })), this._private.styleEnabled && (p.style = r.style().json()), p.data = $t(r.data());
      var m = a.options;
      return p.zoomingEnabled = a.zoomingEnabled, p.userZoomingEnabled = a.userZoomingEnabled, p.zoom = a.zoom, p.minZoom = a.minZoom, p.maxZoom = a.maxZoom, p.panningEnabled = a.panningEnabled, p.userPanningEnabled = a.userPanningEnabled, p.pan = $t(a.pan), p.boxSelectionEnabled = a.boxSelectionEnabled, p.renderer = $t(m.renderer), p.hideEdgesOnViewport = m.hideEdgesOnViewport, p.textureOnViewport = m.textureOnViewport, p.wheelSensitivity = m.wheelSensitivity, p.motionBlur = m.motionBlur, p.multiClickDebounceTime = m.multiClickDebounceTime, p;
    }
  }
});
On.$id = On.getElementById;
[Iy, Fy, Cf, Zo, bn, Vy, Qo, wn, Hy, Mr, Oa].forEach(function(t) {
  ye(On, t);
});
var Wy = {
  fit: !0,
  // whether to fit the viewport to the graph
  directed: !1,
  // whether the tree is directed downwards (or edges can point in any direction if false)
  direction: "downward",
  // determines the direction in which the tree structure is drawn.  The possible values are 'downward', 'upward', 'rightward', or 'leftward'.
  padding: 30,
  // padding on fit
  circle: !1,
  // put depths in concentric circles if true, put depths top down if false
  grid: !1,
  // whether to create an even grid into which the DAG is placed (circle:false only)
  spacingFactor: 1.75,
  // positive spacing factor, larger => more space between nodes (N.B. n/a if causes overlap)
  boundingBox: void 0,
  // constrain layout bounds; { x1, y1, x2, y2 } or { x1, y1, w, h }
  avoidOverlap: !0,
  // prevents node overlap, may overflow boundingBox if not enough space
  nodeDimensionsIncludeLabels: !1,
  // Excludes the label when calculating node bounding boxes for the layout algorithm
  roots: void 0,
  // the roots of the trees
  depthSort: void 0,
  // a sorting function to order nodes at equal depth. e.g. function(a, b){ return a.data('weight') - b.data('weight') }
  animate: !1,
  // whether to transition the node positions
  animationDuration: 500,
  // duration of animation in ms if enabled
  animationEasing: void 0,
  // easing of animation if enabled,
  animateFilter: function(e, r) {
    return !0;
  },
  // a function that determines whether the node should be animated.  All nodes animated by default on animate enabled.  Non-animated nodes are positioned immediately when the layout starts
  ready: void 0,
  // callback on layoutready
  stop: void 0,
  // callback on layoutstop
  transform: function(e, r) {
    return r;
  }
  // transform a given node position. Useful for changing flow direction in discrete layouts
}, Uy = {
  maximal: !1,
  // whether to shift nodes down their natural BFS depths in order to avoid upwards edges (DAGS only); setting acyclic to true sets maximal to true also
  acyclic: !1
  // whether the tree is acyclic and thus a node could be shifted (due to the maximal option) multiple times without causing an infinite loop; setting to true sets maximal to true also; if you are uncertain whether a tree is acyclic, set to false to avoid potential infinite loops
}, _r = function(e) {
  return e.scratch("breadthfirst");
}, Hl = function(e, r) {
  return e.scratch("breadthfirst", r);
};
function Tf(t) {
  this.options = ye({}, Wy, Uy, t);
}
Tf.prototype.run = function() {
  var t = this.options, e = t.cy, r = t.eles, a = r.nodes().filter(function(ge) {
    return ge.isChildless();
  }), n = r, i = t.directed, o = t.acyclic || t.maximal || t.maximalAdjustments > 0, s = !!t.boundingBox, u = wt(s ? t.boundingBox : structuredClone(e.extent())), l;
  if (kt(t.roots))
    l = t.roots;
  else if (Ve(t.roots)) {
    for (var c = [], f = 0; f < t.roots.length; f++) {
      var v = t.roots[f], h = e.getElementById(v);
      c.push(h);
    }
    l = e.collection(c);
  } else if (he(t.roots))
    l = e.$(t.roots);
  else if (i)
    l = a.roots();
  else {
    var d = r.components();
    l = e.collection();
    for (var y = function() {
      var oe = d[g], de = oe.maxDegree(!1), ve = oe.filter(function(xe) {
        return xe.degree(!1) === de;
      });
      l = l.add(ve);
    }, g = 0; g < d.length; g++)
      y();
  }
  var p = [], m = {}, b = function(oe, de) {
    p[de] == null && (p[de] = []);
    var ve = p[de].length;
    p[de].push(oe), Hl(oe, {
      index: ve,
      depth: de
    });
  }, w = function(oe, de) {
    var ve = _r(oe), xe = ve.depth, be = ve.index;
    p[xe][be] = null, oe.isChildless() && b(oe, de);
  };
  n.bfs({
    roots: l,
    directed: t.directed,
    visit: function(oe, de, ve, xe, be) {
      var Se = oe[0], De = Se.id();
      Se.isChildless() && b(Se, be), m[De] = !0;
    }
  });
  for (var C = [], E = 0; E < a.length; E++) {
    var x = a[E];
    m[x.id()] || C.push(x);
  }
  var T = function(oe) {
    for (var de = p[oe], ve = 0; ve < de.length; ve++) {
      var xe = de[ve];
      if (xe == null) {
        de.splice(ve, 1), ve--;
        continue;
      }
      Hl(xe, {
        depth: oe,
        index: ve
      });
    }
  }, k = function(oe, de) {
    for (var ve = _r(oe), xe = oe.incomers().filter(function(Q) {
      return Q.isNode() && r.has(Q);
    }), be = -1, Se = oe.id(), De = 0; De < xe.length; De++) {
      var Oe = xe[De], Ie = _r(Oe);
      be = Math.max(be, Ie.depth);
    }
    if (ve.depth <= be) {
      if (!t.acyclic && de[Se])
        return null;
      var Be = be + 1;
      return w(oe, Be), de[Se] = Be, !0;
    }
    return !1;
  };
  if (i && o) {
    var D = [], P = {}, A = function(oe) {
      return D.push(oe);
    }, B = function() {
      return D.shift();
    };
    for (a.forEach(function(ge) {
      return D.push(ge);
    }); D.length > 0; ) {
      var R = B(), I = k(R, P);
      if (I)
        R.outgoers().filter(function(ge) {
          return ge.isNode() && r.has(ge);
        }).forEach(A);
      else if (I === null) {
        ze("Detected double maximal shift for node `" + R.id() + "`.  Bailing maximal adjustment due to cycle.  Use `options.maximal: true` only on DAGs.");
        break;
      }
    }
  }
  var O = 0;
  if (t.avoidOverlap)
    for (var M = 0; M < a.length; M++) {
      var N = a[M], H = N.layoutDimensions(t), G = H.w, F = H.h;
      O = Math.max(O, G, F);
    }
  var _ = {}, X = function(oe) {
    if (_[oe.id()])
      return _[oe.id()];
    for (var de = _r(oe).depth, ve = oe.neighborhood(), xe = 0, be = 0, Se = 0; Se < ve.length; Se++) {
      var De = ve[Se];
      if (!(De.isEdge() || De.isParent() || !a.has(De))) {
        var Oe = _r(De);
        if (Oe != null) {
          var Ie = Oe.index, Be = Oe.depth;
          if (!(Ie == null || Be == null)) {
            var Q = p[Be].length;
            Be < de && (xe += Ie / Q, be++);
          }
        }
      }
    }
    return be = Math.max(1, be), xe = xe / be, be === 0 && (xe = 0), _[oe.id()] = xe, xe;
  }, J = function(oe, de) {
    var ve = X(oe), xe = X(de), be = ve - xe;
    return be === 0 ? Tc(oe.id(), de.id()) : be;
  };
  t.depthSort !== void 0 && (J = t.depthSort);
  for (var Y = p.length, j = 0; j < Y; j++)
    p[j].sort(J), T(j);
  for (var te = [], ae = 0; ae < C.length; ae++)
    te.push(C[ae]);
  var z = function() {
    for (var oe = 0; oe < Y; oe++)
      T(oe);
  };
  te.length && (p.unshift(te), Y = p.length, z());
  for (var L = 0, V = 0; V < Y; V++)
    L = Math.max(p[V].length, L);
  var W = {
    x: u.x1 + u.w / 2,
    y: u.y1 + u.h / 2
  }, K = a.reduce(function(ge, oe) {
    return (function(de) {
      return {
        w: ge.w === -1 ? de.w : (ge.w + de.w) / 2,
        h: ge.h === -1 ? de.h : (ge.h + de.h) / 2
      };
    })(oe.boundingBox({
      includeLabels: t.nodeDimensionsIncludeLabels
    }));
  }, {
    w: -1,
    h: -1
  }), re = Math.max(
    // only one depth
    Y === 1 ? 0 : (
      // inside a bounding box, no need for top & bottom padding
      s ? (u.h - t.padding * 2 - K.h) / (Y - 1) : (u.h - t.padding * 2 - K.h) / (Y + 1)
    ),
    O
  ), le = p.reduce(function(ge, oe) {
    return Math.max(ge, oe.length);
  }, 0), Pe = function(oe) {
    var de = _r(oe), ve = de.depth, xe = de.index;
    if (t.circle) {
      var be = Math.min(u.w / 2 / Y, u.h / 2 / Y);
      be = Math.max(be, O);
      var Se = be * ve + be - (Y > 0 && p[0].length <= 3 ? be / 2 : 0), De = 2 * Math.PI / p[ve].length * xe;
      return ve === 0 && p[0].length === 1 && (Se = 1), {
        x: W.x + Se * Math.cos(De),
        y: W.y + Se * Math.sin(De)
      };
    } else {
      var Oe = p[ve].length, Ie = Math.max(
        // only one depth
        Oe === 1 ? 0 : (
          // inside a bounding box, no need for left & right padding
          s ? (u.w - t.padding * 2 - K.w) / ((t.grid ? le : Oe) - 1) : (u.w - t.padding * 2 - K.w) / ((t.grid ? le : Oe) + 1)
        ),
        O
      ), Be = {
        x: W.x + (xe + 1 - (Oe + 1) / 2) * Ie,
        y: W.y + (ve + 1 - (Y + 1) / 2) * re
      };
      return Be;
    }
  }, we = {
    downward: 0,
    leftward: 90,
    upward: 180,
    rightward: -90
  };
  Object.keys(we).indexOf(t.direction) === -1 && $e("Invalid direction '".concat(t.direction, "' specified for breadthfirst layout. Valid values are: ").concat(Object.keys(we).join(", ")));
  var me = function(oe) {
    return sh(Pe(oe), u, we[t.direction]);
  };
  return r.nodes().layoutPositions(this, t, me), this;
};
var Ky = {
  fit: !0,
  // whether to fit the viewport to the graph
  padding: 30,
  // the padding on fit
  boundingBox: void 0,
  // constrain layout bounds; { x1, y1, x2, y2 } or { x1, y1, w, h }
  avoidOverlap: !0,
  // prevents node overlap, may overflow boundingBox and radius if not enough space
  nodeDimensionsIncludeLabels: !1,
  // Excludes the label when calculating node bounding boxes for the layout algorithm
  spacingFactor: void 0,
  // Applies a multiplicative factor (>0) to expand or compress the overall area that the nodes take up
  radius: void 0,
  // the radius of the circle
  startAngle: 3 / 2 * Math.PI,
  // where nodes start in radians
  sweep: void 0,
  // how many radians should be between the first and last node (defaults to full circle)
  clockwise: !0,
  // whether the layout should go clockwise (true) or counterclockwise/anticlockwise (false)
  sort: void 0,
  // a sorting function to order the nodes; e.g. function(a, b){ return a.data('weight') - b.data('weight') }
  animate: !1,
  // whether to transition the node positions
  animationDuration: 500,
  // duration of animation in ms if enabled
  animationEasing: void 0,
  // easing of animation if enabled
  animateFilter: function(e, r) {
    return !0;
  },
  // a function that determines whether the node should be animated.  All nodes animated by default on animate enabled.  Non-animated nodes are positioned immediately when the layout starts
  ready: void 0,
  // callback on layoutready
  stop: void 0,
  // callback on layoutstop
  transform: function(e, r) {
    return r;
  }
  // transform a given node position. Useful for changing flow direction in discrete layouts 
};
function Sf(t) {
  this.options = ye({}, Ky, t);
}
Sf.prototype.run = function() {
  var t = this.options, e = t, r = t.cy, a = e.eles, n = e.counterclockwise !== void 0 ? !e.counterclockwise : e.clockwise, i = a.nodes().not(":parent");
  e.sort && (i = i.sort(e.sort));
  for (var o = wt(e.boundingBox ? e.boundingBox : {
    x1: 0,
    y1: 0,
    w: r.width(),
    h: r.height()
  }), s = {
    x: o.x1 + o.w / 2,
    y: o.y1 + o.h / 2
  }, u = e.sweep === void 0 ? 2 * Math.PI - 2 * Math.PI / i.length : e.sweep, l = u / Math.max(1, i.length - 1), c, f = 0, v = 0; v < i.length; v++) {
    var h = i[v], d = h.layoutDimensions(e), y = d.w, g = d.h;
    f = Math.max(f, y, g);
  }
  if (ne(e.radius) ? c = e.radius : i.length <= 1 ? c = 0 : c = Math.min(o.h, o.w) / 2 - f, i.length > 1 && e.avoidOverlap) {
    f *= 1.75;
    var p = Math.cos(l) - Math.cos(0), m = Math.sin(l) - Math.sin(0), b = Math.sqrt(f * f / (p * p + m * m));
    c = Math.max(b, c);
  }
  var w = function(E, x) {
    var T = e.startAngle + x * l * (n ? 1 : -1), k = c * Math.cos(T), D = c * Math.sin(T), P = {
      x: s.x + k,
      y: s.y + D
    };
    return P;
  };
  return a.nodes().layoutPositions(this, e, w), this;
};
var Xy = {
  fit: !0,
  // whether to fit the viewport to the graph
  padding: 30,
  // the padding on fit
  startAngle: 3 / 2 * Math.PI,
  // where nodes start in radians
  sweep: void 0,
  // how many radians should be between the first and last node (defaults to full circle)
  clockwise: !0,
  // whether the layout should go clockwise (true) or counterclockwise/anticlockwise (false)
  equidistant: !1,
  // whether levels have an equal radial distance betwen them, may cause bounding box overflow
  minNodeSpacing: 10,
  // min spacing between outside of nodes (used for radius adjustment)
  boundingBox: void 0,
  // constrain layout bounds; { x1, y1, x2, y2 } or { x1, y1, w, h }
  avoidOverlap: !0,
  // prevents node overlap, may overflow boundingBox if not enough space
  nodeDimensionsIncludeLabels: !1,
  // Excludes the label when calculating node bounding boxes for the layout algorithm
  height: void 0,
  // height of layout area (overrides container height)
  width: void 0,
  // width of layout area (overrides container width)
  spacingFactor: void 0,
  // Applies a multiplicative factor (>0) to expand or compress the overall area that the nodes take up
  concentric: function(e) {
    return e.degree();
  },
  levelWidth: function(e) {
    return e.maxDegree() / 4;
  },
  animate: !1,
  // whether to transition the node positions
  animationDuration: 500,
  // duration of animation in ms if enabled
  animationEasing: void 0,
  // easing of animation if enabled
  animateFilter: function(e, r) {
    return !0;
  },
  // a function that determines whether the node should be animated.  All nodes animated by default on animate enabled.  Non-animated nodes are positioned immediately when the layout starts
  ready: void 0,
  // callback on layoutready
  stop: void 0,
  // callback on layoutstop
  transform: function(e, r) {
    return r;
  }
  // transform a given node position. Useful for changing flow direction in discrete layouts
};
function kf(t) {
  this.options = ye({}, Xy, t);
}
kf.prototype.run = function() {
  for (var t = this.options, e = t, r = e.counterclockwise !== void 0 ? !e.counterclockwise : e.clockwise, a = t.cy, n = e.eles, i = n.nodes().not(":parent"), o = wt(e.boundingBox ? e.boundingBox : {
    x1: 0,
    y1: 0,
    w: a.width(),
    h: a.height()
  }), s = {
    x: o.x1 + o.w / 2,
    y: o.y1 + o.h / 2
  }, u = [], l = 0, c = 0; c < i.length; c++) {
    var f = i[c], v = void 0;
    v = e.concentric(f), u.push({
      value: v,
      node: f
    }), f._private.scratch.concentric = v;
  }
  i.updateStyle();
  for (var h = 0; h < i.length; h++) {
    var d = i[h], y = d.layoutDimensions(e);
    l = Math.max(l, y.w, y.h);
  }
  u.sort(function(re, le) {
    return le.value - re.value;
  });
  for (var g = e.levelWidth(i), p = [[]], m = p[0], b = 0; b < u.length; b++) {
    var w = u[b];
    if (m.length > 0) {
      var C = Math.abs(m[0].value - w.value);
      C >= g && (m = [], p.push(m));
    }
    m.push(w);
  }
  var E = l + e.minNodeSpacing;
  if (!e.avoidOverlap) {
    var x = p.length > 0 && p[0].length > 1, T = Math.min(o.w, o.h) / 2 - E, k = T / (p.length + x ? 1 : 0);
    E = Math.min(E, k);
  }
  for (var D = 0, P = 0; P < p.length; P++) {
    var A = p[P], B = e.sweep === void 0 ? 2 * Math.PI - 2 * Math.PI / A.length : e.sweep, R = A.dTheta = B / Math.max(1, A.length - 1);
    if (A.length > 1 && e.avoidOverlap) {
      var I = Math.cos(R) - Math.cos(0), O = Math.sin(R) - Math.sin(0), M = Math.sqrt(E * E / (I * I + O * O));
      D = Math.max(M, D);
    }
    A.r = D, D += E;
  }
  if (e.equidistant) {
    for (var N = 0, H = 0, G = 0; G < p.length; G++) {
      var F = p[G], _ = F.r - H;
      N = Math.max(N, _);
    }
    H = 0;
    for (var X = 0; X < p.length; X++) {
      var J = p[X];
      X === 0 && (H = J.r), J.r = H, H += N;
    }
  }
  for (var Y = {}, j = 0; j < p.length; j++)
    for (var te = p[j], ae = te.dTheta, z = te.r, L = 0; L < te.length; L++) {
      var V = te[L], W = e.startAngle + (r ? 1 : -1) * ae * L, K = {
        x: s.x + z * Math.cos(W),
        y: s.y + z * Math.sin(W)
      };
      Y[V.node.id()] = K;
    }
  return n.nodes().layoutPositions(this, e, function(re) {
    var le = re.id();
    return Y[le];
  }), this;
};
var Ro, Yy = {
  // Called on `layoutready`
  ready: function() {
  },
  // Called on `layoutstop`
  stop: function() {
  },
  // Whether to animate while running the layout
  // true : Animate continuously as the layout is running
  // false : Just show the end result
  // 'end' : Animate with the end result, from the initial positions to the end positions
  animate: !0,
  // Easing of the animation for animate:'end'
  animationEasing: void 0,
  // The duration of the animation for animate:'end'
  animationDuration: void 0,
  // A function that determines whether the node should be animated
  // All nodes animated by default on animate enabled
  // Non-animated nodes are positioned immediately when the layout starts
  animateFilter: function(e, r) {
    return !0;
  },
  // The layout animates only after this many milliseconds for animate:true
  // (prevents flashing on fast runs)
  animationThreshold: 250,
  // Number of iterations between consecutive screen positions update
  refresh: 20,
  // Whether to fit the network view after when done
  fit: !0,
  // Padding on fit
  padding: 30,
  // Constrain layout bounds; { x1, y1, x2, y2 } or { x1, y1, w, h }
  boundingBox: void 0,
  // Excludes the label when calculating node bounding boxes for the layout algorithm
  nodeDimensionsIncludeLabels: !1,
  // Randomize the initial positions of the nodes (true) or use existing positions (false)
  randomize: !1,
  // Extra spacing between components in non-compound graphs
  componentSpacing: 40,
  // Node repulsion (non overlapping) multiplier
  nodeRepulsion: function(e) {
    return 2048;
  },
  // Node repulsion (overlapping) multiplier
  nodeOverlap: 4,
  // Ideal edge (non nested) length
  idealEdgeLength: function(e) {
    return 32;
  },
  // Divisor to compute edge forces
  edgeElasticity: function(e) {
    return 32;
  },
  // Nesting factor (multiplier) to compute ideal edge length for nested edges
  nestingFactor: 1.2,
  // Gravity force (constant)
  gravity: 1,
  // Maximum number of iterations to perform
  numIter: 1e3,
  // Initial temperature (maximum node displacement)
  initialTemp: 1e3,
  // Cooling factor (how the temperature is reduced between consecutive iterations
  coolingFactor: 0.99,
  // Lower temperature threshold (below this point the layout will end)
  minTemp: 1
};
function ri(t) {
  this.options = ye({}, Yy, t), this.options.layout = this;
  var e = this.options.eles.nodes(), r = this.options.eles.edges(), a = r.filter(function(n) {
    var i = n.source().data("id"), o = n.target().data("id"), s = e.some(function(l) {
      return l.data("id") === i;
    }), u = e.some(function(l) {
      return l.data("id") === o;
    });
    return !s || !u;
  });
  this.options.eles = this.options.eles.not(a);
}
ri.prototype.run = function() {
  var t = this.options, e = t.cy, r = this;
  r.stopped = !1, (t.animate === !0 || t.animate === !1) && r.emit({
    type: "layoutstart",
    layout: r
  }), t.debug === !0 ? Ro = !0 : Ro = !1;
  var a = Zy(e, r, t);
  Ro && Jy(a), t.randomize && jy(a);
  var n = Qt(), i = function() {
    em(a, e, t), t.fit === !0 && e.fit(t.padding);
  }, o = function(v) {
    return !(r.stopped || v >= t.numIter || (tm(a, t), a.temperature = a.temperature * t.coolingFactor, a.temperature < t.minTemp));
  }, s = function() {
    if (t.animate === !0 || t.animate === !1)
      i(), r.one("layoutstop", t.stop), r.emit({
        type: "layoutstop",
        layout: r
      });
    else {
      var v = t.eles.nodes(), h = Df(a, t, v);
      v.layoutPositions(r, t, h);
    }
  }, u = 0, l = !0;
  if (t.animate === !0) {
    var c = function() {
      for (var v = 0; l && v < t.refresh; )
        l = o(u), u++, v++;
      if (!l)
        Wl(a, t), s();
      else {
        var h = Qt();
        h - n >= t.animationThreshold && i(), An(c);
      }
    };
    c();
  } else {
    for (; l; )
      l = o(u), u++;
    Wl(a, t), s();
  }
  return this;
};
ri.prototype.stop = function() {
  return this.stopped = !0, this.thread && this.thread.stop(), this.emit("layoutstop"), this;
};
ri.prototype.destroy = function() {
  return this.thread && this.thread.stop(), this;
};
var Zy = function(e, r, a) {
  for (var n = a.eles.edges(), i = a.eles.nodes(), o = wt(a.boundingBox ? a.boundingBox : {
    x1: 0,
    y1: 0,
    w: e.width(),
    h: e.height()
  }), s = {
    isCompound: e.hasCompoundNodes(),
    layoutNodes: [],
    idToIndex: {},
    nodeSize: i.size(),
    graphSet: [],
    indexToGraph: [],
    layoutEdges: [],
    edgeSize: n.size(),
    temperature: a.initialTemp,
    clientWidth: o.w,
    clientHeight: o.h,
    boundingBox: o
  }, u = a.eles.components(), l = {}, c = 0; c < u.length; c++)
    for (var f = u[c], v = 0; v < f.length; v++) {
      var h = f[v];
      l[h.id()] = c;
    }
  for (var c = 0; c < s.nodeSize; c++) {
    var d = i[c], y = d.layoutDimensions(a), g = {};
    g.isLocked = d.locked(), g.id = d.data("id"), g.parentId = d.data("parent"), g.cmptId = l[d.id()], g.children = [], g.positionX = d.position("x"), g.positionY = d.position("y"), g.offsetX = 0, g.offsetY = 0, g.height = y.w, g.width = y.h, g.maxX = g.positionX + g.width / 2, g.minX = g.positionX - g.width / 2, g.maxY = g.positionY + g.height / 2, g.minY = g.positionY - g.height / 2, g.padLeft = parseFloat(d.style("padding")), g.padRight = parseFloat(d.style("padding")), g.padTop = parseFloat(d.style("padding")), g.padBottom = parseFloat(d.style("padding")), g.nodeRepulsion = Ue(a.nodeRepulsion) ? a.nodeRepulsion(d) : a.nodeRepulsion, s.layoutNodes.push(g), s.idToIndex[g.id] = c;
  }
  for (var p = [], m = 0, b = -1, w = [], c = 0; c < s.nodeSize; c++) {
    var d = s.layoutNodes[c], C = d.parentId;
    C != null ? s.layoutNodes[s.idToIndex[C]].children.push(d.id) : (p[++b] = d.id, w.push(d.id));
  }
  for (s.graphSet.push(w); m <= b; ) {
    var E = p[m++], x = s.idToIndex[E], h = s.layoutNodes[x], T = h.children;
    if (T.length > 0) {
      s.graphSet.push(T);
      for (var c = 0; c < T.length; c++)
        p[++b] = T[c];
    }
  }
  for (var c = 0; c < s.graphSet.length; c++)
    for (var k = s.graphSet[c], v = 0; v < k.length; v++) {
      var D = s.idToIndex[k[v]];
      s.indexToGraph[D] = c;
    }
  for (var c = 0; c < s.edgeSize; c++) {
    var P = n[c], A = {};
    A.id = P.data("id"), A.sourceId = P.data("source"), A.targetId = P.data("target");
    var B = Ue(a.idealEdgeLength) ? a.idealEdgeLength(P) : a.idealEdgeLength, R = Ue(a.edgeElasticity) ? a.edgeElasticity(P) : a.edgeElasticity, I = s.idToIndex[A.sourceId], O = s.idToIndex[A.targetId], M = s.indexToGraph[I], N = s.indexToGraph[O];
    if (M != N) {
      for (var H = Qy(A.sourceId, A.targetId, s), G = s.graphSet[H], F = 0, g = s.layoutNodes[I]; G.indexOf(g.id) === -1; )
        g = s.layoutNodes[s.idToIndex[g.parentId]], F++;
      for (g = s.layoutNodes[O]; G.indexOf(g.id) === -1; )
        g = s.layoutNodes[s.idToIndex[g.parentId]], F++;
      B *= F * a.nestingFactor;
    }
    A.idealLength = B, A.elasticity = R, s.layoutEdges.push(A);
  }
  return s;
}, Qy = function(e, r, a) {
  var n = Pf(e, r, 0, a);
  return 2 > n.count ? 0 : n.graph;
}, Pf = function(e, r, a, n) {
  var i = n.graphSet[a];
  if (-1 < i.indexOf(e) && -1 < i.indexOf(r))
    return {
      count: 2,
      graph: a
    };
  for (var o = 0, s = 0; s < i.length; s++) {
    var u = i[s], l = n.idToIndex[u], c = n.layoutNodes[l].children;
    if (c.length !== 0) {
      var f = n.indexToGraph[n.idToIndex[c[0]]], v = Pf(e, r, f, n);
      if (v.count !== 0)
        if (v.count === 1) {
          if (o++, o === 2)
            break;
        } else
          return v;
    }
  }
  return {
    count: o,
    graph: a
  };
}, Jy, jy = function(e, r) {
  for (var a = e.clientWidth, n = e.clientHeight, i = 0; i < e.nodeSize; i++) {
    var o = e.layoutNodes[i];
    o.children.length === 0 && !o.isLocked && (o.positionX = Math.random() * a, o.positionY = Math.random() * n);
  }
}, Df = function(e, r, a) {
  var n = e.boundingBox, i = {
    x1: 1 / 0,
    x2: -1 / 0,
    y1: 1 / 0,
    y2: -1 / 0
  };
  return r.boundingBox && (a.forEach(function(o) {
    var s = e.layoutNodes[e.idToIndex[o.data("id")]];
    i.x1 = Math.min(i.x1, s.positionX), i.x2 = Math.max(i.x2, s.positionX), i.y1 = Math.min(i.y1, s.positionY), i.y2 = Math.max(i.y2, s.positionY);
  }), i.w = i.x2 - i.x1, i.h = i.y2 - i.y1), function(o, s) {
    var u = e.layoutNodes[e.idToIndex[o.data("id")]];
    if (r.boundingBox) {
      var l = i.w === 0 ? 0.5 : (u.positionX - i.x1) / i.w, c = i.h === 0 ? 0.5 : (u.positionY - i.y1) / i.h;
      return {
        x: n.x1 + l * n.w,
        y: n.y1 + c * n.h
      };
    } else
      return {
        x: u.positionX,
        y: u.positionY
      };
  };
}, em = function(e, r, a) {
  var n = a.layout, i = a.eles.nodes(), o = Df(e, a, i);
  i.positions(o), e.ready !== !0 && (e.ready = !0, n.one("layoutready", a.ready), n.emit({
    type: "layoutready",
    layout: this
  }));
}, tm = function(e, r, a) {
  rm(e, r), im(e), om(e, r), sm(e), um(e);
}, rm = function(e, r) {
  for (var a = 0; a < e.graphSet.length; a++)
    for (var n = e.graphSet[a], i = n.length, o = 0; o < i; o++)
      for (var s = e.layoutNodes[e.idToIndex[n[o]]], u = o + 1; u < i; u++) {
        var l = e.layoutNodes[e.idToIndex[n[u]]];
        am(s, l, e, r);
      }
}, $l = function(e) {
  return -e + 2 * e * Math.random();
}, am = function(e, r, a, n) {
  var i = e.cmptId, o = r.cmptId;
  if (!(i !== o && !a.isCompound)) {
    var s = r.positionX - e.positionX, u = r.positionY - e.positionY, l = 1;
    s === 0 && u === 0 && (s = $l(l), u = $l(l));
    var c = nm(e, r, s, u);
    if (c > 0)
      var f = n.nodeOverlap * c, v = Math.sqrt(s * s + u * u), h = f * s / v, d = f * u / v;
    else
      var y = Nn(e, s, u), g = Nn(r, -1 * s, -1 * u), p = g.x - y.x, m = g.y - y.y, b = p * p + m * m, v = Math.sqrt(b), f = (e.nodeRepulsion + r.nodeRepulsion) / b, h = f * p / v, d = f * m / v;
    e.isLocked || (e.offsetX -= h, e.offsetY -= d), r.isLocked || (r.offsetX += h, r.offsetY += d);
  }
}, nm = function(e, r, a, n) {
  if (a > 0)
    var i = e.maxX - r.minX;
  else
    var i = r.maxX - e.minX;
  if (n > 0)
    var o = e.maxY - r.minY;
  else
    var o = r.maxY - e.minY;
  return i >= 0 && o >= 0 ? Math.sqrt(i * i + o * o) : 0;
}, Nn = function(e, r, a) {
  var n = e.positionX, i = e.positionY, o = e.height || 1, s = e.width || 1, u = a / r, l = o / s, c = {};
  return r === 0 && 0 < a || r === 0 && 0 > a ? (c.x = n, c.y = i + o / 2, c) : 0 < r && -1 * l <= u && u <= l ? (c.x = n + s / 2, c.y = i + s * a / 2 / r, c) : 0 > r && -1 * l <= u && u <= l ? (c.x = n - s / 2, c.y = i - s * a / 2 / r, c) : 0 < a && (u <= -1 * l || u >= l) ? (c.x = n + o * r / 2 / a, c.y = i + o / 2, c) : (0 > a && (u <= -1 * l || u >= l) && (c.x = n - o * r / 2 / a, c.y = i - o / 2), c);
}, im = function(e, r) {
  for (var a = 0; a < e.edgeSize; a++) {
    var n = e.layoutEdges[a], i = e.idToIndex[n.sourceId], o = e.layoutNodes[i], s = e.idToIndex[n.targetId], u = e.layoutNodes[s], l = u.positionX - o.positionX, c = u.positionY - o.positionY;
    if (!(l === 0 && c === 0)) {
      var f = Nn(o, l, c), v = Nn(u, -1 * l, -1 * c), h = v.x - f.x, d = v.y - f.y, y = Math.sqrt(h * h + d * d), g = Math.pow(n.idealLength - y, 2) / n.elasticity;
      if (y !== 0)
        var p = g * h / y, m = g * d / y;
      else
        var p = 0, m = 0;
      o.isLocked || (o.offsetX += p, o.offsetY += m), u.isLocked || (u.offsetX -= p, u.offsetY -= m);
    }
  }
}, om = function(e, r) {
  if (r.gravity !== 0)
    for (var a = 1, n = 0; n < e.graphSet.length; n++) {
      var i = e.graphSet[n], o = i.length;
      if (n === 0)
        var s = e.clientHeight / 2, u = e.clientWidth / 2;
      else
        var l = e.layoutNodes[e.idToIndex[i[0]]], c = e.layoutNodes[e.idToIndex[l.parentId]], s = c.positionX, u = c.positionY;
      for (var f = 0; f < o; f++) {
        var v = e.layoutNodes[e.idToIndex[i[f]]];
        if (!v.isLocked) {
          var h = s - v.positionX, d = u - v.positionY, y = Math.sqrt(h * h + d * d);
          if (y > a) {
            var g = r.gravity * h / y, p = r.gravity * d / y;
            v.offsetX += g, v.offsetY += p;
          }
        }
      }
    }
}, sm = function(e, r) {
  var a = [], n = 0, i = -1;
  for (a.push.apply(a, e.graphSet[0]), i += e.graphSet[0].length; n <= i; ) {
    var o = a[n++], s = e.idToIndex[o], u = e.layoutNodes[s], l = u.children;
    if (0 < l.length && !u.isLocked) {
      for (var c = u.offsetX, f = u.offsetY, v = 0; v < l.length; v++) {
        var h = e.layoutNodes[e.idToIndex[l[v]]];
        h.offsetX += c, h.offsetY += f, a[++i] = l[v];
      }
      u.offsetX = 0, u.offsetY = 0;
    }
  }
}, um = function(e, r) {
  for (var a = 0; a < e.nodeSize; a++) {
    var n = e.layoutNodes[a];
    0 < n.children.length && (n.maxX = void 0, n.minX = void 0, n.maxY = void 0, n.minY = void 0);
  }
  for (var a = 0; a < e.nodeSize; a++) {
    var n = e.layoutNodes[a];
    if (!(0 < n.children.length || n.isLocked)) {
      var i = lm(n.offsetX, n.offsetY, e.temperature);
      n.positionX += i.x, n.positionY += i.y, n.offsetX = 0, n.offsetY = 0, n.minX = n.positionX - n.width, n.maxX = n.positionX + n.width, n.minY = n.positionY - n.height, n.maxY = n.positionY + n.height, Af(n, e);
    }
  }
  for (var a = 0; a < e.nodeSize; a++) {
    var n = e.layoutNodes[a];
    0 < n.children.length && !n.isLocked && (n.positionX = (n.maxX + n.minX) / 2, n.positionY = (n.maxY + n.minY) / 2, n.width = n.maxX - n.minX, n.height = n.maxY - n.minY);
  }
}, lm = function(e, r, a) {
  var n = Math.sqrt(e * e + r * r);
  if (n > a)
    var i = {
      x: a * e / n,
      y: a * r / n
    };
  else
    var i = {
      x: e,
      y: r
    };
  return i;
}, Af = function(e, r) {
  var a = e.parentId;
  if (a != null) {
    var n = r.layoutNodes[r.idToIndex[a]], i = !1;
    if ((n.maxX == null || e.maxX + n.padRight > n.maxX) && (n.maxX = e.maxX + n.padRight, i = !0), (n.minX == null || e.minX - n.padLeft < n.minX) && (n.minX = e.minX - n.padLeft, i = !0), (n.maxY == null || e.maxY + n.padBottom > n.maxY) && (n.maxY = e.maxY + n.padBottom, i = !0), (n.minY == null || e.minY - n.padTop < n.minY) && (n.minY = e.minY - n.padTop, i = !0), i)
      return Af(n, r);
  }
}, Wl = function(e, r) {
  for (var a = e.layoutNodes, n = [], i = 0; i < a.length; i++) {
    var o = a[i], s = o.cmptId, u = n[s] = n[s] || [];
    u.push(o);
  }
  for (var l = 0, i = 0; i < n.length; i++) {
    var c = n[i];
    if (c) {
      c.x1 = 1 / 0, c.x2 = -1 / 0, c.y1 = 1 / 0, c.y2 = -1 / 0;
      for (var f = 0; f < c.length; f++) {
        var v = c[f];
        c.x1 = Math.min(c.x1, v.positionX - v.width / 2), c.x2 = Math.max(c.x2, v.positionX + v.width / 2), c.y1 = Math.min(c.y1, v.positionY - v.height / 2), c.y2 = Math.max(c.y2, v.positionY + v.height / 2);
      }
      c.w = c.x2 - c.x1, c.h = c.y2 - c.y1, l += c.w * c.h;
    }
  }
  n.sort(function(m, b) {
    return b.w * b.h - m.w * m.h;
  });
  for (var h = 0, d = 0, y = 0, g = 0, p = Math.sqrt(l) * e.clientWidth / e.clientHeight, i = 0; i < n.length; i++) {
    var c = n[i];
    if (c) {
      for (var f = 0; f < c.length; f++) {
        var v = c[f];
        v.isLocked || (v.positionX += h - c.x1, v.positionY += d - c.y1);
      }
      h += c.w + r.componentSpacing, y += c.w + r.componentSpacing, g = Math.max(g, c.h), y > p && (d += g + r.componentSpacing, h = 0, y = 0, g = 0);
    }
  }
}, cm = {
  fit: !0,
  // whether to fit the viewport to the graph
  padding: 30,
  // padding used on fit
  boundingBox: void 0,
  // constrain layout bounds; { x1, y1, x2, y2 } or { x1, y1, w, h }
  avoidOverlap: !0,
  // prevents node overlap, may overflow boundingBox if not enough space
  avoidOverlapPadding: 10,
  // extra spacing around nodes when avoidOverlap: true
  nodeDimensionsIncludeLabels: !1,
  // Excludes the label when calculating node bounding boxes for the layout algorithm
  spacingFactor: void 0,
  // Applies a multiplicative factor (>0) to expand or compress the overall area that the nodes take up
  condense: !1,
  // uses all available space on false, uses minimal space on true
  rows: void 0,
  // force num of rows in the grid
  cols: void 0,
  // force num of columns in the grid
  position: function(e) {
  },
  // returns { row, col } for element
  sort: void 0,
  // a sorting function to order the nodes; e.g. function(a, b){ return a.data('weight') - b.data('weight') }
  animate: !1,
  // whether to transition the node positions
  animationDuration: 500,
  // duration of animation in ms if enabled
  animationEasing: void 0,
  // easing of animation if enabled
  animateFilter: function(e, r) {
    return !0;
  },
  // a function that determines whether the node should be animated.  All nodes animated by default on animate enabled.  Non-animated nodes are positioned immediately when the layout starts
  ready: void 0,
  // callback on layoutready
  stop: void 0,
  // callback on layoutstop
  transform: function(e, r) {
    return r;
  }
  // transform a given node position. Useful for changing flow direction in discrete layouts 
};
function Bf(t) {
  this.options = ye({}, cm, t);
}
Bf.prototype.run = function() {
  var t = this.options, e = t, r = t.cy, a = e.eles, n = a.nodes().not(":parent");
  e.sort && (n = n.sort(e.sort));
  var i = wt(e.boundingBox ? e.boundingBox : {
    x1: 0,
    y1: 0,
    w: r.width(),
    h: r.height()
  });
  if (i.h === 0 || i.w === 0)
    a.nodes().layoutPositions(this, e, function(X) {
      return {
        x: i.x1,
        y: i.y1
      };
    });
  else {
    var o = n.size(), s = Math.sqrt(o * i.h / i.w), u = Math.round(s), l = Math.round(i.w / i.h * s), c = function(J) {
      if (J == null)
        return Math.min(u, l);
      var Y = Math.min(u, l);
      Y == u ? u = J : l = J;
    }, f = function(J) {
      if (J == null)
        return Math.max(u, l);
      var Y = Math.max(u, l);
      Y == u ? u = J : l = J;
    }, v = e.rows, h = e.cols != null ? e.cols : e.columns;
    if (v != null && h != null)
      u = v, l = h;
    else if (v != null && h == null)
      u = v, l = Math.ceil(o / u);
    else if (v == null && h != null)
      l = h, u = Math.ceil(o / l);
    else if (l * u > o) {
      var d = c(), y = f();
      (d - 1) * y >= o ? c(d - 1) : (y - 1) * d >= o && f(y - 1);
    } else
      for (; l * u < o; ) {
        var g = c(), p = f();
        (p + 1) * g >= o ? f(p + 1) : c(g + 1);
      }
    var m = i.w / l, b = i.h / u;
    if (e.condense && (m = 0, b = 0), e.avoidOverlap)
      for (var w = 0; w < n.length; w++) {
        var C = n[w], E = C._private.position;
        (E.x == null || E.y == null) && (E.x = 0, E.y = 0);
        var x = C.layoutDimensions(e), T = e.avoidOverlapPadding, k = x.w + T, D = x.h + T;
        m = Math.max(m, k), b = Math.max(b, D);
      }
    for (var P = {}, A = function(J, Y) {
      return !!P["c-" + J + "-" + Y];
    }, B = function(J, Y) {
      P["c-" + J + "-" + Y] = !0;
    }, R = 0, I = 0, O = function() {
      I++, I >= l && (I = 0, R++);
    }, M = {}, N = 0; N < n.length; N++) {
      var H = n[N], G = e.position(H);
      if (G && (G.row !== void 0 || G.col !== void 0)) {
        var F = {
          row: G.row,
          col: G.col
        };
        if (F.col === void 0)
          for (F.col = 0; A(F.row, F.col); )
            F.col++;
        else if (F.row === void 0)
          for (F.row = 0; A(F.row, F.col); )
            F.row++;
        M[H.id()] = F, B(F.row, F.col);
      }
    }
    var _ = function(J, Y) {
      var j, te;
      if (J.locked() || J.isParent())
        return !1;
      var ae = M[J.id()];
      if (ae)
        j = ae.col * m + m / 2 + i.x1, te = ae.row * b + b / 2 + i.y1;
      else {
        for (; A(R, I); )
          O();
        j = I * m + m / 2 + i.x1, te = R * b + b / 2 + i.y1, B(R, I), O();
      }
      return {
        x: j,
        y: te
      };
    };
    n.layoutPositions(this, e, _);
  }
  return this;
};
var fm = {
  ready: function() {
  },
  // on layoutready
  stop: function() {
  }
  // on layoutstop
};
function As(t) {
  this.options = ye({}, fm, t);
}
As.prototype.run = function() {
  var t = this.options, e = t.eles, r = this;
  return t.cy, r.emit("layoutstart"), e.nodes().positions(function() {
    return {
      x: 0,
      y: 0
    };
  }), r.one("layoutready", t.ready), r.emit("layoutready"), r.one("layoutstop", t.stop), r.emit("layoutstop"), this;
};
As.prototype.stop = function() {
  return this;
};
var vm = {
  positions: void 0,
  // map of (node id) => (position obj); or function(node){ return somPos; }
  zoom: void 0,
  // the zoom level to set (prob want fit = false if set)
  pan: void 0,
  // the pan level to set (prob want fit = false if set)
  fit: !0,
  // whether to fit to viewport
  padding: 30,
  // padding on fit
  spacingFactor: void 0,
  // Applies a multiplicative factor (>0) to expand or compress the overall area that the nodes take up
  animate: !1,
  // whether to transition the node positions
  animationDuration: 500,
  // duration of animation in ms if enabled
  animationEasing: void 0,
  // easing of animation if enabled
  animateFilter: function(e, r) {
    return !0;
  },
  // a function that determines whether the node should be animated.  All nodes animated by default on animate enabled.  Non-animated nodes are positioned immediately when the layout starts
  ready: void 0,
  // callback on layoutready
  stop: void 0,
  // callback on layoutstop
  transform: function(e, r) {
    return r;
  }
  // transform a given node position. Useful for changing flow direction in discrete layouts
};
function Rf(t) {
  this.options = ye({}, vm, t);
}
Rf.prototype.run = function() {
  var t = this.options, e = t.eles, r = e.nodes(), a = Ue(t.positions);
  function n(i) {
    if (t.positions == null)
      return Ih(i.position());
    if (a)
      return t.positions(i);
    var o = t.positions[i._private.data.id];
    return o ?? null;
  }
  return r.layoutPositions(this, t, function(i, o) {
    var s = n(i);
    return i.locked() || s == null ? !1 : s;
  }), this;
};
var dm = {
  fit: !0,
  // whether to fit to viewport
  padding: 30,
  // fit padding
  boundingBox: void 0,
  // constrain layout bounds; { x1, y1, x2, y2 } or { x1, y1, w, h }
  animate: !1,
  // whether to transition the node positions
  animationDuration: 500,
  // duration of animation in ms if enabled
  animationEasing: void 0,
  // easing of animation if enabled
  animateFilter: function(e, r) {
    return !0;
  },
  // a function that determines whether the node should be animated.  All nodes animated by default on animate enabled.  Non-animated nodes are positioned immediately when the layout starts
  ready: void 0,
  // callback on layoutready
  stop: void 0,
  // callback on layoutstop
  transform: function(e, r) {
    return r;
  }
  // transform a given node position. Useful for changing flow direction in discrete layouts 
};
function Mf(t) {
  this.options = ye({}, dm, t);
}
Mf.prototype.run = function() {
  var t = this.options, e = t.cy, r = t.eles, a = wt(t.boundingBox ? t.boundingBox : {
    x1: 0,
    y1: 0,
    w: e.width(),
    h: e.height()
  }), n = function(o, s) {
    return {
      x: a.x1 + Math.round(Math.random() * a.w),
      y: a.y1 + Math.round(Math.random() * a.h)
    };
  };
  return r.nodes().layoutPositions(this, t, n), this;
};
var hm = [{
  name: "breadthfirst",
  impl: Tf
}, {
  name: "circle",
  impl: Sf
}, {
  name: "concentric",
  impl: kf
}, {
  name: "cose",
  impl: ri
}, {
  name: "grid",
  impl: Bf
}, {
  name: "null",
  impl: As
}, {
  name: "preset",
  impl: Rf
}, {
  name: "random",
  impl: Mf
}];
function If(t) {
  this.options = t, this.notifications = 0;
}
var Ul = function() {
}, Kl = function() {
  throw new Error("A headless instance can not render images");
};
If.prototype = {
  recalculateRenderedStyle: Ul,
  notify: function() {
    this.notifications++;
  },
  init: Ul,
  isHeadless: function() {
    return !0;
  },
  png: Kl,
  jpg: Kl
};
var Bs = {};
Bs.arrowShapeWidth = 0.3;
Bs.registerArrowShapes = function() {
  var t = this.arrowShapes = {}, e = this, r = function(l, c, f, v, h, d, y) {
    var g = h.x - f / 2 - y, p = h.x + f / 2 + y, m = h.y - f / 2 - y, b = h.y + f / 2 + y, w = g <= l && l <= p && m <= c && c <= b;
    return w;
  }, a = function(l, c, f, v, h) {
    var d = l * Math.cos(v) - c * Math.sin(v), y = l * Math.sin(v) + c * Math.cos(v), g = d * f, p = y * f, m = g + h.x, b = p + h.y;
    return {
      x: m,
      y: b
    };
  }, n = function(l, c, f, v) {
    for (var h = [], d = 0; d < l.length; d += 2) {
      var y = l[d], g = l[d + 1];
      h.push(a(y, g, c, f, v));
    }
    return h;
  }, i = function(l) {
    for (var c = [], f = 0; f < l.length; f++) {
      var v = l[f];
      c.push(v.x, v.y);
    }
    return c;
  }, o = function(l) {
    return l.pstyle("width").pfValue * l.pstyle("arrow-scale").pfValue * 2;
  }, s = function(l, c) {
    he(c) && (c = t[c]), t[l] = ye({
      name: l,
      points: [-0.15, -0.3, 0.15, -0.3, 0.15, 0.3, -0.15, 0.3],
      collide: function(v, h, d, y, g, p) {
        var m = i(n(this.points, d + 2 * p, y, g)), b = Tt(v, h, m);
        return b;
      },
      roughCollide: r,
      draw: function(v, h, d, y) {
        var g = n(this.points, h, d, y);
        e.arrowShapeImpl("polygon")(v, g);
      },
      spacing: function(v) {
        return 0;
      },
      gap: o
    }, c);
  };
  s("none", {
    collide: Bn,
    roughCollide: Bn,
    draw: gs,
    spacing: su,
    gap: su
  }), s("triangle", {
    points: [-0.15, -0.3, 0, 0, 0.15, -0.3]
  }), s("arrow", "triangle"), s("triangle-backcurve", {
    points: t.triangle.points,
    controlPoint: [0, -0.15],
    roughCollide: r,
    draw: function(l, c, f, v, h) {
      var d = n(this.points, c, f, v), y = this.controlPoint, g = a(y[0], y[1], c, f, v);
      e.arrowShapeImpl(this.name)(l, d, g);
    },
    gap: function(l) {
      return o(l) * 0.8;
    }
  }), s("triangle-tee", {
    points: [0, 0, 0.15, -0.3, -0.15, -0.3, 0, 0],
    pointsTee: [-0.15, -0.4, -0.15, -0.5, 0.15, -0.5, 0.15, -0.4],
    collide: function(l, c, f, v, h, d, y) {
      var g = i(n(this.points, f + 2 * y, v, h)), p = i(n(this.pointsTee, f + 2 * y, v, h)), m = Tt(l, c, g) || Tt(l, c, p);
      return m;
    },
    draw: function(l, c, f, v, h) {
      var d = n(this.points, c, f, v), y = n(this.pointsTee, c, f, v);
      e.arrowShapeImpl(this.name)(l, d, y);
    }
  }), s("circle-triangle", {
    radius: 0.15,
    pointsTr: [0, -0.15, 0.15, -0.45, -0.15, -0.45, 0, -0.15],
    collide: function(l, c, f, v, h, d, y) {
      var g = h, p = Math.pow(g.x - l, 2) + Math.pow(g.y - c, 2) <= Math.pow((f + 2 * y) * this.radius, 2), m = i(n(this.points, f + 2 * y, v, h));
      return Tt(l, c, m) || p;
    },
    draw: function(l, c, f, v, h) {
      var d = n(this.pointsTr, c, f, v);
      e.arrowShapeImpl(this.name)(l, d, v.x, v.y, this.radius * c);
    },
    spacing: function(l) {
      return e.getArrowWidth(l.pstyle("width").pfValue, l.pstyle("arrow-scale").value) * this.radius;
    }
  }), s("triangle-cross", {
    points: [0, 0, 0.15, -0.3, -0.15, -0.3, 0, 0],
    baseCrossLinePts: [
      -0.15,
      -0.4,
      // first half of the rectangle
      -0.15,
      -0.4,
      0.15,
      -0.4,
      // second half of the rectangle
      0.15,
      -0.4
    ],
    crossLinePts: function(l, c) {
      var f = this.baseCrossLinePts.slice(), v = c / l, h = 3, d = 5;
      return f[h] = f[h] - v, f[d] = f[d] - v, f;
    },
    collide: function(l, c, f, v, h, d, y) {
      var g = i(n(this.points, f + 2 * y, v, h)), p = i(n(this.crossLinePts(f, d), f + 2 * y, v, h)), m = Tt(l, c, g) || Tt(l, c, p);
      return m;
    },
    draw: function(l, c, f, v, h) {
      var d = n(this.points, c, f, v), y = n(this.crossLinePts(c, h), c, f, v);
      e.arrowShapeImpl(this.name)(l, d, y);
    }
  }), s("vee", {
    points: [-0.15, -0.3, 0, 0, 0.15, -0.3, 0, -0.15],
    gap: function(l) {
      return o(l) * 0.525;
    }
  }), s("circle", {
    radius: 0.15,
    collide: function(l, c, f, v, h, d, y) {
      var g = h, p = Math.pow(g.x - l, 2) + Math.pow(g.y - c, 2) <= Math.pow((f + 2 * y) * this.radius, 2);
      return p;
    },
    draw: function(l, c, f, v, h) {
      e.arrowShapeImpl(this.name)(l, v.x, v.y, this.radius * c);
    },
    spacing: function(l) {
      return e.getArrowWidth(l.pstyle("width").pfValue, l.pstyle("arrow-scale").value) * this.radius;
    }
  }), s("tee", {
    points: [-0.15, 0, -0.15, -0.1, 0.15, -0.1, 0.15, 0],
    spacing: function(l) {
      return 1;
    },
    gap: function(l) {
      return 1;
    }
  }), s("square", {
    points: [-0.15, 0, 0.15, 0, 0.15, -0.3, -0.15, -0.3]
  }), s("diamond", {
    points: [-0.15, -0.15, 0, -0.3, 0.15, -0.15, 0, 0],
    gap: function(l) {
      return l.pstyle("width").pfValue * l.pstyle("arrow-scale").value;
    }
  }), s("chevron", {
    points: [0, 0, -0.15, -0.15, -0.1, -0.2, 0, -0.1, 0.1, -0.2, 0.15, -0.15],
    gap: function(l) {
      return 0.95 * l.pstyle("width").pfValue * l.pstyle("arrow-scale").value;
    }
  });
};
var Or = {};
Or.projectIntoViewport = function(t, e) {
  var r = this.cy, a = this.findContainerClientCoords(), n = a[0], i = a[1], o = a[4], s = r.pan(), u = r.zoom(), l = ((t - n) / o - s.x) / u, c = ((e - i) / o - s.y) / u;
  return [l, c];
};
Or.findContainerClientCoords = function() {
  if (this.containerBB)
    return this.containerBB;
  var t = this.container, e = t.getBoundingClientRect(), r = this.cy.window().getComputedStyle(t), a = function(p) {
    return parseFloat(r.getPropertyValue(p));
  }, n = {
    left: a("padding-left"),
    right: a("padding-right"),
    top: a("padding-top"),
    bottom: a("padding-bottom")
  }, i = {
    left: a("border-left-width"),
    right: a("border-right-width"),
    top: a("border-top-width"),
    bottom: a("border-bottom-width")
  }, o = t.clientWidth, s = t.clientHeight, u = n.left + n.right, l = n.top + n.bottom, c = i.left + i.right, f = e.width / (o + c), v = o - u, h = s - l, d = e.left + n.left + i.left, y = e.top + n.top + i.top;
  return this.containerBB = [d, y, v, h, f];
};
Or.invalidateContainerClientCoordsCache = function() {
  this.containerBB = null;
};
Or.findNearestElement = function(t, e, r, a) {
  return this.findNearestElements(t, e, r, a)[0];
};
Or.findNearestElements = function(t, e, r, a) {
  var n = this, i = this, o = i.getCachedZSortedEles(), s = [], u = i.cy.zoom(), l = i.cy.hasCompoundNodes(), c = (a ? 24 : 8) / u, f = (a ? 8 : 2) / u, v = (a ? 8 : 2) / u, h = 1 / 0, d, y;
  r && (o = o.interactive);
  function g(x, T) {
    if (x.isNode()) {
      if (y)
        return;
      y = x, s.push(x);
    }
    if (x.isEdge() && (T == null || T < h))
      if (d) {
        if (d.pstyle("z-compound-depth").value === x.pstyle("z-compound-depth").value && d.pstyle("z-compound-depth").value === x.pstyle("z-compound-depth").value) {
          for (var k = 0; k < s.length; k++)
            if (s[k].isEdge()) {
              s[k] = x, d = x, h = T ?? h;
              break;
            }
        }
      } else
        s.push(x), d = x, h = T ?? h;
  }
  function p(x) {
    var T = x.outerWidth() + 2 * f, k = x.outerHeight() + 2 * f, D = T / 2, P = k / 2, A = x.position(), B = x.pstyle("corner-radius").value === "auto" ? "auto" : x.pstyle("corner-radius").pfValue, R = x._private.rscratch;
    if (A.x - D <= t && t <= A.x + D && A.y - P <= e && e <= A.y + P) {
      var I = i.nodeShapes[n.getNodeShape(x)];
      if (I.checkPoint(t, e, 0, T, k, A.x, A.y, B, R))
        return g(x, 0), !0;
    }
  }
  function m(x) {
    var T = x._private, k = T.rscratch, D = x.pstyle("width").pfValue, P = x.pstyle("arrow-scale").value, A = D / 2 + c, B = A * A, R = A * 2, N = T.source, H = T.target, I;
    if (k.edgeType === "segments" || k.edgeType === "straight" || k.edgeType === "haystack") {
      for (var O = k.allpts, M = 0; M + 3 < O.length; M += 2)
        if (Kh(t, e, O[M], O[M + 1], O[M + 2], O[M + 3], R) && B > (I = Jh(t, e, O[M], O[M + 1], O[M + 2], O[M + 3])))
          return g(x, I), !0;
    } else if (k.edgeType === "bezier" || k.edgeType === "multibezier" || k.edgeType === "self" || k.edgeType === "compound") {
      for (var O = k.allpts, M = 0; M + 5 < k.allpts.length; M += 4)
        if (Xh(t, e, O[M], O[M + 1], O[M + 2], O[M + 3], O[M + 4], O[M + 5], R) && B > (I = Qh(t, e, O[M], O[M + 1], O[M + 2], O[M + 3], O[M + 4], O[M + 5])))
          return g(x, I), !0;
    }
    for (var N = N || T.source, H = H || T.target, G = n.getArrowWidth(D, P), F = [{
      name: "source",
      x: k.arrowStartX,
      y: k.arrowStartY,
      angle: k.srcArrowAngle
    }, {
      name: "target",
      x: k.arrowEndX,
      y: k.arrowEndY,
      angle: k.tgtArrowAngle
    }, {
      name: "mid-source",
      x: k.midX,
      y: k.midY,
      angle: k.midsrcArrowAngle
    }, {
      name: "mid-target",
      x: k.midX,
      y: k.midY,
      angle: k.midtgtArrowAngle
    }], M = 0; M < F.length; M++) {
      var _ = F[M], X = i.arrowShapes[x.pstyle(_.name + "-arrow-shape").value], J = x.pstyle("width").pfValue;
      if (X.roughCollide(t, e, G, _.angle, {
        x: _.x,
        y: _.y
      }, J, c) && X.collide(t, e, G, _.angle, {
        x: _.x,
        y: _.y
      }, J, c))
        return g(x), !0;
    }
    l && s.length > 0 && (p(N), p(H));
  }
  function b(x, T, k) {
    return bt(x, T, k);
  }
  function w(x, T) {
    var k = x._private, D = v, P;
    T ? P = T + "-" : P = "", x.boundingBox();
    var A = k.labelBounds[T || "main"], B = x.pstyle(P + "label").value, R = x.pstyle("text-events").strValue === "yes";
    if (!(!R || !B)) {
      var I = b(k.rscratch, "labelX", T), O = b(k.rscratch, "labelY", T), M = b(k.rscratch, "labelAngle", T), N = x.pstyle(P + "text-margin-x").pfValue, H = x.pstyle(P + "text-margin-y").pfValue, G = A.x1 - D - N, F = A.x2 + D - N, _ = A.y1 - D - H, X = A.y2 + D - H;
      if (M) {
        var J = Math.cos(M), Y = Math.sin(M), j = function(K, re) {
          return K = K - I, re = re - O, {
            x: K * J - re * Y + I,
            y: K * Y + re * J + O
          };
        }, te = j(G, _), ae = j(G, X), z = j(F, _), L = j(F, X), V = [
          // with the margin added after the rotation is applied
          te.x + N,
          te.y + H,
          z.x + N,
          z.y + H,
          L.x + N,
          L.y + H,
          ae.x + N,
          ae.y + H
        ];
        if (Tt(t, e, V))
          return g(x), !0;
      } else if (or(A, t, e))
        return g(x), !0;
    }
  }
  for (var C = o.length - 1; C >= 0; C--) {
    var E = o[C];
    E.isNode() ? p(E) || w(E) : m(E) || w(E) || w(E, "source") || w(E, "target");
  }
  return s;
};
Or.getAllInBox = function(t, e, r, a) {
  var n = this.getCachedZSortedEles().interactive, i = this.cy.zoom(), o = 2 / i, s = [], u = Math.min(t, r), l = Math.max(t, r), c = Math.min(e, a), f = Math.max(e, a);
  t = u, r = l, e = c, a = f;
  var v = wt({
    x1: t,
    y1: e,
    x2: r,
    y2: a
  }), h = [{
    x: v.x1,
    y: v.y1
  }, {
    x: v.x2,
    y: v.y1
  }, {
    x: v.x2,
    y: v.y2
  }, {
    x: v.x1,
    y: v.y2
  }], d = [[h[0], h[1]], [h[1], h[2]], [h[2], h[3]], [h[3], h[0]]];
  function y(K, re, le) {
    return bt(K, re, le);
  }
  function g(K, re) {
    var le = K._private, Pe = o, we = "";
    K.boundingBox();
    var me = le.labelBounds.main;
    if (!me)
      return null;
    var ge = y(le.rscratch, "labelX", re), oe = y(le.rscratch, "labelY", re), de = y(le.rscratch, "labelAngle", re), ve = K.pstyle(we + "text-margin-x").pfValue, xe = K.pstyle(we + "text-margin-y").pfValue, be = me.x1 - Pe - ve, Se = me.x2 + Pe - ve, De = me.y1 - Pe - xe, Oe = me.y2 + Pe - xe;
    if (de) {
      var Ie = Math.cos(de), Be = Math.sin(de), Q = function(q, $) {
        return q = q - ge, $ = $ - oe, {
          x: q * Ie - $ * Be + ge,
          y: q * Be + $ * Ie + oe
        };
      };
      return [Q(be, De), Q(Se, De), Q(Se, Oe), Q(be, Oe)];
    } else
      return [{
        x: be,
        y: De
      }, {
        x: Se,
        y: De
      }, {
        x: Se,
        y: Oe
      }, {
        x: be,
        y: Oe
      }];
  }
  function p(K, re, le, Pe) {
    function we(me, ge, oe) {
      return (oe.y - me.y) * (ge.x - me.x) > (ge.y - me.y) * (oe.x - me.x);
    }
    return we(K, le, Pe) !== we(re, le, Pe) && we(K, re, le) !== we(K, re, Pe);
  }
  for (var m = 0; m < n.length; m++) {
    var b = n[m];
    if (b.isNode()) {
      var w = b, C = w.pstyle("text-events").strValue === "yes", E = w.pstyle("box-selection").strValue, x = w.pstyle("box-select-labels").strValue === "yes";
      if (E === "none")
        continue;
      var T = (E === "overlap" || x) && C, k = w.boundingBox({
        includeNodes: !0,
        includeEdges: !1,
        includeLabels: T
      });
      if (E === "contain") {
        var D = !1;
        if (x && C) {
          var P = g(w);
          P && Di(P, h) && (s.push(w), D = !0);
        }
        !D && qc(v, k) && s.push(w);
      } else if (E === "overlap" && bs(v, k)) {
        var A = w.boundingBox({
          includeNodes: !0,
          includeEdges: !0,
          includeLabels: !1,
          includeMainLabels: !1,
          includeSourceLabels: !1,
          includeTargetLabels: !1
        }), B = [{
          x: A.x1,
          y: A.y1
        }, {
          x: A.x2,
          y: A.y1
        }, {
          x: A.x2,
          y: A.y2
        }, {
          x: A.x1,
          y: A.y2
        }];
        if (Di(B, h))
          s.push(w);
        else {
          var R = g(w);
          R && Di(R, h) && s.push(w);
        }
      }
    } else {
      var I = b, O = I._private, M = O.rscratch, N = I.pstyle("box-selection").strValue;
      if (N === "none")
        continue;
      if (N === "contain") {
        if (M.startX != null && M.startY != null && !or(v, M.startX, M.startY) || M.endX != null && M.endY != null && !or(v, M.endX, M.endY))
          continue;
        if (M.edgeType === "bezier" || M.edgeType === "multibezier" || M.edgeType === "self" || M.edgeType === "compound" || M.edgeType === "segments" || M.edgeType === "haystack") {
          for (var H = O.rstyle.bezierPts || O.rstyle.linePts || O.rstyle.haystackPts, G = !0, F = 0; F < H.length; F++)
            if (!vu(v, H[F])) {
              G = !1;
              break;
            }
          G && s.push(I);
        } else M.edgeType === "straight" && s.push(I);
      } else if (N === "overlap") {
        var _ = !1;
        if (M.startX != null && M.startY != null && M.endX != null && M.endY != null && (or(v, M.startX, M.startY) || or(v, M.endX, M.endY)))
          s.push(I), _ = !0;
        else if (!_ && M.edgeType === "haystack") {
          for (var X = O.rstyle.haystackPts, J = 0; J < X.length; J++)
            if (vu(v, X[J])) {
              s.push(I), _ = !0;
              break;
            }
        }
        if (!_) {
          var Y = O.rstyle.bezierPts || O.rstyle.linePts || O.rstyle.haystackPts;
          if ((!Y || Y.length < 2) && M.edgeType === "straight" && M.startX != null && M.startY != null && M.endX != null && M.endY != null && (Y = [{
            x: M.startX,
            y: M.startY
          }, {
            x: M.endX,
            y: M.endY
          }]), !Y || Y.length < 2) continue;
          for (var j = 0; j < Y.length - 1; j++) {
            for (var te = Y[j], ae = Y[j + 1], z = 0; z < d.length; z++) {
              var L = Je(d[z], 2), V = L[0], W = L[1];
              if (p(te, ae, V, W)) {
                s.push(I), _ = !0;
                break;
              }
            }
            if (_) break;
          }
        }
      }
    }
  }
  return s;
};
var zn = {};
zn.calculateArrowAngles = function(t) {
  var e = t._private.rscratch, r = e.edgeType === "haystack", a = e.edgeType === "bezier", n = e.edgeType === "multibezier", i = e.edgeType === "segments", o = e.edgeType === "compound", s = e.edgeType === "self", u, l, c, f, v, h, p, m;
  if (r ? (c = e.haystackPts[0], f = e.haystackPts[1], v = e.haystackPts[2], h = e.haystackPts[3]) : (c = e.arrowStartX, f = e.arrowStartY, v = e.arrowEndX, h = e.arrowEndY), p = e.midX, m = e.midY, i)
    u = c - e.segpts[0], l = f - e.segpts[1];
  else if (n || o || s || a) {
    var d = e.allpts, y = it(d[0], d[2], d[4], 0.1), g = it(d[1], d[3], d[5], 0.1);
    u = c - y, l = f - g;
  } else
    u = c - p, l = f - m;
  e.srcArrowAngle = tn(u, l);
  var p = e.midX, m = e.midY;
  if (r && (p = (c + v) / 2, m = (f + h) / 2), u = v - c, l = h - f, i) {
    var d = e.allpts;
    if (d.length / 2 % 2 === 0) {
      var b = d.length / 2, w = b - 2;
      u = d[b] - d[w], l = d[b + 1] - d[w + 1];
    } else if (e.isRound)
      u = e.midVector[1], l = -e.midVector[0];
    else {
      var b = d.length / 2 - 1, w = b - 2;
      u = d[b] - d[w], l = d[b + 1] - d[w + 1];
    }
  } else if (n || o || s) {
    var d = e.allpts, C = e.ctrlpts, E, x, T, k;
    if (C.length / 2 % 2 === 0) {
      var D = d.length / 2 - 1, P = D + 2, A = P + 2;
      E = it(d[D], d[P], d[A], 0), x = it(d[D + 1], d[P + 1], d[A + 1], 0), T = it(d[D], d[P], d[A], 1e-4), k = it(d[D + 1], d[P + 1], d[A + 1], 1e-4);
    } else {
      var P = d.length / 2 - 1, D = P - 2, A = P + 2;
      E = it(d[D], d[P], d[A], 0.4999), x = it(d[D + 1], d[P + 1], d[A + 1], 0.4999), T = it(d[D], d[P], d[A], 0.5), k = it(d[D + 1], d[P + 1], d[A + 1], 0.5);
    }
    u = T - E, l = k - x;
  }
  if (e.midtgtArrowAngle = tn(u, l), e.midDispX = u, e.midDispY = l, u *= -1, l *= -1, i) {
    var d = e.allpts;
    if (d.length / 2 % 2 !== 0) {
      if (!e.isRound) {
        var b = d.length / 2 - 1, B = b + 2;
        u = -(d[B] - d[b]), l = -(d[B + 1] - d[b + 1]);
      }
    }
  }
  if (e.midsrcArrowAngle = tn(u, l), i)
    u = v - e.segpts[e.segpts.length - 2], l = h - e.segpts[e.segpts.length - 1];
  else if (n || o || s || a) {
    var d = e.allpts, R = d.length, y = it(d[R - 6], d[R - 4], d[R - 2], 0.9), g = it(d[R - 5], d[R - 3], d[R - 1], 0.9);
    u = v - y, l = h - g;
  } else
    u = v - p, l = h - m;
  e.tgtArrowAngle = tn(u, l);
};
zn.getArrowWidth = zn.getArrowHeight = function(t, e) {
  var r = this.arrowWidthCache = this.arrowWidthCache || {}, a = r[t + ", " + e];
  return a || (a = Math.max(Math.pow(t * 13.37, 0.9), 29) * e, r[t + ", " + e] = a, a);
};
var Jo, jo, Gt = {}, Dt = {}, Xl, Yl, Pr, xn, Yt, Er, Sr, Vt, Gr, cn, Lf, Of, es, ts, Zl, Ql = function(e, r, a) {
  a.x = r.x - e.x, a.y = r.y - e.y, a.len = Math.sqrt(a.x * a.x + a.y * a.y), a.nx = a.x / a.len, a.ny = a.y / a.len, a.ang = Math.atan2(a.ny, a.nx);
}, gm = function(e, r) {
  r.x = e.x * -1, r.y = e.y * -1, r.nx = e.nx * -1, r.ny = e.ny * -1, r.ang = e.ang > 0 ? -(Math.PI - e.ang) : Math.PI + e.ang;
}, pm = function(e, r, a, n, i) {
  if (e !== Zl ? Ql(r, e, Gt) : gm(Dt, Gt), Ql(r, a, Dt), Xl = Gt.nx * Dt.ny - Gt.ny * Dt.nx, Yl = Gt.nx * Dt.nx - Gt.ny * -Dt.ny, Yt = Math.asin(Math.max(-1, Math.min(1, Xl))), Math.abs(Yt) < 1e-6) {
    Jo = r.x, jo = r.y, Sr = Gr = 0;
    return;
  }
  Pr = 1, xn = !1, Yl < 0 ? Yt < 0 ? Yt = Math.PI + Yt : (Yt = Math.PI - Yt, Pr = -1, xn = !0) : Yt > 0 && (Pr = -1, xn = !0), r.radius !== void 0 ? Gr = r.radius : Gr = n, Er = Yt / 2, cn = Math.min(Gt.len / 2, Dt.len / 2), i ? (Vt = Math.abs(Math.cos(Er) * Gr / Math.sin(Er)), Vt > cn ? (Vt = cn, Sr = Math.abs(Vt * Math.sin(Er) / Math.cos(Er))) : Sr = Gr) : (Vt = Math.min(cn, Gr), Sr = Math.abs(Vt * Math.sin(Er) / Math.cos(Er))), es = r.x + Dt.nx * Vt, ts = r.y + Dt.ny * Vt, Jo = es - Dt.ny * Sr * Pr, jo = ts + Dt.nx * Sr * Pr, Lf = r.x + Gt.nx * Vt, Of = r.y + Gt.ny * Vt, Zl = r;
};
function Nf(t, e) {
  e.radius === 0 ? t.lineTo(e.cx, e.cy) : t.arc(e.cx, e.cy, e.radius, e.startAngle, e.endAngle, e.counterClockwise);
}
function Rs(t, e, r, a) {
  var n = arguments.length > 4 && arguments[4] !== void 0 ? arguments[4] : !0;
  return a === 0 || e.radius === 0 ? {
    cx: e.x,
    cy: e.y,
    radius: 0,
    startX: e.x,
    startY: e.y,
    stopX: e.x,
    stopY: e.y,
    startAngle: void 0,
    endAngle: void 0,
    counterClockwise: void 0
  } : (pm(t, e, r, a, n), {
    cx: Jo,
    cy: jo,
    radius: Sr,
    startX: Lf,
    startY: Of,
    stopX: es,
    stopY: ts,
    startAngle: Gt.ang + Math.PI / 2 * Pr,
    endAngle: Dt.ang - Math.PI / 2 * Pr,
    counterClockwise: xn
  });
}
var za = 0.01, ym = Math.sqrt(2 * za), pt = {};
pt.findMidptPtsEtc = function(t, e) {
  var r = e.posPts, a = e.intersectionPts, n = e.vectorNormInverse, i, o = t.pstyle("source-endpoint"), s = t.pstyle("target-endpoint"), u = o.units != null && s.units != null, l = function(C, E, x, T) {
    var k = T - E, D = x - C, P = Math.sqrt(D * D + k * k);
    return {
      x: -k / P,
      y: D / P
    };
  }, c = t.pstyle("edge-distances").value;
  switch (c) {
    case "node-position":
      i = r;
      break;
    case "intersection":
      i = a;
      break;
    case "endpoints": {
      if (u) {
        var f = this.manualEndptToPx(t.source()[0], o), v = Je(f, 2), h = v[0], d = v[1], y = this.manualEndptToPx(t.target()[0], s), g = Je(y, 2), p = g[0], m = g[1], b = {
          x1: h,
          y1: d,
          x2: p,
          y2: m
        };
        n = l(h, d, p, m), i = b;
      } else
        ze("Edge ".concat(t.id(), " has edge-distances:endpoints specified without manual endpoints specified via source-endpoint and target-endpoint.  Falling back on edge-distances:intersection (default).")), i = a;
      break;
    }
  }
  return {
    midptPts: i,
    vectorNormInverse: n
  };
};
pt.findHaystackPoints = function(t) {
  for (var e = 0; e < t.length; e++) {
    var r = t[e], a = r._private, n = a.rscratch;
    if (!n.haystack) {
      var i = Math.random() * 2 * Math.PI;
      n.source = {
        x: Math.cos(i),
        y: Math.sin(i)
      }, i = Math.random() * 2 * Math.PI, n.target = {
        x: Math.cos(i),
        y: Math.sin(i)
      };
    }
    var o = a.source, s = a.target, u = o.position(), l = s.position(), c = o.width(), f = s.width(), v = o.height(), h = s.height(), d = r.pstyle("haystack-radius").value, y = d / 2;
    n.haystackPts = n.allpts = [n.source.x * c * y + u.x, n.source.y * v * y + u.y, n.target.x * f * y + l.x, n.target.y * h * y + l.y], n.midX = (n.allpts[0] + n.allpts[2]) / 2, n.midY = (n.allpts[1] + n.allpts[3]) / 2, n.edgeType = "haystack", n.haystack = !0, this.storeEdgeProjections(r), this.calculateArrowAngles(r), this.recalculateEdgeLabelProjections(r), this.calculateLabelAngles(r);
  }
};
pt.findSegmentsPoints = function(t, e) {
  var r = t._private.rscratch, a = t.pstyle("segment-weights"), n = t.pstyle("segment-distances"), i = t.pstyle("segment-radii"), o = t.pstyle("radius-type"), s = Math.min(a.pfValue.length, n.pfValue.length), u = i.pfValue[i.pfValue.length - 1], l = o.pfValue[o.pfValue.length - 1];
  r.edgeType = "segments", r.segpts = [], r.radii = [], r.isArcRadius = [];
  for (var c = 0; c < s; c++) {
    var f = a.pfValue[c], v = n.pfValue[c], h = 1 - f, d = f, y = this.findMidptPtsEtc(t, e), g = y.midptPts, p = y.vectorNormInverse, m = {
      x: g.x1 * h + g.x2 * d,
      y: g.y1 * h + g.y2 * d
    };
    r.segpts.push(m.x + p.x * v, m.y + p.y * v), r.radii.push(i.pfValue[c] !== void 0 ? i.pfValue[c] : u), r.isArcRadius.push((o.pfValue[c] !== void 0 ? o.pfValue[c] : l) === "arc-radius");
  }
};
pt.findLoopPoints = function(t, e, r, a) {
  var n = t._private.rscratch, i = e.dirCounts, o = e.srcPos, s = t.pstyle("control-point-distances"), u = s ? s.pfValue[0] : void 0, l = t.pstyle("loop-direction").pfValue, c = t.pstyle("loop-sweep").pfValue, f = t.pstyle("control-point-step-size").pfValue;
  n.edgeType = "self";
  var v = r, h = f;
  a && (v = 0, h = u);
  var d = l - Math.PI / 2, y = d - c / 2, g = d + c / 2, p = l + "_" + c;
  v = i[p] === void 0 ? i[p] = 0 : ++i[p], n.ctrlpts = [o.x + Math.cos(y) * 1.4 * h * (v / 3 + 1), o.y + Math.sin(y) * 1.4 * h * (v / 3 + 1), o.x + Math.cos(g) * 1.4 * h * (v / 3 + 1), o.y + Math.sin(g) * 1.4 * h * (v / 3 + 1)];
};
pt.findCompoundLoopPoints = function(t, e, r, a) {
  var n = t._private.rscratch;
  n.edgeType = "compound";
  var i = e.srcPos, o = e.tgtPos, s = e.srcW, u = e.srcH, l = e.tgtW, c = e.tgtH, f = t.pstyle("control-point-step-size").pfValue, v = t.pstyle("control-point-distances"), h = v ? v.pfValue[0] : void 0, d = r, y = f;
  a && (d = 0, y = h);
  var g = 50, p = {
    x: i.x - s / 2,
    y: i.y - u / 2
  }, m = {
    x: o.x - l / 2,
    y: o.y - c / 2
  }, b = {
    x: Math.min(p.x, m.x),
    y: Math.min(p.y, m.y)
  }, w = 0.5, C = Math.max(w, Math.log(s * za)), E = Math.max(w, Math.log(l * za));
  n.ctrlpts = [b.x, b.y - (1 + Math.pow(g, 1.12) / 100) * y * (d / 3 + 1) * C, b.x - (1 + Math.pow(g, 1.12) / 100) * y * (d / 3 + 1) * E, b.y];
};
pt.findStraightEdgePoints = function(t) {
  t._private.rscratch.edgeType = "straight";
};
pt.findBezierPoints = function(t, e, r, a, n) {
  var i = t._private.rscratch, o = t.pstyle("control-point-step-size").pfValue, s = t.pstyle("control-point-distances"), u = t.pstyle("control-point-weights"), l = s && u ? Math.min(s.value.length, u.value.length) : 1, c = s ? s.pfValue[0] : void 0, f = u.value[0], v = a;
  i.edgeType = v ? "multibezier" : "bezier", i.ctrlpts = [];
  for (var h = 0; h < l; h++) {
    var d = (0.5 - e.eles.length / 2 + r) * o * (n ? -1 : 1), y = void 0, g = ms(d);
    v && (c = s ? s.pfValue[h] : o, f = u.value[h]), a ? y = c : y = c !== void 0 ? g * c : void 0;
    var p = y !== void 0 ? y : d, m = 1 - f, b = f, w = this.findMidptPtsEtc(t, e), C = w.midptPts, E = w.vectorNormInverse, x = {
      x: C.x1 * m + C.x2 * b,
      y: C.y1 * m + C.y2 * b
    };
    i.ctrlpts.push(x.x + E.x * p, x.y + E.y * p);
  }
};
pt.findTaxiPoints = function(t, e) {
  var r = t._private.rscratch;
  r.edgeType = "segments";
  var a = "vertical", n = "horizontal", i = "leftward", o = "rightward", s = "downward", u = "upward", l = "auto", c = e.posPts, f = e.srcW, v = e.srcH, h = e.tgtW, d = e.tgtH, y = t.pstyle("edge-distances").value, g = y !== "node-position", p = t.pstyle("taxi-direction").value, m = p, b = t.pstyle("taxi-turn"), w = b.units === "%", C = b.pfValue, E = C < 0, x = t.pstyle("taxi-turn-min-distance").pfValue, T = g ? (f + h) / 2 : 0, k = g ? (v + d) / 2 : 0, D = c.x2 - c.x1, P = c.y2 - c.y1, A = function($, U) {
    return $ > 0 ? Math.max($ - U, 0) : Math.min($ + U, 0);
  }, B = A(D, T), R = A(P, k), I = !1;
  m === l ? p = Math.abs(B) > Math.abs(R) ? n : a : m === u || m === s ? (p = a, I = !0) : (m === i || m === o) && (p = n, I = !0);
  var O = p === a, M = O ? R : B, N = O ? P : D, H = ms(N), G = !1;
  !(I && (w || E)) && (m === s && N < 0 || m === u && N > 0 || m === i && N > 0 || m === o && N < 0) && (H *= -1, M = H * Math.abs(M), G = !0);
  var F;
  if (w) {
    var _ = C < 0 ? 1 + C : C;
    F = _ * M;
  } else {
    var X = C < 0 ? M : 0;
    F = X + C * H;
  }
  var J = function($) {
    return Math.abs($) < x || Math.abs($) >= Math.abs(M);
  }, Y = J(F), j = J(Math.abs(M) - Math.abs(F)), te = Y || j;
  if (te && !G)
    if (O) {
      var ae = Math.abs(N) <= v / 2, z = Math.abs(D) <= h / 2;
      if (ae) {
        var L = (c.x1 + c.x2) / 2, V = c.y1, W = c.y2;
        r.segpts = [L, V, L, W];
      } else if (z) {
        var K = (c.y1 + c.y2) / 2, re = c.x1, le = c.x2;
        r.segpts = [re, K, le, K];
      } else
        r.segpts = [c.x1, c.y2];
    } else {
      var Pe = Math.abs(N) <= f / 2, we = Math.abs(P) <= d / 2;
      if (Pe) {
        var me = (c.y1 + c.y2) / 2, ge = c.x1, oe = c.x2;
        r.segpts = [ge, me, oe, me];
      } else if (we) {
        var de = (c.x1 + c.x2) / 2, ve = c.y1, xe = c.y2;
        r.segpts = [de, ve, de, xe];
      } else
        r.segpts = [c.x2, c.y1];
    }
  else if (O) {
    var be = c.y1 + F + (g ? v / 2 * H : 0), Se = c.x1, De = c.x2;
    r.segpts = [Se, be, De, be];
  } else {
    var Oe = c.x1 + F + (g ? f / 2 * H : 0), Ie = c.y1, Be = c.y2;
    r.segpts = [Oe, Ie, Oe, Be];
  }
  if (r.isRound) {
    var Q = t.pstyle("taxi-radius").value, S = t.pstyle("radius-type").value[0] === "arc-radius";
    r.radii = new Array(r.segpts.length / 2).fill(Q), r.isArcRadius = new Array(r.segpts.length / 2).fill(S);
  }
};
pt.tryToCorrectInvalidPoints = function(t, e) {
  var r = t._private.rscratch;
  if (r.edgeType === "bezier") {
    var a = e.srcPos, n = e.tgtPos, i = e.srcW, o = e.srcH, s = e.tgtW, u = e.tgtH, l = e.srcShape, c = e.tgtShape, f = e.srcCornerRadius, v = e.tgtCornerRadius, h = e.srcRs, d = e.tgtRs, y = !ne(r.startX) || !ne(r.startY), g = !ne(r.arrowStartX) || !ne(r.arrowStartY), p = !ne(r.endX) || !ne(r.endY), m = !ne(r.arrowEndX) || !ne(r.arrowEndY), b = 3, w = this.getArrowWidth(t.pstyle("width").pfValue, t.pstyle("arrow-scale").value) * this.arrowShapeWidth, C = b * w, E = Br({
      x: r.ctrlpts[0],
      y: r.ctrlpts[1]
    }, {
      x: r.startX,
      y: r.startY
    }), x = E < C, T = Br({
      x: r.ctrlpts[0],
      y: r.ctrlpts[1]
    }, {
      x: r.endX,
      y: r.endY
    }), k = T < C, D = !1;
    if (y || g || x) {
      D = !0;
      var P = {
        // delta
        x: r.ctrlpts[0] - a.x,
        y: r.ctrlpts[1] - a.y
      }, A = Math.sqrt(P.x * P.x + P.y * P.y), B = {
        // normalised delta
        x: P.x / A,
        y: P.y / A
      }, R = Math.max(i, o), I = {
        // *2 radius guarantees outside shape
        x: r.ctrlpts[0] + B.x * 2 * R,
        y: r.ctrlpts[1] + B.y * 2 * R
      }, O = l.intersectLine(a.x, a.y, i, o, I.x, I.y, 0, f, h);
      x ? (r.ctrlpts[0] = r.ctrlpts[0] + B.x * (C - E), r.ctrlpts[1] = r.ctrlpts[1] + B.y * (C - E)) : (r.ctrlpts[0] = O[0] + B.x * C, r.ctrlpts[1] = O[1] + B.y * C);
    }
    if (p || m || k) {
      D = !0;
      var M = {
        // delta
        x: r.ctrlpts[0] - n.x,
        y: r.ctrlpts[1] - n.y
      }, N = Math.sqrt(M.x * M.x + M.y * M.y), H = {
        // normalised delta
        x: M.x / N,
        y: M.y / N
      }, G = Math.max(i, o), F = {
        // *2 radius guarantees outside shape
        x: r.ctrlpts[0] + H.x * 2 * G,
        y: r.ctrlpts[1] + H.y * 2 * G
      }, _ = c.intersectLine(n.x, n.y, s, u, F.x, F.y, 0, v, d);
      k ? (r.ctrlpts[0] = r.ctrlpts[0] + H.x * (C - T), r.ctrlpts[1] = r.ctrlpts[1] + H.y * (C - T)) : (r.ctrlpts[0] = _[0] + H.x * C, r.ctrlpts[1] = _[1] + H.y * C);
    }
    D && this.findEndpoints(t);
  }
};
pt.storeAllpts = function(t) {
  var e = t._private.rscratch;
  if (e.edgeType === "multibezier" || e.edgeType === "bezier" || e.edgeType === "self" || e.edgeType === "compound") {
    e.allpts = [], e.allpts.push(e.startX, e.startY);
    for (var r = 0; r + 1 < e.ctrlpts.length; r += 2)
      e.allpts.push(e.ctrlpts[r], e.ctrlpts[r + 1]), r + 3 < e.ctrlpts.length && e.allpts.push((e.ctrlpts[r] + e.ctrlpts[r + 2]) / 2, (e.ctrlpts[r + 1] + e.ctrlpts[r + 3]) / 2);
    e.allpts.push(e.endX, e.endY);
    var a, n;
    e.ctrlpts.length / 2 % 2 === 0 ? (a = e.allpts.length / 2 - 1, e.midX = e.allpts[a], e.midY = e.allpts[a + 1]) : (a = e.allpts.length / 2 - 3, n = 0.5, e.midX = it(e.allpts[a], e.allpts[a + 2], e.allpts[a + 4], n), e.midY = it(e.allpts[a + 1], e.allpts[a + 3], e.allpts[a + 5], n));
  } else if (e.edgeType === "straight")
    e.allpts = [e.startX, e.startY, e.endX, e.endY], e.midX = (e.startX + e.endX + e.arrowStartX + e.arrowEndX) / 4, e.midY = (e.startY + e.endY + e.arrowStartY + e.arrowEndY) / 4;
  else if (e.edgeType === "segments") {
    if (e.allpts = [], e.allpts.push(e.startX, e.startY), e.allpts.push.apply(e.allpts, e.segpts), e.allpts.push(e.endX, e.endY), e.isRound) {
      e.roundCorners = [];
      for (var i = 2; i + 3 < e.allpts.length; i += 2) {
        var o = e.radii[i / 2 - 1], s = e.isArcRadius[i / 2 - 1];
        e.roundCorners.push(Rs({
          x: e.allpts[i - 2],
          y: e.allpts[i - 1]
        }, {
          x: e.allpts[i],
          y: e.allpts[i + 1],
          radius: o
        }, {
          x: e.allpts[i + 2],
          y: e.allpts[i + 3]
        }, o, s));
      }
    }
    if (e.segpts.length % 4 === 0) {
      var u = e.segpts.length / 2, l = u - 2;
      e.midX = (e.segpts[l] + e.segpts[u]) / 2, e.midY = (e.segpts[l + 1] + e.segpts[u + 1]) / 2;
    } else {
      var c = e.segpts.length / 2 - 1;
      if (!e.isRound)
        e.midX = e.segpts[c], e.midY = e.segpts[c + 1];
      else {
        var f = {
          x: e.segpts[c],
          y: e.segpts[c + 1]
        }, v = e.roundCorners[c / 2];
        if (v.radius === 0) {
          var h = {
            x: e.segpts[c + 2],
            y: e.segpts[c + 3]
          };
          e.midX = f.x, e.midY = f.y, e.midVector = [f.y - h.y, h.x - f.x];
        } else {
          var d = [f.x - v.cx, f.y - v.cy], y = v.radius / Math.sqrt(Math.pow(d[0], 2) + Math.pow(d[1], 2));
          d = d.map(function(g) {
            return g * y;
          }), e.midX = v.cx + d[0], e.midY = v.cy + d[1], e.midVector = d;
        }
      }
    }
  }
};
pt.checkForInvalidEdgeWarning = function(t) {
  var e = t[0]._private.rscratch;
  e.nodesOverlap || ne(e.startX) && ne(e.startY) && ne(e.endX) && ne(e.endY) ? e.loggedErr = !1 : e.loggedErr || (e.loggedErr = !0, ze("Edge `" + t.id() + "` has invalid endpoints and so it is impossible to draw.  Adjust your edge style (e.g. control points) accordingly or use an alternative edge type.  This is expected behaviour when the source node and the target node overlap."));
};
pt.findEdgeControlPoints = function(t) {
  var e = this;
  if (!(!t || t.length === 0)) {
    for (var r = this, a = r.cy, n = a.hasCompoundNodes(), i = new Zt(), o = function(k, D) {
      return [].concat(Pn(k), [D ? 1 : 0]).join("-");
    }, s = [], u = [], l = 0; l < t.length; l++) {
      var c = t[l], f = c._private, v = c.pstyle("curve-style").value;
      if (!(c.removed() || !c.takesUpSpace())) {
        if (v === "haystack") {
          u.push(c);
          continue;
        }
        var h = v === "unbundled-bezier" || ir(v, "segments") || v === "straight" || v === "straight-triangle" || ir(v, "taxi"), d = v === "unbundled-bezier" || v === "bezier", y = f.source, g = f.target, p = y.poolIndex(), m = g.poolIndex(), b = [p, m].sort(), w = o(b, h), C = i.get(w);
        C == null && (C = {
          eles: []
        }, s.push({
          pairId: b,
          edgeIsUnbundled: h
        }), i.set(w, C)), C.eles.push(c), h && (C.hasUnbundled = !0), d && (C.hasBezier = !0);
      }
    }
    for (var E = function() {
      var k = s[x], D = k.pairId, P = k.edgeIsUnbundled, A = o(D, P), B = i.get(A), R;
      if (!B.hasUnbundled) {
        var I = B.eles[0].parallelEdges().filter(function(S) {
          return S.isBundledBezier();
        });
        ps(B.eles), I.forEach(function(S) {
          return B.eles.push(S);
        }), B.eles.sort(function(S, q) {
          return S.poolIndex() - q.poolIndex();
        });
      }
      var O = B.eles[0], M = O.source(), N = O.target();
      if (M.poolIndex() > N.poolIndex()) {
        var H = M;
        M = N, N = H;
      }
      var G = B.srcPos = M.position(), F = B.tgtPos = N.position(), _ = B.srcW = M.outerWidth(), X = B.srcH = M.outerHeight(), J = B.tgtW = N.outerWidth(), Y = B.tgtH = N.outerHeight(), j = B.srcShape = r.nodeShapes[e.getNodeShape(M)], te = B.tgtShape = r.nodeShapes[e.getNodeShape(N)], ae = B.srcCornerRadius = M.pstyle("corner-radius").value === "auto" ? "auto" : M.pstyle("corner-radius").pfValue, z = B.tgtCornerRadius = N.pstyle("corner-radius").value === "auto" ? "auto" : N.pstyle("corner-radius").pfValue, L = B.tgtRs = N._private.rscratch, V = B.srcRs = M._private.rscratch;
      B.dirCounts = {
        north: 0,
        west: 0,
        south: 0,
        east: 0,
        northwest: 0,
        southwest: 0,
        northeast: 0,
        southeast: 0
      };
      for (var W = 0; W < B.eles.length; W++) {
        var K = B.eles[W], re = K[0]._private.rscratch, le = K.pstyle("curve-style").value, Pe = le === "unbundled-bezier" || ir(le, "segments") || ir(le, "taxi"), we = !M.same(K.source());
        if (!B.calculatedIntersection && M !== N && (B.hasBezier || B.hasUnbundled)) {
          B.calculatedIntersection = !0;
          var me = j.intersectLine(G.x, G.y, _, X, F.x, F.y, 0, ae, V), ge = B.srcIntn = me, oe = te.intersectLine(F.x, F.y, J, Y, G.x, G.y, 0, z, L), de = B.tgtIntn = oe, ve = B.intersectionPts = {
            x1: me[0],
            x2: oe[0],
            y1: me[1],
            y2: oe[1]
          }, xe = B.posPts = {
            x1: G.x,
            x2: F.x,
            y1: G.y,
            y2: F.y
          }, be = oe[1] - me[1], Se = oe[0] - me[0], De = Math.sqrt(Se * Se + be * be);
          ne(De) && De >= ym || (De = Math.sqrt(Math.max(Se * Se, za) + Math.max(be * be, za)));
          var Oe = B.vector = {
            x: Se,
            y: be
          }, Ie = B.vectorNorm = {
            x: Oe.x / De,
            y: Oe.y / De
          }, Be = {
            x: -Ie.y,
            y: Ie.x
          };
          B.nodesOverlap = !ne(De) || te.checkPoint(me[0], me[1], 0, J, Y, F.x, F.y, z, L) || j.checkPoint(oe[0], oe[1], 0, _, X, G.x, G.y, ae, V), B.vectorNormInverse = Be, R = {
            nodesOverlap: B.nodesOverlap,
            dirCounts: B.dirCounts,
            calculatedIntersection: !0,
            hasBezier: B.hasBezier,
            hasUnbundled: B.hasUnbundled,
            eles: B.eles,
            srcPos: F,
            srcRs: L,
            tgtPos: G,
            tgtRs: V,
            srcW: J,
            srcH: Y,
            tgtW: _,
            tgtH: X,
            srcIntn: de,
            tgtIntn: ge,
            srcShape: te,
            tgtShape: j,
            posPts: {
              x1: xe.x2,
              y1: xe.y2,
              x2: xe.x1,
              y2: xe.y1
            },
            intersectionPts: {
              x1: ve.x2,
              y1: ve.y2,
              x2: ve.x1,
              y2: ve.y1
            },
            vector: {
              x: -Oe.x,
              y: -Oe.y
            },
            vectorNorm: {
              x: -Ie.x,
              y: -Ie.y
            },
            vectorNormInverse: {
              x: -Be.x,
              y: -Be.y
            }
          };
        }
        var Q = we ? R : B;
        re.nodesOverlap = Q.nodesOverlap, re.srcIntn = Q.srcIntn, re.tgtIntn = Q.tgtIntn, re.isRound = le.startsWith("round"), n && (M.isParent() || M.isChild() || N.isParent() || N.isChild()) && (M.parents().anySame(N) || N.parents().anySame(M) || M.same(N) && M.isParent()) ? e.findCompoundLoopPoints(K, Q, W, Pe) : M === N ? e.findLoopPoints(K, Q, W, Pe) : le.endsWith("segments") ? e.findSegmentsPoints(K, Q) : le.endsWith("taxi") ? e.findTaxiPoints(K, Q) : le === "straight" || !Pe && B.eles.length % 2 === 1 && W === Math.floor(B.eles.length / 2) ? e.findStraightEdgePoints(K) : e.findBezierPoints(K, Q, W, Pe, we), e.findEndpoints(K), e.tryToCorrectInvalidPoints(K, Q), e.checkForInvalidEdgeWarning(K), e.storeAllpts(K), e.storeEdgeProjections(K), e.calculateArrowAngles(K), e.recalculateEdgeLabelProjections(K), e.calculateLabelAngles(K);
      }
    }, x = 0; x < s.length; x++)
      E();
    this.findHaystackPoints(u);
  }
};
function zf(t) {
  var e = [];
  if (t != null) {
    for (var r = 0; r < t.length; r += 2) {
      var a = t[r], n = t[r + 1];
      e.push({
        x: a,
        y: n
      });
    }
    return e;
  }
}
pt.getSegmentPoints = function(t) {
  var e = t[0]._private.rscratch;
  this.recalculateRenderedStyle(t);
  var r = e.edgeType;
  if (r === "segments")
    return zf(e.segpts);
};
pt.getControlPoints = function(t) {
  var e = t[0]._private.rscratch;
  this.recalculateRenderedStyle(t);
  var r = e.edgeType;
  if (r === "bezier" || r === "multibezier" || r === "self" || r === "compound")
    return zf(e.ctrlpts);
};
pt.getEdgeMidpoint = function(t) {
  var e = t[0]._private.rscratch;
  return this.recalculateRenderedStyle(t), {
    x: e.midX,
    y: e.midY
  };
};
var Ka = {};
Ka.manualEndptToPx = function(t, e) {
  var r = this, a = t.position(), n = t.outerWidth(), i = t.outerHeight(), o = t._private.rscratch;
  if (e.value.length === 2) {
    var s = [e.pfValue[0], e.pfValue[1]];
    return e.units[0] === "%" && (s[0] = s[0] * n), e.units[1] === "%" && (s[1] = s[1] * i), s[0] += a.x, s[1] += a.y, s;
  } else {
    var u = e.pfValue[0];
    u = -Math.PI / 2 + u;
    var l = 2 * Math.max(n, i), c = [a.x + Math.cos(u) * l, a.y + Math.sin(u) * l];
    return r.nodeShapes[this.getNodeShape(t)].intersectLine(a.x, a.y, n, i, c[0], c[1], 0, t.pstyle("corner-radius").value === "auto" ? "auto" : t.pstyle("corner-radius").pfValue, o);
  }
};
Ka.findEndpoints = function(t) {
  var e, r, a, n, i = this, o, s = t.source()[0], u = t.target()[0], l = s.position(), c = u.position(), f = t.pstyle("target-arrow-shape").value, v = t.pstyle("source-arrow-shape").value, h = t.pstyle("target-distance-from-node").pfValue, d = t.pstyle("source-distance-from-node").pfValue, y = s._private.rscratch, g = u._private.rscratch, p = t.pstyle("curve-style").value, m = t._private.rscratch, b = m.edgeType, w = ir(p, "taxi"), C = b === "self" || b === "compound", E = b === "bezier" || b === "multibezier" || C, x = b !== "bezier", T = b === "straight" || b === "segments", k = b === "segments", D = E || x || T, P = C || w, A = t.pstyle("source-endpoint"), B = P ? "outside-to-node" : A.value, R = s.pstyle("corner-radius").value === "auto" ? "auto" : s.pstyle("corner-radius").pfValue, I = t.pstyle("target-endpoint"), O = P ? "outside-to-node" : I.value, M = u.pstyle("corner-radius").value === "auto" ? "auto" : u.pstyle("corner-radius").pfValue;
  m.srcManEndpt = A, m.tgtManEndpt = I;
  var N, H, G, F, _ = (e = (I == null || (r = I.pfValue) === null || r === void 0 ? void 0 : r.length) === 2 ? I.pfValue : null) !== null && e !== void 0 ? e : [0, 0], X = (a = (A == null || (n = A.pfValue) === null || n === void 0 ? void 0 : n.length) === 2 ? A.pfValue : null) !== null && a !== void 0 ? a : [0, 0];
  if (E) {
    var J = [m.ctrlpts[0], m.ctrlpts[1]], Y = x ? [m.ctrlpts[m.ctrlpts.length - 2], m.ctrlpts[m.ctrlpts.length - 1]] : J;
    N = Y, H = J;
  } else if (T) {
    var j = k ? m.segpts.slice(0, 2) : [c.x + _[0], c.y + _[1]], te = k ? m.segpts.slice(m.segpts.length - 2) : [l.x + X[0], l.y + X[1]];
    N = te, H = j;
  }
  if (O === "inside-to-node")
    o = [c.x, c.y];
  else if (I.units)
    o = this.manualEndptToPx(u, I);
  else if (O === "outside-to-line")
    o = m.tgtIntn;
  else if (O === "outside-to-node" || O === "outside-to-node-or-label" ? G = N : (O === "outside-to-line" || O === "outside-to-line-or-label") && (G = [l.x, l.y]), o = i.nodeShapes[this.getNodeShape(u)].intersectLine(c.x, c.y, u.outerWidth(), u.outerHeight(), G[0], G[1], 0, M, g), O === "outside-to-node-or-label" || O === "outside-to-line-or-label") {
    var ae = u._private.rscratch, z = ae.labelWidth, L = ae.labelHeight, V = ae.labelX, W = ae.labelY, K = z / 2, re = L / 2, le = u.pstyle("text-valign").value;
    le = ia(le), le === "top" ? W -= re : le === "bottom" && (W += re);
    var Pe = u.pstyle("text-halign").value;
    Pe = na(Pe), Pe === "left" ? V -= K : Pe === "right" && (V += K);
    var we = Ma(G[0], G[1], [V - K, W - re, V + K, W - re, V + K, W + re, V - K, W + re], c.x, c.y);
    if (we.length > 0) {
      var me = l, ge = Tr(me, Xr(o)), oe = Tr(me, Xr(we)), de = ge;
      if (oe < ge && (o = we, de = oe), we.length > 2) {
        var ve = Tr(me, {
          x: we[2],
          y: we[3]
        });
        ve < de && (o = [we[2], we[3]]);
      }
    }
  }
  var xe = rn(o, N, i.arrowShapes[f].spacing(t) + h), be = rn(o, N, i.arrowShapes[f].gap(t) + h);
  if (m.endX = be[0], m.endY = be[1], m.arrowEndX = xe[0], m.arrowEndY = xe[1], B === "inside-to-node")
    o = [l.x, l.y];
  else if (A.units)
    o = this.manualEndptToPx(s, A);
  else if (B === "outside-to-line")
    o = m.srcIntn;
  else if (B === "outside-to-node" || B === "outside-to-node-or-label" ? F = H : (B === "outside-to-line" || B === "outside-to-line-or-label") && (F = [c.x, c.y]), o = i.nodeShapes[this.getNodeShape(s)].intersectLine(l.x, l.y, s.outerWidth(), s.outerHeight(), F[0], F[1], 0, R, y), B === "outside-to-node-or-label" || B === "outside-to-line-or-label") {
    var Se = s._private.rscratch, De = Se.labelWidth, Oe = Se.labelHeight, Ie = Se.labelX, Be = Se.labelY, Q = De / 2, S = Oe / 2, q = s.pstyle("text-valign").value;
    q = ia(q), q === "top" ? Be -= S : q === "bottom" && (Be += S);
    var $ = s.pstyle("text-halign").value;
    $ = na($), $ === "left" ? Ie -= Q : $ === "right" && (Ie += Q);
    var U = Ma(F[0], F[1], [Ie - Q, Be - S, Ie + Q, Be - S, Ie + Q, Be + S, Ie - Q, Be + S], l.x, l.y);
    if (U.length > 0) {
      var Z = c, ce = Tr(Z, Xr(o)), ee = Tr(Z, Xr(U)), ie = ce;
      if (ee < ce && (o = [U[0], U[1]], ie = ee), U.length > 2) {
        var se = Tr(Z, {
          x: U[2],
          y: U[3]
        });
        se < ie && (o = [U[2], U[3]]);
      }
    }
  }
  var pe = rn(o, H, i.arrowShapes[v].spacing(t) + d), Ee = rn(o, H, i.arrowShapes[v].gap(t) + d);
  m.startX = Ee[0], m.startY = Ee[1], m.arrowStartX = pe[0], m.arrowStartY = pe[1], D && (!ne(m.startX) || !ne(m.startY) || !ne(m.endX) || !ne(m.endY) ? m.badLine = !0 : m.badLine = !1);
};
Ka.getSourceEndpoint = function(t) {
  var e = t[0]._private.rscratch;
  switch (this.recalculateRenderedStyle(t), e.edgeType) {
    case "haystack":
      return {
        x: e.haystackPts[0],
        y: e.haystackPts[1]
      };
    default:
      return {
        x: e.arrowStartX,
        y: e.arrowStartY
      };
  }
};
Ka.getTargetEndpoint = function(t) {
  var e = t[0]._private.rscratch;
  switch (this.recalculateRenderedStyle(t), e.edgeType) {
    case "haystack":
      return {
        x: e.haystackPts[2],
        y: e.haystackPts[3]
      };
    default:
      return {
        x: e.arrowEndX,
        y: e.arrowEndY
      };
  }
};
var Ms = {};
function mm(t, e, r) {
  for (var a = function(l, c, f, v) {
    return it(l, c, f, v);
  }, n = e._private, i = n.rstyle.bezierPts, o = 0; o < t.bezierProjPcts.length; o++) {
    var s = t.bezierProjPcts[o];
    i.push({
      x: a(r[0], r[2], r[4], s),
      y: a(r[1], r[3], r[5], s)
    });
  }
}
Ms.storeEdgeProjections = function(t) {
  var e = t._private, r = e.rscratch, a = r.edgeType;
  if (e.rstyle.bezierPts = null, e.rstyle.linePts = null, e.rstyle.haystackPts = null, a === "multibezier" || a === "bezier" || a === "self" || a === "compound") {
    e.rstyle.bezierPts = [];
    for (var n = 0; n + 5 < r.allpts.length; n += 4)
      mm(this, t, r.allpts.slice(n, n + 6));
  } else if (a === "segments")
    for (var i = e.rstyle.linePts = [], n = 0; n + 1 < r.allpts.length; n += 2)
      i.push({
        x: r.allpts[n],
        y: r.allpts[n + 1]
      });
  else if (a === "haystack") {
    var o = r.haystackPts;
    e.rstyle.haystackPts = [{
      x: o[0],
      y: o[1]
    }, {
      x: o[2],
      y: o[3]
    }];
  }
  e.rstyle.arrowWidth = this.getArrowWidth(t.pstyle("width").pfValue, t.pstyle("arrow-scale").value) * this.arrowShapeWidth;
};
Ms.recalculateEdgeProjections = function(t) {
  this.findEdgeControlPoints(t);
};
var Ut = {};
Ut.recalculateNodeLabelProjection = function(t) {
  var e = t.pstyle("label").strValue;
  if (!cr(e)) {
    var r, a, n = t._private, i = t.width(), o = t.height(), s = t.padding(), u = t.position(), l = t.pstyle("text-halign").strValue, c = t.pstyle("text-valign").strValue, f = n.rscratch, v = n.rstyle;
    switch (l) {
      case "left":
        r = u.x - i / 2 - s;
        break;
      case "left-inside":
        r = u.x - i / 2 + s;
        break;
      case "right":
        r = u.x + i / 2 + s;
        break;
      case "right-inside":
        r = u.x + i / 2 - s;
        break;
      default:
        r = u.x;
    }
    switch (c) {
      case "top":
        a = u.y - o / 2 - s;
        break;
      case "top-inside":
        a = u.y - o / 2 + s;
        break;
      case "bottom":
        a = u.y + o / 2 + s;
        break;
      case "bottom-inside":
        a = u.y + o / 2 - s;
        break;
      default:
        a = u.y;
    }
    f.labelX = r, f.labelY = a, v.labelX = r, v.labelY = a, this.calculateLabelAngles(t), this.applyLabelDimensions(t);
  }
};
var Ff = function(e, r) {
  var a = Math.atan(r / e);
  return e === 0 && a < 0 && (a = a * -1), a;
}, qf = function(e, r) {
  var a = r.x - e.x, n = r.y - e.y;
  return Ff(a, n);
}, bm = function(e, r, a, n) {
  var i = Ra(0, n - 1e-3, 1), o = Ra(0, n + 1e-3, 1), s = Qr(e, r, a, i), u = Qr(e, r, a, o);
  return qf(s, u);
};
Ut.recalculateEdgeLabelProjections = function(t) {
  var e, r = t._private, a = r.rscratch, n = this, i = {
    mid: t.pstyle("label").strValue,
    source: t.pstyle("source-label").strValue,
    target: t.pstyle("target-label").strValue
  };
  if (i.mid || i.source || i.target) {
    e = {
      x: a.midX,
      y: a.midY
    };
    var o = function(f, v, h) {
      Ht(r.rscratch, f, v, h), Ht(r.rstyle, f, v, h);
    };
    o("labelX", null, e.x), o("labelY", null, e.y);
    var s = Ff(a.midDispX, a.midDispY);
    o("labelAutoAngle", null, s);
    var u = function() {
      if (u.cache)
        return u.cache;
      for (var f = [], v = 0; v + 5 < a.allpts.length; v += 4) {
        var h = {
          x: a.allpts[v],
          y: a.allpts[v + 1]
        }, d = {
          x: a.allpts[v + 2],
          y: a.allpts[v + 3]
        }, y = {
          x: a.allpts[v + 4],
          y: a.allpts[v + 5]
        };
        f.push({
          p0: h,
          p1: d,
          p2: y,
          startDist: 0,
          length: 0,
          segments: []
        });
      }
      var g = r.rstyle.bezierPts, p = n.bezierProjPcts.length;
      function m(x, T, k, D, P) {
        var A = Br(T, k), B = x.segments[x.segments.length - 1], R = {
          p0: T,
          p1: k,
          t0: D,
          t1: P,
          startDist: B ? B.startDist + B.length : 0,
          length: A
        };
        x.segments.push(R), x.length += A;
      }
      for (var b = 0; b < f.length; b++) {
        var w = f[b], C = f[b - 1];
        C && (w.startDist = C.startDist + C.length), m(w, w.p0, g[b * p], 0, n.bezierProjPcts[0]);
        for (var E = 0; E < p - 1; E++)
          m(w, g[b * p + E], g[b * p + E + 1], n.bezierProjPcts[E], n.bezierProjPcts[E + 1]);
        m(w, g[b * p + p - 1], w.p2, n.bezierProjPcts[p - 1], 1);
      }
      return u.cache = f;
    }, l = function(f) {
      var v, h = f === "source";
      if (i[f]) {
        var d = t.pstyle(f + "-text-offset").pfValue;
        switch (a.edgeType) {
          case "self":
          case "compound":
          case "bezier":
          case "multibezier": {
            for (var y = u(), g, p = 0, m = 0, b = 0; b < y.length; b++) {
              for (var w = y[h ? b : y.length - 1 - b], C = 0; C < w.segments.length; C++) {
                var E = w.segments[h ? C : w.segments.length - 1 - C], x = b === y.length - 1 && C === w.segments.length - 1;
                if (p = m, m += E.length, m >= d || x) {
                  g = {
                    cp: w,
                    segment: E
                  };
                  break;
                }
              }
              if (g)
                break;
            }
            var T = g.cp, k = g.segment, D = (d - p) / k.length, P = k.t1 - k.t0, A = h ? k.t0 + P * D : k.t1 - P * D;
            A = Ra(0, A, 1), e = Qr(T.p0, T.p1, T.p2, A), v = bm(T.p0, T.p1, T.p2, A);
            break;
          }
          case "straight":
          case "segments":
          case "haystack": {
            for (var B = 0, R, I, O, M, N = a.allpts.length, H = 0; H + 3 < N && (h ? (O = {
              x: a.allpts[H],
              y: a.allpts[H + 1]
            }, M = {
              x: a.allpts[H + 2],
              y: a.allpts[H + 3]
            }) : (O = {
              x: a.allpts[N - 2 - H],
              y: a.allpts[N - 1 - H]
            }, M = {
              x: a.allpts[N - 4 - H],
              y: a.allpts[N - 3 - H]
            }), R = Br(O, M), I = B, B += R, !(B >= d)); H += 2)
              ;
            var G = d - I, F = G / R;
            F = Ra(0, F, 1), e = Vh(O, M, F), v = qf(O, M);
            break;
          }
        }
        o("labelX", f, e.x), o("labelY", f, e.y), o("labelAutoAngle", f, v);
      }
    };
    l("source"), l("target"), this.applyLabelDimensions(t);
  }
};
Ut.applyLabelDimensions = function(t) {
  this.applyPrefixedLabelDimensions(t), t.isEdge() && (this.applyPrefixedLabelDimensions(t, "source"), this.applyPrefixedLabelDimensions(t, "target"));
};
Ut.applyPrefixedLabelDimensions = function(t, e) {
  var r = t._private, a = this.getLabelText(t, e), n = Ar(a, t._private.labelDimsKey);
  if (bt(r.rscratch, "prefixedLabelDimsKey", e) !== n) {
    Ht(r.rscratch, "prefixedLabelDimsKey", e, n);
    var i = this.calculateLabelDimensions(t, a), o = t.pstyle("line-height").pfValue, s = t.pstyle("font-size").pfValue, u = t.pstyle("text-wrap").strValue, l = bt(r.rscratch, "labelWrapCachedLines", e) || [], c = u !== "wrap" ? 1 : Math.max(l.length, 1), f = s * o, v = i.width, h = i.height + (c - 1) * (o - 1) * s;
    Ht(r.rstyle, "labelWidth", e, v), Ht(r.rscratch, "labelWidth", e, v), Ht(r.rstyle, "labelHeight", e, h), Ht(r.rscratch, "labelHeight", e, h), Ht(r.rscratch, "labelLineHeight", e, f), Ht(r.rscratch, "labelActualDescent", e, i.labelActualDescent);
  }
};
Ut.getLabelText = function(t, e) {
  var r = t._private, a = e ? e + "-" : "", n = t.pstyle(a + "label").strValue, i = t.pstyle("text-transform").value, o = function(X, J) {
    return J ? (Ht(r.rscratch, X, e, J), J) : bt(r.rscratch, X, e);
  };
  if (!n)
    return "";
  i == "none" || (i == "uppercase" ? n = n.toUpperCase() : i == "lowercase" && (n = n.toLowerCase()));
  var s = t.pstyle("text-wrap").value;
  if (s === "wrap") {
    var u = o("labelKey");
    if (u != null && o("labelWrapKey") === u)
      return o("labelWrapCachedText");
    for (var l = "​", c = n.split(`
`), f = t.pstyle("text-max-width").pfValue, v = t.pstyle("text-overflow-wrap").value, h = v === "anywhere", d = [], y = /[\s\u200b]+|$/g, g = 0; g < c.length; g++) {
      var p = c[g], m = this.calculateLabelDimensions(t, p), b = m.width;
      if (h) {
        var w = p.split("").join(l);
        p = w;
      }
      if (b > f) {
        var C = p.matchAll(y), E = "", x = 0, T = St(C), k;
        try {
          for (T.s(); !(k = T.n()).done; ) {
            var D = k.value, P = D[0], A = p.substring(x, D.index);
            x = D.index + P.length;
            var B = E.length === 0 ? A : E + A + P, R = this.calculateLabelDimensions(t, B), I = R.width;
            I <= f ? E += A + P : (E && d.push(E), E = A + P);
          }
        } catch (_) {
          T.e(_);
        } finally {
          T.f();
        }
        E.match(/^[\s\u200b]+$/) || d.push(E);
      } else
        d.push(p);
    }
    o("labelWrapCachedLines", d), n = o("labelWrapCachedText", d.join(`
`)), o("labelWrapKey", u);
  } else if (s === "ellipsis") {
    var O = t.pstyle("text-max-width").pfValue, M = "", N = "…", H = !1;
    if (this.calculateLabelDimensions(t, n).width < O)
      return n;
    for (var G = 0; G < n.length; G++) {
      var F = this.calculateLabelDimensions(t, M + n[G] + N).width;
      if (F > O)
        break;
      M += n[G], G === n.length - 1 && (H = !0);
    }
    return H || (M += N), M;
  }
  return n;
};
Ut.getLabelJustification = function(t) {
  var e = t.pstyle("text-justification").strValue, r = t.pstyle("text-halign").strValue;
  return e === "auto" ? t.isNode() ? oy(r) : "center" : e;
};
Ut.calculateLabelDimensions = function(t, e) {
  var r = this, a = r.cy.window(), n = a.document, i = 0, o = t.pstyle("font-style").strValue, s = t.pstyle("font-size").pfValue, u = t.pstyle("font-family").strValue, l = t.pstyle("font-weight").strValue, c = t.pstyle("text-metrics").strValue || "font", f = this.labelCalcCanvas, v = this.labelCalcCanvasContext;
  if (!f) {
    f = this.labelCalcCanvas = n.createElement("canvas"), v = this.labelCalcCanvasContext = f.getContext("2d");
    var h = f.style;
    h.position = "absolute", h.left = "-9999px", h.top = "-9999px", h.zIndex = "-1", h.visibility = "hidden", h.pointerEvents = "none";
  }
  v.font = "".concat(o, " ").concat(l, " ").concat(s, "px ").concat(u);
  for (var d = 0, y = 0, g = e.split(`
`), p = g.length, m = 0, b = 0, w = 0; w < p; w++) {
    var C = g[w], E = v.measureText(C), x = Math.ceil(E.width), T = s;
    c === "glyph" && (w === 0 && (b = E.actualBoundingBoxAscent), w === p - 1 && (m = E.actualBoundingBoxDescent)), d = Math.max(x, d), y += T;
  }
  return c === "glyph" && (y -= s - b - m), d += i, y += i, {
    width: d,
    height: y,
    labelActualAscent: b,
    labelActualDescent: m
  };
};
Ut.calculateLabelAngle = function(t, e) {
  var r = t._private, a = r.rscratch, n = t.isEdge(), i = e ? e + "-" : "", o = t.pstyle(i + "text-rotation"), s = o.strValue;
  return s === "none" ? 0 : n && s === "autorotate" ? a.labelAutoAngle : s === "autorotate" ? 0 : o.pfValue;
};
Ut.calculateLabelAngles = function(t) {
  var e = this, r = t.isEdge(), a = t._private, n = a.rscratch;
  n.labelAngle = e.calculateLabelAngle(t), r && (n.sourceLabelAngle = e.calculateLabelAngle(t, "source"), n.targetLabelAngle = e.calculateLabelAngle(t, "target"));
};
var Vf = {}, Jl = 28, jl = !1;
Vf.getNodeShape = function(t) {
  var e = this, r = t.pstyle("shape").value;
  if (r === "cutrectangle" && (t.width() < Jl || t.height() < Jl))
    return jl || (ze("The `cutrectangle` node shape can not be used at small sizes so `rectangle` is used instead"), jl = !0), "rectangle";
  if (t.isParent())
    return r === "rectangle" || r === "roundrectangle" || r === "round-rectangle" || r === "cutrectangle" || r === "cut-rectangle" || r === "barrel" ? r : "rectangle";
  if (r === "polygon") {
    var a = t.pstyle("shape-polygon-points").value;
    return e.nodeShapes.makePolygon(a).name;
  }
  return r;
};
var ai = {};
ai.registerCalculationListeners = function() {
  var t = this.cy, e = t.collection(), r = this, a = function(o) {
    var s = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : !0;
    if (e.merge(o), s)
      for (var u = 0; u < o.length; u++) {
        var l = o[u], c = l._private, f = c.rstyle;
        f.clean = !1, f.cleanConnected = !1;
      }
  };
  r.binder(t).on("bounds.* dirty.*", function(o) {
    var s = o.target;
    a(s);
  }).on("style.* background.*", function(o) {
    var s = o.target;
    a(s, !1);
  });
  var n = function(o) {
    if (o) {
      var s = r.onUpdateEleCalcsFns;
      e.cleanStyle();
      for (var u = 0; u < e.length; u++) {
        var l = e[u], c = l._private.rstyle;
        l.isNode() && !c.cleanConnected && (a(l.connectedEdges()), c.cleanConnected = !0);
      }
      if (s)
        for (var f = 0; f < s.length; f++) {
          var v = s[f];
          v(o, e);
        }
      r.recalculateRenderedStyle(e), e = t.collection();
    }
  };
  r.flushRenderedStyleQueue = function() {
    n(!0);
  }, r.beforeRender(n, r.beforeRenderPriorities.eleCalcs);
};
ai.onUpdateEleCalcs = function(t) {
  var e = this.onUpdateEleCalcsFns = this.onUpdateEleCalcsFns || [];
  e.push(t);
};
ai.recalculateRenderedStyle = function(t, e) {
  var r = function(w) {
    return w._private.rstyle.cleanConnected;
  };
  if (t.length !== 0) {
    var a = [], n = [];
    if (!this.destroyed) {
      e === void 0 && (e = !0);
      for (var i = 0; i < t.length; i++) {
        var o = t[i], s = o._private, u = s.rstyle;
        o.isEdge() && (!r(o.source()) || !r(o.target())) && (u.clean = !1), o.isEdge() && o.isBundledBezier() && o.parallelEdges().some(function(b) {
          return !b._private.rstyle.clean && b.isBundledBezier();
        }) && (u.clean = !1), !(e && u.clean || o.removed()) && o.pstyle("display").value !== "none" && (s.group === "nodes" ? n.push(o) : a.push(o), u.clean = !0);
      }
      for (var l = 0; l < n.length; l++) {
        var c = n[l], f = c._private, v = f.rstyle, h = c.position();
        this.recalculateNodeLabelProjection(c), v.nodeX = h.x, v.nodeY = h.y, v.nodeW = c.pstyle("width").pfValue, v.nodeH = c.pstyle("height").pfValue;
      }
      this.recalculateEdgeProjections(a);
      for (var d = 0; d < a.length; d++) {
        var y = a[d], g = y._private, p = g.rstyle, m = g.rscratch;
        p.srcX = m.arrowStartX, p.srcY = m.arrowStartY, p.tgtX = m.arrowEndX, p.tgtY = m.arrowEndY, p.midX = m.midX, p.midY = m.midY, p.labelAngle = m.labelAngle, p.sourceLabelAngle = m.sourceLabelAngle, p.targetLabelAngle = m.targetLabelAngle;
      }
    }
  }
};
var ni = {};
ni.updateCachedGrabbedEles = function() {
  var t = this.cachedZSortedEles;
  if (t) {
    t.drag = [], t.nondrag = [];
    for (var e = [], r = 0; r < t.length; r++) {
      var a = t[r], n = a._private.rscratch;
      a.grabbed() && !a.isParent() ? e.push(a) : n.inDragLayer ? t.drag.push(a) : t.nondrag.push(a);
    }
    for (var r = 0; r < e.length; r++) {
      var a = e[r];
      t.drag.push(a);
    }
  }
};
ni.invalidateCachedZSortedEles = function() {
  this.cachedZSortedEles = null;
};
ni.getCachedZSortedEles = function(t) {
  if (t || !this.cachedZSortedEles) {
    var e = this.cy.mutableElements().toArray();
    e.sort(xf), e.interactive = e.filter(function(r) {
      return r.interactive();
    }), this.cachedZSortedEles = e, this.updateCachedGrabbedEles();
  } else
    e = this.cachedZSortedEles;
  return e;
};
var _f = {};
[Or, zn, pt, Ka, Ms, Ut, Vf, ai, ni].forEach(function(t) {
  ye(_f, t);
});
var Gf = {};
Gf.getCachedImage = function(t, e, r) {
  var a = this, n = a.imageCache = a.imageCache || {}, i = n[t];
  if (i)
    return i.image.complete || i.image.addEventListener("load", r), i.image;
  i = n[t] = n[t] || {};
  var o = i.image = new Image();
  o.addEventListener("load", r), o.addEventListener("error", function() {
    o.error = !0;
  });
  var s = "data:", u = t.substring(0, s.length).toLowerCase() === s;
  return u || (e = e === "null" ? null : e, o.crossOrigin = e), o.src = t, o;
};
var Hf = function(e, r) {
  var a = e[0];
  !a || a._private.grabbed === r || (a._private.grabbed = r, e.updateStyle(!1));
}, wm = function(e) {
  Hf(e, !0);
}, xm = function(e) {
  Hf(e, !1);
}, ca = {};
ca.registerBinding = function(t, e, r, a) {
  var n = Array.prototype.slice.apply(arguments, [1]);
  if (Array.isArray(t)) {
    for (var i = [], o = 0; o < t.length; o++) {
      var s = t[o];
      if (s !== void 0) {
        var u = this.binder(s);
        i.push(u.on.apply(u, n));
      }
    }
    return i;
  }
  var u = this.binder(t);
  return u.on.apply(u, n);
};
ca.binder = function(t) {
  var e = this, r = e.cy.window(), a = t === r || t === r.document || t === r.document.body || Bd(t);
  if (e.supportsPassiveEvents == null) {
    var n = !1;
    try {
      var i = Object.defineProperty({}, "passive", {
        get: function() {
          return n = !0, !0;
        }
      });
      r.addEventListener("test", null, i);
    } catch {
    }
    e.supportsPassiveEvents = n;
  }
  var o = function(u, l, c) {
    var f = Array.prototype.slice.call(arguments);
    return a && e.supportsPassiveEvents && (f[2] = {
      capture: c ?? !1,
      passive: !1,
      once: !1
    }), e.bindings.push({
      target: t,
      args: f
    }), (t.addEventListener || t.on).apply(t, f), this;
  };
  return {
    on: o,
    addEventListener: o,
    addListener: o,
    bind: o
  };
};
ca.nodeIsDraggable = function(t) {
  return t && t.isNode() && !t.locked() && t.grabbable();
};
ca.nodeIsGrabbable = function(t) {
  return this.nodeIsDraggable(t) && t.interactive();
};
ca.load = function() {
  var t = this, e = t.cy.window(), r = function(S) {
    return S.selected();
  }, a = function(S) {
    var q = S.getRootNode();
    if (q && q.nodeType === 11 && q.host !== void 0)
      return q;
  }, n = function(S, q, $, U) {
    S == null && (S = t.cy);
    for (var Z = 0; Z < q.length; Z++) {
      var ce = q[Z];
      S.emit({
        originalEvent: $,
        type: ce,
        position: U
      });
    }
  }, i = function(S) {
    return S.shiftKey || S.metaKey || S.ctrlKey;
  }, o = function(S, q) {
    var $ = !0;
    if (t.cy.hasCompoundNodes() && S && S.pannable())
      for (var U = 0; q && U < q.length; U++) {
        var S = q[U];
        if (S.isNode() && S.isParent() && !S.pannable()) {
          $ = !1;
          break;
        }
      }
    else
      $ = !0;
    return $;
  }, s = function(S) {
    S[0]._private.rscratch.inDragLayer = !0;
  }, u = function(S) {
    S[0]._private.rscratch.inDragLayer = !1;
  }, l = function(S) {
    S[0]._private.rscratch.isGrabTarget = !0;
  }, c = function(S) {
    S[0]._private.rscratch.isGrabTarget = !1;
  }, f = function(S, q) {
    var $ = q.addToList, U = $.has(S);
    !U && S.grabbable() && !S.locked() && ($.merge(S), wm(S));
  }, v = function(S, q) {
    if (S.cy().hasCompoundNodes() && !(q.inDragLayer == null && q.addToList == null)) {
      var $ = S.descendants();
      q.inDragLayer && ($.forEach(s), $.connectedEdges().forEach(s)), q.addToList && f($, q);
    }
  }, h = function(S, q) {
    q = q || {};
    var $ = S.cy().hasCompoundNodes();
    q.inDragLayer && (S.forEach(s), S.neighborhood().stdFilter(function(U) {
      return !$ || U.isEdge();
    }).forEach(s)), q.addToList && S.forEach(function(U) {
      f(U, q);
    }), v(S, q), g(S, {
      inDragLayer: q.inDragLayer
    }), t.updateCachedGrabbedEles();
  }, d = h, y = function(S) {
    S && (t.getCachedZSortedEles().forEach(function(q) {
      xm(q), u(q), c(q);
    }), t.updateCachedGrabbedEles());
  }, g = function(S, q) {
    if (!(q.inDragLayer == null && q.addToList == null) && S.cy().hasCompoundNodes()) {
      var $ = S.ancestors().orphans();
      if (!$.same(S)) {
        var U = $.descendants().spawnSelf().merge($).unmerge(S).unmerge(S.descendants()), Z = U.connectedEdges();
        q.inDragLayer && (Z.forEach(s), U.forEach(s)), q.addToList && U.forEach(function(ce) {
          f(ce, q);
        });
      }
    }
  }, p = function() {
    document.activeElement != null && document.activeElement.blur != null && document.activeElement.blur();
  }, m = typeof MutationObserver < "u", b = typeof ResizeObserver < "u";
  m ? (t.removeObserver = new MutationObserver(function(Q) {
    for (var S = 0; S < Q.length; S++) {
      var q = Q[S], $ = q.removedNodes;
      if ($)
        for (var U = 0; U < $.length; U++) {
          var Z = $[U];
          if (Z === t.container) {
            t.destroy();
            break;
          }
        }
    }
  }), t.container.parentNode && t.removeObserver.observe(t.container.parentNode, {
    childList: !0
  })) : t.registerBinding(t.container, "DOMNodeRemoved", function(Q) {
    t.destroy();
  });
  var w = Ha(function() {
    t.cy.resize();
  }, 100);
  m && (t.styleObserver = new MutationObserver(w), t.styleObserver.observe(t.container, {
    attributes: !0
  })), t.registerBinding(e, "resize", w), b && (t.resizeObserver = new ResizeObserver(w), t.resizeObserver.observe(t.container));
  var C = function(S, q) {
    for (; S != null; )
      q(S), S = S.parentNode;
  }, E = function() {
    t.invalidateContainerClientCoordsCache();
  };
  C(t.container, function(Q) {
    t.registerBinding(Q, "transitionend", E), t.registerBinding(Q, "animationend", E), t.registerBinding(Q, "scroll", E);
  }), t.registerBinding(t.container, "contextmenu", function(Q) {
    Q.preventDefault();
  });
  var x = function() {
    return t.selection[4] !== 0;
  }, T = function(S) {
    for (var q = t.findContainerClientCoords(), $ = q[0], U = q[1], Z = q[2], ce = q[3], ee = S.touches ? S.touches : [S], ie = !1, se = 0; se < ee.length; se++) {
      var pe = ee[se];
      if ($ <= pe.clientX && pe.clientX <= $ + Z && U <= pe.clientY && pe.clientY <= U + ce) {
        ie = !0;
        break;
      }
    }
    if (!ie)
      return !1;
    for (var Ee = t.container, Ce = S.target, fe = Ce.parentNode, ke = !1; fe; ) {
      if (fe === Ee) {
        ke = !0;
        break;
      }
      fe = fe.parentNode;
    }
    return !!ke;
  };
  t.registerBinding(t.container, "mousedown", function(S) {
    if (T(S) && !(t.hoverData.which === 1 && S.which !== 1)) {
      S.preventDefault(), p(), t.hoverData.capture = !0, t.hoverData.which = S.which;
      var q = t.cy, $ = [S.clientX, S.clientY], U = t.projectIntoViewport($[0], $[1]), Z = t.selection, ce = t.findNearestElements(U[0], U[1], !0, !1), ee = ce[0], ie = t.dragData.possibleDragElements;
      t.hoverData.mdownPos = U, t.hoverData.mdownGPos = $;
      var se = function(Ae) {
        return {
          originalEvent: S,
          type: Ae,
          position: {
            x: U[0],
            y: U[1]
          }
        };
      }, pe = function() {
        t.hoverData.tapholdCancelled = !1, clearTimeout(t.hoverData.tapholdTimeout), t.hoverData.tapholdTimeout = setTimeout(function() {
          if (!t.hoverData.tapholdCancelled) {
            var Ae = t.hoverData.down;
            Ae ? Ae.emit(se("taphold")) : q.emit(se("taphold"));
          }
        }, t.tapholdDuration);
      };
      if (S.which == 3) {
        t.hoverData.cxtStarted = !0;
        var Ee = {
          originalEvent: S,
          type: "cxttapstart",
          position: {
            x: U[0],
            y: U[1]
          }
        };
        ee ? (ee.activate(), ee.emit(Ee), t.hoverData.down = ee) : q.emit(Ee), t.hoverData.downTime = (/* @__PURE__ */ new Date()).getTime(), t.hoverData.cxtDragged = !1;
      } else if (S.which == 1) {
        ee && ee.activate();
        {
          if (ee != null && t.nodeIsGrabbable(ee)) {
            var Ce = function(Ae) {
              Ae.emit(se("grab"));
            };
            if (l(ee), !ee.selected())
              ie = t.dragData.possibleDragElements = q.collection(), d(ee, {
                addToList: ie
              }), ee.emit(se("grabon")).emit(se("grab"));
            else {
              ie = t.dragData.possibleDragElements = q.collection();
              var fe = q.$(function(ke) {
                return ke.isNode() && ke.selected() && t.nodeIsGrabbable(ke);
              });
              h(fe, {
                addToList: ie
              }), ee.emit(se("grabon")), fe.forEach(Ce);
            }
            t.redrawHint("eles", !0), t.redrawHint("drag", !0);
          }
          t.hoverData.down = ee, t.hoverData.downs = ce, t.hoverData.downTime = (/* @__PURE__ */ new Date()).getTime();
        }
        n(ee, ["mousedown", "tapstart", "vmousedown"], S, {
          x: U[0],
          y: U[1]
        }), ee == null ? (Z[4] = 1, t.data.bgActivePosistion = {
          x: U[0],
          y: U[1]
        }, t.redrawHint("select", !0), t.redraw()) : ee.pannable() && (Z[4] = 1), pe();
      }
      Z[0] = Z[2] = U[0], Z[1] = Z[3] = U[1];
    }
  }, !1);
  var k = a(t.container);
  t.registerBinding([e, k], "mousemove", function(S) {
    var q = t.hoverData.capture;
    if (!(!q && !T(S))) {
      var $ = !1, U = t.cy, Z = U.zoom(), ce = [S.clientX, S.clientY], ee = t.projectIntoViewport(ce[0], ce[1]), ie = t.hoverData.mdownPos, se = t.hoverData.mdownGPos, pe = t.selection, Ee = null;
      !t.hoverData.draggingEles && !t.hoverData.dragging && !t.hoverData.selecting && (Ee = t.findNearestElement(ee[0], ee[1], !0, !1));
      var Ce = t.hoverData.last, fe = t.hoverData.down, ke = [ee[0] - pe[2], ee[1] - pe[3]], Ae = t.dragData.possibleDragElements, nt;
      if (se) {
        var Ke = ce[0] - se[0], Rt = Ke * Ke, Xe = ce[1] - se[1], Ze = Xe * Xe, je = Rt + Ze;
        t.hoverData.isOverThresholdDrag = nt = je >= t.desktopTapThreshold2;
      }
      var ut = i(S);
      nt && (t.hoverData.tapholdCancelled = !0);
      var qt = function() {
        var Pt = t.hoverData.dragDelta = t.hoverData.dragDelta || [];
        Pt.length === 0 ? (Pt.push(ke[0]), Pt.push(ke[1])) : (Pt[0] += ke[0], Pt[1] += ke[1]);
      };
      $ = !0, n(Ee, ["mousemove", "vmousemove", "tapdrag"], S, {
        x: ee[0],
        y: ee[1]
      });
      var We = function(Pt) {
        return {
          originalEvent: S,
          type: Pt,
          position: {
            x: ee[0],
            y: ee[1]
          }
        };
      }, Xt = function() {
        t.data.bgActivePosistion = void 0, t.hoverData.selecting || U.emit(We("boxstart")), pe[4] = 1, t.hoverData.selecting = !0, t.redrawHint("select", !0), t.redraw();
      };
      if (t.hoverData.which === 3) {
        if (nt) {
          var Mt = We("cxtdrag");
          fe ? fe.emit(Mt) : U.emit(Mt), t.hoverData.cxtDragged = !0, (!t.hoverData.cxtOver || Ee !== t.hoverData.cxtOver) && (t.hoverData.cxtOver && t.hoverData.cxtOver.emit(We("cxtdragout")), t.hoverData.cxtOver = Ee, Ee && Ee.emit(We("cxtdragover")));
        }
      } else if (t.hoverData.dragging) {
        if ($ = !0, U.panningEnabled() && U.userPanningEnabled()) {
          var tr;
          if (t.hoverData.justStartedPan) {
            var Za = t.hoverData.mdownPos;
            tr = {
              x: (ee[0] - Za[0]) * Z,
              y: (ee[1] - Za[1]) * Z
            }, t.hoverData.justStartedPan = !1;
          } else
            tr = {
              x: ke[0] * Z,
              y: ke[1] * Z
            };
          U.panBy(tr), U.emit(We("dragpan")), t.hoverData.dragged = !0;
        }
        ee = t.projectIntoViewport(S.clientX, S.clientY);
      } else if (pe[4] == 1 && (fe == null || fe.pannable())) {
        if (nt) {
          if (!t.hoverData.dragging && U.boxSelectionEnabled() && (ut || !U.panningEnabled() || !U.userPanningEnabled()))
            Xt();
          else if (!t.hoverData.selecting && U.panningEnabled() && U.userPanningEnabled()) {
            var xr = o(fe, t.hoverData.downs);
            xr && (t.hoverData.dragging = !0, t.hoverData.justStartedPan = !0, pe[4] = 0, t.data.bgActivePosistion = Xr(ie), t.redrawHint("select", !0), t.redraw());
          }
          fe && fe.pannable() && fe.active() && fe.unactivate();
        }
      } else {
        if (fe && fe.pannable() && fe.active() && fe.unactivate(), (!fe || !fe.grabbed()) && Ee != Ce && (Ce && n(Ce, ["mouseout", "tapdragout"], S, {
          x: ee[0],
          y: ee[1]
        }), Ee && n(Ee, ["mouseover", "tapdragover"], S, {
          x: ee[0],
          y: ee[1]
        }), t.hoverData.last = Ee), fe)
          if (nt) {
            if (U.boxSelectionEnabled() && ut)
              fe && fe.grabbed() && (y(Ae), fe.emit(We("freeon")), Ae.emit(We("free")), t.dragData.didDrag && (fe.emit(We("dragfreeon")), Ae.emit(We("dragfree")))), Xt();
            else if (fe && fe.grabbed() && t.nodeIsDraggable(fe)) {
              var Et = !t.dragData.didDrag;
              Et && t.redrawHint("eles", !0), t.dragData.didDrag = !0, t.hoverData.draggingEles || h(Ae, {
                inDragLayer: !0
              });
              var dt = {
                x: 0,
                y: 0
              };
              if (ne(ke[0]) && ne(ke[1]) && (dt.x += ke[0], dt.y += ke[1], Et)) {
                var Ct = t.hoverData.dragDelta;
                Ct && ne(Ct[0]) && ne(Ct[1]) && (dt.x += Ct[0], dt.y += Ct[1]);
              }
              t.hoverData.draggingEles = !0, Ae.silentShift(dt).emit(We("position")).emit(We("drag")), t.redrawHint("drag", !0), t.redraw();
            }
          } else
            qt();
        $ = !0;
      }
      if (pe[2] = ee[0], pe[3] = ee[1], $)
        return S.stopPropagation && S.stopPropagation(), S.preventDefault && S.preventDefault(), !1;
    }
  }, !1);
  var D, P, A;
  t.registerBinding(e, "mouseup", function(S) {
    if (!(t.hoverData.which === 1 && S.which !== 1 && t.hoverData.capture)) {
      var q = t.hoverData.capture;
      if (q) {
        t.hoverData.capture = !1;
        var $ = t.cy, U = t.projectIntoViewport(S.clientX, S.clientY), Z = t.selection, ce = t.findNearestElement(U[0], U[1], !0, !1), ee = t.dragData.possibleDragElements, ie = t.hoverData.down, se = i(S);
        t.data.bgActivePosistion && (t.redrawHint("select", !0), t.redraw()), t.hoverData.tapholdCancelled = !0, t.data.bgActivePosistion = void 0, ie && ie.unactivate();
        var pe = function(Ke) {
          return {
            originalEvent: S,
            type: Ke,
            position: {
              x: U[0],
              y: U[1]
            }
          };
        };
        if (t.hoverData.which === 3) {
          var Ee = pe("cxttapend");
          if (ie ? ie.emit(Ee) : $.emit(Ee), !t.hoverData.cxtDragged) {
            var Ce = pe("cxttap");
            ie ? ie.emit(Ce) : $.emit(Ce);
          }
          t.hoverData.cxtDragged = !1, t.hoverData.which = null;
        } else if (t.hoverData.which === 1) {
          if (n(ce, ["mouseup", "tapend", "vmouseup"], S, {
            x: U[0],
            y: U[1]
          }), !t.dragData.didDrag && // didn't move a node around
          !t.hoverData.dragged && // didn't pan
          !t.hoverData.selecting && // not box selection
          !t.hoverData.isOverThresholdDrag && (n(ie, ["click", "tap", "vclick"], S, {
            x: U[0],
            y: U[1]
          }), P = !1, S.timeStamp - A <= $.multiClickDebounceTime() ? (D && clearTimeout(D), P = !0, A = null, n(ie, ["dblclick", "dbltap", "vdblclick"], S, {
            x: U[0],
            y: U[1]
          })) : (D = setTimeout(function() {
            P || n(ie, ["oneclick", "onetap", "voneclick"], S, {
              x: U[0],
              y: U[1]
            });
          }, $.multiClickDebounceTime()), A = S.timeStamp)), ie == null && !t.dragData.didDrag && !t.hoverData.selecting && !t.hoverData.dragged && !i(S) && ($.$(r).unselect(["tapunselect"]), ee.length > 0 && t.redrawHint("eles", !0), t.dragData.possibleDragElements = ee = $.collection()), ce == ie && !t.dragData.didDrag && !t.hoverData.selecting && ce != null && ce._private.selectable && (t.hoverData.dragging || ($.selectionType() === "additive" || se ? ce.selected() ? ce.unselect(["tapunselect"]) : ce.select(["tapselect"]) : se || ($.$(r).unmerge(ce).unselect(["tapunselect"]), ce.select(["tapselect"]))), t.redrawHint("eles", !0)), t.hoverData.selecting) {
            var fe = $.collection(t.getAllInBox(Z[0], Z[1], Z[2], Z[3]));
            t.redrawHint("select", !0), fe.length > 0 && t.redrawHint("eles", !0), $.emit(pe("boxend"));
            var ke = function(Ke) {
              return Ke.selectable() && !Ke.selected();
            };
            $.selectionType() === "additive" || se || $.$(r).unmerge(fe).unselect(), fe.emit(pe("box")).stdFilter(ke).select().emit(pe("boxselect")), t.redraw();
          }
          if (t.hoverData.dragging && (t.hoverData.dragging = !1, t.redrawHint("select", !0), t.redrawHint("eles", !0), t.redraw()), !Z[4]) {
            t.redrawHint("drag", !0), t.redrawHint("eles", !0);
            var Ae = ie && ie.grabbed();
            y(ee), Ae && (ie.emit(pe("freeon")), ee.emit(pe("free")), t.dragData.didDrag && (ie.emit(pe("dragfreeon")), ee.emit(pe("dragfree"))));
          }
        }
        Z[4] = 0, t.hoverData.down = null, t.hoverData.cxtStarted = !1, t.hoverData.draggingEles = !1, t.hoverData.selecting = !1, t.hoverData.isOverThresholdDrag = !1, t.dragData.didDrag = !1, t.hoverData.dragged = !1, t.hoverData.dragDelta = [], t.hoverData.mdownPos = null, t.hoverData.mdownGPos = null, t.hoverData.which = null;
      }
    }
  }, !1);
  var B = [], R = 4, I, O = 1e5, M = function(S, q) {
    for (var $ = 0; $ < S.length; $++)
      if (S[$] % q !== 0)
        return !1;
    return !0;
  }, N = function(S) {
    for (var q = Math.abs(S[0]), $ = 1; $ < S.length; $++)
      if (Math.abs(S[$]) !== q)
        return !1;
    return !0;
  }, H = function(S) {
    var q = !1, $ = S.deltaY;
    if ($ == null && (S.wheelDeltaY != null ? $ = S.wheelDeltaY / 4 : S.wheelDelta != null && ($ = S.wheelDelta / 4)), $ !== 0) {
      if (I == null)
        if (B.length >= R) {
          var U = B;
          if (I = M(U, 5), !I) {
            var Z = Math.abs(U[0]);
            I = N(U) && Z > 5;
          }
          if (I)
            for (var ce = 0; ce < U.length; ce++)
              O = Math.min(Math.abs(U[ce]), O);
        } else
          B.push($), q = !0;
      else I && (O = Math.min(Math.abs($), O));
      if (!t.scrollingPage) {
        var ee = t.cy, ie = ee.zoom(), se = ee.pan(), pe = t.projectIntoViewport(S.clientX, S.clientY), Ee = [pe[0] * ie + se.x, pe[1] * ie + se.y];
        if (t.hoverData.draggingEles || t.hoverData.dragging || t.hoverData.cxtStarted || x()) {
          S.preventDefault();
          return;
        }
        if (ee.panningEnabled() && ee.userPanningEnabled() && ee.zoomingEnabled() && ee.userZoomingEnabled()) {
          S.preventDefault(), t.data.wheelZooming = !0, clearTimeout(t.data.wheelTimeout), t.data.wheelTimeout = setTimeout(function() {
            t.data.wheelZooming = !1, t.redrawHint("eles", !0), t.redraw();
          }, 150);
          var Ce;
          q && Math.abs($) > 5 && ($ = ms($) * 5), Ce = $ / -250, I && (Ce /= O, Ce *= 3), Ce = Ce * t.wheelSensitivity;
          var fe = S.deltaMode === 1;
          fe && (Ce *= 33);
          var ke = ee.zoom() * Math.pow(10, Ce);
          S.type === "gesturechange" && (ke = t.gestureStartZoom * S.scale), ee.zoom({
            level: ke,
            renderedPosition: {
              x: Ee[0],
              y: Ee[1]
            }
          }), ee.emit({
            type: S.type === "gesturechange" ? "pinchzoom" : "scrollzoom",
            originalEvent: S,
            position: {
              x: pe[0],
              y: pe[1]
            }
          });
        }
      }
    }
  };
  t.registerBinding(t.container, "wheel", H, !0), t.registerBinding(e, "scroll", function(S) {
    t.scrollingPage = !0, clearTimeout(t.scrollingPageTimeout), t.scrollingPageTimeout = setTimeout(function() {
      t.scrollingPage = !1;
    }, 250);
  }, !0), t.registerBinding(t.container, "gesturestart", function(S) {
    t.gestureStartZoom = t.cy.zoom(), t.hasTouchStarted || S.preventDefault();
  }, !0), t.registerBinding(t.container, "gesturechange", function(Q) {
    t.hasTouchStarted || H(Q);
  }, !0), t.registerBinding(t.container, "mouseout", function(S) {
    var q = t.projectIntoViewport(S.clientX, S.clientY);
    t.cy.emit({
      originalEvent: S,
      type: "mouseout",
      position: {
        x: q[0],
        y: q[1]
      }
    });
  }, !1), t.registerBinding(t.container, "mouseover", function(S) {
    var q = t.projectIntoViewport(S.clientX, S.clientY);
    t.cy.emit({
      originalEvent: S,
      type: "mouseover",
      position: {
        x: q[0],
        y: q[1]
      }
    });
  }, !1);
  var G, F, _, X, J, Y, j, te, ae, z, L, V, W, K = function(S, q, $, U) {
    return Math.sqrt(($ - S) * ($ - S) + (U - q) * (U - q));
  }, re = function(S, q, $, U) {
    return ($ - S) * ($ - S) + (U - q) * (U - q);
  }, le;
  t.registerBinding(t.container, "touchstart", le = function(S) {
    if (t.hasTouchStarted = !0, !!T(S)) {
      p(), t.touchData.capture = !0, t.data.bgActivePosistion = void 0;
      var q = t.cy, $ = t.touchData.now, U = t.touchData.earlier;
      if (S.touches[0]) {
        var Z = t.projectIntoViewport(S.touches[0].clientX, S.touches[0].clientY);
        $[0] = Z[0], $[1] = Z[1];
      }
      if (S.touches[1]) {
        var Z = t.projectIntoViewport(S.touches[1].clientX, S.touches[1].clientY);
        $[2] = Z[0], $[3] = Z[1];
      }
      if (S.touches[2]) {
        var Z = t.projectIntoViewport(S.touches[2].clientX, S.touches[2].clientY);
        $[4] = Z[0], $[5] = Z[1];
      }
      var ce = function(ut) {
        return {
          originalEvent: S,
          type: ut,
          position: {
            x: $[0],
            y: $[1]
          }
        };
      };
      if (S.touches[1]) {
        t.touchData.singleTouchMoved = !0, y(t.dragData.touchDragEles);
        var ee = t.findContainerClientCoords();
        ae = ee[0], z = ee[1], L = ee[2], V = ee[3], G = S.touches[0].clientX - ae, F = S.touches[0].clientY - z, _ = S.touches[1].clientX - ae, X = S.touches[1].clientY - z, W = 0 <= G && G <= L && 0 <= _ && _ <= L && 0 <= F && F <= V && 0 <= X && X <= V;
        var ie = q.pan(), se = q.zoom();
        J = K(G, F, _, X), Y = re(G, F, _, X), j = [(G + _) / 2, (F + X) / 2], te = [(j[0] - ie.x) / se, (j[1] - ie.y) / se];
        var pe = 200, Ee = pe * pe;
        if (Y < Ee && !S.touches[2]) {
          var Ce = t.findNearestElement($[0], $[1], !0, !0), fe = t.findNearestElement($[2], $[3], !0, !0);
          Ce && Ce.isNode() ? (Ce.activate().emit(ce("cxttapstart")), t.touchData.start = Ce) : fe && fe.isNode() ? (fe.activate().emit(ce("cxttapstart")), t.touchData.start = fe) : q.emit(ce("cxttapstart")), t.touchData.start && (t.touchData.start._private.grabbed = !1), t.touchData.cxt = !0, t.touchData.cxtDragged = !1, t.data.bgActivePosistion = void 0, t.redraw();
          return;
        }
      }
      if (S.touches[2])
        q.boxSelectionEnabled() && S.preventDefault();
      else if (!S.touches[1]) {
        if (S.touches[0]) {
          var ke = t.findNearestElements($[0], $[1], !0, !0), Ae = ke[0];
          if (Ae != null && (Ae.activate(), t.touchData.start = Ae, t.touchData.starts = ke, t.nodeIsGrabbable(Ae))) {
            var nt = t.dragData.touchDragEles = q.collection(), Ke = null;
            t.redrawHint("eles", !0), t.redrawHint("drag", !0), Ae.selected() ? (Ke = q.$(function(je) {
              return je.selected() && t.nodeIsGrabbable(je);
            }), h(Ke, {
              addToList: nt
            })) : d(Ae, {
              addToList: nt
            }), l(Ae), Ae.emit(ce("grabon")), Ke ? Ke.forEach(function(je) {
              je.emit(ce("grab"));
            }) : Ae.emit(ce("grab"));
          }
          n(Ae, ["touchstart", "tapstart", "vmousedown"], S, {
            x: $[0],
            y: $[1]
          }), Ae == null && (t.data.bgActivePosistion = {
            x: Z[0],
            y: Z[1]
          }, t.redrawHint("select", !0), t.redraw()), t.touchData.singleTouchMoved = !1, t.touchData.singleTouchStartTime = +/* @__PURE__ */ new Date(), clearTimeout(t.touchData.tapholdTimeout), t.touchData.tapholdTimeout = setTimeout(function() {
            t.touchData.singleTouchMoved === !1 && !t.pinching && !t.touchData.selecting && n(t.touchData.start, ["taphold"], S, {
              x: $[0],
              y: $[1]
            });
          }, t.tapholdDuration);
        }
      }
      if (S.touches.length >= 1) {
        for (var Rt = t.touchData.startPosition = [null, null, null, null, null, null], Xe = 0; Xe < $.length; Xe++)
          Rt[Xe] = U[Xe] = $[Xe];
        var Ze = S.touches[0];
        t.touchData.startGPosition = [Ze.clientX, Ze.clientY];
      }
    }
  }, !1);
  var Pe;
  t.registerBinding(e, "touchmove", Pe = function(S) {
    var q = t.touchData.capture;
    if (!(!q && !T(S))) {
      var $ = t.selection, U = t.cy, Z = t.touchData.now, ce = t.touchData.earlier, ee = U.zoom();
      if (S.touches[0]) {
        var ie = t.projectIntoViewport(S.touches[0].clientX, S.touches[0].clientY);
        Z[0] = ie[0], Z[1] = ie[1];
      }
      if (S.touches[1]) {
        var ie = t.projectIntoViewport(S.touches[1].clientX, S.touches[1].clientY);
        Z[2] = ie[0], Z[3] = ie[1];
      }
      if (S.touches[2]) {
        var ie = t.projectIntoViewport(S.touches[2].clientX, S.touches[2].clientY);
        Z[4] = ie[0], Z[5] = ie[1];
      }
      var se = function(wv) {
        return {
          originalEvent: S,
          type: wv,
          position: {
            x: Z[0],
            y: Z[1]
          }
        };
      }, pe = t.touchData.startGPosition, Ee;
      if (q && S.touches[0] && pe) {
        for (var Ce = [], fe = 0; fe < Z.length; fe++)
          Ce[fe] = Z[fe] - ce[fe];
        var ke = S.touches[0].clientX - pe[0], Ae = ke * ke, nt = S.touches[0].clientY - pe[1], Ke = nt * nt, Rt = Ae + Ke;
        Ee = Rt >= t.touchTapThreshold2;
      }
      if (q && t.touchData.cxt) {
        S.preventDefault();
        var Xe = S.touches[0].clientX - ae, Ze = S.touches[0].clientY - z, je = S.touches[1].clientX - ae, ut = S.touches[1].clientY - z, qt = re(Xe, Ze, je, ut), We = qt / Y, Xt = 150, Mt = Xt * Xt, tr = 1.5, Za = tr * tr;
        if (We >= Za || qt >= Mt) {
          t.touchData.cxt = !1, t.data.bgActivePosistion = void 0, t.redrawHint("select", !0);
          var xr = se("cxttapend");
          t.touchData.start ? (t.touchData.start.unactivate().emit(xr), t.touchData.start = null) : U.emit(xr);
        }
      }
      if (q && t.touchData.cxt) {
        var xr = se("cxtdrag");
        t.data.bgActivePosistion = void 0, t.redrawHint("select", !0), t.touchData.start ? t.touchData.start.emit(xr) : U.emit(xr), t.touchData.start && (t.touchData.start._private.grabbed = !1), t.touchData.cxtDragged = !0;
        var Et = t.findNearestElement(Z[0], Z[1], !0, !0);
        (!t.touchData.cxtOver || Et !== t.touchData.cxtOver) && (t.touchData.cxtOver && t.touchData.cxtOver.emit(se("cxtdragout")), t.touchData.cxtOver = Et, Et && Et.emit(se("cxtdragover")));
      } else if (q && S.touches[2] && U.boxSelectionEnabled())
        S.preventDefault(), t.data.bgActivePosistion = void 0, this.lastThreeTouch = +/* @__PURE__ */ new Date(), t.touchData.selecting || U.emit(se("boxstart")), t.touchData.selecting = !0, t.touchData.didSelect = !0, $[4] = 1, !$ || $.length === 0 || $[0] === void 0 ? ($[0] = (Z[0] + Z[2] + Z[4]) / 3, $[1] = (Z[1] + Z[3] + Z[5]) / 3, $[2] = (Z[0] + Z[2] + Z[4]) / 3 + 1, $[3] = (Z[1] + Z[3] + Z[5]) / 3 + 1) : ($[2] = (Z[0] + Z[2] + Z[4]) / 3, $[3] = (Z[1] + Z[3] + Z[5]) / 3), t.redrawHint("select", !0), t.redraw();
      else if (q && S.touches[1] && !t.touchData.didSelect && U.zoomingEnabled() && U.panningEnabled() && U.userZoomingEnabled() && U.userPanningEnabled()) {
        S.preventDefault(), t.data.bgActivePosistion = void 0, t.redrawHint("select", !0);
        var dt = t.dragData.touchDragEles;
        if (dt) {
          t.redrawHint("drag", !0);
          for (var Ct = 0; Ct < dt.length; Ct++) {
            var va = dt[Ct]._private;
            va.grabbed = !1, va.rscratch.inDragLayer = !1;
          }
        }
        var Pt = t.touchData.start, Xe = S.touches[0].clientX - ae, Ze = S.touches[0].clientY - z, je = S.touches[1].clientX - ae, ut = S.touches[1].clientY - z, Os = K(Xe, Ze, je, ut), fv = Os / J;
        if (W) {
          var vv = Xe - G, dv = Ze - F, hv = je - _, gv = ut - X, pv = (vv + hv) / 2, yv = (dv + gv) / 2, da = U.zoom(), ii = da * fv, Qa = U.pan(), Ns = te[0] * da + Qa.x, zs = te[1] * da + Qa.y, mv = {
            x: -ii / da * (Ns - Qa.x - pv) + Ns,
            y: -ii / da * (zs - Qa.y - yv) + zs
          };
          if (Pt && Pt.active()) {
            var dt = t.dragData.touchDragEles;
            y(dt), t.redrawHint("drag", !0), t.redrawHint("eles", !0), Pt.unactivate().emit(se("freeon")), dt.emit(se("free")), t.dragData.didDrag && (Pt.emit(se("dragfreeon")), dt.emit(se("dragfree")));
          }
          U.viewport({
            zoom: ii,
            pan: mv,
            cancelOnFailedZoom: !0
          }), U.emit(se("pinchzoom")), J = Os, G = Xe, F = Ze, _ = je, X = ut, t.pinching = !0;
        }
        if (S.touches[0]) {
          var ie = t.projectIntoViewport(S.touches[0].clientX, S.touches[0].clientY);
          Z[0] = ie[0], Z[1] = ie[1];
        }
        if (S.touches[1]) {
          var ie = t.projectIntoViewport(S.touches[1].clientX, S.touches[1].clientY);
          Z[2] = ie[0], Z[3] = ie[1];
        }
        if (S.touches[2]) {
          var ie = t.projectIntoViewport(S.touches[2].clientX, S.touches[2].clientY);
          Z[4] = ie[0], Z[5] = ie[1];
        }
      } else if (S.touches[0] && !t.touchData.didSelect) {
        var It = t.touchData.start, oi = t.touchData.last, Et;
        if (!t.hoverData.draggingEles && !t.swipePanning && (Et = t.findNearestElement(Z[0], Z[1], !0, !0)), q && It != null && S.preventDefault(), q && It != null && t.nodeIsDraggable(It))
          if (Ee) {
            var dt = t.dragData.touchDragEles, Fs = !t.dragData.didDrag;
            Fs && h(dt, {
              inDragLayer: !0
            }), t.dragData.didDrag = !0;
            var ha = {
              x: 0,
              y: 0
            };
            if (ne(Ce[0]) && ne(Ce[1]) && (ha.x += Ce[0], ha.y += Ce[1], Fs)) {
              t.redrawHint("eles", !0);
              var Lt = t.touchData.dragDelta;
              Lt && ne(Lt[0]) && ne(Lt[1]) && (ha.x += Lt[0], ha.y += Lt[1]);
            }
            t.hoverData.draggingEles = !0, dt.silentShift(ha).emit(se("position")).emit(se("drag")), t.redrawHint("drag", !0), t.touchData.startPosition[0] == ce[0] && t.touchData.startPosition[1] == ce[1] && t.redrawHint("eles", !0), t.redraw();
          } else {
            var Lt = t.touchData.dragDelta = t.touchData.dragDelta || [];
            Lt.length === 0 ? (Lt.push(Ce[0]), Lt.push(Ce[1])) : (Lt[0] += Ce[0], Lt[1] += Ce[1]);
          }
        if (n(It || Et, ["touchmove", "tapdrag", "vmousemove"], S, {
          x: Z[0],
          y: Z[1]
        }), (!It || !It.grabbed()) && Et != oi && (oi && oi.emit(se("tapdragout")), Et && Et.emit(se("tapdragover"))), t.touchData.last = Et, q)
          for (var Ct = 0; Ct < Z.length; Ct++)
            Z[Ct] && t.touchData.startPosition[Ct] && Ee && (t.touchData.singleTouchMoved = !0);
        if (q && (It == null || It.pannable()) && U.panningEnabled() && U.userPanningEnabled()) {
          var bv = o(It, t.touchData.starts);
          bv && (S.preventDefault(), t.data.bgActivePosistion || (t.data.bgActivePosistion = Xr(t.touchData.startPosition)), t.swipePanning ? (U.panBy({
            x: Ce[0] * ee,
            y: Ce[1] * ee
          }), U.emit(se("dragpan"))) : Ee && (t.swipePanning = !0, U.panBy({
            x: ke * ee,
            y: nt * ee
          }), U.emit(se("dragpan")), It && (It.unactivate(), t.redrawHint("select", !0), t.touchData.start = null)));
          var ie = t.projectIntoViewport(S.touches[0].clientX, S.touches[0].clientY);
          Z[0] = ie[0], Z[1] = ie[1];
        }
      }
      for (var fe = 0; fe < Z.length; fe++)
        ce[fe] = Z[fe];
      q && S.touches.length > 0 && !t.hoverData.draggingEles && !t.swipePanning && t.data.bgActivePosistion != null && (t.data.bgActivePosistion = void 0, t.redrawHint("select", !0), t.redraw());
    }
  }, !1);
  var we;
  t.registerBinding(e, "touchcancel", we = function(S) {
    var q = t.touchData.start;
    t.touchData.capture = !1, q && q.unactivate();
  });
  var me, ge, oe, de;
  if (t.registerBinding(e, "touchend", me = function(S) {
    var q = t.touchData.start, $ = t.touchData.capture;
    if ($)
      S.touches.length === 0 && (t.touchData.capture = !1), S.preventDefault();
    else
      return;
    var U = t.selection;
    t.swipePanning = !1, t.hoverData.draggingEles = !1;
    var Z = t.cy, ce = Z.zoom(), ee = t.touchData.now, ie = t.touchData.earlier;
    if (S.touches[0]) {
      var se = t.projectIntoViewport(S.touches[0].clientX, S.touches[0].clientY);
      ee[0] = se[0], ee[1] = se[1];
    }
    if (S.touches[1]) {
      var se = t.projectIntoViewport(S.touches[1].clientX, S.touches[1].clientY);
      ee[2] = se[0], ee[3] = se[1];
    }
    if (S.touches[2]) {
      var se = t.projectIntoViewport(S.touches[2].clientX, S.touches[2].clientY);
      ee[4] = se[0], ee[5] = se[1];
    }
    var pe = function(Mt) {
      return {
        originalEvent: S,
        type: Mt,
        position: {
          x: ee[0],
          y: ee[1]
        }
      };
    };
    q && q.unactivate();
    var Ee;
    if (t.touchData.cxt) {
      if (Ee = pe("cxttapend"), q ? q.emit(Ee) : Z.emit(Ee), !t.touchData.cxtDragged) {
        var Ce = pe("cxttap");
        q ? q.emit(Ce) : Z.emit(Ce);
      }
      t.touchData.start && (t.touchData.start._private.grabbed = !1), t.touchData.cxt = !1, t.touchData.start = null, t.redraw();
      return;
    }
    if (!S.touches[2] && Z.boxSelectionEnabled() && t.touchData.selecting) {
      t.touchData.selecting = !1;
      var fe = Z.collection(t.getAllInBox(U[0], U[1], U[2], U[3]));
      U[0] = void 0, U[1] = void 0, U[2] = void 0, U[3] = void 0, U[4] = 0, t.redrawHint("select", !0), Z.emit(pe("boxend"));
      var ke = function(Mt) {
        return Mt.selectable() && !Mt.selected();
      };
      fe.emit(pe("box")).stdFilter(ke).select().emit(pe("boxselect")), fe.nonempty() && t.redrawHint("eles", !0), t.redraw();
    }
    if (q != null && q.unactivate(), S.touches[2])
      t.data.bgActivePosistion = void 0, t.redrawHint("select", !0);
    else if (!S.touches[1]) {
      if (!S.touches[0]) {
        if (!S.touches[0]) {
          t.data.bgActivePosistion = void 0, t.redrawHint("select", !0);
          var Ae = t.dragData.touchDragEles;
          if (q != null) {
            var nt = q._private.grabbed;
            y(Ae), t.redrawHint("drag", !0), t.redrawHint("eles", !0), nt && (q.emit(pe("freeon")), Ae.emit(pe("free")), t.dragData.didDrag && (q.emit(pe("dragfreeon")), Ae.emit(pe("dragfree")))), n(q, ["touchend", "tapend", "vmouseup", "tapdragout"], S, {
              x: ee[0],
              y: ee[1]
            }), q.unactivate(), t.touchData.start = null;
          } else {
            var Ke = t.findNearestElement(ee[0], ee[1], !0, !0);
            n(Ke, ["touchend", "tapend", "vmouseup", "tapdragout"], S, {
              x: ee[0],
              y: ee[1]
            });
          }
          var Rt = t.touchData.startPosition[0] - ee[0], Xe = Rt * Rt, Ze = t.touchData.startPosition[1] - ee[1], je = Ze * Ze, ut = Xe + je, qt = ut * ce * ce;
          t.touchData.singleTouchMoved || (q || Z.$(":selected").unselect(["tapunselect"]), n(q, ["tap", "vclick"], S, {
            x: ee[0],
            y: ee[1]
          }), ge = !1, S.timeStamp - de <= Z.multiClickDebounceTime() ? (oe && clearTimeout(oe), ge = !0, de = null, n(q, ["dbltap", "vdblclick"], S, {
            x: ee[0],
            y: ee[1]
          })) : (oe = setTimeout(function() {
            ge || n(q, ["onetap", "voneclick"], S, {
              x: ee[0],
              y: ee[1]
            });
          }, Z.multiClickDebounceTime()), de = S.timeStamp)), q != null && !t.dragData.didDrag && q._private.selectable && qt < t.touchTapThreshold2 && !t.pinching && (Z.selectionType() === "single" ? (Z.$(r).unmerge(q).unselect(["tapunselect"]), q.select(["tapselect"])) : q.selected() ? q.unselect(["tapunselect"]) : q.select(["tapselect"]), t.redrawHint("eles", !0)), t.touchData.singleTouchMoved = !0;
        }
      }
    }
    for (var We = 0; We < ee.length; We++)
      ie[We] = ee[We];
    t.dragData.didDrag = !1, S.touches.length === 0 && (t.touchData.dragDelta = [], t.touchData.startPosition = [null, null, null, null, null, null], t.touchData.startGPosition = null, t.touchData.didSelect = !1), S.touches.length < 2 && (S.touches.length === 1 && (t.touchData.startGPosition = [S.touches[0].clientX, S.touches[0].clientY]), t.pinching = !1, t.redrawHint("eles", !0), t.redraw());
  }, !1), typeof TouchEvent > "u") {
    var ve = [], xe = function(S) {
      return {
        clientX: S.clientX,
        clientY: S.clientY,
        force: 1,
        identifier: S.pointerId,
        pageX: S.pageX,
        pageY: S.pageY,
        radiusX: S.width / 2,
        radiusY: S.height / 2,
        screenX: S.screenX,
        screenY: S.screenY,
        target: S.target
      };
    }, be = function(S) {
      return {
        event: S,
        touch: xe(S)
      };
    }, Se = function(S) {
      ve.push(be(S));
    }, De = function(S) {
      for (var q = 0; q < ve.length; q++) {
        var $ = ve[q];
        if ($.event.pointerId === S.pointerId) {
          ve.splice(q, 1);
          return;
        }
      }
    }, Oe = function(S) {
      var q = ve.filter(function($) {
        return $.event.pointerId === S.pointerId;
      })[0];
      q.event = S, q.touch = xe(S);
    }, Ie = function(S) {
      S.touches = ve.map(function(q) {
        return q.touch;
      });
    }, Be = function(S) {
      return S.pointerType === "mouse" || S.pointerType === 4;
    };
    t.registerBinding(t.container, "pointerdown", function(Q) {
      Be(Q) || (Q.preventDefault(), Se(Q), Ie(Q), le(Q));
    }), t.registerBinding(t.container, "pointerup", function(Q) {
      Be(Q) || (De(Q), Ie(Q), me(Q));
    }), t.registerBinding(t.container, "pointercancel", function(Q) {
      Be(Q) || (De(Q), Ie(Q), we(Q));
    }), t.registerBinding(t.container, "pointermove", function(Q) {
      Be(Q) || (Q.preventDefault(), Oe(Q), Ie(Q), Pe(Q));
    });
  }
};
var jt = {};
jt.generatePolygon = function(t, e) {
  return this.nodeShapes[t] = {
    renderer: this,
    name: t,
    points: e,
    draw: function(a, n, i, o, s, u) {
      this.renderer.nodeShapeImpl("polygon", a, n, i, o, s, this.points);
    },
    intersectLine: function(a, n, i, o, s, u, l, c) {
      return Ma(s, u, this.points, a, n, i / 2, o / 2, l);
    },
    checkPoint: function(a, n, i, o, s, u, l, c) {
      return Jt(a, n, this.points, u, l, o, s, [0, -1], i);
    },
    hasMiterBounds: t !== "rectangle",
    miterBounds: function(a, n, i, o, s, u) {
      return Uh(this.points, a, n, i, o, s);
    }
  };
};
jt.generateEllipse = function() {
  return this.nodeShapes.ellipse = {
    renderer: this,
    name: "ellipse",
    draw: function(e, r, a, n, i, o) {
      this.renderer.nodeShapeImpl(this.name, e, r, a, n, i);
    },
    intersectLine: function(e, r, a, n, i, o, s, u) {
      return eg(i, o, e, r, a / 2 + s, n / 2 + s);
    },
    checkPoint: function(e, r, a, n, i, o, s, u) {
      return Dr(e, r, n, i, o, s, a);
    }
  };
};
jt.generateRoundPolygon = function(t, e) {
  return this.nodeShapes[t] = {
    renderer: this,
    name: t,
    points: e,
    getOrCreateCorners: function(a, n, i, o, s, u, l) {
      if (u[l] !== void 0 && u[l + "-cx"] === a && u[l + "-cy"] === n)
        return u[l];
      u[l] = new Array(e.length / 2), u[l + "-cx"] = a, u[l + "-cy"] = n;
      var c = i / 2, f = o / 2;
      s = s === "auto" ? Gc(i, o) : s;
      for (var v = new Array(e.length / 2), h = 0; h < e.length / 2; h++)
        v[h] = {
          x: a + c * e[h * 2],
          y: n + f * e[h * 2 + 1]
        };
      var d, y, g, p, m = v.length;
      for (y = v[m - 1], d = 0; d < m; d++)
        g = v[d % m], p = v[(d + 1) % m], u[l][d] = Rs(y, g, p, s), y = g, g = p;
      return u[l];
    },
    draw: function(a, n, i, o, s, u, l) {
      this.renderer.nodeShapeImpl("round-polygon", a, n, i, o, s, this.points, this.getOrCreateCorners(n, i, o, s, u, l, "drawCorners"));
    },
    intersectLine: function(a, n, i, o, s, u, l, c, f) {
      return rg(s, u, this.points, a, n, i, o, l, this.getOrCreateCorners(a, n, i, o, c, f, "corners"));
    },
    checkPoint: function(a, n, i, o, s, u, l, c, f) {
      return jh(a, n, this.points, u, l, o, s, this.getOrCreateCorners(u, l, o, s, c, f, "corners"));
    }
  };
};
jt.generateRoundRectangle = function() {
  return this.nodeShapes["round-rectangle"] = this.nodeShapes.roundrectangle = {
    renderer: this,
    name: "round-rectangle",
    points: yt(4, 0),
    draw: function(e, r, a, n, i, o) {
      this.renderer.nodeShapeImpl(this.name, e, r, a, n, i, this.points, o);
    },
    intersectLine: function(e, r, a, n, i, o, s, u) {
      return Vc(i, o, e, r, a, n, s, u);
    },
    checkPoint: function(e, r, a, n, i, o, s, u) {
      var l = n / 2, c = i / 2;
      u = u === "auto" ? vr(n, i) : u, u = Math.min(l, c, u);
      var f = u * 2;
      return !!(Jt(e, r, this.points, o, s, n, i - f, [0, -1], a) || Jt(e, r, this.points, o, s, n - f, i, [0, -1], a) || Dr(e, r, f, f, o - l + u, s - c + u, a) || Dr(e, r, f, f, o + l - u, s - c + u, a) || Dr(e, r, f, f, o + l - u, s + c - u, a) || Dr(e, r, f, f, o - l + u, s + c - u, a));
    }
  };
};
jt.generateCutRectangle = function() {
  return this.nodeShapes["cut-rectangle"] = this.nodeShapes.cutrectangle = {
    renderer: this,
    name: "cut-rectangle",
    cornerLength: ws(),
    points: yt(4, 0),
    draw: function(e, r, a, n, i, o) {
      this.renderer.nodeShapeImpl(this.name, e, r, a, n, i, null, o);
    },
    generateCutTrianglePts: function(e, r, a, n, i) {
      var o = i === "auto" ? this.cornerLength : i, s = r / 2, u = e / 2, l = a - u, c = a + u, f = n - s, v = n + s;
      return {
        topLeft: [l, f + o, l + o, f, l + o, f + o],
        topRight: [c - o, f, c, f + o, c - o, f + o],
        bottomRight: [c, v - o, c - o, v, c - o, v - o],
        bottomLeft: [l + o, v, l, v - o, l + o, v - o]
      };
    },
    intersectLine: function(e, r, a, n, i, o, s, u) {
      var l = this.generateCutTrianglePts(a + 2 * s, n + 2 * s, e, r, u), c = [].concat.apply([], [l.topLeft.splice(0, 4), l.topRight.splice(0, 4), l.bottomRight.splice(0, 4), l.bottomLeft.splice(0, 4)]);
      return Ma(i, o, c, e, r);
    },
    checkPoint: function(e, r, a, n, i, o, s, u) {
      var l = u === "auto" ? this.cornerLength : u;
      if (Jt(e, r, this.points, o, s, n, i - 2 * l, [0, -1], a) || Jt(e, r, this.points, o, s, n - 2 * l, i, [0, -1], a))
        return !0;
      var c = this.generateCutTrianglePts(n, i, o, s);
      return Tt(e, r, c.topLeft) || Tt(e, r, c.topRight) || Tt(e, r, c.bottomRight) || Tt(e, r, c.bottomLeft);
    }
  };
};
jt.generateBarrel = function() {
  return this.nodeShapes.barrel = {
    renderer: this,
    name: "barrel",
    points: yt(4, 0),
    draw: function(e, r, a, n, i, o) {
      this.renderer.nodeShapeImpl(this.name, e, r, a, n, i);
    },
    intersectLine: function(e, r, a, n, i, o, s, u) {
      var l = 0.15, c = 0.5, f = 0.85, v = this.generateBarrelBezierPts(a + 2 * s, n + 2 * s, e, r), h = function(g) {
        var p = Qr({
          x: g[0],
          y: g[1]
        }, {
          x: g[2],
          y: g[3]
        }, {
          x: g[4],
          y: g[5]
        }, l), m = Qr({
          x: g[0],
          y: g[1]
        }, {
          x: g[2],
          y: g[3]
        }, {
          x: g[4],
          y: g[5]
        }, c), b = Qr({
          x: g[0],
          y: g[1]
        }, {
          x: g[2],
          y: g[3]
        }, {
          x: g[4],
          y: g[5]
        }, f);
        return [g[0], g[1], p.x, p.y, m.x, m.y, b.x, b.y, g[4], g[5]];
      }, d = [].concat(h(v.topLeft), h(v.topRight), h(v.bottomRight), h(v.bottomLeft));
      return Ma(i, o, d, e, r);
    },
    generateBarrelBezierPts: function(e, r, a, n) {
      var i = r / 2, o = e / 2, s = a - o, u = a + o, l = n - i, c = n + i, f = Ho(e, r), v = f.heightOffset, h = f.widthOffset, d = f.ctrlPtOffsetPct * e, y = {
        topLeft: [s, l + v, s + d, l, s + h, l],
        topRight: [u - h, l, u - d, l, u, l + v],
        bottomRight: [u, c - v, u - d, c, u - h, c],
        bottomLeft: [s + h, c, s + d, c, s, c - v]
      };
      return y.topLeft.isTop = !0, y.topRight.isTop = !0, y.bottomLeft.isBottom = !0, y.bottomRight.isBottom = !0, y;
    },
    checkPoint: function(e, r, a, n, i, o, s, u) {
      var l = Ho(n, i), c = l.heightOffset, f = l.widthOffset;
      if (Jt(e, r, this.points, o, s, n, i - 2 * c, [0, -1], a) || Jt(e, r, this.points, o, s, n - 2 * f, i, [0, -1], a))
        return !0;
      for (var v = this.generateBarrelBezierPts(n, i, o, s), h = function(T, k, D) {
        var P = D[4], A = D[2], B = D[0], R = D[5], I = D[1], O = Math.min(P, B), M = Math.max(P, B), N = Math.min(R, I), H = Math.max(R, I);
        if (O <= T && T <= M && N <= k && k <= H) {
          var G = ag(P, A, B), F = Yh(G[0], G[1], G[2], T), _ = F.filter(function(X) {
            return 0 <= X && X <= 1;
          });
          if (_.length > 0)
            return _[0];
        }
        return null;
      }, d = Object.keys(v), y = 0; y < d.length; y++) {
        var g = d[y], p = v[g], m = h(e, r, p);
        if (m != null) {
          var b = p[5], w = p[3], C = p[1], E = it(b, w, C, m);
          if (p.isTop && E <= r || p.isBottom && r <= E)
            return !0;
        }
      }
      return !1;
    }
  };
};
jt.generateBottomRoundrectangle = function() {
  return this.nodeShapes["bottom-round-rectangle"] = this.nodeShapes.bottomroundrectangle = {
    renderer: this,
    name: "bottom-round-rectangle",
    points: yt(4, 0),
    draw: function(e, r, a, n, i, o) {
      this.renderer.nodeShapeImpl(this.name, e, r, a, n, i, this.points, o);
    },
    intersectLine: function(e, r, a, n, i, o, s, u) {
      var l = e - (a / 2 + s), c = r - (n / 2 + s), f = c, v = e + (a / 2 + s), h = sr(i, o, e, r, l, c, v, f, !1);
      return h.length > 0 ? h : Vc(i, o, e, r, a, n, s, u);
    },
    checkPoint: function(e, r, a, n, i, o, s, u) {
      u = u === "auto" ? vr(n, i) : u;
      var l = 2 * u;
      if (Jt(e, r, this.points, o, s, n, i - l, [0, -1], a) || Jt(e, r, this.points, o, s, n - l, i, [0, -1], a))
        return !0;
      var c = n / 2 + 2 * a, f = i / 2 + 2 * a, v = [o - c, s - f, o - c, s, o + c, s, o + c, s - f];
      return !!(Tt(e, r, v) || Dr(e, r, l, l, o + n / 2 - u, s + i / 2 - u, a) || Dr(e, r, l, l, o - n / 2 + u, s + i / 2 - u, a));
    }
  };
};
jt.registerNodeShapes = function() {
  var t = this.nodeShapes = {}, e = this;
  this.generateEllipse(), this.generatePolygon("triangle", yt(3, 0)), this.generateRoundPolygon("round-triangle", yt(3, 0)), this.generatePolygon("rectangle", yt(4, 0)), t.square = t.rectangle, this.generateRoundRectangle(), this.generateCutRectangle(), this.generateBarrel(), this.generateBottomRoundrectangle();
  {
    var r = [0, 1, 1, 0, 0, -1, -1, 0];
    this.generatePolygon("diamond", r), this.generateRoundPolygon("round-diamond", r);
  }
  this.generatePolygon("pentagon", yt(5, 0)), this.generateRoundPolygon("round-pentagon", yt(5, 0)), this.generatePolygon("hexagon", yt(6, 0)), this.generateRoundPolygon("round-hexagon", yt(6, 0)), this.generatePolygon("heptagon", yt(7, 0)), this.generateRoundPolygon("round-heptagon", yt(7, 0)), this.generatePolygon("octagon", yt(8, 0)), this.generateRoundPolygon("round-octagon", yt(8, 0));
  var a = new Array(20);
  {
    var n = Go(5, 0), i = Go(5, Math.PI / 5), o = 0.5 * (3 - Math.sqrt(5));
    o *= 1.57;
    for (var s = 0; s < i.length / 2; s++)
      i[s * 2] *= o, i[s * 2 + 1] *= o;
    for (var s = 0; s < 20 / 4; s++)
      a[s * 4] = n[s * 2], a[s * 4 + 1] = n[s * 2 + 1], a[s * 4 + 2] = i[s * 2], a[s * 4 + 3] = i[s * 2 + 1];
  }
  a = _c(a), this.generatePolygon("star", a), this.generatePolygon("vee", [-1, -1, 0, -0.333, 1, -1, 0, 1]), this.generatePolygon("rhomboid", [-1, -1, 0.333, -1, 1, 1, -0.333, 1]), this.generatePolygon("right-rhomboid", [-0.333, -1, 1, -1, 0.333, 1, -1, 1]), this.nodeShapes.concavehexagon = this.generatePolygon("concave-hexagon", [-1, -0.95, -0.75, 0, -1, 0.95, 1, 0.95, 0.75, 0, 1, -0.95]);
  {
    var u = [-1, -1, 0.25, -1, 1, 0, 0.25, 1, -1, 1];
    this.generatePolygon("tag", u), this.generateRoundPolygon("round-tag", u);
  }
  t.makePolygon = function(l) {
    var c = l.join("$"), f = "polygon-" + c, v;
    return (v = this[f]) ? v : e.generatePolygon(f, l);
  };
};
var Xa = {};
Xa.timeToRender = function() {
  return this.redrawTotalTime / this.redrawCount;
};
Xa.redraw = function(t) {
  t = t || Nc();
  var e = this;
  e.averageRedrawTime === void 0 && (e.averageRedrawTime = 0), e.lastRedrawTime === void 0 && (e.lastRedrawTime = 0), e.lastDrawTime === void 0 && (e.lastDrawTime = 0), e.requestedFrame = !0, e.renderOptions = t;
};
Xa.beforeRender = function(t, e) {
  if (!this.destroyed) {
    e == null && $e("Priority is not optional for beforeRender");
    var r = this.beforeRenderCallbacks;
    r.push({
      fn: t,
      priority: e
    }), r.sort(function(a, n) {
      return n.priority - a.priority;
    });
  }
};
var ec = function(e, r, a) {
  for (var n = e.beforeRenderCallbacks, i = 0; i < n.length; i++)
    n[i].fn(r, a);
};
Xa.startRenderLoop = function() {
  var t = this, e = t.cy;
  if (!t.renderLoopStarted) {
    t.renderLoopStarted = !0;
    var r = function(n) {
      if (!t.destroyed) {
        if (!e.batching()) if (t.requestedFrame && !t.skipFrame) {
          ec(t, !0, n);
          var i = Qt();
          t.render(t.renderOptions);
          var o = t.lastDrawTime = Qt();
          t.averageRedrawTime === void 0 && (t.averageRedrawTime = o - i), t.redrawCount === void 0 && (t.redrawCount = 0), t.redrawCount++, t.redrawTotalTime === void 0 && (t.redrawTotalTime = 0);
          var s = o - i;
          t.redrawTotalTime += s, t.lastRedrawTime = s, t.averageRedrawTime = t.averageRedrawTime / 2 + s / 2, t.requestedFrame = !1;
        } else
          ec(t, !1, n);
        t.skipFrame = !1, An(r);
      }
    };
    An(r);
  }
};
var Em = function(e) {
  this.init(e);
}, $f = Em, fa = $f.prototype;
fa.clientFunctions = ["redrawHint", "render", "renderTo", "matchCanvasSize", "nodeShapeImpl", "arrowShapeImpl"];
fa.init = function(t) {
  var e = this;
  e.options = t, e.cy = t.cy;
  var r = e.container = t.cy.container(), a = e.cy.window();
  if (a) {
    var n = a.document, i = n.head, o = "__________cytoscape_stylesheet", s = "__________cytoscape_container", u = n.getElementById(o) != null;
    if (r.className.indexOf(s) < 0 && (r.className = (r.className || "") + " " + s), !u) {
      var l = n.createElement("style");
      l.id = o, l.textContent = "." + s + " { position: relative; }", i.insertBefore(l, i.children[0]);
    }
    var c = a.getComputedStyle(r), f = c.getPropertyValue("position");
    f === "static" && ze("A Cytoscape container has style position:static and so can not use UI extensions properly");
  }
  e.selection = [void 0, void 0, void 0, void 0, 0], e.bezierProjPcts = [0.05, 0.225, 0.4, 0.5, 0.6, 0.775, 0.95], e.hoverData = {
    down: null,
    last: null,
    downTime: null,
    triggerMode: null,
    dragging: !1,
    initialPan: [null, null],
    capture: !1
  }, e.dragData = {
    possibleDragElements: []
  }, e.touchData = {
    start: null,
    capture: !1,
    // These 3 fields related to tap, taphold events
    startPosition: [null, null, null, null, null, null],
    singleTouchStartTime: null,
    singleTouchMoved: !0,
    now: [null, null, null, null, null, null],
    earlier: [null, null, null, null, null, null]
  }, e.redraws = 0, e.showFps = t.showFps, e.debug = t.debug, e.webgl = t.webgl, e.hideEdgesOnViewport = t.hideEdgesOnViewport, e.textureOnViewport = t.textureOnViewport, e.wheelSensitivity = t.wheelSensitivity, e.motionBlurEnabled = t.motionBlur, e.forcedPixelRatio = ne(t.pixelRatio) ? t.pixelRatio : null, e.motionBlur = t.motionBlur, e.motionBlurOpacity = t.motionBlurOpacity, e.motionBlurTransparency = 1 - e.motionBlurOpacity, e.motionBlurPxRatio = 1, e.mbPxRBlurry = 1, e.minMbLowQualFrames = 4, e.fullQualityMb = !1, e.clearedForMotionBlur = [], e.desktopTapThreshold = t.desktopTapThreshold, e.desktopTapThreshold2 = t.desktopTapThreshold * t.desktopTapThreshold, e.touchTapThreshold = t.touchTapThreshold, e.touchTapThreshold2 = t.touchTapThreshold * t.touchTapThreshold, e.tapholdDuration = 500, e.bindings = [], e.beforeRenderCallbacks = [], e.beforeRenderPriorities = {
    // higher priority execs before lower one
    animations: 400,
    eleCalcs: 300,
    eleTxrDeq: 200,
    lyrTxrDeq: 150,
    lyrTxrSkip: 100
  }, e.registerNodeShapes(), e.registerArrowShapes(), e.registerCalculationListeners();
};
fa.notify = function(t, e) {
  var r = this, a = r.cy;
  if (!this.destroyed) {
    if (t === "init") {
      r.load();
      return;
    }
    if (t === "destroy") {
      r.destroy();
      return;
    }
    (t === "add" || t === "remove" || t === "move" && a.hasCompoundNodes() || t === "load" || t === "zorder" || t === "mount") && r.invalidateCachedZSortedEles(), t === "viewport" && r.redrawHint("select", !0), t === "gc" && r.redrawHint("gc", !0), (t === "load" || t === "resize" || t === "mount") && (r.invalidateContainerClientCoordsCache(), r.matchCanvasSize(r.container)), r.redrawHint("eles", !0), r.redrawHint("drag", !0), this.startRenderLoop(), this.redraw();
  }
};
fa.destroy = function() {
  var t = this;
  t.destroyed = !0, t.cy.stopAnimationLoop();
  for (var e = 0; e < t.bindings.length; e++) {
    var r = t.bindings[e], a = r, n = a.target;
    (n.off || n.removeEventListener).apply(n, a.args);
  }
  if (t.bindings = [], t.beforeRenderCallbacks = [], t.onUpdateEleCalcsFns = [], t.removeObserver && t.removeObserver.disconnect(), t.styleObserver && t.styleObserver.disconnect(), t.resizeObserver && t.resizeObserver.disconnect(), t.labelCalcDiv)
    try {
      document.body.removeChild(t.labelCalcDiv);
    } catch {
    }
};
fa.isHeadless = function() {
  return !1;
};
[Bs, _f, Gf, ca, jt, Xa].forEach(function(t) {
  ye(fa, t);
});
var Mo = 1e3 / 60, Wf = {
  setupDequeueing: function(e) {
    return function() {
      var a = this, n = this.renderer;
      if (!a.dequeueingSetup) {
        a.dequeueingSetup = !0;
        var i = Ha(function() {
          n.redrawHint("eles", !0), n.redrawHint("drag", !0), n.redraw();
        }, e.deqRedrawThreshold), o = function(l, c) {
          var f = Qt(), v = n.averageRedrawTime, h = n.lastRedrawTime, d = [], y = n.cy.extent(), g = n.getPixelRatio();
          for (l || n.flushRenderedStyleQueue(); ; ) {
            var p = Qt(), m = p - f, b = p - c;
            if (h < Mo) {
              var w = Mo - (l ? v : 0);
              if (b >= e.deqFastCost * w)
                break;
            } else if (l) {
              if (m >= e.deqCost * h || m >= e.deqAvgCost * v)
                break;
            } else if (b >= e.deqNoDrawCost * Mo)
              break;
            var C = e.deq(a, g, y);
            if (C.length > 0)
              for (var E = 0; E < C.length; E++)
                d.push(C[E]);
            else
              break;
          }
          d.length > 0 && (e.onDeqd(a, d), !l && e.shouldRedraw(a, d, g, y) && i());
        }, s = e.priority || gs;
        n.beforeRender(o, s(a));
      }
    };
  }
}, Cm = /* @__PURE__ */ (function() {
  function t(e) {
    var r = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : Bn;
    pr(this, t), this.idsByKey = new Zt(), this.keyForId = new Zt(), this.cachesByLvl = new Zt(), this.lvls = [], this.getKey = e, this.doesEleInvalidateKey = r;
  }
  return yr(t, [{
    key: "getIdsFor",
    value: function(r) {
      r == null && $e("Can not get id list for null key");
      var a = this.idsByKey, n = this.idsByKey.get(r);
      return n || (n = new oa(), a.set(r, n)), n;
    }
  }, {
    key: "addIdForKey",
    value: function(r, a) {
      r != null && this.getIdsFor(r).add(a);
    }
  }, {
    key: "deleteIdForKey",
    value: function(r, a) {
      r != null && this.getIdsFor(r).delete(a);
    }
  }, {
    key: "getNumberOfIdsForKey",
    value: function(r) {
      return r == null ? 0 : this.getIdsFor(r).size;
    }
  }, {
    key: "updateKeyMappingFor",
    value: function(r) {
      var a = r.id(), n = this.keyForId.get(a), i = this.getKey(r);
      this.deleteIdForKey(n, a), this.addIdForKey(i, a), this.keyForId.set(a, i);
    }
  }, {
    key: "deleteKeyMappingFor",
    value: function(r) {
      var a = r.id(), n = this.keyForId.get(a);
      this.deleteIdForKey(n, a), this.keyForId.delete(a);
    }
  }, {
    key: "keyHasChangedFor",
    value: function(r) {
      var a = r.id(), n = this.keyForId.get(a), i = this.getKey(r);
      return n !== i;
    }
  }, {
    key: "isInvalid",
    value: function(r) {
      return this.keyHasChangedFor(r) || this.doesEleInvalidateKey(r);
    }
  }, {
    key: "getCachesAt",
    value: function(r) {
      var a = this.cachesByLvl, n = this.lvls, i = a.get(r);
      return i || (i = new Zt(), a.set(r, i), n.push(r)), i;
    }
  }, {
    key: "getCache",
    value: function(r, a) {
      return this.getCachesAt(a).get(r);
    }
  }, {
    key: "get",
    value: function(r, a) {
      var n = this.getKey(r), i = this.getCache(n, a);
      return i != null && this.updateKeyMappingFor(r), i;
    }
  }, {
    key: "getForCachedKey",
    value: function(r, a) {
      var n = this.keyForId.get(r.id()), i = this.getCache(n, a);
      return i;
    }
  }, {
    key: "hasCache",
    value: function(r, a) {
      return this.getCachesAt(a).has(r);
    }
  }, {
    key: "has",
    value: function(r, a) {
      var n = this.getKey(r);
      return this.hasCache(n, a);
    }
  }, {
    key: "setCache",
    value: function(r, a, n) {
      n.key = r, this.getCachesAt(a).set(r, n);
    }
  }, {
    key: "set",
    value: function(r, a, n) {
      var i = this.getKey(r);
      this.setCache(i, a, n), this.updateKeyMappingFor(r);
    }
  }, {
    key: "deleteCache",
    value: function(r, a) {
      this.getCachesAt(a).delete(r);
    }
  }, {
    key: "delete",
    value: function(r, a) {
      var n = this.getKey(r);
      this.deleteCache(n, a);
    }
  }, {
    key: "invalidateKey",
    value: function(r) {
      var a = this;
      this.lvls.forEach(function(n) {
        return a.deleteCache(r, n);
      });
    }
    // returns true if no other eles reference the invalidated cache (n.b. other eles may need the cache with the same key)
  }, {
    key: "invalidate",
    value: function(r) {
      var a = r.id(), n = this.keyForId.get(a);
      this.deleteKeyMappingFor(r);
      var i = this.doesEleInvalidateKey(r);
      return i && this.invalidateKey(n), i || this.getNumberOfIdsForKey(n) === 0;
    }
  }]);
})(), tc = 25, fn = 50, En = -4, rs = 3, Uf = 7.99, Tm = 8, Sm = 1024, km = 1024, Pm = 1024, Dm = 0.2, Am = 0.8, Bm = 10, Rm = 0.15, Mm = 0.1, Im = 0.9, Lm = 0.9, Om = 100, Nm = 1, Zr = {
  dequeue: "dequeue",
  downscale: "downscale",
  highQuality: "highQuality"
}, zm = ft({
  getKey: null,
  doesEleInvalidateKey: Bn,
  drawElement: null,
  getBoundingBox: null,
  getRotationPoint: null,
  getRotationOffset: null,
  isVisible: Ic,
  allowEdgeTxrCaching: !0,
  allowParentTxrCaching: !0
}), Ta = function(e, r) {
  var a = this;
  a.renderer = e, a.onDequeues = [];
  var n = zm(r);
  ye(a, n), a.lookup = new Cm(n.getKey, n.doesEleInvalidateKey), a.setupDequeueing();
}, at = Ta.prototype;
at.reasons = Zr;
at.getTextureQueue = function(t) {
  var e = this;
  return e.eleImgCaches = e.eleImgCaches || {}, e.eleImgCaches[t] = e.eleImgCaches[t] || [];
};
at.getRetiredTextureQueue = function(t) {
  var e = this, r = e.eleImgCaches.retired = e.eleImgCaches.retired || {}, a = r[t] = r[t] || [];
  return a;
};
at.getElementQueue = function() {
  var t = this, e = t.eleCacheQueue = t.eleCacheQueue || new $a(function(r, a) {
    return a.reqs - r.reqs;
  });
  return e;
};
at.getElementKeyToQueue = function() {
  var t = this, e = t.eleKeyToCacheQueue = t.eleKeyToCacheQueue || {};
  return e;
};
at.getElement = function(t, e, r, a, n) {
  var i = this, o = this.renderer, s = o.cy.zoom(), u = this.lookup;
  if (!e || e.w === 0 || e.h === 0 || isNaN(e.w) || isNaN(e.h) || !t.visible() || t.removed() || !i.allowEdgeTxrCaching && t.isEdge() || !i.allowParentTxrCaching && t.isParent())
    return null;
  if (a == null && (a = Math.ceil(ys(s * r))), a < En)
    a = En;
  else if (s >= Uf || a > rs)
    return null;
  var l = Math.pow(2, a), c = e.h * l, f = e.w * l, v = o.eleTextBiggerThanMin(t, l);
  if (!this.isVisible(t, v))
    return null;
  var h = u.get(t, a);
  if (h && h.invalidated && (h.invalidated = !1, h.texture.invalidatedWidth -= h.width), h)
    return h;
  var d;
  if (c <= tc ? d = tc : c <= fn ? d = fn : d = Math.ceil(c / fn) * fn, c > Pm || f > km)
    return null;
  var y = i.getTextureQueue(d), g = y[y.length - 2], p = function() {
    return i.recycleTexture(d, f) || i.addTexture(d, f);
  };
  g || (g = y[y.length - 1]), g || (g = p()), g.width - g.usedWidth < f && (g = p());
  for (var m = function(O) {
    return O && O.scaledLabelShown === v;
  }, b = n && n === Zr.dequeue, w = n && n === Zr.highQuality, C = n && n === Zr.downscale, E, x = a + 1; x <= rs; x++) {
    var T = u.get(t, x);
    if (T) {
      E = T;
      break;
    }
  }
  var k = E && E.level === a + 1 ? E : null, D = function() {
    g.context.drawImage(k.texture.canvas, k.x, 0, k.width, k.height, g.usedWidth, 0, f, c);
  };
  if (g.context.setTransform(1, 0, 0, 1, 0, 0), g.context.clearRect(g.usedWidth, 0, f, d), m(k))
    D();
  else if (m(E))
    if (w) {
      for (var P = E.level; P > a; P--)
        k = i.getElement(t, e, r, P, Zr.downscale);
      D();
    } else
      return i.queueElement(t, E.level - 1), E;
  else {
    var A;
    if (!b && !w && !C)
      for (var B = a - 1; B >= En; B--) {
        var R = u.get(t, B);
        if (R) {
          A = R;
          break;
        }
      }
    if (m(A))
      return i.queueElement(t, a), A;
    g.context.translate(g.usedWidth, 0), g.context.scale(l, l), this.drawElement(g.context, t, e, v, !1), g.context.scale(1 / l, 1 / l), g.context.translate(-g.usedWidth, 0);
  }
  return h = {
    x: g.usedWidth,
    texture: g,
    level: a,
    scale: l,
    width: f,
    height: c,
    scaledLabelShown: v
  }, g.usedWidth += Math.ceil(f + Tm), g.eleCaches.push(h), u.set(t, a, h), i.checkTextureFullness(g), h;
};
at.invalidateElements = function(t) {
  for (var e = 0; e < t.length; e++)
    this.invalidateElement(t[e]);
};
at.invalidateElement = function(t) {
  var e = this, r = e.lookup, a = [], n = r.isInvalid(t);
  if (n) {
    for (var i = En; i <= rs; i++) {
      var o = r.getForCachedKey(t, i);
      o && a.push(o);
    }
    var s = r.invalidate(t);
    if (s)
      for (var u = 0; u < a.length; u++) {
        var l = a[u], c = l.texture;
        c.invalidatedWidth += l.width, l.invalidated = !0, e.checkTextureUtility(c);
      }
    e.removeFromQueue(t);
  }
};
at.checkTextureUtility = function(t) {
  t.invalidatedWidth >= Dm * t.width && this.retireTexture(t);
};
at.checkTextureFullness = function(t) {
  var e = this, r = e.getTextureQueue(t.height);
  t.usedWidth / t.width > Am && t.fullnessChecks >= Bm ? fr(r, t) : t.fullnessChecks++;
};
at.retireTexture = function(t) {
  var e = this, r = t.height, a = e.getTextureQueue(r), n = this.lookup;
  fr(a, t), t.retired = !0;
  for (var i = t.eleCaches, o = 0; o < i.length; o++) {
    var s = i[o];
    n.deleteCache(s.key, s.level);
  }
  ps(i);
  var u = e.getRetiredTextureQueue(r);
  u.push(t);
};
at.addTexture = function(t, e) {
  var r = this, a = r.getTextureQueue(t), n = {};
  return a.push(n), n.eleCaches = [], n.height = t, n.width = Math.max(Sm, e), n.usedWidth = 0, n.invalidatedWidth = 0, n.fullnessChecks = 0, n.canvas = r.renderer.makeOffscreenCanvas(n.width, n.height), n.context = n.canvas.getContext("2d"), n;
};
at.recycleTexture = function(t, e) {
  for (var r = this, a = r.getTextureQueue(t), n = r.getRetiredTextureQueue(t), i = 0; i < n.length; i++) {
    var o = n[i];
    if (o.width >= e)
      return o.retired = !1, o.usedWidth = 0, o.invalidatedWidth = 0, o.fullnessChecks = 0, ps(o.eleCaches), o.context.setTransform(1, 0, 0, 1, 0, 0), o.context.clearRect(0, 0, o.width, o.height), fr(n, o), a.push(o), o;
  }
};
at.queueElement = function(t, e) {
  var r = this, a = r.getElementQueue(), n = r.getElementKeyToQueue(), i = this.getKey(t), o = n[i];
  if (o)
    o.level = Math.max(o.level, e), o.eles.merge(t), o.reqs++, a.updateItem(o);
  else {
    var s = {
      eles: t.spawn().merge(t),
      level: e,
      reqs: 1,
      key: i
    };
    a.push(s), n[i] = s;
  }
};
at.dequeue = function(t) {
  for (var e = this, r = e.getElementQueue(), a = e.getElementKeyToQueue(), n = [], i = e.lookup, o = 0; o < Nm && r.size() > 0; o++) {
    var s = r.pop(), u = s.key, l = s.eles[0], c = i.hasCache(l, s.level);
    if (a[u] = null, c)
      continue;
    n.push(s);
    var f = e.getBoundingBox(l);
    e.getElement(l, f, t, s.level, Zr.dequeue);
  }
  return n;
};
at.removeFromQueue = function(t) {
  var e = this, r = e.getElementQueue(), a = e.getElementKeyToQueue(), n = this.getKey(t), i = a[n];
  i != null && (i.eles.length === 1 ? (i.reqs = hs, r.updateItem(i), r.pop(), a[n] = null) : i.eles.unmerge(t));
};
at.onDequeue = function(t) {
  this.onDequeues.push(t);
};
at.offDequeue = function(t) {
  fr(this.onDequeues, t);
};
at.setupDequeueing = Wf.setupDequeueing({
  deqRedrawThreshold: Om,
  deqCost: Rm,
  deqAvgCost: Mm,
  deqNoDrawCost: Im,
  deqFastCost: Lm,
  deq: function(e, r, a) {
    return e.dequeue(r, a);
  },
  onDeqd: function(e, r) {
    for (var a = 0; a < e.onDequeues.length; a++) {
      var n = e.onDequeues[a];
      n(r);
    }
  },
  shouldRedraw: function(e, r, a, n) {
    for (var i = 0; i < r.length; i++)
      for (var o = r[i].eles, s = 0; s < o.length; s++) {
        var u = o[s].boundingBox();
        if (bs(u, n))
          return !0;
      }
    return !1;
  },
  priority: function(e) {
    return e.renderer.beforeRenderPriorities.eleTxrDeq;
  }
});
var Fm = 1, ka = -4, Fn = 2, qm = 3.99, Vm = 50, _m = 50, Gm = 0.15, Hm = 0.1, $m = 0.9, Wm = 0.9, Um = 1, rc = 250, Km = 4e3 * 4e3, ac = 32767, Xm = !0, Kf = function(e) {
  var r = this, a = r.renderer = e, n = a.cy;
  r.layersByLevel = {}, r.firstGet = !0, r.lastInvalidationTime = Qt() - 2 * rc, r.skipping = !1, r.eleTxrDeqs = n.collection(), r.scheduleElementRefinement = Ha(function() {
    r.refineElementTextures(r.eleTxrDeqs), r.eleTxrDeqs.unmerge(r.eleTxrDeqs);
  }, _m), a.beforeRender(function(o, s) {
    s - r.lastInvalidationTime <= rc ? r.skipping = !0 : r.skipping = !1;
  }, a.beforeRenderPriorities.lyrTxrSkip);
  var i = function(s, u) {
    return u.reqs - s.reqs;
  };
  r.layersQueue = new $a(i), r.setupDequeueing();
}, vt = Kf.prototype, nc = 0, Ym = Math.pow(2, 53) - 1;
vt.makeLayer = function(t, e) {
  var r = Math.pow(2, e), a = Math.ceil(t.w * r), n = Math.ceil(t.h * r), i = this.renderer.makeOffscreenCanvas(a, n), o = {
    id: nc = ++nc % Ym,
    bb: t,
    level: e,
    width: a,
    height: n,
    canvas: i,
    context: i.getContext("2d"),
    eles: [],
    elesQueue: [],
    reqs: 0
  }, s = o.context, u = -o.bb.x1, l = -o.bb.y1;
  return s.scale(r, r), s.translate(u, l), o;
};
vt.getLayers = function(t, e, r) {
  var a = this, n = a.renderer, i = n.cy, o = i.zoom(), s = a.firstGet;
  if (a.firstGet = !1, r == null) {
    if (r = Math.ceil(ys(o * e)), r < ka)
      r = ka;
    else if (o >= qm || r > Fn)
      return null;
  }
  a.validateLayersElesOrdering(r, t);
  var u = a.layersByLevel, l = Math.pow(2, r), c = u[r] = u[r] || [], f, v = a.levelIsComplete(r, t), h, d = function() {
    var D = function(I) {
      if (a.validateLayersElesOrdering(I, t), a.levelIsComplete(I, t))
        return h = u[I], !0;
    }, P = function(I) {
      if (!h)
        for (var O = r + I; ka <= O && O <= Fn && !D(O); O += I)
          ;
    };
    P(1), P(-1);
    for (var A = c.length - 1; A >= 0; A--) {
      var B = c[A];
      B.invalid && fr(c, B);
    }
  };
  if (!v)
    d();
  else
    return c;
  var y = function() {
    if (!f) {
      f = wt();
      for (var D = 0; D < t.length; D++)
        Hh(f, t[D].boundingBox());
    }
    return f;
  }, g = function(D) {
    D = D || {};
    var P = D.after;
    y();
    var A = Math.ceil(f.w * l), B = Math.ceil(f.h * l);
    if (A > ac || B > ac)
      return null;
    var R = A * B;
    if (R > Km)
      return null;
    var I = a.makeLayer(f, r);
    if (P != null) {
      var O = c.indexOf(P) + 1;
      c.splice(O, 0, I);
    } else (D.insert === void 0 || D.insert) && c.unshift(I);
    return I;
  };
  if (a.skipping && !s)
    return null;
  for (var p = null, m = t.length / Fm, b = !s, w = 0; w < t.length; w++) {
    var C = t[w], E = C._private.rscratch, x = E.imgLayerCaches = E.imgLayerCaches || {}, T = x[r];
    if (T) {
      p = T;
      continue;
    }
    if ((!p || p.eles.length >= m || !qc(p.bb, C.boundingBox())) && (p = g({
      insert: !0,
      after: p
    }), !p))
      return null;
    h || b ? a.queueLayer(p, C) : a.drawEleInLayer(p, C, r, e), p.eles.push(C), x[r] = p;
  }
  return h || (b ? null : c);
};
vt.getEleLevelForLayerLevel = function(t, e) {
  return t;
};
vt.drawEleInLayer = function(t, e, r, a) {
  var n = this, i = this.renderer, o = t.context, s = e.boundingBox();
  s.w === 0 || s.h === 0 || !e.visible() || (r = n.getEleLevelForLayerLevel(r, a), i.setImgSmoothing(o, !1), i.drawCachedElement(o, e, null, null, r, Xm), i.setImgSmoothing(o, !0));
};
vt.levelIsComplete = function(t, e) {
  var r = this, a = r.layersByLevel[t];
  if (!a || a.length === 0)
    return !1;
  for (var n = 0, i = 0; i < a.length; i++) {
    var o = a[i];
    if (o.reqs > 0 || o.invalid)
      return !1;
    n += o.eles.length;
  }
  return n === e.length;
};
vt.validateLayersElesOrdering = function(t, e) {
  var r = this.layersByLevel[t];
  if (r)
    for (var a = 0; a < r.length; a++) {
      for (var n = r[a], i = -1, o = 0; o < e.length; o++)
        if (n.eles[0] === e[o]) {
          i = o;
          break;
        }
      if (i < 0) {
        this.invalidateLayer(n);
        continue;
      }
      for (var s = i, o = 0; o < n.eles.length; o++)
        if (n.eles[o] !== e[s + o]) {
          this.invalidateLayer(n);
          break;
        }
    }
};
vt.updateElementsInLayers = function(t, e) {
  for (var r = this, a = qa(t[0]), n = 0; n < t.length; n++)
    for (var i = a ? null : t[n], o = a ? t[n] : t[n].ele, s = o._private.rscratch, u = s.imgLayerCaches = s.imgLayerCaches || {}, l = ka; l <= Fn; l++) {
      var c = u[l];
      c && (i && r.getEleLevelForLayerLevel(c.level) !== i.level || e(c, o, i));
    }
};
vt.haveLayers = function() {
  for (var t = this, e = !1, r = ka; r <= Fn; r++) {
    var a = t.layersByLevel[r];
    if (a && a.length > 0) {
      e = !0;
      break;
    }
  }
  return e;
};
vt.invalidateElements = function(t) {
  var e = this;
  t.length !== 0 && (e.lastInvalidationTime = Qt(), !(t.length === 0 || !e.haveLayers()) && e.updateElementsInLayers(t, function(a, n, i) {
    e.invalidateLayer(a);
  }));
};
vt.invalidateLayer = function(t) {
  if (this.lastInvalidationTime = Qt(), !t.invalid) {
    var e = t.level, r = t.eles, a = this.layersByLevel[e];
    fr(a, t), t.elesQueue = [], t.invalid = !0, t.replacement && (t.replacement.invalid = !0);
    for (var n = 0; n < r.length; n++) {
      var i = r[n]._private.rscratch.imgLayerCaches;
      i && (i[e] = null);
    }
  }
};
vt.refineElementTextures = function(t) {
  var e = this;
  e.updateElementsInLayers(t, function(a, n, i) {
    var o = a.replacement;
    if (o || (o = a.replacement = e.makeLayer(a.bb, a.level), o.replaces = a, o.eles = a.eles), !o.reqs)
      for (var s = 0; s < o.eles.length; s++)
        e.queueLayer(o, o.eles[s]);
  });
};
vt.enqueueElementRefinement = function(t) {
  this.eleTxrDeqs.merge(t), this.scheduleElementRefinement();
};
vt.queueLayer = function(t, e) {
  var r = this, a = r.layersQueue, n = t.elesQueue, i = n.hasId = n.hasId || {};
  if (!t.replacement) {
    if (e) {
      if (i[e.id()])
        return;
      n.push(e), i[e.id()] = !0;
    }
    t.reqs ? (t.reqs++, a.updateItem(t)) : (t.reqs = 1, a.push(t));
  }
};
vt.dequeue = function(t) {
  for (var e = this, r = e.layersQueue, a = [], n = 0; n < Um && r.size() !== 0; ) {
    var i = r.peek();
    if (i.replacement) {
      r.pop();
      continue;
    }
    if (i.replaces && i !== i.replaces.replacement) {
      r.pop();
      continue;
    }
    if (i.invalid) {
      r.pop();
      continue;
    }
    var o = i.elesQueue.shift();
    o && (e.drawEleInLayer(i, o, i.level, t), n++), a.length === 0 && a.push(!0), i.elesQueue.length === 0 && (r.pop(), i.reqs = 0, i.replaces && e.applyLayerReplacement(i), e.requestRedraw());
  }
  return a;
};
vt.applyLayerReplacement = function(t) {
  var e = this, r = e.layersByLevel[t.level], a = t.replaces, n = r.indexOf(a);
  if (!(n < 0 || a.invalid)) {
    r[n] = t;
    for (var i = 0; i < t.eles.length; i++) {
      var o = t.eles[i]._private, s = o.imgLayerCaches = o.imgLayerCaches || {};
      s && (s[t.level] = t);
    }
    e.requestRedraw();
  }
};
vt.requestRedraw = Ha(function() {
  var t = this.renderer;
  t.redrawHint("eles", !0), t.redrawHint("drag", !0), t.redraw();
}, 100);
vt.setupDequeueing = Wf.setupDequeueing({
  deqRedrawThreshold: Vm,
  deqCost: Gm,
  deqAvgCost: Hm,
  deqNoDrawCost: $m,
  deqFastCost: Wm,
  deq: function(e, r) {
    return e.dequeue(r);
  },
  onDeqd: gs,
  shouldRedraw: Ic,
  priority: function(e) {
    return e.renderer.beforeRenderPriorities.lyrTxrDeq;
  }
});
var Xf = {}, ic;
function Zm(t, e) {
  for (var r = 0; r < e.length; r++) {
    var a = e[r];
    t.lineTo(a.x, a.y);
  }
}
function Qm(t, e, r) {
  for (var a, n = 0; n < e.length; n++) {
    var i = e[n];
    n === 0 && (a = i), t.lineTo(i.x, i.y);
  }
  t.quadraticCurveTo(r.x, r.y, a.x, a.y);
}
function oc(t, e, r) {
  t.beginPath && t.beginPath();
  for (var a = e, n = 0; n < a.length; n++) {
    var i = a[n];
    t.lineTo(i.x, i.y);
  }
  var o = r, s = r[0];
  t.moveTo(s.x, s.y);
  for (var n = 1; n < o.length; n++) {
    var i = o[n];
    t.lineTo(i.x, i.y);
  }
  t.closePath && t.closePath();
}
function Jm(t, e, r, a, n) {
  t.beginPath && t.beginPath(), t.arc(r, a, n, 0, Math.PI * 2, !1);
  var i = e, o = i[0];
  t.moveTo(o.x, o.y);
  for (var s = 0; s < i.length; s++) {
    var u = i[s];
    t.lineTo(u.x, u.y);
  }
  t.closePath && t.closePath();
}
function jm(t, e, r, a) {
  t.arc(e, r, a, 0, Math.PI * 2, !1);
}
Xf.arrowShapeImpl = function(t) {
  return (ic || (ic = {
    polygon: Zm,
    "triangle-backcurve": Qm,
    "triangle-tee": oc,
    "circle-triangle": Jm,
    "triangle-cross": oc,
    circle: jm
  }))[t];
};
var Kt = {};
Kt.drawElement = function(t, e, r, a, n, i) {
  var o = this;
  e.isNode() ? o.drawNode(t, e, r, a, n, i) : o.drawEdge(t, e, r, a, n, i);
};
Kt.drawElementOverlay = function(t, e) {
  var r = this;
  e.isNode() ? r.drawNodeOverlay(t, e) : r.drawEdgeOverlay(t, e);
};
Kt.drawElementUnderlay = function(t, e) {
  var r = this;
  e.isNode() ? r.drawNodeUnderlay(t, e) : r.drawEdgeUnderlay(t, e);
};
Kt.drawCachedElementPortion = function(t, e, r, a, n, i, o, s) {
  var u = this, l = r.getBoundingBox(e);
  if (!(l.w === 0 || l.h === 0)) {
    var c = r.getElement(e, l, a, n, i);
    if (c != null) {
      var f = s(u, e);
      if (f === 0)
        return;
      var v = o(u, e), h = l.x1, d = l.y1, y = l.w, g = l.h, p, m, b, w, C;
      if (v !== 0) {
        var E = r.getRotationPoint(e);
        b = E.x, w = E.y, t.translate(b, w), t.rotate(v), C = u.getImgSmoothing(t), C || u.setImgSmoothing(t, !0);
        var x = r.getRotationOffset(e);
        p = x.x, m = x.y;
      } else
        p = h, m = d;
      var T;
      f !== 1 && (T = t.globalAlpha, t.globalAlpha = T * f), t.drawImage(c.texture.canvas, c.x, 0, c.width, c.height, p, m, y, g), f !== 1 && (t.globalAlpha = T), v !== 0 && (t.rotate(-v), t.translate(-b, -w), C || u.setImgSmoothing(t, !1));
    } else
      r.drawElement(t, e);
  }
};
var e0 = function() {
  return 0;
}, t0 = function(e, r) {
  return e.getTextAngle(r, null);
}, r0 = function(e, r) {
  return e.getTextAngle(r, "source");
}, a0 = function(e, r) {
  return e.getTextAngle(r, "target");
}, n0 = function(e, r) {
  return r.effectiveOpacity();
}, Io = function(e, r) {
  return r.pstyle("text-opacity").pfValue * r.effectiveOpacity();
};
Kt.drawCachedElement = function(t, e, r, a, n, i) {
  var o = this, s = o.data, u = s.eleTxrCache, l = s.lblTxrCache, c = s.slbTxrCache, f = s.tlbTxrCache, v = e.boundingBox(), h = i === !0 ? u.reasons.highQuality : null;
  if (!(v.w === 0 || v.h === 0 || !e.visible()) && (!a || bs(v, a))) {
    var d = e.isEdge(), y = e.element()._private.rscratch.badLine;
    o.drawElementUnderlay(t, e), o.drawCachedElementPortion(t, e, u, r, n, h, e0, n0), (!d || !y) && o.drawCachedElementPortion(t, e, l, r, n, h, t0, Io), d && !y && (o.drawCachedElementPortion(t, e, c, r, n, h, r0, Io), o.drawCachedElementPortion(t, e, f, r, n, h, a0, Io)), o.drawElementOverlay(t, e);
  }
};
Kt.drawElements = function(t, e) {
  for (var r = this, a = 0; a < e.length; a++) {
    var n = e[a];
    r.drawElement(t, n);
  }
};
Kt.drawCachedElements = function(t, e, r, a) {
  for (var n = this, i = 0; i < e.length; i++) {
    var o = e[i];
    n.drawCachedElement(t, o, r, a);
  }
};
Kt.drawCachedNodes = function(t, e, r, a) {
  for (var n = this, i = 0; i < e.length; i++) {
    var o = e[i];
    o.isNode() && n.drawCachedElement(t, o, r, a);
  }
};
Kt.drawLayeredElements = function(t, e, r, a) {
  var n = this, i = n.data.lyrTxrCache.getLayers(e, r);
  if (i)
    for (var o = 0; o < i.length; o++) {
      var s = i[o], u = s.bb;
      u.w === 0 || u.h === 0 || t.drawImage(s.canvas, u.x1, u.y1, u.w, u.h);
    }
  else
    n.drawCachedElements(t, e, r, a);
};
var er = {};
er.drawEdge = function(t, e, r) {
  var a = arguments.length > 3 && arguments[3] !== void 0 ? arguments[3] : !0, n = arguments.length > 4 && arguments[4] !== void 0 ? arguments[4] : !0, i = arguments.length > 5 && arguments[5] !== void 0 ? arguments[5] : !0, o = this, s = e._private.rscratch;
  if (!(i && !e.visible()) && !(s.badLine || s.allpts == null || isNaN(s.allpts[0]))) {
    var u;
    r && (u = r, t.translate(-u.x1, -u.y1));
    var l = i ? e.pstyle("opacity").value : 1, c = i ? e.pstyle("line-opacity").value : 1, f = e.pstyle("curve-style").value, v = e.pstyle("line-style").value, h = e.pstyle("width").pfValue, d = e.pstyle("line-cap").value, y = e.pstyle("line-outline-width").value, g = e.pstyle("line-outline-color").value, p = l * c, m = l * c, b = function() {
      var I = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : p;
      f === "straight-triangle" ? (o.eleStrokeStyle(t, e, I), o.drawEdgeTrianglePath(e, t, s.allpts)) : (t.lineWidth = h, t.lineCap = d, o.eleStrokeStyle(t, e, I), o.drawEdgePath(e, t, s.allpts, v), t.lineCap = "butt");
    }, w = function() {
      var I = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : p;
      if (t.lineWidth = h + y, t.lineCap = d, y > 0)
        o.colorStrokeStyle(t, g[0], g[1], g[2], I);
      else {
        t.lineCap = "butt";
        return;
      }
      f === "straight-triangle" ? o.drawEdgeTrianglePath(e, t, s.allpts) : (o.drawEdgePath(e, t, s.allpts, v), t.lineCap = "butt");
    }, C = function() {
      n && o.drawEdgeOverlay(t, e);
    }, E = function() {
      n && o.drawEdgeUnderlay(t, e);
    }, x = function() {
      var I = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : m;
      o.drawArrowheads(t, e, I);
    }, T = function() {
      o.drawElementText(t, e, null, a);
    };
    t.lineJoin = "round";
    var k = e.pstyle("ghost").value === "yes";
    if (k) {
      var D = e.pstyle("ghost-offset-x").pfValue, P = e.pstyle("ghost-offset-y").pfValue, A = e.pstyle("ghost-opacity").value, B = p * A;
      t.translate(D, P), b(B), x(B), t.translate(-D, -P);
    } else
      w();
    E(), b(), x(), C(), T(), r && t.translate(u.x1, u.y1);
  }
};
var Yf = function(e) {
  if (!["overlay", "underlay"].includes(e))
    throw new Error("Invalid state");
  return function(r, a) {
    if (a.visible()) {
      var n = a.pstyle("".concat(e, "-opacity")).value;
      if (n !== 0) {
        var i = this, o = i.usePaths(), s = a._private.rscratch, u = a.pstyle("".concat(e, "-padding")).pfValue, l = 2 * u, c = a.pstyle("".concat(e, "-color")).value;
        r.lineWidth = l, s.edgeType === "self" && !o ? r.lineCap = "butt" : r.lineCap = "round", i.colorStrokeStyle(r, c[0], c[1], c[2], n), i.drawEdgePath(a, r, s.allpts, "solid");
      }
    }
  };
};
er.drawEdgeOverlay = Yf("overlay");
er.drawEdgeUnderlay = Yf("underlay");
er.drawEdgePath = function(t, e, r, a) {
  var n = t._private.rscratch, i = e, o, s = !1, u = this.usePaths(), l = t.pstyle("line-dash-pattern").pfValue, c = t.pstyle("line-dash-offset").pfValue;
  if (u) {
    var f = r.join("$"), v = n.pathCacheKey && n.pathCacheKey === f;
    v ? (o = e = n.pathCache, s = !0) : (o = e = new Path2D(), n.pathCacheKey = f, n.pathCache = o);
  }
  if (i.setLineDash)
    switch (a) {
      case "dotted":
        i.setLineDash([1, 1]);
        break;
      case "dashed":
        i.setLineDash(l), i.lineDashOffset = c;
        break;
      case "solid":
        i.setLineDash([]);
        break;
    }
  if (!s && !n.badLine)
    switch (e.beginPath && e.beginPath(), e.moveTo(r[0], r[1]), n.edgeType) {
      case "bezier":
      case "self":
      case "compound":
      case "multibezier":
        for (var h = 2; h + 3 < r.length; h += 4)
          e.quadraticCurveTo(r[h], r[h + 1], r[h + 2], r[h + 3]);
        break;
      case "straight":
      case "haystack":
        for (var d = 2; d + 1 < r.length; d += 2)
          e.lineTo(r[d], r[d + 1]);
        break;
      case "segments":
        if (n.isRound) {
          var y = St(n.roundCorners), g;
          try {
            for (y.s(); !(g = y.n()).done; ) {
              var p = g.value;
              Nf(e, p);
            }
          } catch (b) {
            y.e(b);
          } finally {
            y.f();
          }
          e.lineTo(r[r.length - 2], r[r.length - 1]);
        } else
          for (var m = 2; m + 1 < r.length; m += 2)
            e.lineTo(r[m], r[m + 1]);
        break;
    }
  e = i, u ? e.stroke(o) : e.stroke(), e.setLineDash && e.setLineDash([]);
};
er.drawEdgeTrianglePath = function(t, e, r) {
  e.fillStyle = e.strokeStyle;
  for (var a = t.pstyle("width").pfValue, n = 0; n + 1 < r.length; n += 2) {
    var i = [r[n + 2] - r[n], r[n + 3] - r[n + 1]], o = Math.sqrt(i[0] * i[0] + i[1] * i[1]), s = [i[1] / o, -i[0] / o], u = [s[0] * a / 2, s[1] * a / 2];
    e.beginPath(), e.moveTo(r[n] - u[0], r[n + 1] - u[1]), e.lineTo(r[n] + u[0], r[n + 1] + u[1]), e.lineTo(r[n + 2], r[n + 3]), e.closePath(), e.fill();
  }
};
er.drawArrowheads = function(t, e, r) {
  var a = e._private.rscratch, n = a.edgeType === "haystack";
  n || this.drawArrowhead(t, e, "source", a.arrowStartX, a.arrowStartY, a.srcArrowAngle, r), this.drawArrowhead(t, e, "mid-target", a.midX, a.midY, a.midtgtArrowAngle, r), this.drawArrowhead(t, e, "mid-source", a.midX, a.midY, a.midsrcArrowAngle, r), n || this.drawArrowhead(t, e, "target", a.arrowEndX, a.arrowEndY, a.tgtArrowAngle, r);
};
er.drawArrowhead = function(t, e, r, a, n, i, o) {
  if (!(isNaN(a) || a == null || isNaN(n) || n == null || isNaN(i) || i == null)) {
    var s = this, u = e.pstyle(r + "-arrow-shape").value;
    if (u !== "none") {
      var l = e.pstyle(r + "-arrow-fill").value === "hollow" ? "both" : "filled", c = e.pstyle(r + "-arrow-fill").value, f = e.pstyle("width").pfValue, v = e.pstyle(r + "-arrow-width"), h = v.value === "match-line" ? f : v.pfValue;
      v.units === "%" && (h *= f);
      var d = e.pstyle("opacity").value;
      o === void 0 && (o = d);
      var y = t.globalCompositeOperation;
      (o !== 1 || c === "hollow") && (t.globalCompositeOperation = "destination-out", s.colorFillStyle(t, 255, 255, 255, 1), s.colorStrokeStyle(t, 255, 255, 255, 1), s.drawArrowShape(e, t, l, f, u, h, a, n, i), t.globalCompositeOperation = y);
      var g = e.pstyle(r + "-arrow-color").value;
      s.colorFillStyle(t, g[0], g[1], g[2], o), s.colorStrokeStyle(t, g[0], g[1], g[2], o), s.drawArrowShape(e, t, c, f, u, h, a, n, i);
    }
  }
};
er.drawArrowShape = function(t, e, r, a, n, i, o, s, u) {
  var l = this, c = this.usePaths() && n !== "triangle-cross", f = !1, v, h = e, d = {
    x: o,
    y: s
  }, y = t.pstyle("arrow-scale").value, g = this.getArrowWidth(a, y), p = l.arrowShapes[n];
  if (c) {
    var m = l.arrowPathCache = l.arrowPathCache || [], b = Ar(n), w = m[b];
    w != null ? (v = e = w, f = !0) : (v = e = new Path2D(), m[b] = v);
  }
  f || (e.beginPath && e.beginPath(), c ? p.draw(e, 1, 0, {
    x: 0,
    y: 0
  }, 1) : p.draw(e, g, u, d, a), e.closePath && e.closePath()), e = h, c && (e.translate(o, s), e.rotate(u), e.scale(g, g)), (r === "filled" || r === "both") && (c ? e.fill(v) : e.fill()), (r === "hollow" || r === "both") && (e.lineWidth = i / (c ? g : 1), e.lineJoin = "miter", c ? e.stroke(v) : e.stroke()), c && (e.scale(1 / g, 1 / g), e.rotate(-u), e.translate(-o, -s));
};
var Is = {};
Is.safeDrawImage = function(t, e, r, a, n, i, o, s, u, l) {
  if (!(n <= 0 || i <= 0 || u <= 0 || l <= 0))
    try {
      t.drawImage(e, r, a, n, i, o, s, u, l);
    } catch (c) {
      ze(c);
    }
};
Is.drawInscribedImage = function(t, e, r, a, n) {
  var i = this, o = r.position(), s = o.x, u = o.y, l = r.cy().style(), c = l.getIndexedStyle.bind(l), f = c(r, "background-fit", "value", a), v = c(r, "background-repeat", "value", a), h = r.width(), d = r.height(), y = r.padding() * 2, g = h + (c(r, "background-width-relative-to", "value", a) === "inner" ? 0 : y), p = d + (c(r, "background-height-relative-to", "value", a) === "inner" ? 0 : y), m = r._private.rscratch, b = c(r, "background-clip", "value", a), w = b === "node", C = c(r, "background-image-opacity", "value", a) * n, E = c(r, "background-image-smoothing", "value", a), x = r.pstyle("corner-radius").value;
  x !== "auto" && (x = r.pstyle("corner-radius").pfValue);
  var T = e.width || e.cachedW, k = e.height || e.cachedH;
  (T == null || k == null) && (document.body.appendChild(e), T = e.cachedW = e.width || e.offsetWidth, k = e.cachedH = e.height || e.offsetHeight, document.body.removeChild(e));
  var D = T, P = k;
  if (c(r, "background-width", "value", a) !== "auto" && (c(r, "background-width", "units", a) === "%" ? D = c(r, "background-width", "pfValue", a) * g : D = c(r, "background-width", "pfValue", a)), c(r, "background-height", "value", a) !== "auto" && (c(r, "background-height", "units", a) === "%" ? P = c(r, "background-height", "pfValue", a) * p : P = c(r, "background-height", "pfValue", a)), !(D === 0 || P === 0)) {
    if (f === "contain") {
      var A = Math.min(g / D, p / P);
      D *= A, P *= A;
    } else if (f === "cover") {
      var A = Math.max(g / D, p / P);
      D *= A, P *= A;
    }
    var B = s - g / 2, R = c(r, "background-position-x", "units", a), I = c(r, "background-position-x", "pfValue", a);
    R === "%" ? B += (g - D) * I : B += I;
    var O = c(r, "background-offset-x", "units", a), M = c(r, "background-offset-x", "pfValue", a);
    O === "%" ? B += (g - D) * M : B += M;
    var N = u - p / 2, H = c(r, "background-position-y", "units", a), G = c(r, "background-position-y", "pfValue", a);
    H === "%" ? N += (p - P) * G : N += G;
    var F = c(r, "background-offset-y", "units", a), _ = c(r, "background-offset-y", "pfValue", a);
    F === "%" ? N += (p - P) * _ : N += _, m.pathCache && (B -= s, N -= u, s = 0, u = 0);
    var X = t.globalAlpha;
    t.globalAlpha = C;
    var J = i.getImgSmoothing(t), Y = !1;
    if (E === "no" && J ? (i.setImgSmoothing(t, !1), Y = !0) : E === "yes" && !J && (i.setImgSmoothing(t, !0), Y = !0), v === "no-repeat")
      w && (t.save(), m.pathCache ? t.clip(m.pathCache) : (i.nodeShapes[i.getNodeShape(r)].draw(t, s, u, g, p, x, m), t.clip())), i.safeDrawImage(t, e, 0, 0, T, k, B, N, D, P), w && t.restore();
    else {
      var j = t.createPattern(e, v);
      t.fillStyle = j, i.nodeShapes[i.getNodeShape(r)].draw(t, s, u, g, p, x, m), t.translate(B, N), t.fill(), t.translate(-B, -N);
    }
    t.globalAlpha = X, Y && i.setImgSmoothing(t, J);
  }
};
var Nr = {};
Nr.eleTextBiggerThanMin = function(t, e) {
  if (!e) {
    var r = t.cy().zoom(), a = this.getPixelRatio(), n = Math.ceil(ys(r * a));
    e = Math.pow(2, n);
  }
  var i = t.pstyle("font-size").pfValue * e, o = t.pstyle("min-zoomed-font-size").pfValue;
  return !(i < o);
};
Nr.drawElementText = function(t, e, r, a, n) {
  var i = arguments.length > 5 && arguments[5] !== void 0 ? arguments[5] : !0, o = this;
  if (a == null) {
    if (i && !o.eleTextBiggerThanMin(e))
      return;
  } else if (a === !1)
    return;
  if (e.isNode()) {
    var s = e.pstyle("label");
    if (!s || !s.value)
      return;
    var u = o.getLabelJustification(e), l = e.pstyle("text-metrics").strValue === "glyph";
    t.textAlign = u, t.textBaseline = l ? "alphabetic" : "bottom";
  } else {
    var c = e.element()._private.rscratch.badLine, f = e.pstyle("label"), v = e.pstyle("source-label"), h = e.pstyle("target-label");
    if (c || (!f || !f.value) && (!v || !v.value) && (!h || !h.value))
      return;
    t.textAlign = "center", t.textBaseline = "bottom";
  }
  var d = !r, y;
  r && (y = r, t.translate(-y.x1, -y.y1)), n == null ? (o.drawText(t, e, null, d, i), e.isEdge() && (o.drawText(t, e, "source", d, i), o.drawText(t, e, "target", d, i))) : o.drawText(t, e, n, d, i), r && t.translate(y.x1, y.y1);
};
Nr.getFontCache = function(t) {
  var e;
  this.fontCaches = this.fontCaches || [];
  for (var r = 0; r < this.fontCaches.length; r++)
    if (e = this.fontCaches[r], e.context === t)
      return e;
  return e = {
    context: t
  }, this.fontCaches.push(e), e;
};
Nr.setupTextStyle = function(t, e) {
  var r = arguments.length > 2 && arguments[2] !== void 0 ? arguments[2] : !0, a = e.pstyle("font-style").strValue, n = e.pstyle("font-size").pfValue + "px", i = e.pstyle("font-family").strValue, o = e.pstyle("font-weight").strValue, s = r ? e.effectiveOpacity() * e.pstyle("text-opacity").value : 1, u = e.pstyle("text-outline-opacity").value * s, l = e.pstyle("color").value, c = e.pstyle("text-outline-color").value;
  t.font = a + " " + o + " " + n + " " + i, t.lineJoin = "round", this.colorFillStyle(t, l[0], l[1], l[2], s), this.colorStrokeStyle(t, c[0], c[1], c[2], u);
};
function i0(t, e, r, a, n) {
  var i = Math.min(a, n), o = i / 2, s = e + a / 2, u = r + n / 2;
  t.beginPath(), t.arc(s, u, o, 0, Math.PI * 2), t.closePath();
}
function sc(t, e, r, a, n) {
  var i = arguments.length > 5 && arguments[5] !== void 0 ? arguments[5] : 5, o = Math.min(i, a / 2, n / 2);
  t.beginPath(), t.moveTo(e + o, r), t.lineTo(e + a - o, r), t.quadraticCurveTo(e + a, r, e + a, r + o), t.lineTo(e + a, r + n - o), t.quadraticCurveTo(e + a, r + n, e + a - o, r + n), t.lineTo(e + o, r + n), t.quadraticCurveTo(e, r + n, e, r + n - o), t.lineTo(e, r + o), t.quadraticCurveTo(e, r, e + o, r), t.closePath();
}
Nr.getTextAngle = function(t, e) {
  var r, a = t._private, n = a.rscratch, i = e ? e + "-" : "", o = t.pstyle(i + "text-rotation");
  if (o.strValue === "autorotate") {
    var s = bt(n, "labelAngle", e);
    r = t.isEdge() ? s : 0;
  } else o.strValue === "none" ? r = 0 : r = o.pfValue;
  return r;
};
Nr.drawText = function(t, e, r) {
  var a = arguments.length > 3 && arguments[3] !== void 0 ? arguments[3] : !0, n = arguments.length > 4 && arguments[4] !== void 0 ? arguments[4] : !0, i = e._private, o = i.rscratch, s = n ? e.effectiveOpacity() : 1;
  if (!(n && (s === 0 || e.pstyle("text-opacity").value === 0))) {
    r === "main" && (r = null);
    var u = bt(o, "labelX", r), l = bt(o, "labelY", r), c, f, v = this.getLabelText(e, r);
    if (v != null && v !== "" && !isNaN(u) && !isNaN(l)) {
      this.setupTextStyle(t, e, n);
      var h = r ? r + "-" : "", d = bt(o, "labelWidth", r), y = bt(o, "labelHeight", r), g = bt(o, "labelActualDescent", r), p = e.pstyle(h + "text-margin-x").pfValue, m = e.pstyle(h + "text-margin-y").pfValue, b = e.isEdge(), w = e.pstyle("text-halign").value, C = e.pstyle("text-valign").value;
      b && (w = "center", C = "center"), u += p, l += m;
      var E;
      a ? E = this.getTextAngle(e, r) : E = 0, E !== 0 && (c = u, f = l, t.translate(c, f), t.rotate(E), u = 0, l = 0);
      var x = na(w), T = ia(C);
      switch (T) {
        case "top":
          break;
        case "center":
          l += y / 2;
          break;
        case "bottom":
          l += y;
          break;
      }
      var k = e.pstyle("text-background-opacity").value, D = e.pstyle("text-border-opacity").value, P = e.pstyle("text-border-width").pfValue, A = e.pstyle("text-background-padding").pfValue, B = e.pstyle("text-background-shape").strValue, R = B === "round-rectangle" || B === "roundrectangle", I = B === "circle", O = 2;
      if (k > 0 || P > 0 && D > 0) {
        var M = t.fillStyle, N = t.strokeStyle, H = t.lineWidth, G = e.pstyle("text-background-color").value, F = e.pstyle("text-border-color").value, _ = e.pstyle("text-border-style").value, X = k > 0, J = P > 0 && D > 0, Y = u - A;
        switch (x) {
          case "left":
            Y -= d;
            break;
          case "center":
            Y -= d / 2;
            break;
        }
        var j = l - y - A, te = d + 2 * A, ae = y + 2 * A;
        if (X && (t.fillStyle = "rgba(".concat(G[0], ",").concat(G[1], ",").concat(G[2], ",").concat(k * s, ")")), J && (t.strokeStyle = "rgba(".concat(F[0], ",").concat(F[1], ",").concat(F[2], ",").concat(D * s, ")"), t.lineWidth = P, t.setLineDash))
          switch (_) {
            case "dotted":
              t.setLineDash([1, 1]);
              break;
            case "dashed":
              t.setLineDash([4, 2]);
              break;
            case "double":
              t.lineWidth = P / 4, t.setLineDash([]);
              break;
            case "solid":
            default:
              t.setLineDash([]);
              break;
          }
        if (R ? (t.beginPath(), sc(t, Y, j, te, ae, O)) : I ? (t.beginPath(), i0(t, Y, j, te, ae)) : (t.beginPath(), t.rect(Y, j, te, ae)), X && t.fill(), J && t.stroke(), J && _ === "double") {
          var z = P / 2;
          t.beginPath(), R ? sc(t, Y + z, j + z, te - 2 * z, ae - 2 * z, O) : t.rect(Y + z, j + z, te - 2 * z, ae - 2 * z), t.stroke();
        }
        t.fillStyle = M, t.strokeStyle = N, t.lineWidth = H, t.setLineDash && t.setLineDash([]);
      }
      var L = 2 * e.pstyle("text-outline-width").pfValue;
      if (L > 0 && (t.lineWidth = L), l -= g, e.pstyle("text-wrap").value === "wrap") {
        var V = bt(o, "labelWrapCachedLines", r), W = bt(o, "labelLineHeight", r), K = d / 2, re = this.getLabelJustification(e);
        switch (re === "auto" || (x === "left" ? re === "left" ? u += -d : re === "center" && (u += -K) : x === "center" ? re === "left" ? u += -K : re === "right" && (u += K) : x === "right" && (re === "center" ? u += K : re === "right" && (u += d))), T) {
          case "top":
            l -= (V.length - 1) * W;
            break;
          case "center":
          case "bottom":
            l -= (V.length - 1) * W;
            break;
        }
        for (var le = 0; le < V.length; le++)
          L > 0 && t.strokeText(V[le], u, l), t.fillText(V[le], u, l), l += W;
      } else
        L > 0 && t.strokeText(v, u, l), t.fillText(v, u, l);
      E !== 0 && (t.rotate(-E), t.translate(-c, -f));
    }
  }
};
var br = {};
br.drawNode = function(t, e, r) {
  var a = arguments.length > 3 && arguments[3] !== void 0 ? arguments[3] : !0, n = arguments.length > 4 && arguments[4] !== void 0 ? arguments[4] : !0, i = arguments.length > 5 && arguments[5] !== void 0 ? arguments[5] : !0, o = this, s, u, l = e._private, c = l.rscratch, f = e.position();
  if (!(!ne(f.x) || !ne(f.y)) && !(i && !e.visible())) {
    var v = i ? e.effectiveOpacity() : 1, h = o.usePaths(), d, y = !1, g = e.padding();
    s = e.width() + 2 * g, u = e.height() + 2 * g;
    var p;
    r && (p = r, t.translate(-p.x1, -p.y1));
    for (var m = e.pstyle("background-image"), b = m.value, w = new Array(b.length), C = new Array(b.length), E = 0, x = 0; x < b.length; x++) {
      var T = b[x], k = w[x] = T != null && T !== "none";
      if (k) {
        var D = e.cy().style().getIndexedStyle(e, "background-image-crossorigin", "value", x);
        E++, C[x] = o.getCachedImage(T, D, function() {
          l.backgroundTimestamp = Date.now(), e.emitAndNotify("background");
        });
      }
    }
    var P = e.pstyle("background-blacken").value, A = e.pstyle("border-width").pfValue, B = e.pstyle("background-opacity").value * v, R = e.pstyle("border-color").value, I = e.pstyle("border-style").value, O = e.pstyle("border-join").value, M = e.pstyle("border-cap").value, N = e.pstyle("border-position").value, H = e.pstyle("border-dash-pattern").pfValue, G = e.pstyle("border-dash-offset").pfValue, F = e.pstyle("border-opacity").value * v, _ = e.pstyle("outline-width").pfValue, X = e.pstyle("outline-color").value, J = e.pstyle("outline-style").value, Y = e.pstyle("outline-opacity").value * v, j = e.pstyle("outline-offset").value, te = e.pstyle("corner-radius").value;
    te !== "auto" && (te = e.pstyle("corner-radius").pfValue);
    var ae = function() {
      var S = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : B;
      o.eleFillStyle(t, e, S);
    }, z = function() {
      var S = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : F;
      o.colorStrokeStyle(t, R[0], R[1], R[2], S);
    }, L = function() {
      var S = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : Y;
      o.colorStrokeStyle(t, X[0], X[1], X[2], S);
    }, V = function(S, q, $, U) {
      var Z = o.nodePathCache = o.nodePathCache || [], ce = Mc($ === "polygon" ? $ + "," + U.join(",") : $, "" + q, "" + S, "" + te), ee = Z[ce], ie, se = !1;
      return ee != null ? (ie = ee, se = !0, c.pathCache = ie) : (ie = new Path2D(), Z[ce] = c.pathCache = ie), {
        path: ie,
        cacheHit: se
      };
    }, W = e.pstyle("shape").strValue, K = e.pstyle("shape-polygon-points").pfValue;
    if (h) {
      t.translate(f.x, f.y);
      var re = V(s, u, W, K);
      d = re.path, y = re.cacheHit;
    }
    var le = function() {
      if (!y) {
        var S = f;
        h && (S = {
          x: 0,
          y: 0
        }), o.nodeShapes[o.getNodeShape(e)].draw(d || t, S.x, S.y, s, u, te, c);
      }
      h ? t.fill(d) : t.fill();
    }, Pe = function() {
      for (var S = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : v, q = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : !0, $ = l.backgrounding, U = 0, Z = 0; Z < C.length; Z++) {
        var ce = e.cy().style().getIndexedStyle(e, "background-image-containment", "value", Z);
        if (q && ce === "over" || !q && ce === "inside") {
          U++;
          continue;
        }
        w[Z] && C[Z].complete && !C[Z].error && (U++, o.drawInscribedImage(t, C[Z], e, Z, S));
      }
      l.backgrounding = U !== E, $ !== l.backgrounding && e.updateStyle(!1);
    }, we = function() {
      var S = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : !1, q = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : v;
      o.hasPie(e) && (o.drawPie(t, e, q), S && (h || o.nodeShapes[o.getNodeShape(e)].draw(t, f.x, f.y, s, u, te, c)));
    }, me = function() {
      var S = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : !1, q = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : v;
      o.hasStripe(e) && (t.save(), h ? t.clip(c.pathCache) : (o.nodeShapes[o.getNodeShape(e)].draw(t, f.x, f.y, s, u, te, c), t.clip()), o.drawStripe(t, e, q), t.restore(), S && (h || o.nodeShapes[o.getNodeShape(e)].draw(t, f.x, f.y, s, u, te, c)));
    }, ge = function() {
      var S = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : v, q = (P > 0 ? P : -P) * S, $ = P > 0 ? 0 : 255;
      P !== 0 && (o.colorFillStyle(t, $, $, $, q), h ? t.fill(d) : t.fill());
    }, oe = function() {
      if (A > 0) {
        if (t.lineWidth = A, t.lineCap = M, t.lineJoin = O, t.setLineDash)
          switch (I) {
            case "dotted":
              t.setLineDash([1, 1]);
              break;
            case "dashed":
              t.setLineDash(H), t.lineDashOffset = G;
              break;
            case "solid":
            case "double":
              t.setLineDash([]);
              break;
          }
        if (N !== "center") {
          if (t.save(), t.lineWidth *= 2, N === "inside")
            h ? t.clip(d) : t.clip();
          else {
            var S = new Path2D();
            S.rect(-s / 2 - A, -u / 2 - A, s + 2 * A, u + 2 * A), S.addPath(d), t.clip(S, "evenodd");
          }
          h ? t.stroke(d) : t.stroke(), t.restore();
        } else
          h ? t.stroke(d) : t.stroke();
        if (I === "double") {
          t.lineWidth = A / 3;
          var q = t.globalCompositeOperation;
          t.globalCompositeOperation = "destination-out", h ? t.stroke(d) : t.stroke(), t.globalCompositeOperation = q;
        }
        t.setLineDash && t.setLineDash([]);
      }
    }, de = function() {
      if (_ > 0) {
        if (t.lineWidth = _, t.lineCap = "butt", t.setLineDash)
          switch (J) {
            case "dotted":
              t.setLineDash([1, 1]);
              break;
            case "dashed":
              t.setLineDash([4, 2]);
              break;
            case "solid":
            case "double":
              t.setLineDash([]);
              break;
          }
        var S = f;
        h && (S = {
          x: 0,
          y: 0
        });
        var q = o.getNodeShape(e), $ = A;
        N === "inside" && ($ = 0), N === "outside" && ($ *= 2);
        var U = (s + $ + (_ + j)) / s, Z = (u + $ + (_ + j)) / u, ce = s * U, ee = u * Z, ie = o.nodeShapes[q].points, se;
        if (h) {
          var pe = V(ce, ee, q, ie);
          se = pe.path;
        }
        if (q === "ellipse")
          o.drawEllipsePath(se || t, S.x, S.y, ce, ee);
        else if (["round-diamond", "round-heptagon", "round-hexagon", "round-octagon", "round-pentagon", "round-polygon", "round-triangle", "round-tag"].includes(q)) {
          var Ee = 0, Ce = 0, fe = 0;
          q === "round-diamond" ? Ee = ($ + j + _) * 1.4 : q === "round-heptagon" ? (Ee = ($ + j + _) * 1.075, fe = -($ / 2 + j + _) / 35) : q === "round-hexagon" ? Ee = ($ + j + _) * 1.12 : q === "round-pentagon" ? (Ee = ($ + j + _) * 1.13, fe = -($ / 2 + j + _) / 15) : q === "round-tag" ? (Ee = ($ + j + _) * 1.12, Ce = ($ / 2 + _ + j) * 0.07) : q === "round-triangle" && (Ee = ($ + j + _) * (Math.PI / 2), fe = -($ + j / 2 + _) / Math.PI), Ee !== 0 && (U = (s + Ee) / s, ce = s * U, ["round-hexagon", "round-tag"].includes(q) || (Z = (u + Ee) / u, ee = u * Z)), te = te === "auto" ? Gc(ce, ee) : te;
          for (var ke = ce / 2, Ae = ee / 2, nt = te + ($ + _ + j) / 2, Ke = new Array(ie.length / 2), Rt = new Array(ie.length / 2), Xe = 0; Xe < ie.length / 2; Xe++)
            Ke[Xe] = {
              x: S.x + Ce + ke * ie[Xe * 2],
              y: S.y + fe + Ae * ie[Xe * 2 + 1]
            };
          var Ze, je, ut, qt, We = Ke.length;
          for (je = Ke[We - 1], Ze = 0; Ze < We; Ze++)
            ut = Ke[Ze % We], qt = Ke[(Ze + 1) % We], Rt[Ze] = Rs(je, ut, qt, nt), je = ut, ut = qt;
          o.drawRoundPolygonPath(se || t, S.x + Ce, S.y + fe, s * U, u * Z, ie, Rt);
        } else if (["roundrectangle", "round-rectangle"].includes(q))
          te = te === "auto" ? vr(ce, ee) : te, o.drawRoundRectanglePath(se || t, S.x, S.y, ce, ee, te + ($ + _ + j) / 2);
        else if (["cutrectangle", "cut-rectangle"].includes(q))
          te = te === "auto" ? ws() : te, o.drawCutRectanglePath(se || t, S.x, S.y, ce, ee, null, te + ($ + _ + j) / 4);
        else if (["bottomroundrectangle", "bottom-round-rectangle"].includes(q))
          te = te === "auto" ? vr(ce, ee) : te, o.drawBottomRoundRectanglePath(se || t, S.x, S.y, ce, ee, te + ($ + _ + j) / 2);
        else if (q === "barrel")
          o.drawBarrelPath(se || t, S.x, S.y, ce, ee);
        else if (q.startsWith("polygon") || ["rhomboid", "right-rhomboid", "round-tag", "tag", "vee"].includes(q)) {
          var Xt = ($ + _ + j) / s;
          ie = Rn(Mn(ie, Xt)), o.drawPolygonPath(se || t, S.x, S.y, s, u, ie);
        } else {
          var Mt = ($ + _ + j) / s;
          ie = Rn(Mn(ie, -Mt)), o.drawPolygonPath(se || t, S.x, S.y, s, u, ie);
        }
        if (h ? t.stroke(se) : t.stroke(), J === "double") {
          t.lineWidth = $ / 3;
          var tr = t.globalCompositeOperation;
          t.globalCompositeOperation = "destination-out", h ? t.stroke(se) : t.stroke(), t.globalCompositeOperation = tr;
        }
        t.setLineDash && t.setLineDash([]);
      }
    }, ve = function() {
      n && o.drawNodeOverlay(t, e, f, s, u);
    }, xe = function() {
      n && o.drawNodeUnderlay(t, e, f, s, u);
    }, be = function() {
      o.drawElementText(t, e, null, a);
    }, Se = e.pstyle("ghost").value === "yes";
    if (Se) {
      var De = e.pstyle("ghost-offset-x").pfValue, Oe = e.pstyle("ghost-offset-y").pfValue, Ie = e.pstyle("ghost-opacity").value, Be = Ie * v;
      t.translate(De, Oe), L(), de(), ae(Ie * B), le(), Pe(Be, !0), z(Ie * F), oe(), we(P !== 0 || A !== 0), me(P !== 0 || A !== 0), Pe(Be, !1), ge(Be), t.translate(-De, -Oe);
    }
    h && t.translate(-f.x, -f.y), xe(), h && t.translate(f.x, f.y), L(), de(), ae(), le(), Pe(v, !0), z(), oe(), we(P !== 0 || A !== 0), me(P !== 0 || A !== 0), Pe(v, !1), ge(), h && t.translate(-f.x, -f.y), be(), ve(), r && t.translate(p.x1, p.y1);
  }
};
var Zf = function(e) {
  if (!["overlay", "underlay"].includes(e))
    throw new Error("Invalid state");
  return function(r, a, n, i, o) {
    var s = this;
    if (a.visible()) {
      var u = a.pstyle("".concat(e, "-padding")).pfValue, l = a.pstyle("".concat(e, "-opacity")).value, c = a.pstyle("".concat(e, "-color")).value, f = a.pstyle("".concat(e, "-shape")).value, v = a.pstyle("".concat(e, "-corner-radius")).value;
      if (l > 0) {
        if (n = n || a.position(), i == null || o == null) {
          var h = a.padding();
          i = a.width() + 2 * h, o = a.height() + 2 * h;
        }
        s.colorFillStyle(r, c[0], c[1], c[2], l), s.nodeShapes[f].draw(r, n.x, n.y, i + u * 2, o + u * 2, v), r.fill();
      }
    }
  };
};
br.drawNodeOverlay = Zf("overlay");
br.drawNodeUnderlay = Zf("underlay");
br.hasPie = function(t) {
  return t = t[0], t._private.hasPie;
};
br.hasStripe = function(t) {
  return t = t[0], t._private.hasStripe;
};
br.drawPie = function(t, e, r, a) {
  e = e[0], a = a || e.position();
  var n = e.cy().style(), i = e.pstyle("pie-size"), o = e.pstyle("pie-hole"), s = e.pstyle("pie-start-angle").pfValue, u = a.x, l = a.y, c = e.width(), f = e.height(), v = Math.min(c, f) / 2, h, d = 0, y = this.usePaths();
  if (y && (u = 0, l = 0), i.units === "%" ? v = v * i.pfValue : i.pfValue !== void 0 && (v = i.pfValue / 2), o.units === "%" ? h = v * o.pfValue : o.pfValue !== void 0 && (h = o.pfValue / 2), !(h >= v))
    for (var g = 1; g <= n.pieBackgroundN; g++) {
      var p = e.pstyle("pie-" + g + "-background-size").value, m = e.pstyle("pie-" + g + "-background-color").value, b = e.pstyle("pie-" + g + "-background-opacity").value * r, w = p / 100;
      w + d > 1 && (w = 1 - d);
      var C = 1.5 * Math.PI + 2 * Math.PI * d;
      C += s;
      var E = 2 * Math.PI * w, x = C + E;
      p === 0 || d >= 1 || d + w > 1 || (h === 0 ? (t.beginPath(), t.moveTo(u, l), t.arc(u, l, v, C, x), t.closePath()) : (t.beginPath(), t.arc(u, l, v, C, x), t.arc(u, l, h, x, C, !0), t.closePath()), this.colorFillStyle(t, m[0], m[1], m[2], b), t.fill(), d += w);
    }
};
br.drawStripe = function(t, e, r, a) {
  e = e[0], a = a || e.position();
  var n = e.cy().style(), i = a.x, o = a.y, s = e.width(), u = e.height(), l = 0, c = this.usePaths();
  t.save();
  var f = e.pstyle("stripe-direction").value, v = e.pstyle("stripe-size");
  switch (f) {
    case "vertical":
      break;
    // default
    case "righward":
      t.rotate(-Math.PI / 2);
      break;
  }
  var h = s, d = u;
  v.units === "%" ? (h = h * v.pfValue, d = d * v.pfValue) : v.pfValue !== void 0 && (h = v.pfValue, d = v.pfValue), c && (i = 0, o = 0), o -= h / 2, i -= d / 2;
  for (var y = 1; y <= n.stripeBackgroundN; y++) {
    var g = e.pstyle("stripe-" + y + "-background-size").value, p = e.pstyle("stripe-" + y + "-background-color").value, m = e.pstyle("stripe-" + y + "-background-opacity").value * r, b = g / 100;
    b + l > 1 && (b = 1 - l), !(g === 0 || l >= 1 || l + b > 1) && (t.beginPath(), t.rect(i, o + d * l, h, d * b), t.closePath(), this.colorFillStyle(t, p[0], p[1], p[2], m), t.fill(), l += b);
  }
  t.restore();
};
var xt = {}, o0 = 100;
xt.getPixelRatio = function() {
  var t = this.data.contexts[0];
  if (this.forcedPixelRatio != null)
    return this.forcedPixelRatio;
  var e = this.cy.window(), r = t.backingStorePixelRatio || t.webkitBackingStorePixelRatio || t.mozBackingStorePixelRatio || t.msBackingStorePixelRatio || t.oBackingStorePixelRatio || t.backingStorePixelRatio || 1;
  return (e.devicePixelRatio || 1) / r;
};
xt.paintCache = function(t) {
  for (var e = this.paintCaches = this.paintCaches || [], r = !0, a, n = 0; n < e.length; n++)
    if (a = e[n], a.context === t) {
      r = !1;
      break;
    }
  return r && (a = {
    context: t
  }, e.push(a)), a;
};
xt.createGradientStyleFor = function(t, e, r, a, n) {
  var i, o = this.usePaths(), s = r.pstyle(e + "-gradient-stop-colors").value, u = r.pstyle(e + "-gradient-stop-positions").pfValue;
  if (a === "radial-gradient")
    if (r.isEdge()) {
      var l = r.sourceEndpoint(), c = r.targetEndpoint(), f = r.midpoint(), v = Br(l, f), h = Br(c, f);
      i = t.createRadialGradient(f.x, f.y, 0, f.x, f.y, Math.max(v, h));
    } else {
      var d = o ? {
        x: 0,
        y: 0
      } : r.position(), y = r.paddedWidth(), g = r.paddedHeight();
      i = t.createRadialGradient(d.x, d.y, 0, d.x, d.y, Math.max(y, g));
    }
  else if (r.isEdge()) {
    var p = r.sourceEndpoint(), m = r.targetEndpoint();
    i = t.createLinearGradient(p.x, p.y, m.x, m.y);
  } else {
    var b = o ? {
      x: 0,
      y: 0
    } : r.position(), w = r.paddedWidth(), C = r.paddedHeight(), E = w / 2, x = C / 2, T = r.pstyle("background-gradient-direction").value;
    switch (T) {
      case "to-bottom":
        i = t.createLinearGradient(b.x, b.y - x, b.x, b.y + x);
        break;
      case "to-top":
        i = t.createLinearGradient(b.x, b.y + x, b.x, b.y - x);
        break;
      case "to-left":
        i = t.createLinearGradient(b.x + E, b.y, b.x - E, b.y);
        break;
      case "to-right":
        i = t.createLinearGradient(b.x - E, b.y, b.x + E, b.y);
        break;
      case "to-bottom-right":
      case "to-right-bottom":
        i = t.createLinearGradient(b.x - E, b.y - x, b.x + E, b.y + x);
        break;
      case "to-top-right":
      case "to-right-top":
        i = t.createLinearGradient(b.x - E, b.y + x, b.x + E, b.y - x);
        break;
      case "to-bottom-left":
      case "to-left-bottom":
        i = t.createLinearGradient(b.x + E, b.y - x, b.x - E, b.y + x);
        break;
      case "to-top-left":
      case "to-left-top":
        i = t.createLinearGradient(b.x + E, b.y + x, b.x - E, b.y - x);
        break;
    }
  }
  if (!i) return null;
  for (var k = u.length === s.length, D = s.length, P = 0; P < D; P++)
    i.addColorStop(k ? u[P] : P / (D - 1), "rgba(" + s[P][0] + "," + s[P][1] + "," + s[P][2] + "," + n + ")");
  return i;
};
xt.gradientFillStyle = function(t, e, r, a) {
  var n = this.createGradientStyleFor(t, "background", e, r, a);
  if (!n) return null;
  t.fillStyle = n;
};
xt.colorFillStyle = function(t, e, r, a, n) {
  t.fillStyle = "rgba(" + e + "," + r + "," + a + "," + n + ")";
};
xt.eleFillStyle = function(t, e, r) {
  var a = e.pstyle("background-fill").value;
  if (a === "linear-gradient" || a === "radial-gradient")
    this.gradientFillStyle(t, e, a, r);
  else {
    var n = e.pstyle("background-color").value;
    this.colorFillStyle(t, n[0], n[1], n[2], r);
  }
};
xt.gradientStrokeStyle = function(t, e, r, a) {
  var n = this.createGradientStyleFor(t, "line", e, r, a);
  if (!n) return null;
  t.strokeStyle = n;
};
xt.colorStrokeStyle = function(t, e, r, a, n) {
  t.strokeStyle = "rgba(" + e + "," + r + "," + a + "," + n + ")";
};
xt.eleStrokeStyle = function(t, e, r) {
  var a = e.pstyle("line-fill").value;
  if (a === "linear-gradient" || a === "radial-gradient")
    this.gradientStrokeStyle(t, e, a, r);
  else {
    var n = e.pstyle("line-color").value;
    this.colorStrokeStyle(t, n[0], n[1], n[2], r);
  }
};
xt.matchCanvasSize = function(t) {
  var e = this, r = e.data, a = e.findContainerClientCoords(), n = a[2], i = a[3], o = e.getPixelRatio(), s = e.motionBlurPxRatio;
  (t === e.data.bufferCanvases[e.MOTIONBLUR_BUFFER_NODE] || t === e.data.bufferCanvases[e.MOTIONBLUR_BUFFER_DRAG]) && (o = s);
  var u = n * o, l = i * o, c;
  if (!(u === e.canvasWidth && l === e.canvasHeight)) {
    e.fontCaches = null;
    var f = r.canvasContainer;
    f.style.width = n + "px", f.style.height = i + "px";
    for (var v = 0; v < e.CANVAS_LAYERS; v++)
      c = r.canvases[v], c.width = u, c.height = l, c.style.width = n + "px", c.style.height = i + "px";
    for (var v = 0; v < e.BUFFER_COUNT; v++)
      c = r.bufferCanvases[v], c.width = u, c.height = l, c.style.width = n + "px", c.style.height = i + "px";
    e.textureMult = 1, o <= 1 && (c = r.bufferCanvases[e.TEXTURE_BUFFER], e.textureMult = 2, c.width = u * e.textureMult, c.height = l * e.textureMult), e.canvasWidth = u, e.canvasHeight = l, e.pixelRatio = o;
  }
};
xt.renderTo = function(t, e, r, a) {
  this.render({
    forcedContext: t,
    forcedZoom: e,
    forcedPan: r,
    drawAllLayers: !0,
    forcedPxRatio: a
  });
};
xt.clearCanvas = function() {
  var t = this, e = t.data;
  function r(a) {
    a.clearRect(0, 0, t.canvasWidth, t.canvasHeight);
  }
  r(e.contexts[t.NODE]), r(e.contexts[t.DRAG]);
};
xt.render = function(t) {
  var e = this;
  t = t || Nc();
  var r = e.cy, a = t.forcedContext, n = t.drawAllLayers, i = t.drawOnlyNodeLayer, o = t.forcedZoom, s = t.forcedPan, u = t.forcedPxRatio === void 0 ? this.getPixelRatio() : t.forcedPxRatio, l = e.data, c = l.canvasNeedsRedraw, f = e.textureOnViewport && !a && (e.pinching || e.hoverData.dragging || e.swipePanning || e.data.wheelZooming), v = t.motionBlur !== void 0 ? t.motionBlur : e.motionBlur, h = e.motionBlurPxRatio, d = r.hasCompoundNodes(), y = e.hoverData.draggingEles, g = !!(e.hoverData.selecting || e.touchData.selecting);
  v = v && !a && e.motionBlurEnabled && !g;
  var p = v;
  a || (e.prevPxRatio !== u && (e.invalidateContainerClientCoordsCache(), e.matchCanvasSize(e.container), e.redrawHint("eles", !0), e.redrawHint("drag", !0)), e.prevPxRatio = u), !a && e.motionBlurTimeout && clearTimeout(e.motionBlurTimeout), v && (e.mbFrames == null && (e.mbFrames = 0), e.mbFrames++, e.mbFrames < 3 && (p = !1), e.mbFrames > e.minMbLowQualFrames && (e.motionBlurPxRatio = e.mbPxRBlurry)), e.clearingMotionBlur && (e.motionBlurPxRatio = 1), e.textureDrawLastFrame && !f && (c[e.NODE] = !0, c[e.SELECT_BOX] = !0);
  var m = r.style(), b = r.zoom(), w = o !== void 0 ? o : b, C = r.pan(), E = {
    x: C.x,
    y: C.y
  }, x = {
    zoom: b,
    pan: {
      x: C.x,
      y: C.y
    }
  }, T = e.prevViewport, k = T === void 0 || x.zoom !== T.zoom || x.pan.x !== T.pan.x || x.pan.y !== T.pan.y;
  !k && !(y && !d) && (e.motionBlurPxRatio = 1), s && (E = s), w *= u, E.x *= u, E.y *= u;
  var D = e.getCachedZSortedEles();
  function P(z, L, V, W, K) {
    var re = z.globalCompositeOperation;
    z.globalCompositeOperation = "destination-out", e.colorFillStyle(z, 255, 255, 255, e.motionBlurTransparency), z.fillRect(L, V, W, K), z.globalCompositeOperation = re;
  }
  function A(z, L) {
    var V, W, K, re;
    !e.clearingMotionBlur && (z === l.bufferContexts[e.MOTIONBLUR_BUFFER_NODE] || z === l.bufferContexts[e.MOTIONBLUR_BUFFER_DRAG]) ? (V = {
      x: C.x * h,
      y: C.y * h
    }, W = b * h, K = e.canvasWidth * h, re = e.canvasHeight * h) : (V = E, W = w, K = e.canvasWidth, re = e.canvasHeight), z.setTransform(1, 0, 0, 1, 0, 0), L === "motionBlur" ? P(z, 0, 0, K, re) : !a && (L === void 0 || L) && z.clearRect(0, 0, K, re), n || (z.translate(V.x, V.y), z.scale(W, W)), s && z.translate(s.x, s.y), o && z.scale(o, o);
  }
  if (f || (e.textureDrawLastFrame = !1), f) {
    if (e.textureDrawLastFrame = !0, !e.textureCache) {
      e.textureCache = {}, e.textureCache.bb = r.mutableElements().boundingBox(), e.textureCache.texture = e.data.bufferCanvases[e.TEXTURE_BUFFER];
      var B = e.data.bufferContexts[e.TEXTURE_BUFFER];
      B.setTransform(1, 0, 0, 1, 0, 0), B.clearRect(0, 0, e.canvasWidth * e.textureMult, e.canvasHeight * e.textureMult), e.render({
        forcedContext: B,
        drawOnlyNodeLayer: !0,
        forcedPxRatio: u * e.textureMult
      });
      var x = e.textureCache.viewport = {
        zoom: r.zoom(),
        pan: r.pan(),
        width: e.canvasWidth,
        height: e.canvasHeight
      };
      x.mpan = {
        x: (0 - x.pan.x) / x.zoom,
        y: (0 - x.pan.y) / x.zoom
      };
    }
    c[e.DRAG] = !1, c[e.NODE] = !1;
    var R = l.contexts[e.NODE], I = e.textureCache.texture, x = e.textureCache.viewport;
    R.setTransform(1, 0, 0, 1, 0, 0), v ? P(R, 0, 0, x.width, x.height) : R.clearRect(0, 0, x.width, x.height);
    var O = m.core("outside-texture-bg-color").value, M = m.core("outside-texture-bg-opacity").value;
    e.colorFillStyle(R, O[0], O[1], O[2], M), R.fillRect(0, 0, x.width, x.height);
    var b = r.zoom();
    A(R, !1), R.clearRect(x.mpan.x, x.mpan.y, x.width / x.zoom / u, x.height / x.zoom / u), R.drawImage(I, x.mpan.x, x.mpan.y, x.width / x.zoom / u, x.height / x.zoom / u);
  } else e.textureOnViewport && !a && (e.textureCache = null);
  var N = r.extent(), H = e.pinching || e.hoverData.dragging || e.swipePanning || e.data.wheelZooming || e.hoverData.draggingEles || e.cy.animated(), G = e.hideEdgesOnViewport && H, F = [];
  if (F[e.NODE] = !c[e.NODE] && v && !e.clearedForMotionBlur[e.NODE] || e.clearingMotionBlur, F[e.NODE] && (e.clearedForMotionBlur[e.NODE] = !0), F[e.DRAG] = !c[e.DRAG] && v && !e.clearedForMotionBlur[e.DRAG] || e.clearingMotionBlur, F[e.DRAG] && (e.clearedForMotionBlur[e.DRAG] = !0), c[e.NODE] || n || i || F[e.NODE]) {
    var _ = v && !F[e.NODE] && h !== 1, R = a || (_ ? e.data.bufferContexts[e.MOTIONBLUR_BUFFER_NODE] : l.contexts[e.NODE]), X = v && !_ ? "motionBlur" : void 0;
    A(R, X), G ? e.drawCachedNodes(R, D.nondrag, u, N) : e.drawLayeredElements(R, D.nondrag, u, N), e.debug && e.drawDebugPoints(R, D.nondrag), !n && !v && (c[e.NODE] = !1);
  }
  if (!i && (c[e.DRAG] || n || F[e.DRAG])) {
    var _ = v && !F[e.DRAG] && h !== 1, R = a || (_ ? e.data.bufferContexts[e.MOTIONBLUR_BUFFER_DRAG] : l.contexts[e.DRAG]);
    A(R, v && !_ ? "motionBlur" : void 0), G ? e.drawCachedNodes(R, D.drag, u, N) : e.drawCachedElements(R, D.drag, u, N), e.debug && e.drawDebugPoints(R, D.drag), !n && !v && (c[e.DRAG] = !1);
  }
  if (this.drawSelectionRectangle(t, A), v && h !== 1) {
    var J = l.contexts[e.NODE], Y = e.data.bufferCanvases[e.MOTIONBLUR_BUFFER_NODE], j = l.contexts[e.DRAG], te = e.data.bufferCanvases[e.MOTIONBLUR_BUFFER_DRAG], ae = function(L, V, W) {
      L.setTransform(1, 0, 0, 1, 0, 0), W || !p ? L.clearRect(0, 0, e.canvasWidth, e.canvasHeight) : P(L, 0, 0, e.canvasWidth, e.canvasHeight);
      var K = h;
      L.drawImage(
        V,
        // img
        0,
        0,
        // sx, sy
        e.canvasWidth * K,
        e.canvasHeight * K,
        // sw, sh
        0,
        0,
        // x, y
        e.canvasWidth,
        e.canvasHeight
        // w, h
      );
    };
    (c[e.NODE] || F[e.NODE]) && (ae(J, Y, F[e.NODE]), c[e.NODE] = !1), (c[e.DRAG] || F[e.DRAG]) && (ae(j, te, F[e.DRAG]), c[e.DRAG] = !1);
  }
  e.prevViewport = x, e.clearingMotionBlur && (e.clearingMotionBlur = !1, e.motionBlurCleared = !0, e.motionBlur = !0), v && (e.motionBlurTimeout = setTimeout(function() {
    e.motionBlurTimeout = null, e.clearedForMotionBlur[e.NODE] = !1, e.clearedForMotionBlur[e.DRAG] = !1, e.motionBlur = !1, e.clearingMotionBlur = !f, e.mbFrames = 0, c[e.NODE] = !0, c[e.DRAG] = !0, e.redraw();
  }, o0)), a || r.emit("render");
};
var ba;
xt.drawSelectionRectangle = function(t, e) {
  var r = this, a = r.cy, n = r.data, i = a.style(), o = t.drawOnlyNodeLayer, s = t.drawAllLayers, u = n.canvasNeedsRedraw, l = t.forcedContext;
  if (r.showFps || !o && u[r.SELECT_BOX] && !s) {
    var c = l || n.contexts[r.SELECT_BOX];
    if (e(c), r.selection[4] == 1 && (r.hoverData.selecting || r.touchData.selecting)) {
      var f = r.cy.zoom(), v = i.core("selection-box-border-width").value / f;
      c.lineWidth = v, c.fillStyle = "rgba(" + i.core("selection-box-color").value[0] + "," + i.core("selection-box-color").value[1] + "," + i.core("selection-box-color").value[2] + "," + i.core("selection-box-opacity").value + ")", c.fillRect(r.selection[0], r.selection[1], r.selection[2] - r.selection[0], r.selection[3] - r.selection[1]), v > 0 && (c.strokeStyle = "rgba(" + i.core("selection-box-border-color").value[0] + "," + i.core("selection-box-border-color").value[1] + "," + i.core("selection-box-border-color").value[2] + "," + i.core("selection-box-opacity").value + ")", c.strokeRect(r.selection[0], r.selection[1], r.selection[2] - r.selection[0], r.selection[3] - r.selection[1]));
    }
    if (n.bgActivePosistion && !r.hoverData.selecting) {
      var f = r.cy.zoom(), h = n.bgActivePosistion;
      c.fillStyle = "rgba(" + i.core("active-bg-color").value[0] + "," + i.core("active-bg-color").value[1] + "," + i.core("active-bg-color").value[2] + "," + i.core("active-bg-opacity").value + ")", c.beginPath(), c.arc(h.x, h.y, i.core("active-bg-size").pfValue / f, 0, 2 * Math.PI), c.fill();
    }
    var d = r.lastRedrawTime;
    if (r.showFps && d) {
      d = Math.round(d);
      var y = Math.round(1e3 / d), g = "1 frame = " + d + " ms = " + y + " fps";
      if (c.setTransform(1, 0, 0, 1, 0, 0), c.fillStyle = "rgba(255, 0, 0, 0.75)", c.strokeStyle = "rgba(255, 0, 0, 0.75)", c.font = "30px Arial", !ba) {
        var p = c.measureText(g);
        ba = p.actualBoundingBoxAscent;
      }
      c.fillText(g, 0, ba);
      var m = 60;
      c.strokeRect(0, ba + 10, 250, 20), c.fillRect(0, ba + 10, 250 * Math.min(y / m, 1), 20);
    }
    s || (u[r.SELECT_BOX] = !1);
  }
};
function uc(t, e, r) {
  var a = t.createShader(e);
  if (t.shaderSource(a, r), t.compileShader(a), !t.getShaderParameter(a, t.COMPILE_STATUS))
    throw new Error(t.getShaderInfoLog(a));
  return a;
}
function s0(t, e, r) {
  var a = uc(t, t.VERTEX_SHADER, e), n = uc(t, t.FRAGMENT_SHADER, r), i = t.createProgram();
  if (t.attachShader(i, a), t.attachShader(i, n), t.linkProgram(i), !t.getProgramParameter(i, t.LINK_STATUS))
    throw new Error("Could not initialize shaders");
  return i;
}
function u0(t, e, r) {
  r === void 0 && (r = e);
  var a = t.makeOffscreenCanvas(e, r), n = a.context = a.getContext("2d");
  return a.clear = function() {
    return n.clearRect(0, 0, a.width, a.height);
  }, a.clear(), a;
}
function Ls(t) {
  var e = t.pixelRatio, r = t.cy.zoom(), a = t.cy.pan();
  return {
    zoom: r * e,
    pan: {
      x: a.x * e,
      y: a.y * e
    }
  };
}
function l0(t) {
  var e = t.pixelRatio, r = t.cy.zoom();
  return r * e;
}
function c0(t, e, r, a, n) {
  var i = a * r + e.x, o = n * r + e.y;
  return o = Math.round(t.canvasHeight - o), [i, o];
}
function f0(t, e) {
  return e.picking ? !0 : t.pstyle("background-fill").value !== "solid" || t.pstyle("background-image").strValue !== "none" ? !1 : t.pstyle("border-width").value === 0 || t.pstyle("border-opacity").value === 0 ? !0 : t.pstyle("border-style").value === "solid";
}
function v0(t, e) {
  if (t.length !== e.length)
    return !1;
  for (var r = 0; r < t.length; r++)
    if (t[r] !== e[r])
      return !1;
  return !0;
}
function Cr(t, e, r) {
  var a = t[0] / 255, n = t[1] / 255, i = t[2] / 255, o = e, s = r || new Array(4);
  return s[0] = a * o, s[1] = n * o, s[2] = i * o, s[3] = o, s;
}
function Hr(t, e) {
  var r = e || new Array(4);
  return r[0] = (t >> 0 & 255) / 255, r[1] = (t >> 8 & 255) / 255, r[2] = (t >> 16 & 255) / 255, r[3] = (t >> 24 & 255) / 255, r;
}
function d0(t) {
  return t[0] + (t[1] << 8) + (t[2] << 16) + (t[3] << 24);
}
function h0(t, e) {
  var r = t.createTexture();
  return r.buffer = function(a) {
    t.bindTexture(t.TEXTURE_2D, r), t.texParameteri(t.TEXTURE_2D, t.TEXTURE_WRAP_S, t.CLAMP_TO_EDGE), t.texParameteri(t.TEXTURE_2D, t.TEXTURE_WRAP_T, t.CLAMP_TO_EDGE), t.texParameteri(t.TEXTURE_2D, t.TEXTURE_MAG_FILTER, t.LINEAR), t.texParameteri(t.TEXTURE_2D, t.TEXTURE_MIN_FILTER, t.LINEAR_MIPMAP_NEAREST), t.pixelStorei(t.UNPACK_PREMULTIPLY_ALPHA_WEBGL, !0), t.texImage2D(t.TEXTURE_2D, 0, t.RGBA, t.RGBA, t.UNSIGNED_BYTE, a), t.generateMipmap(t.TEXTURE_2D), t.bindTexture(t.TEXTURE_2D, null);
  }, r.deleteTexture = function() {
    t.deleteTexture(r);
  }, r;
}
function Qf(t, e) {
  switch (e) {
    case "float":
      return [1, t.FLOAT, 4];
    case "vec2":
      return [2, t.FLOAT, 4];
    case "vec3":
      return [3, t.FLOAT, 4];
    case "vec4":
      return [4, t.FLOAT, 4];
    case "int":
      return [1, t.INT, 4];
    case "ivec2":
      return [2, t.INT, 4];
  }
}
function Jf(t, e, r) {
  switch (e) {
    case t.FLOAT:
      return new Float32Array(r);
    case t.INT:
      return new Int32Array(r);
  }
}
function g0(t, e, r, a, n, i) {
  switch (e) {
    case t.FLOAT:
      return new Float32Array(r.buffer, i * a, n);
    case t.INT:
      return new Int32Array(r.buffer, i * a, n);
  }
}
function p0(t, e, r, a) {
  var n = Qf(t, e), i = Je(n, 2), o = i[0], s = i[1], u = Jf(t, s, a), l = t.createBuffer();
  return t.bindBuffer(t.ARRAY_BUFFER, l), t.bufferData(t.ARRAY_BUFFER, u, t.STATIC_DRAW), s === t.FLOAT ? t.vertexAttribPointer(r, o, s, !1, 0, 0) : s === t.INT && t.vertexAttribIPointer(r, o, s, 0, 0), t.enableVertexAttribArray(r), t.bindBuffer(t.ARRAY_BUFFER, null), l;
}
function _t(t, e, r, a) {
  var n = Qf(t, r), i = Je(n, 3), o = i[0], s = i[1], u = i[2], l = Jf(t, s, e * o), c = o * u, f = t.createBuffer();
  t.bindBuffer(t.ARRAY_BUFFER, f), t.bufferData(t.ARRAY_BUFFER, e * c, t.DYNAMIC_DRAW), t.enableVertexAttribArray(a), s === t.FLOAT ? t.vertexAttribPointer(a, o, s, !1, c, 0) : s === t.INT && t.vertexAttribIPointer(a, o, s, c, 0), t.vertexAttribDivisor(a, 1), t.bindBuffer(t.ARRAY_BUFFER, null);
  for (var v = new Array(e), h = 0; h < e; h++)
    v[h] = g0(t, s, l, c, o, h);
  return f.dataArray = l, f.stride = c, f.size = o, f.getView = function(d) {
    return v[d];
  }, f.setPoint = function(d, y, g) {
    var p = v[d];
    p[0] = y, p[1] = g;
  }, f.bufferSubData = function(d) {
    t.bindBuffer(t.ARRAY_BUFFER, f), d ? t.bufferSubData(t.ARRAY_BUFFER, 0, l, 0, d * o) : t.bufferSubData(t.ARRAY_BUFFER, 0, l);
  }, f;
}
function y0(t, e, r) {
  for (var a = 9, n = new Float32Array(e * a), i = new Array(e), o = 0; o < e; o++) {
    var s = o * a * 4;
    i[o] = new Float32Array(n.buffer, s, a);
  }
  var u = t.createBuffer();
  t.bindBuffer(t.ARRAY_BUFFER, u), t.bufferData(t.ARRAY_BUFFER, n.byteLength, t.DYNAMIC_DRAW);
  for (var l = 0; l < 3; l++) {
    var c = r + l;
    t.enableVertexAttribArray(c), t.vertexAttribPointer(c, 3, t.FLOAT, !1, 36, l * 12), t.vertexAttribDivisor(c, 1);
  }
  return t.bindBuffer(t.ARRAY_BUFFER, null), u.getMatrixView = function(f) {
    return i[f];
  }, u.setData = function(f, v) {
    i[v].set(f, 0);
  }, u.bufferSubData = function() {
    t.bindBuffer(t.ARRAY_BUFFER, u), t.bufferSubData(t.ARRAY_BUFFER, 0, n);
  }, u;
}
function m0(t) {
  var e = t.createFramebuffer();
  t.bindFramebuffer(t.FRAMEBUFFER, e);
  var r = t.createTexture();
  return t.bindTexture(t.TEXTURE_2D, r), t.texParameteri(t.TEXTURE_2D, t.TEXTURE_MIN_FILTER, t.LINEAR), t.texParameteri(t.TEXTURE_2D, t.TEXTURE_WRAP_S, t.CLAMP_TO_EDGE), t.texParameteri(t.TEXTURE_2D, t.TEXTURE_WRAP_T, t.CLAMP_TO_EDGE), t.framebufferTexture2D(t.FRAMEBUFFER, t.COLOR_ATTACHMENT0, t.TEXTURE_2D, r, 0), t.bindFramebuffer(t.FRAMEBUFFER, null), e.setFramebufferAttachmentSizes = function(a, n) {
    t.bindTexture(t.TEXTURE_2D, r), t.texImage2D(t.TEXTURE_2D, 0, t.RGBA, a, n, 0, t.RGBA, t.UNSIGNED_BYTE, null);
  }, e;
}
var lc = typeof Float32Array < "u" ? Float32Array : Array;
Math.hypot || (Math.hypot = function() {
  for (var t = 0, e = arguments.length; e--; )
    t += arguments[e] * arguments[e];
  return Math.sqrt(t);
});
function Lo() {
  var t = new lc(9);
  return lc != Float32Array && (t[1] = 0, t[2] = 0, t[3] = 0, t[5] = 0, t[6] = 0, t[7] = 0), t[0] = 1, t[4] = 1, t[8] = 1, t;
}
function cc(t) {
  return t[0] = 1, t[1] = 0, t[2] = 0, t[3] = 0, t[4] = 1, t[5] = 0, t[6] = 0, t[7] = 0, t[8] = 1, t;
}
function b0(t, e, r) {
  var a = e[0], n = e[1], i = e[2], o = e[3], s = e[4], u = e[5], l = e[6], c = e[7], f = e[8], v = r[0], h = r[1], d = r[2], y = r[3], g = r[4], p = r[5], m = r[6], b = r[7], w = r[8];
  return t[0] = v * a + h * o + d * l, t[1] = v * n + h * s + d * c, t[2] = v * i + h * u + d * f, t[3] = y * a + g * o + p * l, t[4] = y * n + g * s + p * c, t[5] = y * i + g * u + p * f, t[6] = m * a + b * o + w * l, t[7] = m * n + b * s + w * c, t[8] = m * i + b * u + w * f, t;
}
function Cn(t, e, r) {
  var a = e[0], n = e[1], i = e[2], o = e[3], s = e[4], u = e[5], l = e[6], c = e[7], f = e[8], v = r[0], h = r[1];
  return t[0] = a, t[1] = n, t[2] = i, t[3] = o, t[4] = s, t[5] = u, t[6] = v * a + h * o + l, t[7] = v * n + h * s + c, t[8] = v * i + h * u + f, t;
}
function fc(t, e, r) {
  var a = e[0], n = e[1], i = e[2], o = e[3], s = e[4], u = e[5], l = e[6], c = e[7], f = e[8], v = Math.sin(r), h = Math.cos(r);
  return t[0] = h * a + v * o, t[1] = h * n + v * s, t[2] = h * i + v * u, t[3] = h * o - v * a, t[4] = h * s - v * n, t[5] = h * u - v * i, t[6] = l, t[7] = c, t[8] = f, t;
}
function as(t, e, r) {
  var a = r[0], n = r[1];
  return t[0] = a * e[0], t[1] = a * e[1], t[2] = a * e[2], t[3] = n * e[3], t[4] = n * e[4], t[5] = n * e[5], t[6] = e[6], t[7] = e[7], t[8] = e[8], t;
}
function w0(t, e, r) {
  return t[0] = 2 / e, t[1] = 0, t[2] = 0, t[3] = 0, t[4] = -2 / r, t[5] = 0, t[6] = -1, t[7] = 1, t[8] = 1, t;
}
var x0 = /* @__PURE__ */ (function() {
  function t(e, r, a, n) {
    pr(this, t), this.debugID = Math.floor(Math.random() * 1e4), this.r = e, this.texSize = r, this.texRows = a, this.texHeight = Math.floor(r / a), this.enableWrapping = !0, this.locked = !1, this.texture = null, this.needsBuffer = !0, this.freePointer = {
      x: 0,
      row: 0
    }, this.keyToLocation = /* @__PURE__ */ new Map(), this.canvas = n(e, r, r), this.scratch = n(e, r, this.texHeight, "scratch");
  }
  return yr(t, [{
    key: "lock",
    value: function() {
      this.locked = !0;
    }
  }, {
    key: "getKeys",
    value: function() {
      return new Set(this.keyToLocation.keys());
    }
  }, {
    key: "getScale",
    value: function(r) {
      var a = r.w, n = r.h, i = this.texHeight, o = this.texSize, s = i / n, u = a * s, l = n * s;
      return u > o && (s = o / a, u = a * s, l = n * s), {
        scale: s,
        texW: u,
        texH: l
      };
    }
  }, {
    key: "draw",
    value: function(r, a, n) {
      var i = this;
      if (this.locked) throw new Error("can't draw, atlas is locked");
      var o = this.texSize, s = this.texRows, u = this.texHeight, l = this.getScale(a), c = l.scale, f = l.texW, v = l.texH, h = function(b, w) {
        if (n && w) {
          var C = w.context, E = b.x, x = b.row, T = E, k = u * x;
          C.save(), C.translate(T, k), C.scale(c, c), n(C, a), C.restore();
        }
      }, d = [null, null], y = function() {
        h(i.freePointer, i.canvas), d[0] = {
          x: i.freePointer.x,
          y: i.freePointer.row * u,
          w: f,
          h: v
        }, d[1] = {
          // create a second location with a width of 0, for convenience
          x: i.freePointer.x + f,
          y: i.freePointer.row * u,
          w: 0,
          h: v
        }, i.freePointer.x += f, i.freePointer.x == o && (i.freePointer.x = 0, i.freePointer.row++);
      }, g = function() {
        var b = i.scratch, w = i.canvas;
        b.clear(), h({
          x: 0,
          row: 0
        }, b);
        var C = o - i.freePointer.x, E = f - C, x = u;
        {
          var T = i.freePointer.x, k = i.freePointer.row * u, D = C;
          w.context.drawImage(b, 0, 0, D, x, T, k, D, x), d[0] = {
            x: T,
            y: k,
            w: D,
            h: v
          };
        }
        {
          var P = C, A = (i.freePointer.row + 1) * u, B = E;
          w && w.context.drawImage(b, P, 0, B, x, 0, A, B, x), d[1] = {
            x: 0,
            y: A,
            w: B,
            h: v
          };
        }
        i.freePointer.x = E, i.freePointer.row++;
      }, p = function() {
        i.freePointer.x = 0, i.freePointer.row++;
      };
      if (this.freePointer.x + f <= o)
        y();
      else {
        if (this.freePointer.row >= s - 1)
          return !1;
        this.freePointer.x === o ? (p(), y()) : this.enableWrapping ? g() : (p(), y());
      }
      return this.keyToLocation.set(r, d), this.needsBuffer = !0, d;
    }
  }, {
    key: "getOffsets",
    value: function(r) {
      return this.keyToLocation.get(r);
    }
  }, {
    key: "isEmpty",
    value: function() {
      return this.freePointer.x === 0 && this.freePointer.row === 0;
    }
  }, {
    key: "canFit",
    value: function(r) {
      if (this.locked) return !1;
      var a = this.texSize, n = this.texRows, i = this.getScale(r), o = i.texW;
      return this.freePointer.x + o > a ? this.freePointer.row < n - 1 : !0;
    }
    // called on every frame
  }, {
    key: "bufferIfNeeded",
    value: function(r) {
      this.texture || (this.texture = h0(r, this.debugID)), this.needsBuffer && (this.texture.buffer(this.canvas), this.needsBuffer = !1, this.locked && (this.canvas = null, this.scratch = null));
    }
  }, {
    key: "dispose",
    value: function() {
      this.texture && (this.texture.deleteTexture(), this.texture = null), this.canvas = null, this.scratch = null, this.locked = !0;
    }
  }]);
})(), E0 = /* @__PURE__ */ (function() {
  function t(e, r, a, n) {
    pr(this, t), this.r = e, this.texSize = r, this.texRows = a, this.createTextureCanvas = n, this.atlases = [], this.styleKeyToAtlas = /* @__PURE__ */ new Map(), this.markedKeys = /* @__PURE__ */ new Set();
  }
  return yr(t, [{
    key: "getKeys",
    value: function() {
      return new Set(this.styleKeyToAtlas.keys());
    }
  }, {
    key: "_createAtlas",
    value: function() {
      var r = this.r, a = this.texSize, n = this.texRows, i = this.createTextureCanvas;
      return new x0(r, a, n, i);
    }
  }, {
    key: "_getScratchCanvas",
    value: function() {
      if (!this.scratch) {
        var r = this.r, a = this.texSize, n = this.texRows, i = this.createTextureCanvas, o = Math.floor(a / n);
        this.scratch = i(r, a, o, "scratch");
      }
      return this.scratch;
    }
  }, {
    key: "draw",
    value: function(r, a, n) {
      var i = this.styleKeyToAtlas.get(r);
      return i || (i = this.atlases[this.atlases.length - 1], (!i || !i.canFit(a)) && (i && i.lock(), i = this._createAtlas(), this.atlases.push(i)), i.draw(r, a, n), this.styleKeyToAtlas.set(r, i)), i;
    }
  }, {
    key: "getAtlas",
    value: function(r) {
      return this.styleKeyToAtlas.get(r);
    }
  }, {
    key: "hasAtlas",
    value: function(r) {
      return this.styleKeyToAtlas.has(r);
    }
  }, {
    key: "markKeyForGC",
    value: function(r) {
      this.markedKeys.add(r);
    }
  }, {
    key: "gc",
    value: function() {
      var r = this, a = this.markedKeys;
      if (a.size === 0) {
        console.log("nothing to garbage collect");
        return;
      }
      var n = [], i = /* @__PURE__ */ new Map(), o = null, s = St(this.atlases), u;
      try {
        var l = function() {
          var f = u.value, v = f.getKeys(), h = C0(a, v);
          if (h.size === 0)
            return n.push(f), v.forEach(function(C) {
              return i.set(C, f);
            }), 1;
          o || (o = r._createAtlas(), n.push(o));
          var d = St(v), y;
          try {
            for (d.s(); !(y = d.n()).done; ) {
              var g = y.value;
              if (!h.has(g)) {
                var p = f.getOffsets(g), m = Je(p, 2), b = m[0], w = m[1];
                o.canFit({
                  w: b.w + w.w,
                  h: b.h
                }) || (o.lock(), o = r._createAtlas(), n.push(o)), f.canvas && (r._copyTextureToNewAtlas(g, f, o), i.set(g, o));
              }
            }
          } catch (C) {
            d.e(C);
          } finally {
            d.f();
          }
          f.dispose();
        };
        for (s.s(); !(u = s.n()).done; )
          l();
      } catch (c) {
        s.e(c);
      } finally {
        s.f();
      }
      this.atlases = n, this.styleKeyToAtlas = i, this.markedKeys = /* @__PURE__ */ new Set();
    }
  }, {
    key: "_copyTextureToNewAtlas",
    value: function(r, a, n) {
      var i = a.getOffsets(r), o = Je(i, 2), s = o[0], u = o[1];
      if (u.w === 0)
        n.draw(r, s, function(v) {
          v.drawImage(a.canvas, s.x, s.y, s.w, s.h, 0, 0, s.w, s.h);
        });
      else {
        var l = this._getScratchCanvas();
        l.clear(), l.context.drawImage(a.canvas, s.x, s.y, s.w, s.h, 0, 0, s.w, s.h), l.context.drawImage(a.canvas, u.x, u.y, u.w, u.h, s.w, 0, u.w, u.h);
        var c = s.w + u.w, f = s.h;
        n.draw(r, {
          w: c,
          h: f
        }, function(v) {
          v.drawImage(
            l,
            0,
            0,
            c,
            f,
            0,
            0,
            c,
            f
            // the destination context has already been translated to the correct position
          );
        });
      }
    }
  }, {
    key: "getCounts",
    value: function() {
      return {
        keyCount: this.styleKeyToAtlas.size,
        atlasCount: new Set(this.styleKeyToAtlas.values()).size
      };
    }
  }]);
})();
function C0(t, e) {
  return t.intersection ? t.intersection(e) : new Set(Pn(t).filter(function(r) {
    return e.has(r);
  }));
}
var T0 = /* @__PURE__ */ (function() {
  function t(e, r) {
    pr(this, t), this.r = e, this.globalOptions = r, this.atlasSize = r.webglTexSize, this.maxAtlasesPerBatch = r.webglTexPerBatch, this.renderTypes = /* @__PURE__ */ new Map(), this.collections = /* @__PURE__ */ new Map(), this.typeAndIdToKey = /* @__PURE__ */ new Map();
  }
  return yr(t, [{
    key: "getAtlasSize",
    value: function() {
      return this.atlasSize;
    }
  }, {
    key: "addAtlasCollection",
    value: function(r, a) {
      var n = this.globalOptions, i = n.webglTexSize, o = n.createTextureCanvas, s = a.texRows, u = this._cacheScratchCanvas(o), l = new E0(this.r, i, s, u);
      this.collections.set(r, l);
    }
  }, {
    key: "addRenderType",
    value: function(r, a) {
      var n = a.collection;
      if (!this.collections.has(n)) throw new Error("invalid atlas collection name '".concat(n, "'"));
      var i = this.collections.get(n), o = ye({
        type: r,
        atlasCollection: i
      }, a);
      this.renderTypes.set(r, o);
    }
  }, {
    key: "getRenderTypeOpts",
    value: function(r) {
      return this.renderTypes.get(r);
    }
  }, {
    key: "getAtlasCollection",
    value: function(r) {
      return this.collections.get(r);
    }
  }, {
    key: "_cacheScratchCanvas",
    value: function(r) {
      var a = -1, n = -1, i = null;
      return function(o, s, u, l) {
        return l ? ((!i || s != a || u != n) && (a = s, n = u, i = r(o, s, u)), i) : r(o, s, u);
      };
    }
  }, {
    key: "_key",
    value: function(r, a) {
      return "".concat(r, "-").concat(a);
    }
    /** Marks textues associated with the element for garbage collection. */
  }, {
    key: "invalidate",
    value: function(r) {
      var a = this, n = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : {}, i = n.forceRedraw, o = i === void 0 ? !1 : i, s = n.filterEle, u = s === void 0 ? function() {
        return !0;
      } : s, l = n.filterType, c = l === void 0 ? function() {
        return !0;
      } : l, f = !1, v = !1, h = St(r), d;
      try {
        for (h.s(); !(d = h.n()).done; ) {
          var y = d.value;
          if (u(y)) {
            var g = St(this.renderTypes.values()), p;
            try {
              var m = function() {
                var w = p.value, C = w.type;
                if (c(C)) {
                  var E = a.collections.get(w.collection), x = w.getKey(y), T = Array.isArray(x) ? x : [x];
                  if (o)
                    T.forEach(function(A) {
                      return E.markKeyForGC(A);
                    }), v = !0;
                  else {
                    var k = w.getID ? w.getID(y) : y.id(), D = a._key(C, k), P = a.typeAndIdToKey.get(D);
                    P !== void 0 && !v0(T, P) && (f = !0, a.typeAndIdToKey.delete(D), P.forEach(function(A) {
                      return E.markKeyForGC(A);
                    }));
                  }
                }
              };
              for (g.s(); !(p = g.n()).done; )
                m();
            } catch (b) {
              g.e(b);
            } finally {
              g.f();
            }
          }
        }
      } catch (b) {
        h.e(b);
      } finally {
        h.f();
      }
      return v && (this.gc(), f = !1), f;
    }
    /** Garbage collect */
  }, {
    key: "gc",
    value: function() {
      var r = St(this.collections.values()), a;
      try {
        for (r.s(); !(a = r.n()).done; ) {
          var n = a.value;
          n.gc();
        }
      } catch (i) {
        r.e(i);
      } finally {
        r.f();
      }
    }
  }, {
    key: "getOrCreateAtlas",
    value: function(r, a, n, i) {
      var o = this.renderTypes.get(a), s = this.collections.get(o.collection), u = !1, l = s.draw(i, n, function(v) {
        o.drawClipped ? (v.save(), v.beginPath(), v.rect(0, 0, n.w, n.h), v.clip(), o.drawElement(v, r, n, !0, !0), v.restore()) : o.drawElement(v, r, n, !0, !0), u = !0;
      });
      if (u) {
        var c = o.getID ? o.getID(r) : r.id(), f = this._key(a, c);
        this.typeAndIdToKey.has(f) ? this.typeAndIdToKey.get(f).push(i) : this.typeAndIdToKey.set(f, [i]);
      }
      return l;
    }
  }, {
    key: "getAtlasInfo",
    value: function(r, a) {
      var n = this, i = this.renderTypes.get(a), o = i.getKey(r), s = Array.isArray(o) ? o : [o];
      return s.map(function(u) {
        var l = i.getBoundingBox(r, u), c = n.getOrCreateAtlas(r, a, l, u), f = c.getOffsets(u), v = Je(f, 2), h = v[0], d = v[1];
        return {
          atlas: c,
          tex: h,
          tex1: h,
          tex2: d,
          bb: l
        };
      });
    }
  }, {
    key: "getDebugInfo",
    value: function() {
      var r = [], a = St(this.collections), n;
      try {
        for (a.s(); !(n = a.n()).done; ) {
          var i = Je(n.value, 2), o = i[0], s = i[1], u = s.getCounts(), l = u.keyCount, c = u.atlasCount;
          r.push({
            type: o,
            keyCount: l,
            atlasCount: c
          });
        }
      } catch (f) {
        a.e(f);
      } finally {
        a.f();
      }
      return r;
    }
  }]);
})(), S0 = /* @__PURE__ */ (function() {
  function t(e) {
    pr(this, t), this.globalOptions = e, this.atlasSize = e.webglTexSize, this.maxAtlasesPerBatch = e.webglTexPerBatch, this.batchAtlases = [];
  }
  return yr(t, [{
    key: "getMaxAtlasesPerBatch",
    value: function() {
      return this.maxAtlasesPerBatch;
    }
  }, {
    key: "getAtlasSize",
    value: function() {
      return this.atlasSize;
    }
  }, {
    key: "getIndexArray",
    value: function() {
      return Array.from({
        length: this.maxAtlasesPerBatch
      }, function(r, a) {
        return a;
      });
    }
  }, {
    key: "startBatch",
    value: function() {
      this.batchAtlases = [];
    }
  }, {
    key: "getAtlasCount",
    value: function() {
      return this.batchAtlases.length;
    }
  }, {
    key: "getAtlases",
    value: function() {
      return this.batchAtlases;
    }
  }, {
    key: "canAddToCurrentBatch",
    value: function(r) {
      return this.batchAtlases.length === this.maxAtlasesPerBatch ? this.batchAtlases.includes(r) : !0;
    }
  }, {
    key: "getAtlasIndexForBatch",
    value: function(r) {
      var a = this.batchAtlases.indexOf(r);
      if (a < 0) {
        if (this.batchAtlases.length === this.maxAtlasesPerBatch)
          throw new Error("cannot add more atlases to batch");
        this.batchAtlases.push(r), a = this.batchAtlases.length - 1;
      }
      return a;
    }
  }]);
})(), k0 = `
  float circleSD(vec2 p, float r) {
    return distance(vec2(0), p) - r; // signed distance
  }
`, P0 = `
  float rectangleSD(vec2 p, vec2 b) {
    vec2 d = abs(p)-b;
    return distance(vec2(0),max(d,0.0)) + min(max(d.x,d.y),0.0);
  }
`, D0 = `
  float roundRectangleSD(vec2 p, vec2 b, vec4 cr) {
    cr.xy = (p.x > 0.0) ? cr.xy : cr.zw;
    cr.x  = (p.y > 0.0) ? cr.x  : cr.y;
    vec2 q = abs(p) - b + cr.x;
    return min(max(q.x, q.y), 0.0) + distance(vec2(0), max(q, 0.0)) - cr.x;
  }
`, A0 = `
  float ellipseSD(vec2 p, vec2 ab) {
    p = abs( p ); // symmetry

    // find root with Newton solver
    vec2 q = ab*(p-ab);
    float w = (q.x<q.y)? 1.570796327 : 0.0;
    for( int i=0; i<5; i++ ) {
      vec2 cs = vec2(cos(w),sin(w));
      vec2 u = ab*vec2( cs.x,cs.y);
      vec2 v = ab*vec2(-cs.y,cs.x);
      w = w + dot(p-u,v)/(dot(p-u,u)+dot(v,v));
    }
    
    // compute final point and distance
    float d = length(p-ab*vec2(cos(w),sin(w)));
    
    // return signed distance
    return (dot(p/ab,p/ab)>1.0) ? d : -d;
  }
`, Pa = {
  SCREEN: {
    name: "screen",
    screen: !0
  },
  PICKING: {
    name: "picking",
    picking: !0
  }
}, qn = {
  // render the texture just like in RENDER_TARGET.SCREEN mode
  IGNORE: 1,
  // don't render the texture at all
  USE_BB: 2
  // render the bounding box as an opaque rectangle
}, Oo = 0, vc = 1, dc = 2, No = 3, $r = 4, vn = 5, wa = 6, xa = 7, B0 = /* @__PURE__ */ (function() {
  function t(e, r, a) {
    pr(this, t), this.r = e, this.gl = r, this.maxInstances = a.webglBatchSize, this.atlasSize = a.webglTexSize, this.bgColor = a.bgColor, this.debug = a.webglDebug, this.batchDebugInfo = [], a.enableWrapping = !0, a.createTextureCanvas = u0, this.atlasManager = new T0(e, a), this.batchManager = new S0(a), this.simpleShapeOptions = /* @__PURE__ */ new Map(), this.program = this._createShaderProgram(Pa.SCREEN), this.pickingProgram = this._createShaderProgram(Pa.PICKING), this.vao = this._createVAO();
  }
  return yr(t, [{
    key: "addAtlasCollection",
    value: function(r, a) {
      this.atlasManager.addAtlasCollection(r, a);
    }
    /**
     * @typedef { Object } TextureRenderTypeOpts
     * @property { string } collection - name of atlas collection to render textures to
     * @property { function } getKey - returns the "style key" for an element, may be a single value or an array for multi-line lables
     * @property { function } drawElement - uses a canvas renderer to draw the element to the texture atlas
     * @property { boolean  } drawClipped - if true the context will be clipped to the bounding box before drawElement() is called, may affect performance
     * @property { function } getBoundingBox - returns the bounding box for an element
     * @property { function } getRotation
     * @property { function } getRotationPoint
     * @property { function } getRotationOffset
     * @property { function } isVisible - an extra check for visibility in addition to ele.visible()
     * @property { function } getTexPickingMode - returns a value from the TEX_PICKING_MODE enum
     */
    /**
     * @param { string } typeName
     * @param { TextureRenderTypeOpts } opts
     */
  }, {
    key: "addTextureAtlasRenderType",
    value: function(r, a) {
      this.atlasManager.addRenderType(r, a);
    }
    /**
     * @typedef { Object } SimpleShapeRenderTypeOpts
     * @property { function } getBoundingBox - returns the bounding box for an element
     * @property { function } isVisible - this is an extra check for visibility in addition to ele.visible()
     * @property { function } isSimple - check if element is a simple shape, or if it needs to fall back to texture rendering
     * @property { ShapeVisualProperties } shapeProps
     */
    /**
     * @typedef { Object } ShapeVisualProperties
     * @property { string } shape
     * @property { string } color
     * @property { string } opacity
     * @property { string } padding
     * @property { string } radius
     * @property { boolean } border
    */
    /**
     * @param { string } typeName
     * @param { SimpleShapeRenderTypeOpts } opts
     */
  }, {
    key: "addSimpleShapeRenderType",
    value: function(r, a) {
      this.simpleShapeOptions.set(r, a);
    }
    /**
     * Inform the atlasManager when element style keys may have changed.
     * The atlasManager can then mark unused textures for "garbage collection".
     */
  }, {
    key: "invalidate",
    value: function(r) {
      var a = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : {}, n = a.type, i = this.atlasManager;
      return n ? i.invalidate(r, {
        filterType: function(s) {
          return s === n;
        },
        forceRedraw: !0
      }) : i.invalidate(r);
    }
    /**
     * Run texture garbage collection.
     */
  }, {
    key: "gc",
    value: function() {
      this.atlasManager.gc();
    }
  }, {
    key: "_createShaderProgram",
    value: function(r) {
      var a = this.gl, n = `#version 300 es
      precision highp float;

      uniform mat3 uPanZoomMatrix;
      uniform int  uAtlasSize;
      
      // instanced
      in vec2 aPosition; // a vertex from the unit square
      
      in mat3 aTransform; // used to transform verticies, eg into a bounding box
      in int aVertType; // the type of thing we are rendering

      // the z-index that is output when using picking mode
      in vec4 aIndex;
      
      // For textures
      in int aAtlasId; // which shader unit/atlas to use
      in vec4 aTex; // x/y/w/h of texture in atlas

      // for edges
      in vec4 aPointAPointB;
      in vec4 aPointCPointD;
      in vec2 aLineWidth; // also used for node border width

      // simple shapes
      in vec4 aCornerRadius; // for round-rectangle [top-right, bottom-right, top-left, bottom-left]
      in vec4 aColor; // also used for edges
      in vec4 aBorderColor; // aLineWidth is used for border width

      // output values passed to the fragment shader
      out vec2 vTexCoord;
      out vec4 vColor;
      out vec2 vPosition;
      // flat values are not interpolated
      flat out int vAtlasId; 
      flat out int vVertType;
      flat out vec2 vTopRight;
      flat out vec2 vBotLeft;
      flat out vec4 vCornerRadius;
      flat out vec4 vBorderColor;
      flat out vec2 vBorderWidth;
      flat out vec4 vIndex;
      
      void main(void) {
        int vid = gl_VertexID;
        vec2 position = aPosition; // TODO make this a vec3, simplifies some code below

        if(aVertType == `.concat(Oo, `) {
          float texX = aTex.x; // texture coordinates
          float texY = aTex.y;
          float texW = aTex.z;
          float texH = aTex.w;

          if(vid == 1 || vid == 2 || vid == 4) {
            texX += texW;
          }
          if(vid == 2 || vid == 4 || vid == 5) {
            texY += texH;
          }

          float d = float(uAtlasSize);
          vTexCoord = vec2(texX / d, texY / d); // tex coords must be between 0 and 1

          gl_Position = vec4(uPanZoomMatrix * aTransform * vec3(position, 1.0), 1.0);
        }
        else if(aVertType == `).concat($r, " || aVertType == ").concat(xa, ` 
             || aVertType == `).concat(vn, " || aVertType == ").concat(wa, `) { // simple shapes

          // the bounding box is needed by the fragment shader
          vBotLeft  = (aTransform * vec3(0, 0, 1)).xy; // flat
          vTopRight = (aTransform * vec3(1, 1, 1)).xy; // flat
          vPosition = (aTransform * vec3(position, 1)).xy; // will be interpolated

          // calculations are done in the fragment shader, just pass these along
          vColor = aColor;
          vCornerRadius = aCornerRadius;
          vBorderColor = aBorderColor;
          vBorderWidth = aLineWidth;

          gl_Position = vec4(uPanZoomMatrix * aTransform * vec3(position, 1.0), 1.0);
        }
        else if(aVertType == `).concat(vc, `) {
          vec2 source = aPointAPointB.xy;
          vec2 target = aPointAPointB.zw;

          // adjust the geometry so that the line is centered on the edge
          position.y = position.y - 0.5;

          // stretch the unit square into a long skinny rectangle
          vec2 xBasis = target - source;
          vec2 yBasis = normalize(vec2(-xBasis.y, xBasis.x));
          vec2 point = source + xBasis * position.x + yBasis * aLineWidth[0] * position.y;

          gl_Position = vec4(uPanZoomMatrix * vec3(point, 1.0), 1.0);
          vColor = aColor;
        } 
        else if(aVertType == `).concat(dc, `) {
          vec2 pointA = aPointAPointB.xy;
          vec2 pointB = aPointAPointB.zw;
          vec2 pointC = aPointCPointD.xy;
          vec2 pointD = aPointCPointD.zw;

          // adjust the geometry so that the line is centered on the edge
          position.y = position.y - 0.5;

          vec2 p0, p1, p2, pos;
          if(position.x == 0.0) { // The left side of the unit square
            p0 = pointA;
            p1 = pointB;
            p2 = pointC;
            pos = position;
          } else { // The right side of the unit square, use same approach but flip the geometry upside down
            p0 = pointD;
            p1 = pointC;
            p2 = pointB;
            pos = vec2(0.0, -position.y);
          }

          vec2 p01 = p1 - p0;
          vec2 p12 = p2 - p1;
          vec2 p21 = p1 - p2;

          // Find the normal vector.
          vec2 tangent = normalize(normalize(p12) + normalize(p01));
          vec2 normal = vec2(-tangent.y, tangent.x);

          // Find the vector perpendicular to p0 -> p1.
          vec2 p01Norm = normalize(vec2(-p01.y, p01.x));

          // Determine the bend direction.
          float sigma = sign(dot(p01 + p21, normal));
          float width = aLineWidth[0];

          if(sign(pos.y) == -sigma) {
            // This is an intersecting vertex. Adjust the position so that there's no overlap.
            vec2 point = 0.5 * width * normal * -sigma / dot(normal, p01Norm);
            gl_Position = vec4(uPanZoomMatrix * vec3(p1 + point, 1.0), 1.0);
          } else {
            // This is a non-intersecting vertex. Treat it like a mitre join.
            vec2 point = 0.5 * width * normal * sigma * dot(normal, p01Norm);
            gl_Position = vec4(uPanZoomMatrix * vec3(p1 + point, 1.0), 1.0);
          }

          vColor = aColor;
        } 
        else if(aVertType == `).concat(No, ` && vid < 3) {
          // massage the first triangle into an edge arrow
          if(vid == 0)
            position = vec2(-0.15, -0.3);
          if(vid == 1)
            position = vec2(  0.0,  0.0);
          if(vid == 2)
            position = vec2( 0.15, -0.3);

          gl_Position = vec4(uPanZoomMatrix * aTransform * vec3(position, 1.0), 1.0);
          vColor = aColor;
        }
        else {
          gl_Position = vec4(2.0, 0.0, 0.0, 1.0); // discard vertex by putting it outside webgl clip space
        }

        vAtlasId = aAtlasId;
        vVertType = aVertType;
        vIndex = aIndex;
      }
    `), i = this.batchManager.getIndexArray(), o = `#version 300 es
      precision highp float;

      // declare texture unit for each texture atlas in the batch
      `.concat(i.map(function(l) {
        return "uniform sampler2D uTexture".concat(l, ";");
      }).join(`
	`), `

      uniform vec4 uBGColor;
      uniform float uZoom;

      in vec2 vTexCoord;
      in vec4 vColor;
      in vec2 vPosition; // model coordinates

      flat in int vAtlasId;
      flat in vec4 vIndex;
      flat in int vVertType;
      flat in vec2 vTopRight;
      flat in vec2 vBotLeft;
      flat in vec4 vCornerRadius;
      flat in vec4 vBorderColor;
      flat in vec2 vBorderWidth;

      out vec4 outColor;

      `).concat(k0, `
      `).concat(P0, `
      `).concat(D0, `
      `).concat(A0, `

      vec4 blend(vec4 top, vec4 bot) { // blend colors with premultiplied alpha
        return vec4( 
          top.rgb + (bot.rgb * (1.0 - top.a)),
          top.a   + (bot.a   * (1.0 - top.a)) 
        );
      }

      vec4 distInterp(vec4 cA, vec4 cB, float d) { // interpolate color using Signed Distance
        // scale to the zoom level so that borders don't look blurry when zoomed in
        // note 1.5 is an aribitrary value chosen because it looks good
        return mix(cA, cB, 1.0 - smoothstep(0.0, 1.5 / uZoom, abs(d))); 
      }

      void main(void) {
        if(vVertType == `).concat(Oo, `) {
          // look up the texel from the texture unit
          `).concat(i.map(function(l) {
        return "if(vAtlasId == ".concat(l, ") outColor = texture(uTexture").concat(l, ", vTexCoord);");
      }).join(`
	else `), `
        } 
        else if(vVertType == `).concat(No, `) {
          // mimics how canvas renderer uses context.globalCompositeOperation = 'destination-out';
          outColor = blend(vColor, uBGColor);
          outColor.a = 1.0; // make opaque, masks out line under arrow
        }
        else if(vVertType == `).concat($r, ` && vBorderWidth == vec2(0.0)) { // simple rectangle with no border
          outColor = vColor; // unit square is already transformed to the rectangle, nothing else needs to be done
        }
        else if(vVertType == `).concat($r, " || vVertType == ").concat(xa, ` 
          || vVertType == `).concat(vn, " || vVertType == ").concat(wa, `) { // use SDF

          float outerBorder = vBorderWidth[0];
          float innerBorder = vBorderWidth[1];
          float borderPadding = outerBorder * 2.0;
          float w = vTopRight.x - vBotLeft.x - borderPadding;
          float h = vTopRight.y - vBotLeft.y - borderPadding;
          vec2 b = vec2(w/2.0, h/2.0); // half width, half height
          vec2 p = vPosition - vec2(vTopRight.x - b[0] - outerBorder, vTopRight.y - b[1] - outerBorder); // translate to center

          float d; // signed distance
          if(vVertType == `).concat($r, `) {
            d = rectangleSD(p, b);
          } else if(vVertType == `).concat(xa, ` && w == h) {
            d = circleSD(p, b.x); // faster than ellipse
          } else if(vVertType == `).concat(xa, `) {
            d = ellipseSD(p, b);
          } else {
            d = roundRectangleSD(p, b, vCornerRadius.wzyx);
          }

          // use the distance to interpolate a color to smooth the edges of the shape, doesn't need multisampling
          // we must smooth colors inwards, because we can't change pixels outside the shape's bounding box
          if(d > 0.0) {
            if(d > outerBorder) {
              discard;
            } else {
              outColor = distInterp(vBorderColor, vec4(0), d - outerBorder);
            }
          } else {
            if(d > innerBorder) {
              vec4 outerColor = outerBorder == 0.0 ? vec4(0) : vBorderColor;
              vec4 innerBorderColor = blend(vBorderColor, vColor);
              outColor = distInterp(innerBorderColor, outerColor, d);
            } 
            else {
              vec4 outerColor;
              if(innerBorder == 0.0 && outerBorder == 0.0) {
                outerColor = vec4(0);
              } else if(innerBorder == 0.0) {
                outerColor = vBorderColor;
              } else {
                outerColor = blend(vBorderColor, vColor);
              }
              outColor = distInterp(vColor, outerColor, d - innerBorder);
            }
          }
        }
        else {
          outColor = vColor;
        }

        `).concat(r.picking ? `if(outColor.a == 0.0) discard;
             else outColor = vIndex;` : "", `
      }
    `), s = s0(a, n, o);
      s.aPosition = a.getAttribLocation(s, "aPosition"), s.aIndex = a.getAttribLocation(s, "aIndex"), s.aVertType = a.getAttribLocation(s, "aVertType"), s.aTransform = a.getAttribLocation(s, "aTransform"), s.aAtlasId = a.getAttribLocation(s, "aAtlasId"), s.aTex = a.getAttribLocation(s, "aTex"), s.aPointAPointB = a.getAttribLocation(s, "aPointAPointB"), s.aPointCPointD = a.getAttribLocation(s, "aPointCPointD"), s.aLineWidth = a.getAttribLocation(s, "aLineWidth"), s.aColor = a.getAttribLocation(s, "aColor"), s.aCornerRadius = a.getAttribLocation(s, "aCornerRadius"), s.aBorderColor = a.getAttribLocation(s, "aBorderColor"), s.uPanZoomMatrix = a.getUniformLocation(s, "uPanZoomMatrix"), s.uAtlasSize = a.getUniformLocation(s, "uAtlasSize"), s.uBGColor = a.getUniformLocation(s, "uBGColor"), s.uZoom = a.getUniformLocation(s, "uZoom"), s.uTextures = [];
      for (var u = 0; u < this.batchManager.getMaxAtlasesPerBatch(); u++)
        s.uTextures.push(a.getUniformLocation(s, "uTexture".concat(u)));
      return s;
    }
  }, {
    key: "_createVAO",
    value: function() {
      var r = [0, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1];
      this.vertexCount = r.length / 2;
      var a = this.maxInstances, n = this.gl, i = this.program, o = n.createVertexArray();
      return n.bindVertexArray(o), p0(n, "vec2", i.aPosition, r), this.transformBuffer = y0(n, a, i.aTransform), this.indexBuffer = _t(n, a, "vec4", i.aIndex), this.vertTypeBuffer = _t(n, a, "int", i.aVertType), this.atlasIdBuffer = _t(n, a, "int", i.aAtlasId), this.texBuffer = _t(n, a, "vec4", i.aTex), this.pointAPointBBuffer = _t(n, a, "vec4", i.aPointAPointB), this.pointCPointDBuffer = _t(n, a, "vec4", i.aPointCPointD), this.lineWidthBuffer = _t(n, a, "vec2", i.aLineWidth), this.colorBuffer = _t(n, a, "vec4", i.aColor), this.cornerRadiusBuffer = _t(n, a, "vec4", i.aCornerRadius), this.borderColorBuffer = _t(n, a, "vec4", i.aBorderColor), n.bindVertexArray(null), o;
    }
  }, {
    key: "buffers",
    get: function() {
      var r = this;
      return this._buffers || (this._buffers = Object.keys(this).filter(function(a) {
        return ir(a, "Buffer");
      }).map(function(a) {
        return r[a];
      })), this._buffers;
    }
  }, {
    key: "startFrame",
    value: function(r) {
      var a = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : Pa.SCREEN;
      this.panZoomMatrix = r, this.renderTarget = a, this.batchDebugInfo = [], this.wrappedCount = 0, this.simpleCount = 0, this.startBatch();
    }
  }, {
    key: "startBatch",
    value: function() {
      this.instanceCount = 0, this.batchManager.startBatch();
    }
  }, {
    key: "endFrame",
    value: function() {
      this.endBatch();
    }
  }, {
    key: "_isVisible",
    value: function(r, a) {
      return r.visible() ? a && a.isVisible ? a.isVisible(r) : !0 : !1;
    }
    /**
     * Draws a texture using the texture atlas.
     */
  }, {
    key: "drawTexture",
    value: function(r, a, n) {
      var i = this.atlasManager, o = this.batchManager, s = i.getRenderTypeOpts(n);
      if (this._isVisible(r, s) && !(r.isEdge() && !this._isValidEdge(r))) {
        if (this.renderTarget.picking && s.getTexPickingMode) {
          var u = s.getTexPickingMode(r);
          if (u === qn.IGNORE)
            return;
          if (u == qn.USE_BB) {
            this.drawPickingRectangle(r, a, n);
            return;
          }
        }
        var l = i.getAtlasInfo(r, n), c = St(l), f;
        try {
          for (c.s(); !(f = c.n()).done; ) {
            var v = f.value, h = v.atlas, d = v.tex1, y = v.tex2;
            o.canAddToCurrentBatch(h) || this.endBatch();
            for (var g = o.getAtlasIndexForBatch(h), p = 0, m = [[d, !0], [y, !1]]; p < m.length; p++) {
              var b = Je(m[p], 2), w = b[0], C = b[1];
              if (w.w != 0) {
                var E = this.instanceCount;
                this.vertTypeBuffer.getView(E)[0] = Oo;
                var x = this.indexBuffer.getView(E);
                Hr(a, x);
                var T = this.atlasIdBuffer.getView(E);
                T[0] = g;
                var k = this.texBuffer.getView(E);
                k[0] = w.x, k[1] = w.y, k[2] = w.w, k[3] = w.h;
                var D = this.transformBuffer.getMatrixView(E);
                this.setTransformMatrix(r, D, s, v, C), this.instanceCount++, C || this.wrappedCount++, this.instanceCount >= this.maxInstances && this.endBatch();
              }
            }
          }
        } catch (P) {
          c.e(P);
        } finally {
          c.f();
        }
      }
    }
    /**
     * matrix is expected to be a 9 element array
     * this function follows same pattern as CRp.drawCachedElementPortion(...)
     */
  }, {
    key: "setTransformMatrix",
    value: function(r, a, n, i) {
      var o = arguments.length > 4 && arguments[4] !== void 0 ? arguments[4] : !0, s = 0;
      if (n.shapeProps && n.shapeProps.padding && (s = r.pstyle(n.shapeProps.padding).pfValue), i) {
        var u = i.bb, l = i.tex1, c = i.tex2, f = l.w / (l.w + c.w);
        o || (f = 1 - f);
        var v = this._getAdjustedBB(u, s, o, f);
        this._applyTransformMatrix(a, v, n, r);
      } else {
        var h = n.getBoundingBox(r), d = this._getAdjustedBB(h, s, !0, 1);
        this._applyTransformMatrix(a, d, n, r);
      }
    }
  }, {
    key: "_applyTransformMatrix",
    value: function(r, a, n, i) {
      var o, s;
      cc(r);
      var u = n.getRotation ? n.getRotation(i) : 0;
      if (u !== 0) {
        var l = n.getRotationPoint(i), c = l.x, f = l.y;
        Cn(r, r, [c, f]), fc(r, r, u);
        var v = n.getRotationOffset(i);
        o = v.x + (a.xOffset || 0), s = v.y + (a.yOffset || 0);
      } else
        o = a.x1, s = a.y1;
      Cn(r, r, [o, s]), as(r, r, [a.w, a.h]);
    }
    /**
     * Adjusts a node or label BB to accomodate padding and split for wrapped textures.
     * @param bb - the original bounding box
     * @param padding - the padding to add to the bounding box
     * @param first - whether this is the first part of a wrapped texture
     * @param ratio - the ratio of the texture width of part of the text to the entire texture
     */
  }, {
    key: "_getAdjustedBB",
    value: function(r, a, n, i) {
      var o = r.x1, s = r.y1, u = r.w, l = r.h, c = r.yOffset;
      a && (o -= a, s -= a, u += 2 * a, l += 2 * a);
      var f = 0, v = u * i;
      return n && i < 1 ? u = v : !n && i < 1 && (f = u - v, o += f, u = v), {
        x1: o,
        y1: s,
        w: u,
        h: l,
        xOffset: f,
        yOffset: c
      };
    }
    /**
     * Draw a solid opaque rectangle matching the element's Bounding Box.
     * Used by the PICKING mode to make the entire BB of a label clickable.
     */
  }, {
    key: "drawPickingRectangle",
    value: function(r, a, n) {
      var i = this.atlasManager.getRenderTypeOpts(n), o = this.instanceCount;
      this.vertTypeBuffer.getView(o)[0] = $r;
      var s = this.indexBuffer.getView(o);
      Hr(a, s);
      var u = this.colorBuffer.getView(o);
      Cr([0, 0, 0], 1, u);
      var l = this.transformBuffer.getMatrixView(o);
      this.setTransformMatrix(r, l, i), this.simpleCount++, this.instanceCount++, this.instanceCount >= this.maxInstances && this.endBatch();
    }
    /**
     * Draw a node using either a texture or a "simple shape".
     */
  }, {
    key: "drawNode",
    value: function(r, a, n) {
      var i = this.simpleShapeOptions.get(n);
      if (this._isVisible(r, i)) {
        var o = i.shapeProps, s = this._getVertTypeForShape(r, o.shape);
        if (s === void 0 || i.isSimple && !i.isSimple(r, this.renderTarget)) {
          this.drawTexture(r, a, n);
          return;
        }
        var u = this.instanceCount;
        if (this.vertTypeBuffer.getView(u)[0] = s, s === vn || s === wa) {
          var l = i.getBoundingBox(r), c = this._getCornerRadius(r, o.radius, l), f = this.cornerRadiusBuffer.getView(u);
          f[0] = c, f[1] = c, f[2] = c, f[3] = c, s === wa && (f[0] = 0, f[2] = 0);
        }
        var v = this.indexBuffer.getView(u);
        Hr(a, v);
        var h = this.renderTarget.picking ? 1 : n === "node-body" ? r.effectiveOpacity() : 1, d = this.renderTarget.picking ? 1 : r.pstyle(o.opacity).value * h, y = r.pstyle(o.color).value, g = this.colorBuffer.getView(u);
        Cr(y, d, g);
        var p = this.lineWidthBuffer.getView(u);
        if (p[0] = 0, p[1] = 0, o.border) {
          var m = r.pstyle("border-width").value;
          if (m > 0) {
            var b = r.pstyle("border-color").value, w = h * r.pstyle("border-opacity").value, C = this.borderColorBuffer.getView(u);
            Cr(b, w, C);
            var E = r.pstyle("border-position").value;
            if (E === "inside")
              p[0] = 0, p[1] = -m;
            else if (E === "outside")
              p[0] = m, p[1] = 0;
            else {
              var x = m / 2;
              p[0] = x, p[1] = -x;
            }
          }
        }
        var T = this.transformBuffer.getMatrixView(u);
        this.setTransformMatrix(r, T, i), this.simpleCount++, this.instanceCount++, this.instanceCount >= this.maxInstances && this.endBatch();
      }
    }
  }, {
    key: "_getVertTypeForShape",
    value: function(r, a) {
      var n = r.pstyle(a).value;
      switch (n) {
        case "rectangle":
          return $r;
        case "ellipse":
          return xa;
        case "roundrectangle":
        case "round-rectangle":
          return vn;
        case "bottom-round-rectangle":
          return wa;
        default:
          return;
      }
    }
  }, {
    key: "_getCornerRadius",
    value: function(r, a, n) {
      var i = n.w, o = n.h;
      if (r.pstyle(a).value === "auto")
        return vr(i, o);
      var s = r.pstyle(a).pfValue, u = i / 2, l = o / 2;
      return Math.min(s, l, u);
    }
    /**
     * Only supports drawing triangles at the moment.
     */
  }, {
    key: "drawEdgeArrow",
    value: function(r, a, n) {
      if (r.visible()) {
        var i = r._private.rscratch, o, s, u;
        if (n === "source" ? (o = i.arrowStartX, s = i.arrowStartY, u = i.srcArrowAngle) : (o = i.arrowEndX, s = i.arrowEndY, u = i.tgtArrowAngle), !(isNaN(o) || o == null || isNaN(s) || s == null || isNaN(u) || u == null)) {
          var l = r.pstyle(n + "-arrow-shape").value;
          if (l !== "none") {
            var c = r.pstyle(n + "-arrow-color").value, f = r.pstyle("opacity").value, v = r.pstyle("line-opacity").value, h = f * v, d = r.pstyle("width").pfValue, y = r.pstyle("arrow-scale").value, g = this.r.getArrowWidth(d, y), p = this.instanceCount, m = this.transformBuffer.getMatrixView(p);
            cc(m), Cn(m, m, [o, s]), as(m, m, [g, g]), fc(m, m, u), this.vertTypeBuffer.getView(p)[0] = No;
            var b = this.indexBuffer.getView(p);
            Hr(a, b);
            var w = this.colorBuffer.getView(p);
            Cr(c, h, w), this.instanceCount++, this.instanceCount >= this.maxInstances && this.endBatch();
          }
        }
      }
    }
    /**
     * Draw straight-line or bezier curve edges.
     */
  }, {
    key: "drawEdgeLine",
    value: function(r, a) {
      if (r.visible()) {
        var n = this._getEdgePoints(r);
        if (n) {
          var i = r.pstyle("opacity").value, o = r.pstyle("line-opacity").value, s = r.pstyle("width").pfValue, u = r.pstyle("line-color").value, l = i * o;
          if (n.length / 2 + this.instanceCount > this.maxInstances && this.endBatch(), n.length == 4) {
            var c = this.instanceCount;
            this.vertTypeBuffer.getView(c)[0] = vc;
            var f = this.indexBuffer.getView(c);
            Hr(a, f);
            var v = this.colorBuffer.getView(c);
            Cr(u, l, v);
            var h = this.lineWidthBuffer.getView(c);
            h[0] = s;
            var d = this.pointAPointBBuffer.getView(c);
            d[0] = n[0], d[1] = n[1], d[2] = n[2], d[3] = n[3], this.instanceCount++, this.instanceCount >= this.maxInstances && this.endBatch();
          } else
            for (var y = 0; y < n.length - 2; y += 2) {
              var g = this.instanceCount;
              this.vertTypeBuffer.getView(g)[0] = dc;
              var p = this.indexBuffer.getView(g);
              Hr(a, p);
              var m = this.colorBuffer.getView(g);
              Cr(u, l, m);
              var b = this.lineWidthBuffer.getView(g);
              b[0] = s;
              var w = n[y - 2], C = n[y - 1], E = n[y], x = n[y + 1], T = n[y + 2], k = n[y + 3], D = n[y + 4], P = n[y + 5];
              y == 0 && (w = 2 * E - T + 1e-3, C = 2 * x - k + 1e-3), y == n.length - 4 && (D = 2 * T - E + 1e-3, P = 2 * k - x + 1e-3);
              var A = this.pointAPointBBuffer.getView(g);
              A[0] = w, A[1] = C, A[2] = E, A[3] = x;
              var B = this.pointCPointDBuffer.getView(g);
              B[0] = T, B[1] = k, B[2] = D, B[3] = P, this.instanceCount++, this.instanceCount >= this.maxInstances && this.endBatch();
            }
        }
      }
    }
  }, {
    key: "_isValidEdge",
    value: function(r) {
      var a = r._private.rscratch;
      return !(a.badLine || a.allpts == null || isNaN(a.allpts[0]));
    }
  }, {
    key: "_getEdgePoints",
    value: function(r) {
      var a = r._private.rscratch;
      if (this._isValidEdge(r)) {
        var n = a.allpts;
        if (n.length == 4)
          return n;
        var i = this._getNumSegments(r);
        return this._getCurveSegmentPoints(n, i);
      }
    }
  }, {
    key: "_getNumSegments",
    value: function(r) {
      var a = 15;
      return Math.min(Math.max(a, 5), this.maxInstances);
    }
  }, {
    key: "_getCurveSegmentPoints",
    value: function(r, a) {
      if (r.length == 4)
        return r;
      for (var n = Array((a + 1) * 2), i = 0; i <= a; i++)
        if (i == 0)
          n[0] = r[0], n[1] = r[1];
        else if (i == a)
          n[i * 2] = r[r.length - 2], n[i * 2 + 1] = r[r.length - 1];
        else {
          var o = i / a;
          this._setCurvePoint(r, o, n, i * 2);
        }
      return n;
    }
  }, {
    key: "_setCurvePoint",
    value: function(r, a, n, i) {
      if (r.length <= 2)
        n[i] = r[0], n[i + 1] = r[1];
      else {
        for (var o = Array(r.length - 2), s = 0; s < o.length; s += 2) {
          var u = (1 - a) * r[s] + a * r[s + 2], l = (1 - a) * r[s + 1] + a * r[s + 3];
          o[s] = u, o[s + 1] = l;
        }
        return this._setCurvePoint(o, a, n, i);
      }
    }
  }, {
    key: "endBatch",
    value: function() {
      var r = this.gl, a = this.vao, n = this.vertexCount, i = this.instanceCount;
      if (i !== 0) {
        var o = this.renderTarget.picking ? this.pickingProgram : this.program;
        r.useProgram(o), r.bindVertexArray(a);
        var s = St(this.buffers), u;
        try {
          for (s.s(); !(u = s.n()).done; ) {
            var l = u.value;
            l.bufferSubData(i);
          }
        } catch (d) {
          s.e(d);
        } finally {
          s.f();
        }
        for (var c = this.batchManager.getAtlases(), f = 0; f < c.length; f++)
          c[f].bufferIfNeeded(r);
        for (var v = 0; v < c.length; v++)
          r.activeTexture(r.TEXTURE0 + v), r.bindTexture(r.TEXTURE_2D, c[v].texture), r.uniform1i(o.uTextures[v], v);
        r.uniform1f(o.uZoom, l0(this.r)), r.uniformMatrix3fv(o.uPanZoomMatrix, !1, this.panZoomMatrix), r.uniform1i(o.uAtlasSize, this.batchManager.getAtlasSize());
        var h = Cr(this.bgColor, 1);
        r.uniform4fv(o.uBGColor, h), r.drawArraysInstanced(r.TRIANGLES, 0, n, i), r.bindVertexArray(null), r.bindTexture(r.TEXTURE_2D, null), this.debug && this.batchDebugInfo.push({
          count: i,
          // instance count
          atlasCount: c.length
        }), this.startBatch();
      }
    }
  }, {
    key: "getDebugInfo",
    value: function() {
      var r = this.atlasManager.getDebugInfo(), a = r.reduce(function(o, s) {
        return o + s.atlasCount;
      }, 0), n = this.batchDebugInfo, i = n.reduce(function(o, s) {
        return o + s.count;
      }, 0);
      return {
        atlasInfo: r,
        totalAtlases: a,
        wrappedCount: this.wrappedCount,
        simpleCount: this.simpleCount,
        batchCount: n.length,
        batchInfo: n,
        totalInstances: i
      };
    }
  }]);
})(), jf = {};
jf.initWebgl = function(t, e) {
  var r = this, a = r.data.contexts[r.WEBGL];
  t.bgColor = R0(r), t.webglTexSize = Math.min(t.webglTexSize, a.getParameter(a.MAX_TEXTURE_SIZE)), t.webglTexRows = Math.min(t.webglTexRows, 54), t.webglTexRowsNodes = Math.min(t.webglTexRowsNodes, 54), t.webglBatchSize = Math.min(t.webglBatchSize, 16384), t.webglTexPerBatch = Math.min(t.webglTexPerBatch, a.getParameter(a.MAX_TEXTURE_IMAGE_UNITS)), r.webglDebug = t.webglDebug, r.webglDebugShowAtlases = t.webglDebugShowAtlases, r.pickingFrameBuffer = m0(a), r.pickingFrameBuffer.needsDraw = !0, r.drawing = new B0(r, a, t);
  var n = function(f) {
    return function(v) {
      return r.getTextAngle(v, f);
    };
  }, i = function(f) {
    return function(v) {
      var h = v.pstyle(f);
      return h && h.value;
    };
  }, o = function(f) {
    return function(v) {
      return v.pstyle("".concat(f, "-opacity")).value > 0;
    };
  }, s = function(f) {
    var v = f.pstyle("text-events").strValue === "yes";
    return v ? qn.USE_BB : qn.IGNORE;
  }, u = function(f) {
    var v = f.position(), h = v.x, d = v.y, y = f.outerWidth(), g = f.outerHeight();
    return {
      w: y,
      h: g,
      x1: h - y / 2,
      y1: d - g / 2
    };
  };
  r.drawing.addAtlasCollection("node", {
    texRows: t.webglTexRowsNodes
  }), r.drawing.addAtlasCollection("label", {
    texRows: t.webglTexRows
  }), r.drawing.addTextureAtlasRenderType("node-body", {
    collection: "node",
    getKey: e.getStyleKey,
    getBoundingBox: e.getElementBox,
    drawElement: e.drawElement
  }), r.drawing.addSimpleShapeRenderType("node-body", {
    getBoundingBox: u,
    isSimple: f0,
    shapeProps: {
      shape: "shape",
      color: "background-color",
      opacity: "background-opacity",
      radius: "corner-radius",
      border: !0
    }
  }), r.drawing.addSimpleShapeRenderType("node-overlay", {
    getBoundingBox: u,
    isVisible: o("overlay"),
    shapeProps: {
      shape: "overlay-shape",
      color: "overlay-color",
      opacity: "overlay-opacity",
      padding: "overlay-padding",
      radius: "overlay-corner-radius"
    }
  }), r.drawing.addSimpleShapeRenderType("node-underlay", {
    getBoundingBox: u,
    isVisible: o("underlay"),
    shapeProps: {
      shape: "underlay-shape",
      color: "underlay-color",
      opacity: "underlay-opacity",
      padding: "underlay-padding",
      radius: "underlay-corner-radius"
    }
  }), r.drawing.addTextureAtlasRenderType("label", {
    // node label or edge mid label
    collection: "label",
    getTexPickingMode: s,
    getKey: zo(e.getLabelKey, null),
    getBoundingBox: Fo(e.getLabelBox, null),
    drawClipped: !0,
    drawElement: e.drawLabel,
    getRotation: n(null),
    getRotationPoint: e.getLabelRotationPoint,
    getRotationOffset: e.getLabelRotationOffset,
    isVisible: i("label")
  }), r.drawing.addTextureAtlasRenderType("edge-source-label", {
    collection: "label",
    getTexPickingMode: s,
    getKey: zo(e.getSourceLabelKey, "source"),
    getBoundingBox: Fo(e.getSourceLabelBox, "source"),
    drawClipped: !0,
    drawElement: e.drawSourceLabel,
    getRotation: n("source"),
    getRotationPoint: e.getSourceLabelRotationPoint,
    getRotationOffset: e.getSourceLabelRotationOffset,
    isVisible: i("source-label")
  }), r.drawing.addTextureAtlasRenderType("edge-target-label", {
    collection: "label",
    getTexPickingMode: s,
    getKey: zo(e.getTargetLabelKey, "target"),
    getBoundingBox: Fo(e.getTargetLabelBox, "target"),
    drawClipped: !0,
    drawElement: e.drawTargetLabel,
    getRotation: n("target"),
    getRotationPoint: e.getTargetLabelRotationPoint,
    getRotationOffset: e.getTargetLabelRotationOffset,
    isVisible: i("target-label")
  });
  var l = Ha(function() {
    console.log("garbage collect flag set"), r.data.gc = !0;
  }, 1e4);
  r.onUpdateEleCalcs(function(c, f) {
    var v = !1;
    f && f.length > 0 && (v |= r.drawing.invalidate(f)), v && l();
  }), M0(r);
};
function R0(t) {
  var e = t.cy.container(), r = e && e.style && e.style.backgroundColor || "white";
  return Sc(r);
}
function ev(t, e) {
  var r = t._private.rscratch;
  return bt(r, "labelWrapCachedLines", e) || [];
}
var zo = function(e, r) {
  return function(a) {
    var n = e(a), i = ev(a, r);
    return i.length > 1 ? i.map(function(o, s) {
      return "".concat(n, "_").concat(s);
    }) : n;
  };
}, Fo = function(e, r) {
  return function(a, n) {
    var i = e(a);
    if (typeof n == "string") {
      var o = n.indexOf("_");
      if (o > 0) {
        var s = Number(n.substring(o + 1)), u = ev(a, r), l = i.h / u.length, c = l * s, f = i.y1 + c;
        return {
          x1: i.x1,
          w: i.w,
          y1: f,
          h: l,
          yOffset: c
        };
      }
    }
    return i;
  };
};
function M0(t) {
  {
    var e = t.render;
    t.render = function(i) {
      i = i || {};
      var o = t.cy;
      t.webgl && (o.zoom() > Uf ? (I0(t), e.call(t, i)) : (L0(t), rv(t, i, Pa.SCREEN)));
    };
  }
  {
    var r = t.matchCanvasSize;
    t.matchCanvasSize = function(i) {
      r.call(t, i), t.pickingFrameBuffer.setFramebufferAttachmentSizes(t.canvasWidth, t.canvasHeight), t.pickingFrameBuffer.needsDraw = !0;
    };
  }
  t.findNearestElements = function(i, o, s, u) {
    return V0(t, i, o);
  };
  {
    var a = t.invalidateCachedZSortedEles;
    t.invalidateCachedZSortedEles = function() {
      a.call(t), t.pickingFrameBuffer.needsDraw = !0;
    };
  }
  {
    var n = t.notify;
    t.notify = function(i, o) {
      n.call(t, i, o), i === "viewport" || i === "bounds" ? t.pickingFrameBuffer.needsDraw = !0 : i === "background" && t.drawing.invalidate(o, {
        type: "node-body"
      });
    };
  }
}
function I0(t) {
  var e = t.data.contexts[t.WEBGL];
  e.clear(e.COLOR_BUFFER_BIT | e.DEPTH_BUFFER_BIT);
}
function L0(t) {
  var e = function(a) {
    a.save(), a.setTransform(1, 0, 0, 1, 0, 0), a.clearRect(0, 0, t.canvasWidth, t.canvasHeight), a.restore();
  };
  e(t.data.contexts[t.NODE]), e(t.data.contexts[t.DRAG]);
}
function O0(t) {
  var e = t.canvasWidth, r = t.canvasHeight, a = Ls(t), n = a.pan, i = a.zoom, o = Lo();
  Cn(o, o, [n.x, n.y]), as(o, o, [i, i]);
  var s = Lo();
  w0(s, e, r);
  var u = Lo();
  return b0(u, s, o), u;
}
function tv(t, e) {
  var r = t.canvasWidth, a = t.canvasHeight, n = Ls(t), i = n.pan, o = n.zoom;
  e.setTransform(1, 0, 0, 1, 0, 0), e.clearRect(0, 0, r, a), e.translate(i.x, i.y), e.scale(o, o);
}
function N0(t, e) {
  t.drawSelectionRectangle(e, function(r) {
    return tv(t, r);
  });
}
function z0(t) {
  var e = t.data.contexts[t.NODE];
  e.save(), tv(t, e), e.strokeStyle = "rgba(0, 0, 0, 0.3)", e.beginPath(), e.moveTo(-1e3, 0), e.lineTo(1e3, 0), e.stroke(), e.beginPath(), e.moveTo(0, -1e3), e.lineTo(0, 1e3), e.stroke(), e.restore();
}
function F0(t) {
  var e = function(n, i, o) {
    for (var s = n.atlasManager.getAtlasCollection(i), u = t.data.contexts[t.NODE], l = s.atlases, c = 0; c < l.length; c++) {
      var f = l[c], v = f.canvas;
      if (v) {
        var h = v.width, d = v.height, y = h * c, g = v.height * o, p = 0.4;
        u.save(), u.scale(p, p), u.drawImage(v, y, g), u.strokeStyle = "black", u.rect(y, g, h, d), u.stroke(), u.restore();
      }
    }
  }, r = 0;
  e(t.drawing, "node", r++), e(t.drawing, "label", r++);
}
function q0(t, e, r, a, n) {
  var i, o, s, u, l = Ls(t), c = l.pan, f = l.zoom;
  {
    var v = c0(t, c, f, e, r), h = Je(v, 2), d = h[0], y = h[1], g = 6;
    i = d - g / 2, o = y - g / 2, s = g, u = g;
  }
  if (s === 0 || u === 0)
    return [];
  var p = t.data.contexts[t.WEBGL];
  p.bindFramebuffer(p.FRAMEBUFFER, t.pickingFrameBuffer), t.pickingFrameBuffer.needsDraw && (p.viewport(0, 0, p.canvas.width, p.canvas.height), rv(t, null, Pa.PICKING), t.pickingFrameBuffer.needsDraw = !1);
  var m = s * u, b = new Uint8Array(m * 4);
  p.readPixels(i, o, s, u, p.RGBA, p.UNSIGNED_BYTE, b), p.bindFramebuffer(p.FRAMEBUFFER, null);
  for (var w = /* @__PURE__ */ new Set(), C = 0; C < m; C++) {
    var E = b.slice(C * 4, C * 4 + 4), x = d0(E) - 1;
    x >= 0 && w.add(x);
  }
  return w;
}
function V0(t, e, r) {
  var a = q0(t, e, r), n = t.getCachedZSortedEles(), i, o, s = St(a), u;
  try {
    for (s.s(); !(u = s.n()).done; ) {
      var l = u.value, c = n[l];
      if (!i && c.isNode() && (i = c), !o && c.isEdge() && (o = c), i && o)
        break;
    }
  } catch (f) {
    s.e(f);
  } finally {
    s.f();
  }
  return [i, o].filter(Boolean);
}
function qo(t, e, r) {
  var a = t.drawing;
  e += 1, r.isNode() ? (a.drawNode(r, e, "node-underlay"), a.drawNode(r, e, "node-body"), a.drawTexture(r, e, "label"), a.drawNode(r, e, "node-overlay")) : (a.drawEdgeLine(r, e), a.drawEdgeArrow(r, e, "source"), a.drawEdgeArrow(r, e, "target"), a.drawTexture(r, e, "label"), a.drawTexture(r, e, "edge-source-label"), a.drawTexture(r, e, "edge-target-label"));
}
function rv(t, e, r) {
  var a;
  t.webglDebug && (a = performance.now());
  var n = t.drawing, i = 0;
  if (r.screen && t.data.canvasNeedsRedraw[t.SELECT_BOX] && N0(t, e), t.data.canvasNeedsRedraw[t.NODE] || r.picking) {
    var o = t.data.contexts[t.WEBGL];
    r.screen ? (o.clearColor(0, 0, 0, 0), o.enable(o.BLEND), o.blendFunc(o.ONE, o.ONE_MINUS_SRC_ALPHA)) : o.disable(o.BLEND), o.clear(o.COLOR_BUFFER_BIT | o.DEPTH_BUFFER_BIT), o.viewport(0, 0, o.canvas.width, o.canvas.height);
    var s = O0(t), u = t.getCachedZSortedEles();
    if (i = u.length, n.startFrame(s, r), r.screen) {
      for (var l = 0; l < u.nondrag.length; l++)
        qo(t, l, u.nondrag[l]);
      for (var c = 0; c < u.drag.length; c++)
        qo(t, c, u.drag[c]);
    } else if (r.picking)
      for (var f = 0; f < u.length; f++)
        qo(t, f, u[f]);
    n.endFrame(), r.screen && t.webglDebugShowAtlases && (z0(t), F0(t)), t.data.canvasNeedsRedraw[t.NODE] = !1, t.data.canvasNeedsRedraw[t.DRAG] = !1;
  }
  if (t.webglDebug) {
    var v = performance.now(), h = !1, d = Math.ceil(v - a), y = n.getDebugInfo(), g = ["".concat(i, " elements"), "".concat(y.totalInstances, " instances"), "".concat(y.batchCount, " batches"), "".concat(y.totalAtlases, " atlases"), "".concat(y.wrappedCount, " wrapped textures"), "".concat(y.simpleCount, " simple shapes")].join(", ");
    if (h)
      console.log("WebGL (".concat(r.name, ") - time ").concat(d, "ms, ").concat(g));
    else {
      console.log("WebGL (".concat(r.name, ") - frame time ").concat(d, "ms")), console.log("Totals:"), console.log("  ".concat(g)), console.log("Texture Atlases Used:");
      var p = y.atlasInfo, m = St(p), b;
      try {
        for (m.s(); !(b = m.n()).done; ) {
          var w = b.value;
          console.log("  ".concat(w.type, ": ").concat(w.keyCount, " keys, ").concat(w.atlasCount, " atlases"));
        }
      } catch (C) {
        m.e(C);
      } finally {
        m.f();
      }
      console.log("");
    }
  }
  t.data.gc && (console.log("Garbage Collect!"), t.data.gc = !1, n.gc());
}
var wr = {};
wr.drawPolygonPath = function(t, e, r, a, n, i) {
  var o = a / 2, s = n / 2;
  t.beginPath && t.beginPath(), t.moveTo(e + o * i[0], r + s * i[1]);
  for (var u = 1; u < i.length / 2; u++)
    t.lineTo(e + o * i[u * 2], r + s * i[u * 2 + 1]);
  t.closePath();
};
wr.drawRoundPolygonPath = function(t, e, r, a, n, i, o) {
  o.forEach(function(s) {
    return Nf(t, s);
  }), t.closePath();
};
wr.drawRoundRectanglePath = function(t, e, r, a, n, i) {
  var o = a / 2, s = n / 2, u = i === "auto" ? vr(a, n) : Math.min(i, s, o);
  t.beginPath && t.beginPath(), t.moveTo(e, r - s), t.arcTo(e + o, r - s, e + o, r, u), t.arcTo(e + o, r + s, e, r + s, u), t.arcTo(e - o, r + s, e - o, r, u), t.arcTo(e - o, r - s, e, r - s, u), t.lineTo(e, r - s), t.closePath();
};
wr.drawBottomRoundRectanglePath = function(t, e, r, a, n, i) {
  var o = a / 2, s = n / 2, u = i === "auto" ? vr(a, n) : i;
  t.beginPath && t.beginPath(), t.moveTo(e, r - s), t.lineTo(e + o, r - s), t.lineTo(e + o, r), t.arcTo(e + o, r + s, e, r + s, u), t.arcTo(e - o, r + s, e - o, r, u), t.lineTo(e - o, r - s), t.lineTo(e, r - s), t.closePath();
};
wr.drawCutRectanglePath = function(t, e, r, a, n, i, o) {
  var s = a / 2, u = n / 2, l = o === "auto" ? ws() : o;
  t.beginPath && t.beginPath(), t.moveTo(e - s + l, r - u), t.lineTo(e + s - l, r - u), t.lineTo(e + s, r - u + l), t.lineTo(e + s, r + u - l), t.lineTo(e + s - l, r + u), t.lineTo(e - s + l, r + u), t.lineTo(e - s, r + u - l), t.lineTo(e - s, r - u + l), t.closePath();
};
wr.drawBarrelPath = function(t, e, r, a, n) {
  var i = a / 2, o = n / 2, s = e - i, u = e + i, l = r - o, c = r + o, f = Ho(a, n), v = f.widthOffset, h = f.heightOffset, d = f.ctrlPtOffsetPct * v;
  t.beginPath && t.beginPath(), t.moveTo(s, l + h), t.lineTo(s, c - h), t.quadraticCurveTo(s + d, c, s + v, c), t.lineTo(u - v, c), t.quadraticCurveTo(u - d, c, u, c - h), t.lineTo(u, l + h), t.quadraticCurveTo(u - d, l, u - v, l), t.lineTo(s + v, l), t.quadraticCurveTo(s + d, l, s, l + h), t.closePath();
};
var hc = Math.sin(0), gc = Math.cos(0), ns = {}, is = {}, av = Math.PI / 40;
for (var Wr = 0 * Math.PI; Wr < 2 * Math.PI; Wr += av)
  ns[Wr] = Math.sin(Wr), is[Wr] = Math.cos(Wr);
wr.drawEllipsePath = function(t, e, r, a, n) {
  if (t.beginPath && t.beginPath(), t.ellipse)
    t.ellipse(e, r, a / 2, n / 2, 0, 0, 2 * Math.PI);
  else
    for (var i, o, s = a / 2, u = n / 2, l = 0 * Math.PI; l < 2 * Math.PI; l += av)
      i = e - s * ns[l] * hc + s * is[l] * gc, o = r + u * is[l] * hc + u * ns[l] * gc, l === 0 ? t.moveTo(i, o) : t.lineTo(i, o);
  t.closePath();
};
var Ya = {};
Ya.createBuffer = function(t, e) {
  var r = document.createElement("canvas");
  return r.width = t, r.height = e, [r, r.getContext("2d")];
};
Ya.bufferCanvasImage = function(t) {
  var e = this.cy, r = e.mutableElements(), a = r.boundingBox(), n = this.findContainerClientCoords(), i = t.full ? Math.ceil(a.w) : n[2], o = t.full ? Math.ceil(a.h) : n[3], s = ne(t.maxWidth) || ne(t.maxHeight), u = this.getPixelRatio(), l = 1;
  if (t.scale !== void 0)
    i *= t.scale, o *= t.scale, l = t.scale;
  else if (s) {
    var c = 1 / 0, f = 1 / 0;
    ne(t.maxWidth) && (c = l * t.maxWidth / i), ne(t.maxHeight) && (f = l * t.maxHeight / o), l = Math.min(c, f), i *= l, o *= l;
  }
  s || (i *= u, o *= u, l *= u);
  var v = document.createElement("canvas");
  v.width = i, v.height = o, v.style.width = i + "px", v.style.height = o + "px";
  var h = v.getContext("2d");
  if (i > 0 && o > 0) {
    h.clearRect(0, 0, i, o), h.globalCompositeOperation = "source-over";
    var d = this.getCachedZSortedEles();
    if (t.full)
      h.translate(-a.x1 * l, -a.y1 * l), h.scale(l, l), this.drawElements(h, d), h.scale(1 / l, 1 / l), h.translate(a.x1 * l, a.y1 * l);
    else {
      var y = e.pan(), g = {
        x: y.x * l,
        y: y.y * l
      };
      l *= e.zoom(), h.translate(g.x, g.y), h.scale(l, l), this.drawElements(h, d), h.scale(1 / l, 1 / l), h.translate(-g.x, -g.y);
    }
    t.bg && (h.globalCompositeOperation = "destination-over", h.fillStyle = t.bg, h.rect(0, 0, i, o), h.fill());
  }
  return v;
};
function _0(t, e) {
  for (var r = atob(t), a = new ArrayBuffer(r.length), n = new Uint8Array(a), i = 0; i < r.length; i++)
    n[i] = r.charCodeAt(i);
  return new Blob([a], {
    type: e
  });
}
function pc(t) {
  var e = t.indexOf(",");
  return t.substr(e + 1);
}
function nv(t, e, r) {
  var a = function() {
    return e.toDataURL(r, t.quality);
  };
  switch (t.output) {
    case "blob-promise":
      return new sa(function(n, i) {
        try {
          e.toBlob(function(o) {
            o != null ? n(o) : i(new Error("`canvas.toBlob()` sent a null value in its callback"));
          }, r, t.quality);
        } catch (o) {
          i(o);
        }
      });
    case "blob":
      return _0(pc(a()), r);
    case "base64":
      return pc(a());
    case "base64uri":
    default:
      return a();
  }
}
Ya.png = function(t) {
  return nv(t, this.bufferCanvasImage(t), "image/png");
};
Ya.jpg = function(t) {
  return nv(t, this.bufferCanvasImage(t), "image/jpeg");
};
var iv = {};
iv.nodeShapeImpl = function(t, e, r, a, n, i, o, s) {
  switch (t) {
    case "ellipse":
      return this.drawEllipsePath(e, r, a, n, i);
    case "polygon":
      return this.drawPolygonPath(e, r, a, n, i, o);
    case "round-polygon":
      return this.drawRoundPolygonPath(e, r, a, n, i, o, s);
    case "roundrectangle":
    case "round-rectangle":
      return this.drawRoundRectanglePath(e, r, a, n, i, s);
    case "cutrectangle":
    case "cut-rectangle":
      return this.drawCutRectanglePath(e, r, a, n, i, o, s);
    case "bottomroundrectangle":
    case "bottom-round-rectangle":
      return this.drawBottomRoundRectanglePath(e, r, a, n, i, s);
    case "barrel":
      return this.drawBarrelPath(e, r, a, n, i);
  }
};
var G0 = ov, Te = ov.prototype;
Te.CANVAS_LAYERS = 3;
Te.SELECT_BOX = 0;
Te.DRAG = 1;
Te.NODE = 2;
Te.WEBGL = 3;
Te.CANVAS_TYPES = ["2d", "2d", "2d", "webgl2"];
Te.BUFFER_COUNT = 3;
Te.TEXTURE_BUFFER = 0;
Te.MOTIONBLUR_BUFFER_NODE = 1;
Te.MOTIONBLUR_BUFFER_DRAG = 2;
function ov(t) {
  var e = this, r = e.cy.window(), a = r.document;
  t.webgl && (Te.CANVAS_LAYERS = e.CANVAS_LAYERS = 4, console.log("webgl rendering enabled")), e.data = {
    canvases: new Array(Te.CANVAS_LAYERS),
    contexts: new Array(Te.CANVAS_LAYERS),
    canvasNeedsRedraw: new Array(Te.CANVAS_LAYERS),
    bufferCanvases: new Array(Te.BUFFER_COUNT),
    bufferContexts: new Array(Te.CANVAS_LAYERS)
  };
  var n = "-webkit-tap-highlight-color", i = "rgba(0,0,0,0)";
  e.data.canvasContainer = a.createElement("div");
  var o = e.data.canvasContainer.style;
  e.data.canvasContainer.style[n] = i, o.position = "relative", o.zIndex = "0", o.overflow = "hidden";
  var s = t.cy.container();
  s.appendChild(e.data.canvasContainer), s.style[n] = i;
  var u = {
    "-webkit-user-select": "none",
    "-moz-user-select": "-moz-none",
    "user-select": "none",
    "-webkit-tap-highlight-color": "rgba(0,0,0,0)",
    "outline-style": "none"
  };
  Id() && (u["-ms-touch-action"] = "none", u["touch-action"] = "none");
  for (var l = 0; l < Te.CANVAS_LAYERS; l++) {
    var c = e.data.canvases[l] = a.createElement("canvas"), f = Te.CANVAS_TYPES[l];
    e.data.contexts[l] = c.getContext(f), e.data.contexts[l] || $e("Could not create canvas of type " + f), Object.keys(u).forEach(function(z) {
      c.style[z] = u[z];
    }), c.style.position = "absolute", c.setAttribute("data-id", "layer" + l), c.style.zIndex = String(Te.CANVAS_LAYERS - l), e.data.canvasContainer.appendChild(c), e.data.canvasNeedsRedraw[l] = !1;
  }
  e.data.topCanvas = e.data.canvases[0], e.data.canvases[Te.NODE].setAttribute("data-id", "layer" + Te.NODE + "-node"), e.data.canvases[Te.SELECT_BOX].setAttribute("data-id", "layer" + Te.SELECT_BOX + "-selectbox"), e.data.canvases[Te.DRAG].setAttribute("data-id", "layer" + Te.DRAG + "-drag"), e.data.canvases[Te.WEBGL] && e.data.canvases[Te.WEBGL].setAttribute("data-id", "layer" + Te.WEBGL + "-webgl");
  for (var l = 0; l < Te.BUFFER_COUNT; l++)
    e.data.bufferCanvases[l] = a.createElement("canvas"), e.data.bufferContexts[l] = e.data.bufferCanvases[l].getContext("2d"), e.data.bufferCanvases[l].style.position = "absolute", e.data.bufferCanvases[l].setAttribute("data-id", "buffer" + l), e.data.bufferCanvases[l].style.zIndex = String(-l - 1), e.data.bufferCanvases[l].style.visibility = "hidden";
  e.pathsEnabled = !0;
  var v = wt(), h = function(L) {
    return {
      x: (L.x1 + L.x2) / 2,
      y: (L.y1 + L.y2) / 2
    };
  }, d = function(L) {
    return {
      x: -L.w / 2,
      y: -L.h / 2
    };
  }, y = function(L) {
    var V = L[0]._private, W = V.oldBackgroundTimestamp === V.backgroundTimestamp;
    return !W;
  }, g = function(L) {
    return L[0]._private.nodeKey;
  }, p = function(L) {
    return L[0]._private.labelStyleKey;
  }, m = function(L) {
    return L[0]._private.sourceLabelStyleKey;
  }, b = function(L) {
    return L[0]._private.targetLabelStyleKey;
  }, w = function(L, V, W, K, re) {
    return e.drawElement(L, V, W, !1, !1, re);
  }, C = function(L, V, W, K, re) {
    return e.drawElementText(L, V, W, K, "main", re);
  }, E = function(L, V, W, K, re) {
    return e.drawElementText(L, V, W, K, "source", re);
  }, x = function(L, V, W, K, re) {
    return e.drawElementText(L, V, W, K, "target", re);
  }, T = function(L) {
    return L.boundingBox(), L[0]._private.bodyBounds;
  }, k = function(L) {
    return L.boundingBox(), L[0]._private.labelBounds.main || v;
  }, D = function(L) {
    return L.boundingBox(), L[0]._private.labelBounds.source || v;
  }, P = function(L) {
    return L.boundingBox(), L[0]._private.labelBounds.target || v;
  }, A = function(L, V) {
    return V;
  }, B = function(L) {
    return h(T(L));
  }, R = function(L, V, W) {
    var K = L ? L + "-" : "";
    return {
      x: V.x + W.pstyle(K + "text-margin-x").pfValue,
      y: V.y + W.pstyle(K + "text-margin-y").pfValue
    };
  }, I = function(L, V, W) {
    var K = L[0]._private.rscratch;
    return {
      x: K[V],
      y: K[W]
    };
  }, O = function(L) {
    return R("", I(L, "labelX", "labelY"), L);
  }, M = function(L) {
    return R("source", I(L, "sourceLabelX", "sourceLabelY"), L);
  }, N = function(L) {
    return R("target", I(L, "targetLabelX", "targetLabelY"), L);
  }, H = function(L) {
    return d(T(L));
  }, G = function(L) {
    return d(D(L));
  }, F = function(L) {
    return d(P(L));
  }, _ = function(L) {
    var V = k(L), W = d(k(L));
    if (L.isNode()) {
      switch (na(L.pstyle("text-halign").value)) {
        case "left":
          W.x = -V.w - (V.leftPad || 0);
          break;
        case "right":
          W.x = -(V.rightPad || 0);
          break;
      }
      switch (ia(L.pstyle("text-valign").value)) {
        case "top":
          W.y = -V.h - (V.topPad || 0);
          break;
        case "bottom":
          W.y = -(V.botPad || 0);
          break;
      }
    }
    return W;
  }, X = e.data.eleTxrCache = new Ta(e, {
    getKey: g,
    doesEleInvalidateKey: y,
    drawElement: w,
    getBoundingBox: T,
    getRotationPoint: B,
    getRotationOffset: H,
    allowEdgeTxrCaching: !1,
    allowParentTxrCaching: !1
  }), J = e.data.lblTxrCache = new Ta(e, {
    getKey: p,
    drawElement: C,
    getBoundingBox: k,
    getRotationPoint: O,
    getRotationOffset: _,
    isVisible: A
  }), Y = e.data.slbTxrCache = new Ta(e, {
    getKey: m,
    drawElement: E,
    getBoundingBox: D,
    getRotationPoint: M,
    getRotationOffset: G,
    isVisible: A
  }), j = e.data.tlbTxrCache = new Ta(e, {
    getKey: b,
    drawElement: x,
    getBoundingBox: P,
    getRotationPoint: N,
    getRotationOffset: F,
    isVisible: A
  }), te = e.data.lyrTxrCache = new Kf(e);
  e.onUpdateEleCalcs(function(L, V) {
    X.invalidateElements(V), J.invalidateElements(V), Y.invalidateElements(V), j.invalidateElements(V), te.invalidateElements(V);
    for (var W = 0; W < V.length; W++) {
      var K = V[W]._private;
      K.oldBackgroundTimestamp = K.backgroundTimestamp;
    }
  });
  var ae = function(L) {
    for (var V = 0; V < L.length; V++)
      te.enqueueElementRefinement(L[V].ele);
  };
  X.onDequeue(ae), J.onDequeue(ae), Y.onDequeue(ae), j.onDequeue(ae), t.webgl && e.initWebgl(t, {
    getStyleKey: g,
    getLabelKey: p,
    getSourceLabelKey: m,
    getTargetLabelKey: b,
    drawElement: w,
    drawLabel: C,
    drawSourceLabel: E,
    drawTargetLabel: x,
    getElementBox: T,
    getLabelBox: k,
    getSourceLabelBox: D,
    getTargetLabelBox: P,
    getElementRotationPoint: B,
    getElementRotationOffset: H,
    getLabelRotationPoint: O,
    getSourceLabelRotationPoint: M,
    getTargetLabelRotationPoint: N,
    getLabelRotationOffset: _,
    getSourceLabelRotationOffset: G,
    getTargetLabelRotationOffset: F
  });
}
Te.redrawHint = function(t, e) {
  var r = this;
  switch (t) {
    case "eles":
      r.data.canvasNeedsRedraw[Te.NODE] = e;
      break;
    case "drag":
      r.data.canvasNeedsRedraw[Te.DRAG] = e;
      break;
    case "select":
      r.data.canvasNeedsRedraw[Te.SELECT_BOX] = e;
      break;
    case "gc":
      r.data.gc = !0;
      break;
  }
};
var H0 = typeof Path2D < "u";
Te.path2dEnabled = function(t) {
  if (t === void 0)
    return this.pathsEnabled;
  this.pathsEnabled = !!t;
};
Te.usePaths = function() {
  return H0 && this.pathsEnabled;
};
Te.setImgSmoothing = function(t, e) {
  t.imageSmoothingEnabled != null ? t.imageSmoothingEnabled = e : (t.webkitImageSmoothingEnabled = e, t.mozImageSmoothingEnabled = e, t.msImageSmoothingEnabled = e);
};
Te.getImgSmoothing = function(t) {
  return t.imageSmoothingEnabled != null ? t.imageSmoothingEnabled : t.webkitImageSmoothingEnabled || t.mozImageSmoothingEnabled || t.msImageSmoothingEnabled;
};
Te.makeOffscreenCanvas = function(t, e) {
  var r;
  if ((typeof OffscreenCanvas > "u" ? "undefined" : rt(OffscreenCanvas)) !== "undefined")
    r = new OffscreenCanvas(t, e);
  else {
    var a = this.cy.window(), n = a.document;
    r = n.createElement("canvas"), r.width = t, r.height = e;
  }
  return r;
};
[Xf, Kt, er, Is, Nr, br, xt, jf, wr, Ya, iv].forEach(function(t) {
  ye(Te, t);
});
var $0 = [{
  name: "null",
  impl: If
}, {
  name: "base",
  impl: $f
}, {
  name: "canvas",
  impl: G0
}], W0 = [{
  type: "layout",
  extensions: hm
}, {
  type: "renderer",
  extensions: $0
}], sv = {}, uv = {};
function lv(t, e, r) {
  var a = r, n = function(T) {
    ze("Can not register `" + e + "` for `" + t + "` since `" + T + "` already exists in the prototype and can not be overridden");
  };
  if (t === "core") {
    if (Na.prototype[e])
      return n(e);
    Na.prototype[e] = r;
  } else if (t === "collection") {
    if (ct.prototype[e])
      return n(e);
    ct.prototype[e] = r;
  } else if (t === "layout") {
    for (var i = function(T) {
      this.options = T, r.call(this, T), Me(this._private) || (this._private = {}), this._private.cy = T.cy, this._private.listeners = [], this.createEmitter();
    }, o = i.prototype = Object.create(r.prototype), s = [], u = 0; u < s.length; u++) {
      var l = s[u];
      o[l] = o[l] || function() {
        return this;
      };
    }
    o.start && !o.run ? o.run = function() {
      return this.start(), this;
    } : !o.start && o.run && (o.start = function() {
      return this.run(), this;
    });
    var c = r.prototype.stop;
    o.stop = function() {
      var x = this.options;
      if (x && x.animate) {
        var T = this.animations;
        if (T)
          for (var k = 0; k < T.length; k++)
            T[k].stop();
      }
      return c ? c.call(this) : this.emit("layoutstop"), this;
    }, o.destroy || (o.destroy = function() {
      return this;
    }), o.cy = function() {
      return this._private.cy;
    };
    var f = function(T) {
      return T._private.cy;
    }, v = {
      addEventFields: function(T, k) {
        k.layout = T, k.cy = f(T), k.target = T;
      },
      bubble: function() {
        return !0;
      },
      parent: function(T) {
        return f(T);
      }
    };
    ye(o, {
      createEmitter: function() {
        return this._private.emitter = new Qn(v, this), this;
      },
      emitter: function() {
        return this._private.emitter;
      },
      on: function(T, k) {
        return this.emitter().on(T, k), this;
      },
      one: function(T, k) {
        return this.emitter().one(T, k), this;
      },
      once: function(T, k) {
        return this.emitter().one(T, k), this;
      },
      removeListener: function(T, k) {
        return this.emitter().removeListener(T, k), this;
      },
      removeAllListeners: function() {
        return this.emitter().removeAllListeners(), this;
      },
      emit: function(T, k) {
        return this.emitter().emit(T, k), this;
      }
    }), Ne.eventAliasesOn(o), a = i;
  } else if (t === "renderer" && e !== "null" && e !== "base") {
    var h = cv("renderer", "base"), d = h.prototype, y = r, g = r.prototype, p = function() {
      h.apply(this, arguments), y.apply(this, arguments);
    }, m = p.prototype;
    for (var b in d) {
      var w = d[b], C = g[b] != null;
      if (C)
        return n(b);
      m[b] = w;
    }
    for (var E in g)
      m[E] = g[E];
    d.clientFunctions.forEach(function(x) {
      m[x] = m[x] || function() {
        $e("Renderer does not implement `renderer." + x + "()` on its prototype");
      };
    }), a = p;
  } else if (t === "__proto__" || t === "constructor" || t === "prototype")
    return $e(t + " is an illegal type to be registered, possibly lead to prototype pollutions");
  return kc({
    map: sv,
    keys: [t, e],
    value: a
  });
}
function cv(t, e) {
  return Pc({
    map: sv,
    keys: [t, e]
  });
}
function U0(t, e, r, a, n) {
  return kc({
    map: uv,
    keys: [t, e, r, a],
    value: n
  });
}
function K0(t, e, r, a) {
  return Pc({
    map: uv,
    keys: [t, e, r, a]
  });
}
var os = function() {
  if (arguments.length === 2)
    return cv.apply(null, arguments);
  if (arguments.length === 3)
    return lv.apply(null, arguments);
  if (arguments.length === 4)
    return K0.apply(null, arguments);
  if (arguments.length === 5)
    return U0.apply(null, arguments);
  $e("Invalid extension access syntax");
};
Na.prototype.extension = os;
W0.forEach(function(t) {
  t.extensions.forEach(function(e) {
    lv(t.type, e.name, e.impl);
  });
});
var Vn = function() {
  if (!(this instanceof Vn))
    return new Vn();
  this.length = 0;
}, Ir = Vn.prototype;
Ir.instanceString = function() {
  return "stylesheet";
};
Ir.selector = function(t) {
  var e = this.length++;
  return this[e] = {
    selector: t,
    properties: []
  }, this;
};
Ir.css = function(t, e) {
  var r = this.length - 1;
  if (he(t))
    this[r].properties.push({
      name: t,
      value: e
    });
  else if (Me(t))
    for (var a = t, n = Object.keys(a), i = 0; i < n.length; i++) {
      var o = n[i], s = a[o];
      if (s != null) {
        var u = ot.properties[o] || ot.properties[_n(o)];
        if (u != null) {
          var l = u.name, c = s;
          this[r].properties.push({
            name: l,
            value: c
          });
        }
      }
    }
  return this;
};
Ir.style = Ir.css;
Ir.generateStyle = function(t) {
  var e = new ot(t);
  return this.appendToStyle(e);
};
Ir.appendToStyle = function(t) {
  for (var e = 0; e < this.length; e++) {
    var r = this[e], a = r.selector, n = r.properties;
    t.selector(a);
    for (var i = 0; i < n.length; i++) {
      var o = n[i];
      t.css(o.name, o.value);
    }
  }
  return t;
};
var X0 = "3.34.0", Lr = function(e) {
  if (e === void 0 && (e = {}), Me(e))
    return new Na(e);
  if (he(e))
    return os.apply(os, arguments);
};
Lr.use = function(t) {
  var e = Array.prototype.slice.call(arguments, 1);
  return e.unshift(Lr), t.apply(null, e), this;
};
Lr.warnings = function(t) {
  return Lc(t);
};
Lr.version = X0;
Lr.stylesheet = Lr.Stylesheet = Vn;
const Y0 = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  default: Lr
}, Symbol.toStringTag, { value: "Module" }));
