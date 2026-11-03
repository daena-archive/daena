var WP = Object.defineProperty;
var zP = (n, e, t) => e in n ? WP(n, e, { enumerable: !0, configurable: !0, writable: !0, value: t }) : n[e] = t;
var Bs = (n, e, t) => zP(n, typeof e != "symbol" ? e + "" : e, t);
const UP = "daena.timeline", VP = "Timeline", KP = "0.1.0", XP = "daena-archive", ZP = ">=1.0.0 <2.0.0", QP = "sandboxed", JP = { ui: "dist/ui/index.html", wasm: "dist/service.wasm" }, eI = ["entity.read", "entity.write", "entity.delete", "document.read", "document.write", "field.read:self", "field.read:shared", "field.write:self", "record.read:self", "record.write:self", "relationship.read", "relationship.write", "asset.read:self", "asset.register", "search.query", "schema.overlay", "service.provide:daena.timeline.resolve-date@1", "service.call:daena.maps/navigation"], tI = { "daena.lore": { version: ">=0.1.0 <1.0.0", required: !1 } }, rI = ["timeline"], iI = [{ namespace: "timeline", entityTypes: [{ id: "event", name: "Timeline event", icon: { kind: "catalog", id: "event" }, iconColor: { kind: "preset", id: "amber" } }, { id: "encounter", name: "Encounter", icon: { kind: "catalog", id: "encounter" }, iconColor: { kind: "preset", id: "ember" } }, { id: "era", name: "Era", icon: { kind: "catalog", id: "era" }, iconColor: { kind: "preset", id: "pine" } }, { id: "calendar", name: "Calendar", icon: { kind: "catalog", id: "calendar" }, iconColor: { kind: "preset", id: "brass" } }], fields: [{ key: "startsAt", label: "Starts", type: "date", required: !1, entityTypes: ["event", "encounter", "era"] }, { key: "endsAt", label: "Ends", type: "date", entityTypes: ["event", "encounter", "era"] }, { key: "description", label: "Description", type: "text", entityTypes: ["calendar", "era"] }, { key: "era", label: "Era", type: "relationship", relationshipType: "during", targetEntityTypes: ["era"], entityTypes: ["event", "encounter"] }, { key: "calendar", label: "Calendar", type: "relationship", relationshipType: "uses_calendar", targetEntityTypes: ["calendar"], entityTypes: ["era"] }, { key: "location", label: "Location", type: "relationship", relationshipType: "occurred_at", targetEntityTypes: ["daena.lore:place"], entityTypes: ["encounter"], metadataFields: [{ key: "significance", label: "Significance", type: "enum", options: ["central", "peripheral"] }] }, { key: "participants", label: "Participants", type: "relationship", relationshipType: "involves", targetEntityTypes: ["daena.lore:person"], entityTypes: ["encounter"], metadataFields: [{ key: "role", label: "Role", type: "enum", options: ["protagonist", "participant", "witness", "instigator"] }, { key: "outcome", label: "Outcome", type: "enum", options: ["survived", "perished", "absent"] }] }, { key: "profileChanges", label: "Profile changes", type: "relationship", relationshipType: "changes_profile", targetEntityTypes: ["daena.lore:person", "daena.lore:place", "daena.lore:faction", "daena.lore:artifact", "daena.lore:culture", "daena.lore:concept"], entityTypes: ["event", "encounter"] }] }], nI = [{ id: "event", name: "Timeline event", entityType: "event", description: "A moment, turning point, or span of history.", fields: { startsAt: "", endsAt: "", era: [], profileChanges: [] } }, { id: "encounter", name: "Encounter", entityType: "encounter", description: "A meeting or interaction between people at a place and time.", fields: { startsAt: "", endsAt: "", location: [], participants: [], era: [], profileChanges: [] } }, { id: "era", name: "Era", entityType: "era", description: "A named period of Timeline history.", fields: { startsAt: "", endsAt: "", description: "", calendar: [] } }, { id: "calendar", name: "Calendar", entityType: "calendar", description: "A way this world names and divides time. Leave it empty to keep using Gregorian dates.", fields: { description: "" } }], sI = [{ id: "calendar-definition", ownerEntityTypes: ["calendar"], schema: { type: "object", properties: { schemaVersion: { type: "number" }, startingYear: { type: "number" }, epoch: { type: "object" }, dateFormat: { type: "string" }, months: { type: "array" }, weekdays: { type: "array" }, seasons: { type: "array" }, allowNegativeYears: { type: "boolean" }, eraLabels: { type: "object" } }, required: ["schemaVersion"], additionalProperties: !1 } }], aI = [], oI = [{ id: "show-on-map", title: "Show event on map", action: { type: "refresh-view" }, exposure: ["view"] }], lI = { provides: [{ name: "daena.timeline.resolve-date", major: 1 }], consumes: [{ name: "daena.maps/navigation", major: 1 }] }, uI = { publishes: [], subscribes: [] }, cI = [{ id: "timeline-v1", from: 0, to: 1, recovery: "backup", operations: [{ kind: "create-namespace", namespace: "timeline" }] }], dI = {
  manifestVersion: 1,
  id: UP,
  name: VP,
  version: KP,
  publisher: XP,
  hostApi: ZP,
  kind: QP,
  entrypoints: JP,
  capabilities: eI,
  dependencies: tI,
  namespaces: rI,
  schemas: iI,
  templates: nI,
  records: sI,
  views: aI,
  commands: oI,
  services: lI,
  events: uI,
  migrations: cI
}, hI = ["agriculture", "anchor", "animal", "art", "artifact", "bird", "calendar", "camp", "castle", "collection", "compass", "concept", "craft", "crown", "culture", "danger", "encounter", "era", "event", "faction", "fire", "fish", "flower", "forest", "group", "heart", "home", "ice", "insect", "journey", "key", "language", "law", "library", "lock", "magic", "manuscript", "map", "mine", "moon", "mountain", "music", "object", "person", "place", "plant", "reference", "religion", "science", "scroll", "settlement", "ship", "spirit", "star", "storm", "sun", "theatre", "unknown", "vision", "wand", "water", "wealth", "world"], fI = ["brass", "copper", "ember", "moss", "pine", "ocean", "sky", "frost", "amber", "gold", "sand", "rose", "plum", "violet", "slate", "ink"], sD = ["ink", "ink-soft", "ink-faint", "ink-muted", "line", "line-soft", "line-strong", "surface", "surface-muted", "surface-warm", "surface-subtle", "surface-quiet", "canvas", "accent", "accent-dark", "accent-soft", "accent-bg", "on-accent", "on-bright-accent", "brass-ink", "danger", "danger-bg", "danger-line", "success", "success-bg", "success-line", "warning", "warning-bg", "warning-line", "info", "info-bg", "info-line", "rail-bg", "rail-surface", "rail-surface-strong", "rail-popover", "rail-text", "rail-text-soft", "rail-text-muted", "rail-text-faint", "rail-accent", "rail-accent-hover", "rail-online", "rail-offline", "rail-border"];
new Set(sD);
const pI = /^#(?:[0-9A-Fa-f]{3}|[0-9A-Fa-f]{6}|[0-9A-Fa-f]{8})$/;
function Xp(n, e) {
  (n.resolved === "light" || n.resolved === "dark") && (e.dataset.theme = n.resolved, e.style.colorScheme = n.resolved), (n.preference === "light" || n.preference === "dark" || n.preference === "system") && (e.dataset.themePreference = n.preference);
  for (const t of sD) {
    const r = n.tokens[t];
    typeof r == "string" && pI.test(r) ? e.style.setProperty(`--${t}`, r) : e.style.removeProperty(`--${t}`);
  }
}
class mI extends Error {
  constructor(t) {
    super(t.message);
    Bs(this, "code");
    Bs(this, "retryable");
    Bs(this, "details");
    this.name = "PluginRpcException", this.code = t.code, this.retryable = t.retryable, this.details = t.details;
  }
}
function uo(n, e) {
  return `${n}@${e}`;
}
async function $e(n, e, t, r) {
  try {
    return await n.call(e, t, r);
  } catch (s) {
    throw Zp(s) ? new mI(s) : s;
  }
}
function Zp(n) {
  return typeof n == "object" && n !== null && typeof n.code == "string" && typeof n.message == "string" && typeof n.retryable == "boolean";
}
function It(n, e, t = !1, r) {
  return { code: n, message: e, retryable: t, details: r };
}
function Av(n) {
  var e;
  if (n === "pluginId" && typeof document < "u") {
    const t = (e = document.body) == null ? void 0 : e.dataset.plugin;
    if (t) return t;
  }
  if (n === "projectId" && typeof location < "u") {
    const t = new URLSearchParams(location.search).get("project");
    if (t) return t;
  }
  throw new Error(`plugin ${n} is not available in the current runtime`);
}
function Nv(n) {
  return new TextEncoder().encode(n).byteLength;
}
let qv = !1;
function vI() {
  qv || typeof window > "u" || (qv = !0, window.addEventListener("daena:appearance", (n) => {
    const e = n.detail;
    !dr(e) || !dr(e.tokens) || typeof document > "u" || Xp(e, document.documentElement);
  }));
}
function gI(n, e) {
  return dr(n) && Zp(n.error) ? n.error : dr(n) && typeof n.error == "string" ? It("transport.host", n.error) : It("transport.protocol", e);
}
function yI(n = {}) {
  var b;
  const e = n.pluginId ?? Av("pluginId"), t = n.projectId ?? Av("projectId"), r = n.endpoint ?? "/__rpc", s = n.fetch ?? ((b = globalThis.fetch) == null ? void 0 : b.bind(globalThis)), a = n.maxRequestBytes ?? 256 * 1024, o = n.maxResponseBytes ?? 256 * 1024;
  if (!s) throw new Error("plugin RPC requires fetch");
  let l = 0, c, d;
  async function f(_) {
    const x = JSON.stringify(_);
    if (Nv(x) > a)
      throw It("payload.invalid", "plugin RPC request exceeds payload limit");
    let T;
    try {
      T = await s(r, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: x
      });
    } catch (D) {
      throw It("transport.unavailable", D instanceof Error ? D.message : String(D), !0);
    }
    const S = await T.text();
    if (Nv(S) > o)
      throw It("transport.protocol", "plugin RPC response exceeds payload limit");
    let k;
    try {
      k = JSON.parse(S);
    } catch {
      throw It("transport.protocol", "plugin RPC returned invalid JSON");
    }
    if (!T.ok) throw gI(k, `plugin RPC HTTP request failed (${T.status})`);
    return k;
  }
  async function p() {
    const _ = await f({ op: "bootstrap", pluginId: e, projectId: t });
    if (!dr(_) || _.rpcVersion !== 1 || _.pluginId !== e || _.projectId !== t || typeof _.sessionId != "string" || !_.sessionId || typeof _.version != "string" || typeof _.hostApi != "string" || !Array.isArray(_.grantedCapabilities) || !Array.isArray(_.optionalFeatures))
      throw It("transport.protocol", "plugin bootstrap response is invalid");
    return c = _.sessionId, typeof document < "u" && dr(_.appearance) && dr(_.appearance.tokens) && Xp(_.appearance, document.documentElement), _;
  }
  async function m() {
    c || (d ?? (d = p().finally(() => {
      d = void 0;
    })), await d);
  }
  async function g(_, x, T) {
    var D, I;
    if (_ === "plugin.bootstrap") return p();
    await m();
    const S = T ?? ((I = (D = globalThis.crypto) == null ? void 0 : D.randomUUID) == null ? void 0 : I.call(D)) ?? `${e}-${++l}`, k = await f({
      op: "rpc",
      request: { rpcVersion: 1, sessionId: c, requestId: S, method: _, payload: x }
    });
    if (!dr(k) || k.rpcVersion !== 1 || k.requestId !== S || typeof k.ok != "boolean")
      throw It("transport.protocol", "plugin RPC response does not match the request");
    if (!k.ok)
      throw Zp(k.error) ? k.error : It("transport.protocol", "plugin RPC returned an invalid error");
    if (!("result" in k)) throw It("transport.protocol", "plugin RPC success has no result");
    return k.result;
  }
  return vI(), { call: g };
}
function dr(n) {
  return typeof n == "object" && n !== null && !Array.isArray(n);
}
function Xi(n) {
  if (!dr(n) || typeof n.id != "string" || typeof n.name != "string" || typeof n.deleted != "boolean" || typeof n.revision != "string")
    throw It("transport.protocol", "broker returned an invalid entity record");
  const e = n.entityType !== void 0 ? n.entityType : n.entity_type, t = n.createdAt ?? n.created_at, r = n.updatedAt ?? n.updated_at;
  if (e !== null && typeof e != "string" || typeof t != "string" || typeof r != "string")
    throw It("transport.protocol", "broker returned an invalid entity record");
  return {
    id: n.id,
    name: n.name,
    entityType: e,
    deleted: n.deleted,
    createdAt: t,
    updatedAt: r,
    revision: n.revision
  };
}
function bI(n) {
  if (!dr(n) || !Array.isArray(n.items) || !Array.isArray(n.typeCounts) || typeof n.total != "number" || typeof n.offset != "number" || typeof n.limit != "number" || typeof n.hasMore != "boolean")
    throw It("transport.protocol", "broker returned an invalid entity page");
  const e = n.typeCounts.map((t) => {
    if (!dr(t) || typeof t.count != "number")
      throw It("transport.protocol", "broker returned an invalid entity type count");
    const r = t.entityType;
    if (r != null && typeof r != "string")
      throw It("transport.protocol", "broker returned an invalid entity type count");
    return { entityType: r, count: t.count };
  });
  return {
    items: n.items.map(Xi),
    total: n.total,
    offset: n.offset,
    limit: n.limit,
    hasMore: n.hasMore,
    typeCounts: e
  };
}
new Set(hI);
new Set(fI);
function _I(n) {
  return {
    call: (e, t, r) => $e(n, e, t, r),
    bootstrap: () => $e(n, "plugin.bootstrap", {}),
    listEntities: async (e) => (await $e(n, "entity.list", e ? { entityType: e } : {})).map(Xi),
    queryEntities: async (e = {}) => bI(await $e(n, "entity.query", e)),
    getEntity: async (e) => {
      const t = await $e(n, "entity.get", { id: e });
      return t == null ? null : Xi(t);
    },
    getEntities: async (e) => (await $e(n, "entity.getMany", { ids: e })).map(Xi),
    createEntity: async (e, t, r) => Xi(
      await $e(
        n,
        "entity.create",
        { name: e, type: t ?? null },
        r == null ? void 0 : r.requestId
      )
    ),
    updateEntity: async (e, t, r, s) => Xi(
      await $e(
        n,
        "entity.update",
        { id: e, name: t ?? null, type: r ?? null, expectedRevision: s == null ? void 0 : s.expectedRevision },
        s == null ? void 0 : s.requestId
      )
    ),
    deleteEntity: (e, t) => $e(
      n,
      "entity.delete",
      { id: e, expectedRevision: t == null ? void 0 : t.expectedRevision },
      t == null ? void 0 : t.requestId
    ),
    publishEvent: (e, t, r) => $e(n, "event.publish", { type: uo(e, t), payload: r }),
    subscribeEvent: (e, t) => $e(n, "event.subscribe", { type: uo(e, t) }),
    pollEvents: (e, t) => $e(n, "event.poll", { type: uo(e, t) }),
    callService: (e, t, r, s = 5e3) => $e(n, "service.call", { name: e, major: t, payload: r, deadlineMs: s }),
    getAppVersion: () => $e(n, "app.version", {}),
    getAppearance: () => $e(n, "appearance.get", {}),
    beginAssetRead: (e, t) => $e(n, "asset.read.begin", { assetId: e, namespace: t }),
    beginSharedAssetRead: (e) => $e(n, "asset.shared.read.begin", { path: e }),
    updateAssetMetadata: (e, t) => $e(n, "asset.update", e, t == null ? void 0 : t.requestId),
    deleteAsset: (e, t) => $e(n, "asset.delete", e, t == null ? void 0 : t.requestId),
    beginAssetReplace: (e, t) => $e(n, "asset.replace.begin", e, t == null ? void 0 : t.requestId),
    commitAssetReplace: (e, t, r) => $e(n, "asset.replace.commit", { handle: e, contentHash: t }, r == null ? void 0 : r.requestId),
    cancelAssetTransfer: (e) => $e(n, "asset.transfer.cancel", { handle: e }),
    startAiRequest: (e) => $e(n, "ai.request.start", e),
    pollAiRequest: (e) => $e(n, "ai.request.poll", { requestId: e }),
    cancelAiRequest: (e) => $e(n, "ai.request.cancel", { requestId: e }),
    getAiResult: (e) => $e(n, "ai.request.result", { requestId: e }),
    getAiCitations: (e) => $e(n, "ai.request.citations", { requestId: e })
  };
}
function wI(n) {
  const e = n.includes(":") ? n.split(":").pop() ?? n : n, t = e.replace(/([a-z0-9])([A-Z])/g, "$1 $2").replace(/[-_]+/g, " ").trim().replace(/\s+/g, " ");
  return t ? t.replace(/\b\w/g, (r) => r.toUpperCase()) : e || n;
}
function xI(n) {
  const e = /* @__PURE__ */ new Map();
  for (const t of n)
    for (const r of t.schemas ?? [])
      for (const s of r.entityTypes ?? [])
        if (!(!s.id || !s.name)) {
          if (e.set(s.id, s.name), t.id && !s.id.includes(":"))
            e.set(`${t.id}:${s.id}`, s.name);
          else if (s.id.includes(":")) {
            const a = s.id.split(":").pop() ?? s.id;
            a && !e.has(a) && e.set(a, s.name);
          }
        }
  return e;
}
function Qp(n, e) {
  if (!n) return "Unknown type";
  const t = e.get(n);
  if (t) return t;
  if (n.includes(":")) {
    const r = n.split(":").pop() ?? n, s = e.get(r);
    if (s) return s;
  }
  return wI(n);
}
const Jp = "gregorian";
function SI(n) {
  return !n || n === Jp;
}
function TI(n) {
  return typeof n == "string" && n.trim() ? n.trim() : Jp;
}
function em(n) {
  return n.era === "BCE" && n.year > 0 ? -n.year : n.year;
}
function EI(n) {
  return n.era === "BCE" && n.year < 0 ? { ...n, year: -n.year } : n;
}
function DI(n) {
  const e = EI(n), t = em(e), r = e.precision === "year" ? [t] : e.precision === "month" ? [t, e.month] : [t, e.month, e.day];
  if (!r.every((a) => typeof a == "number" && Number.isFinite(a))) return "";
  if (["year", "month", "day"].includes(e.precision ?? "day")) return r.join("-");
  const s = e.precision === "hour" ? [e.hour] : e.precision === "minute" ? [e.hour, e.minute] : [e.hour, e.minute, e.second];
  return s.every((a) => typeof a == "number" && Number.isFinite(a)) ? `${r.join("-")}T${s.join(":")}` : "";
}
function wr(n) {
  if (n && typeof n == "object") {
    const o = n;
    if (typeof o.year == "number" && Number.isFinite(o.year)) {
      const l = o.era ?? "CE", c = o.precision ?? (o.day !== void 0 ? "day" : o.month !== void 0 ? "month" : "year");
      return {
        ...o,
        calendar: TI(o.calendar),
        era: l,
        year: l === "BCE" && o.year < 0 ? -o.year : o.year,
        precision: c
      };
    }
  }
  if (typeof n != "string") return null;
  const e = /^(-?\d+)(?:-(\d+)(?:-(\d+))?)?(?:T(\d+)(?::(\d+)(?::(\d+))?)?)?$/.exec(n.trim());
  if (!e) return null;
  const t = Number(e[1]), r = t < 0 ? "BCE" : "CE", s = r === "BCE" ? -t : t, a = e[6] ? "second" : e[5] ? "minute" : e[4] ? "hour" : e[3] ? "day" : e[2] ? "month" : "year";
  return {
    calendar: Jp,
    era: r,
    year: s,
    precision: a,
    ...e[2] ? { month: Number(e[2]) } : {},
    ...e[3] ? { day: Number(e[3]) } : {},
    ...e[4] ? { hour: Number(e[4]) } : {},
    ...e[5] ? { minute: Number(e[5]) } : {},
    ...e[6] ? { second: Number(e[6]) } : {}
  };
}
function co(n) {
  const e = wr(n);
  return e ? DI(e) || "Undated" : typeof n == "string" && n ? n : "Undated";
}
const OI = "calendar-definition", kI = 1, CI = "D MMMM YYYY";
function RI(n, e, t) {
  const r = e.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "");
  return `${n}-${r || t + 1}`;
}
function Zi(n) {
  return n && typeof n == "object" && !Array.isArray(n) ? n : {};
}
function gp(n) {
  return typeof n == "number" && Number.isFinite(n) ? n : void 0;
}
function qr(n) {
  const e = gp(n);
  return e !== void 0 && Number.isInteger(e) && e > 0 ? e : void 0;
}
function nn(n) {
  return typeof n == "string" ? n.trim() : "";
}
function ho(n, e, t) {
  const r = Zi(e), s = nn(r.name) || `Day ${t + 1}`, a = nn(r.shortName);
  return {
    id: nn(r.id) || RI(n, s, t),
    name: s,
    ...a ? { shortName: a } : {}
  };
}
function PI(n) {
  const e = Zi(n), t = Array.isArray(e.months) ? e.months.map((b, _) => {
    const x = ho("month", b, _), T = qr(Zi(b).days) ?? 1;
    return { ...x, days: T };
  }) : [], r = Array.isArray(e.weekdays) ? e.weekdays.map((b, _) => ho("weekday", b, _)) : [], s = Array.isArray(e.seasons) ? e.seasons.map((b, _) => {
    const x = ho("season", b, _), T = Zi(b);
    return {
      ...x,
      startMonth: qr(T.startMonth) ?? 1,
      startDay: qr(T.startDay) ?? 1,
      endMonth: qr(T.endMonth) ?? 1,
      endDay: qr(T.endDay) ?? 1
    };
  }) : [], a = Zi(e.epoch), o = gp(a.year), l = {
    schemaVersion: kI,
    months: t,
    weekdays: r,
    seasons: s
  }, c = gp(e.startingYear);
  c !== void 0 && Number.isInteger(c) && (l.startingYear = c), o !== void 0 && Number.isInteger(o) && (l.epoch = {
    year: o,
    ...qr(a.month) ? { month: qr(a.month) } : {},
    ...qr(a.day) ? { day: qr(a.day) } : {}
  });
  const d = nn(e.dateFormat);
  d && (l.dateFormat = d);
  const f = e.allowNegativeYears;
  typeof f == "boolean" && (l.allowNegativeYears = f);
  const p = Zi(e.eraLabels), m = nn(p.bce), g = nn(p.ce);
  return (m || g) && (l.eraLabels = { ...m ? { bce: m } : {}, ...g ? { ce: g } : {} }), l;
}
function aD(n) {
  return n.months.length === 0 ? null : n.months.reduce((e, t) => e + t.days, 0);
}
function tm(n) {
  return n.months.length > 0 || n.weekdays.length > 0 || n.seasons.length > 0;
}
function yp(n, e, t) {
  const r = /* @__PURE__ */ new Date(0);
  return r.setUTCFullYear(n, e - 1, t), r.setUTCHours(0, 0, 0, 0), Math.round(r.getTime() / 864e5);
}
function $n(n) {
  return n.startingYear ?? 1;
}
function Lv(n) {
  const e = n.epoch ?? { year: 1, month: 1, day: 1 };
  return yp(e.year, e.month ?? 1, e.day ?? 1);
}
function ea(n, e, t) {
  var o;
  if (n.months.length === 0) return t ?? 1;
  const r = Math.max(1, Math.min(n.months.length, e ?? 1)) - 1, s = n.months.slice(0, r).reduce((l, c) => l + c.days, 0), a = ((o = n.months[r]) == null ? void 0 : o.days) ?? 1;
  return s + Math.max(1, Math.min(a, t ?? 1));
}
function Fv(n, e) {
  const t = wr(n);
  if (!t || !e || !tm(e) || e.months.length === 0) return null;
  const r = aD(e);
  if (!r) return null;
  const s = ba(t, e);
  if (!s || !Number.isFinite(s.year)) return null;
  const a = $n(e);
  if ((s.precision ?? "year") === "year" || s.month === void 0)
    return (s.year - a) * r;
  const o = ea(e, s.month, s.day ?? 1);
  return (s.year - a) * r + (o - 1);
}
function $v(n, e) {
  var s;
  if (n.months.length === 0) return { day: e };
  let t = e;
  for (const [a, o] of n.months.entries()) {
    if (t <= o.days) return { month: a + 1, day: t };
    t -= o.days;
  }
  const r = n.months.length;
  return { month: r, day: ((s = n.months[r - 1]) == null ? void 0 : s.days) ?? 1 };
}
function Ws(n, e, t) {
  if (!e || n.seasons.length === 0) return;
  const r = ea(n, e, t ?? 1);
  for (const s of n.seasons) {
    const a = ea(n, s.startMonth, s.startDay), o = ea(n, s.endMonth, s.endDay);
    if (a <= o) {
      if (r >= a && r <= o) return s.name;
    } else if (r >= a || r <= o)
      return s.name;
  }
}
function ba(n, e) {
  var m, g;
  const t = wr(n);
  if (!t) return null;
  const r = em(t);
  if (!e || !tm(e))
    return {
      year: r,
      month: t.month,
      day: t.day,
      precision: t.precision ?? "year"
    };
  const s = aD(e);
  if (!s)
    return {
      year: $n(e) + (r - (((m = e.epoch) == null ? void 0 : m.year) ?? 1)),
      precision: "year"
    };
  if ((t.precision ?? "year") === "year" || t.month === void 0)
    return {
      year: $n(e) + (r - (((g = e.epoch) == null ? void 0 : g.year) ?? 1)),
      precision: "year"
    };
  if ((t.precision ?? "day") === "month" || t.day === void 0) {
    const _ = yp(r, t.month, 1) - Lv(e), x = Math.floor(_ / s);
    let T = _ % s + 1;
    T <= 0 && (T += s);
    const S = $v(e, T), k = $n(e) + x, D = e.weekdays.length > 0 ? (_ % e.weekdays.length + e.weekdays.length) % e.weekdays.length : void 0;
    return {
      year: k,
      month: S.month,
      ...D !== void 0 ? { weekday: D } : {},
      ...Ws(e, S.month, 1) ? { season: Ws(e, S.month, 1) } : {},
      precision: "month"
    };
  }
  const o = yp(r, t.month, t.day ?? 1) - Lv(e), l = Math.floor(o / s);
  let c = o % s + 1;
  c <= 0 && (c += s);
  const d = $v(e, c), f = $n(e) + l, p = e.weekdays.length > 0 ? (o % e.weekdays.length + e.weekdays.length) % e.weekdays.length : void 0;
  return {
    year: f,
    ...d,
    ...p !== void 0 ? { weekday: p } : {},
    ...Ws(e, d.month, d.day) ? { season: Ws(e, d.month, d.day) } : {},
    precision: t.precision ?? "day"
  };
}
function II(n, e) {
  var r, s, a, o;
  if (n < 0) {
    const l = ((s = (r = e.eraLabels) == null ? void 0 : r.bce) == null ? void 0 : s.trim()) || "BCE";
    return l ? ` ${l}` : "";
  }
  const t = (o = (a = e.eraLabels) == null ? void 0 : a.ce) == null ? void 0 : o.trim();
  return t ? ` ${t}` : "";
}
function oD(n, e) {
  var f;
  const t = ((f = e.dateFormat) == null ? void 0 : f.trim()) || CI, r = n.month !== void 0 ? e.months[n.month - 1] : void 0, s = n.weekday !== void 0 ? e.weekdays[n.weekday] : void 0, a = (p) => String(p).padStart(2, "0"), o = n.year < 0 ? -n.year : n.year, l = {
    YYYY: String(o),
    YY: String(Math.abs(o)).slice(-2).padStart(2, "0"),
    MMMM: (r == null ? void 0 : r.name) ?? "",
    MMM: (r == null ? void 0 : r.shortName) || (r == null ? void 0 : r.name) || "",
    MM: n.month !== void 0 ? a(n.month) : "",
    M: n.month !== void 0 ? String(n.month) : "",
    DD: n.day !== void 0 ? a(n.day) : "",
    D: n.day !== void 0 ? String(n.day) : "",
    WWWW: (s == null ? void 0 : s.name) ?? "",
    WWW: (s == null ? void 0 : s.shortName) || (s == null ? void 0 : s.name) || "",
    SSSS: n.season ?? ""
  }, c = t.replace(/YYYY|MMMM|WWWW|SSSS|MMM|WWW|YY|MM|DD|M|D/g, (p) => l[p] ?? p).replace(/\s+,/g, ",").replace(/[/\-.]{2,}/g, (p) => p[0] ?? "").replace(/\s{2,}/g, " ").replace(/^[/\-.,\s]+|[/\-.,\s]+$/g, "").trim(), d = II(n.year, e);
  return d ? `${c}${d}` : c;
}
function lD(n, e) {
  const t = wr(n);
  if (!t) return typeof n == "string" && n ? n : "Undated";
  if (!e || !tm(e) || e.months.length === 0)
    return co(t);
  const r = ba(t, e);
  return r && oD(r, e) || co(t);
}
const ta = "daena.maps/navigation";
function jv(n) {
  return {
    id: n.id,
    name: n.name,
    type: n.type ?? n.entityType ?? n.entity_type ?? null
  };
}
function MI(n) {
  return n.some(
    (e) => {
      var t;
      return e.id === "daena.maps" && (((t = e.services) == null ? void 0 : t.provides) ?? []).some((r) => r.name === ta && r.major === 1);
    }
  );
}
async function AI(n, e = { id: "daena.timeline" }) {
  const t = await n.call("schema.list", {}), r = MI(t);
  return {
    module: t.find((a) => a.id === e.id) ?? e,
    listEnabledManifests: async () => t,
    mapsNavigationEnabled: () => r,
    async listEntities() {
      const a = [];
      let o = 0;
      for (; ; ) {
        const l = await n.call("entity.query", {
          offset: o,
          limit: 200
        });
        if (a.push(...l.items.map(jv)), !l.hasMore) return a;
        o += l.items.length;
      }
    },
    async listOwnFields(a) {
      const o = await n.call("field.list", { entityId: a, namespace: "timeline" });
      return Object.fromEntries(o.map((l) => [l.key, l.value]));
    },
    async listSharedFields(a, o) {
      return (await n.call("field.list", { entityId: a, namespace: o, sharedOnly: !0 })).map((c) => ({
        entityId: c.entityId ?? c.entity_id ?? a,
        namespace: c.namespace,
        key: c.key,
        value: c.value,
        revision: c.revision
      }));
    },
    async listRelationships(a) {
      return (await n.call("relationship.list", { entityId: a })).map((l) => ({
        type: l.relationshipType ?? l.relationship_type ?? "",
        targetId: l.targetId ?? l.target_id ?? ""
      }));
    },
    async getEntity(a) {
      const o = await n.call("entity.get", { id: a });
      return o == null ? null : jv(o);
    },
    async listCalendarDefinition(a) {
      var l;
      return ((l = (await n.call("record.list", {
        collection: OI,
        ownerEntityId: a,
        limit: 1
      }))[0]) == null ? void 0 : l.value) ?? null;
    },
    async listLocations(a) {
      return n.call("service.call", {
        name: ta,
        major: 1,
        payload: { operation: "listLocations", entityId: a }
      });
    },
    async focusEntity(a) {
      return n.call("service.call", {
        name: ta,
        major: 1,
        payload: { operation: "focusEntity", entityId: a }
      });
    },
    async openMap(a) {
      await n.call("service.call", {
        name: ta,
        major: 1,
        payload: { operation: "openMap", mapEntityId: a.mapEntityId, linkId: a.linkId }
      });
    }
  };
}
function NI(n, e) {
  return !n.entityTypes || e != null && n.entityTypes.includes(e);
}
function qI(n, e) {
  var r, s;
  const t = [];
  for (const a of n)
    if (a.id !== e)
      for (const o of a.schemas)
        for (const l of o.fields) {
          if (l.type !== "date" || !l.shared) continue;
          const c = l.timeline;
          if (!c) continue;
          const d = c.role ?? "point";
          (d === "start" || d === "end") && !((r = c.group) != null && r.trim()) || t.push({
            namespace: o.namespace,
            key: l.key,
            label: ((s = c.label) == null ? void 0 : s.trim()) || l.label,
            entityTypes: l.entityTypes,
            role: d,
            group: c.group ?? void 0,
            layer: c.layer ?? "dates"
          });
        }
  return t.sort(
    (a, o) => a.namespace.localeCompare(o.namespace) || (a.group ?? a.key).localeCompare(o.group ?? o.key) || a.key.localeCompare(o.key)
  );
}
function LI(n, e, t) {
  var o, l, c, d;
  const r = new Map(e.map((f) => [`${f.namespace}:${f.key}`, f.value])), s = [], a = /* @__PURE__ */ new Map();
  for (const f of t) {
    if (!NI(f, n.type)) continue;
    const p = r.get(`${f.namespace}:${f.key}`);
    if (!wr(p)) continue;
    if (f.role === "point") {
      s.push({
        id: `${f.namespace}:${f.key}:${n.id}`,
        entity: n,
        namespace: f.namespace,
        layer: f.layer,
        label: n.name,
        startValue: p,
        startLabel: f.label,
        pointRole: "point"
      });
      continue;
    }
    const m = f.group, g = `${f.namespace}:${m}:${f.layer}`, b = a.get(g) ?? { namespace: f.namespace, group: m, layer: f.layer };
    b[f.role] = { value: p, label: f.label }, a.set(g, b);
  }
  for (const f of a.values())
    !f.start && !f.end || s.push({
      id: `${f.namespace}:${f.group}:${n.id}`,
      entity: n,
      namespace: f.namespace,
      layer: f.layer,
      label: n.name,
      startValue: ((o = f.start) == null ? void 0 : o.value) ?? ((l = f.end) == null ? void 0 : l.value),
      endValue: f.start && f.end ? f.end.value : void 0,
      startLabel: ((c = f.start) == null ? void 0 : c.label) ?? ((d = f.end) == null ? void 0 : d.label),
      endLabel: f.start && f.end ? f.end.label : void 0,
      pointRole: f.start && f.end ? void 0 : f.start ? "start" : "end"
    });
  return s.sort((f, p) => f.id.localeCompare(p.id));
}
function Ji(n) {
  const e = wr(n);
  if (!e) return null;
  const t = /* @__PURE__ */ new Date(0);
  return t.setUTCFullYear(em(e), (e.month ?? 1) - 1, e.day ?? 1), t.setUTCHours(e.hour ?? 0, e.minute ?? 0, e.second ?? 0, 0), Number.isFinite(t.getTime()) ? { date: t, source: e } : null;
}
function FI(n, e, t) {
  const r = wr(n), s = r == null ? void 0 : r.calendar;
  if (s && !SI(s) && t[s])
    return t[s];
  for (const a of e)
    if (t[a])
      return t[a];
  return null;
}
function Yv(n, e, t) {
  const r = Fv(n, t), s = Fv(e, t);
  if (r !== null && s !== null) return r - s;
  const a = Ji(n), o = Ji(e);
  return !a || !o ? null : a.date.getTime() - o.date.getTime();
}
function $I(n, e, t, r) {
  if (!wr(n)) return !1;
  if (e) {
    const s = Yv(n, e, r ?? null);
    if (s !== null && s < 0) return !0;
  }
  if (t) {
    const s = Yv(n, t, r ?? null);
    if (s !== null && s > 0) return !0;
  }
  return !1;
}
function jI(n) {
  if (n.eraIds.includes(n.eraId)) return !0;
  const e = n.startValue ?? n.endValue;
  return !e || !n.eraStart && !n.eraEnd ? !1 : !$I(e, n.eraStart, n.eraEnd, n.calendarDefinition ?? null);
}
function YI(n) {
  if (!n || typeof n != "object") return null;
  const e = n;
  return e.contractVersion !== 1 || e.kind !== "physical-offset-years" || e.reference !== "accepted-source" || typeof e.startOffsetYears != "number" || !Number.isSafeInteger(e.startOffsetYears) || typeof e.endOffsetYears != "number" || !Number.isSafeInteger(e.endOffsetYears) || e.startOffsetYears < -1e5 || e.endOffsetYears > 1e5 || e.startOffsetYears > e.endOffsetYears ? null : e.startOffsetYears;
}
function uD(n) {
  return `${n >= 0 ? "+" : ""}${n} years from accepted source`;
}
function fo(n, e, t) {
  const r = (1 - Math.abs(2 * t - 1)) * e, s = n / 60, a = r * (1 - Math.abs(s % 2 - 1)), [o, l, c] = s < 1 ? [r, a, 0] : s < 2 ? [a, r, 0] : s < 3 ? [0, r, a] : s < 4 ? [0, a, r] : s < 5 ? [a, 0, r] : [r, 0, a], d = t - r / 2;
  return `#${[o, l, c].map(
    (f) => Math.round((f + d) * 255).toString(16).padStart(2, "0")
  ).join("")}`;
}
function HI(n) {
  return {
    fill: fo(n, 0.42, 0.82),
    border: fo(n, 0.44, 0.42),
    text: fo(n, 0.38, 0.22)
  };
}
function Hv(n, e) {
  return n === "lifelines" ? { fill: "#dfeae2", border: "#4f705a", text: "#284234" } : n === "dates" ? { fill: "#f3e4cf", border: "#a56d32", text: "#5f3d20" } : HI(BI(e));
}
function bp(n) {
  return n.layer === "lifelines" ? "lifelines" : n.layer === "dates" ? "dates" : "events";
}
function cD(n) {
  return n.layer === "lifelines" ? "Lifeline" : n.layer === "dates" ? "Project date" : n.entity.type === "daena.timeline:era" ? "Era" : "Timeline event";
}
function Gv(n, e) {
  return e ? [
    n.entity.name,
    n.entity.type,
    n.startLabel,
    n.endLabel,
    n.locationName,
    ...n.participantNames
  ].filter(Boolean).some((t) => String(t).toLocaleLowerCase().includes(e)) : !0;
}
function ra(n) {
  if (n.end) return n.end;
  const e = new Date(n.start.getTime());
  return e.setUTCFullYear(e.getUTCFullYear() + 1), e;
}
function GI(n, e) {
  const t = e.getTime();
  return n.filter((s) => s.start.getTime() <= t && t <= ra(s).getTime()).sort(
    (s, a) => ra(s).getTime() - s.start.getTime() - (ra(a).getTime() - a.start.getTime())
  )[0];
}
function BI(n) {
  let e = 2166136261;
  for (let t = 0; t < n.length; t += 1)
    e ^= n.charCodeAt(t), e = Math.imul(e, 16777619);
  return (e >>> 0) % 360;
}
function WI(n) {
  if (n instanceof Date) return Number.isFinite(n.getTime()) ? n : null;
  if (typeof n == "number" && Number.isFinite(n)) {
    const e = new Date(n);
    return Number.isFinite(e.getTime()) ? e : null;
  }
  if (n && typeof n == "object") {
    const e = n;
    if (typeof e.toDate == "function") {
      const t = e.toDate();
      return t instanceof Date && Number.isFinite(t.getTime()) ? t : null;
    }
    if (typeof e.valueOf == "function") {
      const t = new Date(e.valueOf());
      return Number.isFinite(t.getTime()) ? t : null;
    }
  }
  return null;
}
function zI(n) {
  return n === "year" ? "year" : n === "month" ? "month" : n === "day" || n === "weekday" || n === "week" ? "day" : n === "hour" ? "hour" : n === "minute" ? "minute" : "second";
}
function _p(n, e = "day") {
  return {
    calendar: "gregorian",
    era: "CE",
    year: n.getUTCFullYear(),
    ...e !== "year" ? { month: n.getUTCMonth() + 1 } : {},
    ...["year", "month"].includes(e ?? "day") ? {} : { day: n.getUTCDate() },
    ...["hour", "minute", "second"].includes(e ?? "day") ? { hour: n.getUTCHours() } : {},
    ...["minute", "second"].includes(e ?? "day") ? { minute: n.getUTCMinutes() } : {},
    ...e === "second" ? { second: n.getUTCSeconds() } : {},
    precision: e
  };
}
function Bv(n, e, t) {
  const r = WI(n);
  if (!r) return "";
  const s = zI(t);
  if (!e) return lD(_p(r, s), null);
  const a = ba(_p(r, "day"), e);
  return a ? oD(
    {
      year: a.year,
      ...s !== "year" ? { month: a.month } : {},
      ...["year", "month"].includes(s ?? "day") ? {} : { day: a.day },
      ...s !== "year" ? { weekday: a.weekday, season: a.season } : {}
    },
    e
  ) : "";
}
function UI(n, e) {
  var r;
  const t = (r = wr(n)) == null ? void 0 : r.calendar;
  return t ? e.get(t) ?? null : null;
}
function Wv(n, e) {
  return lD(n, UI(n, e));
}
function rm(n, e) {
  const t = Wv(n.startValue, e), r = n.endValue ? Wv(n.endValue, e) : "";
  if (!n.endValue || r === "Undated" || r === t)
    return n.startLabel ? `${n.startLabel}: ${t}` : t;
  const s = n.startLabel ? `${n.startLabel}: ${t}` : t, a = n.endLabel ? `${n.endLabel}: ${r}` : r;
  return `${s} – ${a}`;
}
function jn(n, e) {
  var t;
  return ((t = ba(_p(n.start, "day"), e)) == null ? void 0 : t.year) ?? n.start.getUTCFullYear();
}
async function VI(n, e) {
  const t = e.filter((s) => s.type === "daena.timeline:calendar"), r = await Promise.all(
    t.map(async (s) => {
      try {
        const a = await n(s.id);
        return {
          id: s.id,
          name: s.name,
          definition: PI(a ?? {})
        };
      } catch {
        return null;
      }
    })
  );
  return [
    { id: "gregorian", name: "Gregorian", definition: null },
    ...r.filter((s) => s !== null)
  ];
}
function dD(n) {
  return [n.locationName, n.participantNames.length ? n.participantNames.join(", ") : ""].filter(
    Boolean
  ).join(" · ");
}
function KI() {
  const n = document.createElement("style");
  return n.textContent = `
    .timeline-shell { display: grid; gap: 0; background: var(--surface); }
    .timeline-toolbar { display: flex; align-items: flex-end; justify-content: space-between; gap: 12px; padding: 10px 14px; border-bottom: 1px solid var(--line-soft); background: var(--surface); }
    .timeline-toolbar-controls { display: flex; align-items: flex-end; flex: 1; flex-wrap: wrap; gap: 8px; }
    .timeline-toolbar-actions { display: flex; align-items: center; gap: 5px; }
    .timeline-toolbar button { border: 1px solid var(--line-strong); border-radius: 7px; padding: 7px 9px; background: var(--surface); color: var(--ink-muted); font: 600 10px Inter, ui-sans-serif, system-ui, sans-serif; cursor: pointer; }
    .timeline-toolbar button:hover, .timeline-toolbar button:focus-visible { border-color: var(--accent); color: var(--theme-warning-text, #55351f); outline: none; }
    .timeline-filter-field { display: grid; gap: 3px; color: var(--ink-muted); font: 600 8px Inter, ui-sans-serif, system-ui, sans-serif; letter-spacing: .045em; text-transform: uppercase; }
    .timeline-filter-field input, .timeline-filter-field select { height: 30px; border: 1px solid var(--line-strong); border-radius: 7px; padding: 0 9px; background: var(--surface); color: var(--theme-neutral-text-soft, #51483e); font: 500 10px Inter, ui-sans-serif, system-ui, sans-serif; letter-spacing: 0; text-transform: none; }
    .timeline-filter-field input:focus-visible, .timeline-filter-field select:focus-visible { border-color: var(--accent); box-shadow: 0 0 0 2px rgba(180, 119, 63, .12); outline: none; }
    .timeline-search { width: min(230px, 28vw); }
    .timeline-scope, .timeline-calendar, .timeline-type-filter { min-width: 120px; }
    .timeline-layerbar { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 8px 14px; border-bottom: 1px solid var(--line-soft); background: var(--theme-warning-bg, #fbf8f0); }
    .timeline-layer-chips, .timeline-legend { display: flex; align-items: center; flex-wrap: wrap; gap: 6px; }
    .timeline-layer-chip { display: inline-flex; align-items: center; gap: 6px; border: 1px solid var(--line-strong); border-radius: 999px; padding: 5px 9px; background: var(--surface); color: var(--theme-neutral-text-soft, #766c60); font: 600 10px Inter, ui-sans-serif, system-ui, sans-serif; cursor: pointer; }
    .timeline-layer-chip[aria-pressed="true"] { border-color: var(--theme-warning-border, #9c6c3d); background: var(--theme-warning-bg, #f1e4d3); color: var(--theme-warning-text, #51351f); }
    .timeline-layer-count { min-width: 17px; border-radius: 999px; padding: 1px 5px; background: rgba(48, 44, 38, .08); text-align: center; font-size: 9px; }
    .timeline-legend { color: var(--ink-muted); font: 9px Inter, ui-sans-serif, system-ui, sans-serif; }
    .timeline-legend-item { display: inline-flex; align-items: center; gap: 4px; white-space: nowrap; }
    .timeline-legend-dot { width: 7px; height: 7px; border: 2px solid var(--theme-warning-border, #a56d32); border-radius: 50%; background: var(--surface); }
    .timeline-legend-range { width: 18px; height: 6px; border: 1px solid var(--theme-success-border, #4f705a); border-radius: 999px; background: var(--theme-success-bg, #dfeae2); }
    .timeline-legend-approx { width: 18px; height: 6px; border: 1px dashed var(--ink-muted); border-radius: 2px; }
    .timeline-workspace { display: grid; grid-template-columns: var(--timeline-outline-width, 300px) 8px minmax(360px, 1fr) minmax(220px, 270px); min-height: 420px; }
    .timeline-outline { overflow: auto; max-height: min(62vh, 620px); padding: 10px; border-right: 1px solid var(--line-soft); background: var(--surface); }
    .timeline-outline-resize { cursor: col-resize; background: var(--surface-warm); touch-action: none; }
    .timeline-outline-resize: hover, .timeline-outline-resize:focus-visible { background: var(--theme-warning-border, #dfcfb8); outline: none; }
    .timeline-outline-heading { display: block; margin: 4px 5px 8px; color: var(--ink-muted); font: 600 10px Inter, ui-sans-serif, system-ui, sans-serif; letter-spacing: .04em; text-transform: uppercase; }
    .timeline-outline-year { margin: 12px 5px 5px; color: var(--ink-muted); font: 600 12px var(--font-display, Georgia, serif); }
    .timeline-event-card { display: grid; width: 100%; gap: 4px; margin: 2px 0; padding: 8px 9px; border: 1px solid transparent; border-radius: 8px; background: transparent; color: inherit; text-align: left; cursor: pointer; }
    .timeline-event-card:hover, .timeline-event-card:focus-visible, .timeline-event-card.is-selected { border-color: var(--line-strong); background: var(--theme-warning-bg, #f7f1e7); outline: none; }
    .timeline-event-card strong { color: var(--theme-neutral-text, #302c26); font: 600 11px Inter, ui-sans-serif, system-ui, sans-serif; }
    .timeline-event-card small { color: var(--ink-muted); font: 10px/1.35 Inter, ui-sans-serif, system-ui, sans-serif; }
    .timeline-card-meta { display: flex; align-items: center; justify-content: space-between; gap: 6px; }
    .timeline-card-kind { border-radius: 999px; padding: 2px 5px; background: var(--theme-warning-bg, #eee6da); color: var(--theme-neutral-text-soft, #786b5d); font: 600 8px Inter, ui-sans-serif, system-ui, sans-serif; letter-spacing: .025em; text-transform: uppercase; }
    .timeline-canvas { position: relative; height: min(62vh, 620px); min-height: 420px; background: radial-gradient(circle at 50% 42%, var(--theme-warning-bg, #fffdf7) 0, var(--theme-warning-bg, #fbf8f0) 52%, var(--surface-warm) 100%); }
    .timeline-canvas .vis-timeline { border: 0; background: transparent; }
    .timeline-canvas .vis-panel.vis-background, .timeline-canvas .vis-panel.vis-center, .timeline-canvas .vis-panel.vis-left, .timeline-canvas .vis-panel.vis-right, .timeline-canvas .vis-panel.vis-top, .timeline-canvas .vis-panel.vis-bottom { border-color: var(--line-soft); }
    .timeline-canvas .vis-time-axis .vis-text { color: var(--ink-muted); font: 10px Inter, ui-sans-serif, system-ui, sans-serif; }
    .timeline-canvas .vis-time-axis .vis-grid.vis-minor { border-color: var(--theme-warning-border, #efe7db); }
    .timeline-canvas .vis-time-axis .vis-grid.vis-major { border-color: var(--theme-warning-border, #e0d5c4); }
    .timeline-canvas .vis-item { border-width: 1px; border-radius: 7px; font: 600 11px Inter, ui-sans-serif, system-ui, sans-serif; box-shadow: 0 0 0 1px rgba(48, 44, 38, 0.06); }
    .timeline-canvas .vis-item.vis-selected { box-shadow: 0 0 0 2px rgba(139, 92, 46, 0.28); }
    .timeline-canvas .vis-item.vis-background { background: rgba(180, 119, 63, 0.12); color: var(--ink-muted); border: 0; box-shadow: none; font: 600 10px Inter, ui-sans-serif, system-ui, sans-serif; }
    .timeline-canvas .vis-item.timeline-lifeline { border-radius: 999px; }
    .timeline-canvas .vis-item.timeline-imprecise { border-style: dashed; }
    .timeline-canvas .vis-item.vis-point .vis-dot { border-width: 2px; }
    .timeline-canvas .vis-item .vis-item-content { padding: 3px 8px; }
    .timeline-canvas .vis-labelset .vis-label, .timeline-canvas .vis-foreground .vis-group { border-color: var(--line-soft); }
    .timeline-canvas .vis-labelset .vis-label { background: color-mix(in srgb, var(--surface) 88%, transparent); color: var(--ink-soft); font: 600 10px Inter, ui-sans-serif, system-ui, sans-serif; }
    .timeline-canvas .vis-labelset .vis-label .vis-inner { padding: 8px 10px; }
    .timeline-canvas .vis-item.timeline-lifeline .vis-item-content { display: inline-flex; align-items: center; gap: 6px; }
    .timeline-canvas .vis-item.timeline-lifeline .vis-item-content::before, .timeline-canvas .vis-item.timeline-lifeline .vis-item-content::after { content: ""; width: 6px; height: 6px; flex: 0 0 auto; border: 1px solid currentColor; border-radius: 50%; background: var(--surface); }
    .timeline-details { display: grid; align-content: start; gap: 9px; min-height: 55px; padding: 16px; background: var(--surface); color: var(--ink-muted); font: 11px/1.45 Inter, ui-sans-serif, system-ui, sans-serif; }
    .timeline-inspector { overflow: auto; max-height: min(62vh, 620px); border-left: 1px solid var(--line-soft); }
    .timeline-details strong { color: var(--theme-neutral-text, #302c26); font: 500 18px/1.15 var(--font-display, Georgia, serif); }
    .timeline-details small { color: var(--ink-muted); }
    .timeline-inspector-kicker { color: var(--theme-warning-text, #9a6a3b) !important; font: 700 8px Inter, ui-sans-serif, system-ui, sans-serif !important; letter-spacing: .08em; text-transform: uppercase; }
    .timeline-inspector-chip { width: fit-content; border-radius: 999px; padding: 3px 7px; background: var(--theme-warning-bg, #f1ebe1); color: var(--theme-neutral-text-soft, #6f6559) !important; font-size: 9px !important; }
    .timeline-inspector-divider { width: 32px; height: 1px; margin: 2px 0; border: 0; background: var(--line-strong); }
    .timeline-map-button { width: fit-content; padding: 5px 9px; border: 1px solid var(--line-strong); border-radius: 7px; background: var(--surface); color: var(--ink-muted); font: 600 10px Inter, ui-sans-serif, system-ui, sans-serif; cursor: pointer; }
    .timeline-map-button:hover, .timeline-map-button:focus-visible { border-color: var(--accent); color: var(--theme-warning-text, #55351f); outline: none; }
    .timeline-empty, .timeline-undated, .timeline-error { margin: 0; padding: 28px 18px; color: var(--ink-muted); font: 12px/1.5 Inter, ui-sans-serif, system-ui, sans-serif; }
    .timeline-undated { padding: 12px 15px; border-top: 1px solid var(--line-soft); background: var(--surface); }
    .timeline-undated-list { display: flex; flex-wrap: wrap; gap: 7px; margin-top: 8px; }
    .timeline-undated-list button { border: 1px solid var(--line-strong); border-radius: 999px; padding: 5px 8px; background: var(--surface); color: var(--ink-muted); font: 600 10px Inter, ui-sans-serif, system-ui, sans-serif; cursor: pointer; }
    .timeline-undated-list button:hover, .timeline-undated-list button:focus-visible { border-color: var(--accent); color: var(--theme-warning-text, #55351f); outline: none; }
    .timeline-error { color: var(--theme-danger-text, #9a4d3f); }
    @media (max-width: 1100px) {
      .timeline-workspace { grid-template-columns: var(--timeline-outline-width, 270px) 8px minmax(340px, 1fr); }
      .timeline-inspector { grid-column: 1 / -1; max-height: none; border-top: 1px solid var(--line-soft); border-left: 0; }
      .timeline-details { grid-template-columns: minmax(160px, .7fr) minmax(220px, 1.3fr); }
    }
    @media (max-width: 760px) {
      .timeline-toolbar { align-items: stretch; flex-direction: column; }
      .timeline-search { width: min(100%, 320px); }
      .timeline-layerbar { align-items: flex-start; flex-direction: column; }
      .timeline-workspace { grid-template-columns: 1fr !important; }
      .timeline-outline { max-height: 260px; border-right: 0; border-bottom: 1px solid var(--line-soft); }
      .timeline-outline-resize { display: none; }
      .timeline-canvas { height: 420px; min-height: 320px; }
      .timeline-inspector { grid-column: auto; }
      .timeline-details { grid-template-columns: 1fr; }
    }
  `, n;
}
function zs(n, e = n) {
  const t = document.createElement("button");
  return t.type = "button", t.textContent = n, t.setAttribute("aria-label", e), t.title = e, t;
}
function An(n, e) {
  const t = document.createElement("label");
  t.className = "timeline-filter-field";
  const r = document.createElement("span");
  return r.textContent = n, t.append(r, e), t;
}
function po(n, e, t, r) {
  const s = document.createElement("button");
  s.type = "button", s.className = "timeline-layer-chip", s.setAttribute("aria-pressed", String(t));
  const a = document.createElement("span");
  a.textContent = n;
  const o = document.createElement("span");
  return o.className = "timeline-layer-count", o.textContent = String(e), s.append(a, o), s.onclick = r, s;
}
function XI() {
  const n = document.createElement("div");
  n.className = "timeline-legend", n.setAttribute("aria-label", "Timeline legend");
  for (const [e, t] of [
    ["timeline-legend-dot", "Known date"],
    ["timeline-legend-range", "Date span"],
    ["timeline-legend-approx", "Year or month precision"]
  ]) {
    const r = document.createElement("span");
    r.className = "timeline-legend-item";
    const s = document.createElement("span");
    s.className = e, r.append(s, document.createTextNode(t)), n.append(r);
  }
  return n;
}
async function ZI(n, e) {
  try {
    const t = await n.focusEntity(e);
    if (t.status === "multiple-links" && t.locations.length > 0) {
      const r = t.locations[0];
      await n.openMap({ mapEntityId: r.mapEntityId, linkId: r.id });
    }
  } catch (t) {
    const r = t instanceof Error ? t.message : String(t);
    if (r.includes("not-on-map") || r.includes("link-unresolved") || r.includes("map-unavailable") || r.includes("forbidden") || r.includes("denied"))
      return;
    console.error("show on map failed", t);
  }
}
async function hD(n, e, t, r) {
  if (e.mapsNavigationEnabled())
    try {
      if ((await e.listLocations(t)).length === 0 || n.dataset.selectionKey !== r) return;
      const a = document.createElement("button");
      a.type = "button", a.className = "timeline-map-button", a.textContent = "Show on map", a.onclick = () => void ZI(e, t), n.append(a);
    } catch {
    }
}
function zv(n, e, t, r, s, a) {
  const o = e.id;
  n.dataset.selectionKey = o, n.replaceChildren();
  const l = document.createElement("small");
  l.className = "timeline-inspector-kicker", l.textContent = cD(e);
  const c = document.createElement("strong");
  c.textContent = e.entity.name;
  const d = document.createElement("small");
  d.className = "timeline-inspector-chip", d.textContent = Qp(e.entity.type, a);
  const f = document.createElement("hr");
  f.className = "timeline-inspector-divider";
  const p = document.createElement("small");
  p.textContent = rm(e, r);
  const m = document.createElement("small");
  m.textContent = `Displayed on the ${s} calendar`, n.append(l, c, d, f, p, m);
  const g = dD(e);
  if (g) {
    const b = document.createElement("small");
    b.textContent = g, n.append(b);
  }
  hD(n, t, e.entity.id, o);
}
function QI(n, e, t) {
  const r = e.entity.id;
  n.dataset.selectionKey = r, n.replaceChildren();
  const s = document.createElement("small");
  s.className = "timeline-inspector-kicker", s.textContent = e.relativeYear === void 0 ? "Unplaced item" : "Relative chronology";
  const a = document.createElement("strong");
  a.textContent = e.entity.name;
  const o = document.createElement("small");
  o.textContent = e.relativeYear === void 0 ? "No date yet — add a start or end date to place this event in the chronology." : `Relative chronology: ${uD(e.relativeYear)}. It is kept relative rather than converted to a Gregorian date.`, n.append(s, a, o);
  const l = dD(e);
  if (l) {
    const c = document.createElement("small");
    c.textContent = l, n.append(c);
  }
  hD(n, t, e.entity.id, r);
}
function JI(n, e, t, r, s, a, o) {
  const l = document.createElement("span");
  l.className = "timeline-outline-heading", l.textContent = "Chronological outline", n.append(l);
  let c = null;
  for (const d of e) {
    const f = jn(d, t);
    if (f !== c) {
      c = f;
      const T = document.createElement("div");
      T.className = "timeline-outline-year", T.textContent = String(f), n.append(T);
    }
    const p = document.createElement("button");
    p.type = "button", p.className = `timeline-event-card${d.id === s ? " is-selected" : ""}`, p.dataset.eventId = d.id;
    const m = document.createElement("strong");
    m.textContent = d.entity.name;
    const g = document.createElement("small");
    g.textContent = rm(d, r);
    const b = document.createElement("span");
    b.className = "timeline-card-meta";
    const _ = document.createElement("span");
    _.className = "timeline-card-kind", _.textContent = cD(d);
    const x = document.createElement("small");
    x.textContent = Qp(d.entity.type, o), b.append(_, x), p.append(m, g, b), p.onclick = () => a(d), n.append(p);
  }
}
function eM(n, e) {
  const t = [n.startValue, n.endValue].filter((a) => a !== void 0).some((a) => {
    var o;
    return ["year", "month"].includes(((o = wr(a)) == null ? void 0 : o.precision) ?? "day");
  }), r = n.pointRole && n.pointRole !== "point" && n.startLabel ? `${n.entity.name} · ${n.startLabel}` : n.entity.name, s = {
    id: n.id,
    content: r,
    group: bp(n),
    start: n.start,
    title: `${n.entity.name}
${rm(n, e)}`,
    className: [
      `timeline-${bp(n)}`,
      n.layer === "lifelines" ? "timeline-lifeline" : "",
      t ? "timeline-imprecise" : ""
    ].filter(Boolean).join(" "),
    style: `background-color:${n.colors.fill};border-color:${n.colors.border};color:${n.colors.text};`
  };
  return n.end && n.end.getTime() !== n.start.getTime() ? (s.end = n.end, s.type = "range") : n.pointRole ? s.type = "point" : s.type = "box", s;
}
function tM(n) {
  const e = /* @__PURE__ */ new Map();
  for (const t of n) {
    const r = bp(t);
    e.set(r, (e.get(r) ?? 0) + 1);
  }
  return [
    { id: "events", label: "Events" },
    { id: "lifelines", label: "Lifelines" },
    { id: "dates", label: "Project dates" }
  ].filter(({ id: t }) => e.has(t)).map(({ id: t, label: r }) => ({
    id: t,
    content: `${r} (${e.get(t) ?? 0})`,
    className: `timeline-group-${t}`
  }));
}
function rM(n, e) {
  const t = document.createElement("p");
  t.className = "timeline-error", t.textContent = e instanceof Error ? e.message : String(e), n.append(t);
}
function iM(n, e) {
  let t = !1, r = null, s = null, a = null, o = 300, l = !1, c = !0, d = !1, f = !0, p = "", m = "", g = null, b = "gregorian", _ = "", x = null, T = null, S = null;
  const k = KI(), D = async () => {
    var I;
    try {
      s == null || s.disconnect(), s = null, S == null || S(), S = null, r == null || r.destroy(), r = null;
      let C = x;
      if (!C) {
        const [L, ue] = await Promise.all([
          e.listEntities(),
          e.listEnabledManifests()
        ]), fe = await VI(e.listCalendarDefinition, L), Te = new Set(
          e.module.schemas.flatMap((Ie) => Ie.entityTypes.map((at) => at.id))
        ), Qe = qI(ue, e.module.id), ft = [...new Set(Qe.map((Ie) => Ie.namespace))], Rt = await Promise.all(
          L.map(async (Ie) => {
            var ge;
            const at = Te.has(Ie.type ?? ""), er = at ? await e.listOwnFields(Ie.id) : {}, Bt = (await Promise.all(
              ft.map(async (ee) => {
                try {
                  return await e.listSharedFields(Ie.id, ee);
                } catch {
                  return [];
                }
              })
            )).flat();
            let Ee = [];
            try {
              at && (Ee = await e.listSharedFields(Ie.id, "maps"));
            } catch {
            }
            const De = YI(
              (ge = Ee.find((ee) => ee.key === "physicalChronology")) == null ? void 0 : ge.value
            ), ne = at ? await e.listRelationships(Ie.id) : [], ce = ne.filter(
              (ee) => ee.type === "occurred_at" || ee.type === "involves"
            ), de = await Promise.all(
              ce.map((ee) => e.getEntity(ee.targetId))
            );
            return {
              entity: Ie,
              fields: er,
              locationName: ce.map(
                (ee, Je) => {
                  var wt;
                  return ee.type === "occurred_at" ? (wt = de[Je]) == null ? void 0 : wt.name : void 0;
                }
              ).find((ee) => !!ee),
              participantNames: ce.map(
                (ee, Je) => {
                  var wt;
                  return ee.type === "involves" ? (wt = de[Je]) == null ? void 0 : wt.name : void 0;
                }
              ).filter((ee) => !!ee),
              eraIds: ne.filter((ee) => ee.type === "during").map((ee) => ee.targetId),
              calendarIds: ne.filter((ee) => ee.type === "uses_calendar").map((ee) => ee.targetId),
              relativeYear: De,
              contributions: LI(Ie, Bt, Qe)
            };
          })
        );
        C = {
          calendarOptions: fe,
          loaded: Rt,
          typeLabels: xI([e.module, ...ue])
        }, x = C;
      }
      const { calendarOptions: P, loaded: R, typeLabels: M } = C, q = (L) => Qp(L, M);
      P.some((L) => L.id === b) || (b = "gregorian");
      const F = P.find((L) => L.id === b) ?? P[0], H = (F == null ? void 0 : F.definition) ?? null, Z = new Map(
        P.filter(
          (L) => L.definition !== null
        ).map((L) => [L.id, L.definition])
      ), ve = new Set(
        e.module.schemas.flatMap((L) => L.entityTypes.map((ue) => ue.id))
      ), Me = [], Y = [], J = [], z = [];
      for (const L of R) {
        if (L.entity.type !== "daena.timeline:calendar" && (ve.has(L.entity.type ?? "") || L.relativeYear !== null)) {
          const ue = L.fields.startsAt ?? L.fields.endsAt, fe = Ji(ue);
          if (!fe)
            L.entity.type !== "daena.timeline:era" && J.push({ ...L, relativeYear: L.relativeYear ?? void 0 });
          else {
            const Te = L.fields.endsAt, Qe = Te ? Ji(Te) : null, ft = {
              id: L.entity.id,
              entity: L.entity,
              startValue: ue,
              endValue: Te,
              layer: "timeline",
              locationName: L.locationName,
              participantNames: L.participantNames,
              eraIds: L.eraIds,
              start: fe.date,
              end: Qe && Qe.date.getTime() >= fe.date.getTime() ? Qe.date : null,
              colors: Hv("timeline", L.entity.id)
            };
            L.entity.type === "daena.timeline:era" ? z.push(ft) : Me.push(ft);
          }
        }
        for (const ue of L.contributions) {
          const fe = Ji(ue.startValue);
          if (!fe) continue;
          const Te = ue.endValue ? Ji(ue.endValue) : null;
          Y.push({
            id: `contribution:${ue.id}`,
            entity: ue.entity,
            startValue: ue.startValue,
            endValue: ue.endValue,
            startLabel: ue.startLabel,
            endLabel: ue.endLabel,
            pointRole: ue.pointRole,
            layer: ue.layer,
            participantNames: [],
            eraIds: L.eraIds,
            start: fe.date,
            end: Te && Te.date.getTime() >= fe.date.getTime() ? Te.date : null,
            colors: Hv(ue.layer, ue.entity.id)
          });
        }
      }
      const te = R.filter((L) => L.entity.type === "daena.timeline:era").map((L) => ({
        id: L.entity.id,
        name: L.entity.name,
        start: L.fields.startsAt,
        end: L.fields.endsAt,
        calendarIds: L.calendarIds
      }));
      _ && !te.some((L) => L.id === _) && (_ = "");
      const K = te.find((L) => L.id === _), re = Object.fromEntries(Z.entries()), ae = (L) => !K || jI({
        eraIds: L.eraIds,
        startValue: L.startValue,
        endValue: L.endValue,
        eraId: K.id,
        eraStart: K.start,
        eraEnd: K.end,
        calendarDefinition: FI(
          L.startValue ?? L.endValue,
          K.calendarIds,
          re
        )
      }), be = Me.filter(ae), se = Y.filter(ae), j = K ? J.filter((L) => L.eraIds.includes(K.id)) : J, xe = K ? z.filter((L) => L.entity.id === K.id) : z, lt = [...be, ...se], st = {
        events: be.length + xe.length,
        lifelines: se.filter((L) => L.layer === "lifelines").length,
        dates: se.filter((L) => L.layer === "dates").length
      }, ht = [
        ...new Set(
          [...lt, ...xe].map((L) => L.entity.type).filter((L) => !!L)
        )
      ].sort((L, ue) => q(L).localeCompare(q(ue)));
      m && !ht.includes(m) && (m = "");
      const Er = p.trim().toLocaleLowerCase(), Lt = [
        ...c ? be : [],
        ...se.filter(
          (L) => L.layer === "dates" && d || L.layer === "lifelines" && f
        )
      ].filter((L) => !m || L.entity.type === m).filter((L) => Gv(L, Er)).sort((L, ue) => L.start.getTime() - ue.start.getTime() || L.id.localeCompare(ue.id)), Qt = (c ? xe : []).filter((L) => !m || L.entity.type === m).filter((L) => Gv(L, Er)), Ft = [
        .../* @__PURE__ */ new Set([
          ...Lt.map((L) => jn(L, H)),
          ...Qt.map((L) => jn(L, H))
        ])
      ];
      a !== null && !Ft.includes(a) && (a = null);
      const pt = a === null ? Lt : Lt.filter((L) => jn(L, H) === a), _t = a === null ? Qt : Qt.filter((L) => jn(L, H) === a);
      if (g && !pt.some((L) => L.id === g) && !_t.some((L) => L.id === g) && (g = null), t) return;
      n.replaceChildren(), n.className = "timeline-projection";
      const Ct = document.createElement("div");
      Ct.className = "timeline-shell";
      const $t = (F == null ? void 0 : F.name) ?? "Gregorian";
      (I = e.reportSurfaceMeta) == null || I.call(e, {
        subtitle: j.length > 0 ? `${Lt.length} placed · ${j.length} unplaced or relative · ${$t}` : `${Lt.length} items · ${$t}`
      }), Ct.append(k);
      let Jt = null;
      if (lt.length === 0 && xe.length === 0 && j.length === 0) {
        const L = document.createElement("p");
        L.className = "timeline-empty", L.textContent = "No timeline items yet.", Ct.append(L), n.append(Ct);
        return;
      }
      if (lt.length === 0 && xe.length === 0) {
        const L = document.createElement("p");
        L.className = "timeline-empty", L.textContent = se.length > 0 ? "No items are visible with the current date layers." : "No dated timeline items to plot yet.";
        const ue = document.createElement("div");
        Jt = ue, ue.className = "timeline-details";
        const fe = document.createElement("small");
        fe.textContent = "Choose an unplaced item below to inspect it.", ue.append(fe), Ct.append(L, ue);
      } else {
        const L = document.createElement("div");
        L.className = "timeline-toolbar";
        const ue = document.createElement("div");
        ue.className = "timeline-toolbar-controls";
        const fe = document.createElement("input");
        fe.type = "search", fe.className = "timeline-search", fe.value = p, fe.placeholder = "Name, type, place…", fe.setAttribute("aria-label", "Search timeline"), fe.oninput = () => {
          p = fe.value;
          const G = fe.selectionStart ?? fe.value.length;
          T !== null && window.clearTimeout(T), T = window.setTimeout(() => {
            T = null, D().then(() => {
              const V = n.querySelector(".timeline-search");
              V == null || V.focus(), V == null || V.setSelectionRange(G, G);
            });
          }, 160);
        };
        const Te = document.createElement("select");
        Te.className = "timeline-scope", Te.setAttribute("aria-label", "Chronology scope");
        const Qe = document.createElement("option");
        Qe.value = "", Qe.textContent = "All dates", Te.append(Qe);
        for (const G of Ft) {
          const V = document.createElement("option");
          V.value = String(G), V.textContent = String(G), G === a && (V.selected = !0), Te.append(V);
        }
        Te.onchange = () => {
          a = Te.value ? Number(Te.value) : null, g = null, D();
        };
        const ft = document.createElement("select");
        ft.className = "timeline-calendar", ft.setAttribute("aria-label", "Displayed calendar");
        for (const G of P) {
          const V = document.createElement("option");
          V.value = G.id, V.textContent = G.name, V.selected = G.id === b, ft.append(V);
        }
        ft.onchange = () => {
          b = ft.value, a = null, g = null, D();
        };
        const Rt = document.createElement("select");
        Rt.className = "timeline-type-filter";
        const Ie = document.createElement("option");
        Ie.value = "", Ie.textContent = "All entity types", Rt.append(Ie);
        for (const G of ht) {
          const V = document.createElement("option");
          V.value = G, V.textContent = q(G), V.selected = G === m, Rt.append(V);
        }
        Rt.onchange = () => {
          m = Rt.value, a = null, g = null, D();
        };
        const at = document.createElement("select");
        at.className = "timeline-era-filter", at.setAttribute("aria-label", "Era");
        const er = document.createElement("option");
        er.value = "", er.textContent = "All history", at.append(er);
        for (const G of te) {
          const V = document.createElement("option");
          V.value = G.id, V.textContent = G.name, V.selected = G.id === _, at.append(V);
        }
        at.onchange = () => {
          _ = at.value;
          const G = te.find((_e) => _e.id === _), V = G == null ? void 0 : G.calendarIds[0];
          V && P.some((_e) => _e.id === V) && (b = V), a = null, g = null, D();
        }, ue.append(
          An("Search", fe),
          An("Era", at),
          An("Calendar", ft),
          An("Entity type", Rt),
          An("Year", Te)
        );
        const Bt = document.createElement("div");
        Bt.className = "timeline-toolbar-actions";
        const Ee = zs("+", "Zoom in"), De = zs("−", "Zoom out"), ne = zs("Fit", "Fit timeline to visible items"), ce = zs(l ? "Show outline" : "Hide outline");
        ce.onclick = () => {
          l = !l, D();
        }, Bt.append(De, Ee, ne, ce), L.append(ue, Bt);
        const de = document.createElement("div");
        de.className = "timeline-layerbar";
        const ge = document.createElement("div");
        ge.className = "timeline-layer-chips", ge.append(
          po("Events", st.events, c, () => {
            c = !c, a = null, g = null, D();
          }),
          po("Lifelines", st.lifelines, f, () => {
            f = !f, a = null, g = null, D();
          }),
          po("Project dates", st.dates, d, () => {
            d = !d, a = null, g = null, D();
          })
        ), de.append(ge, XI());
        const ee = document.createElement("div");
        ee.className = "timeline-workspace", l ? ee.style.gridTemplateColumns = "minmax(360px, 1fr) minmax(220px, 270px)" : ee.style.setProperty("--timeline-outline-width", `${o}px`);
        const Je = document.createElement("aside");
        Je.className = "timeline-outline";
        const wt = document.createElement("div");
        wt.className = "timeline-canvas";
        const jt = document.createElement("div");
        Jt = jt, jt.className = "timeline-details timeline-inspector";
        const Fi = document.createElement("small");
        Fi.textContent = "Select an item to inspect its dates, type, and related context.", jt.append(Fi);
        const lr = (G) => {
          g = G.id;
          for (const V of Je.querySelectorAll(".timeline-event-card"))
            V.classList.toggle("is-selected", V.dataset.eventId === G.id);
          if (G.entity.type === "daena.timeline:era") {
            if (_ !== G.entity.id) {
              _ = G.entity.id;
              const V = te.find((oe) => oe.id === G.entity.id), _e = V == null ? void 0 : V.calendarIds[0];
              _e && P.some((oe) => oe.id === _e) && (b = _e), D();
              return;
            }
            r == null || r.setSelection([]), r == null || r.moveTo(G.start, { animation: { duration: 220, easingFunction: "easeInOutQuad" } });
          } else
            r == null || r.setSelection([G.id], {
              focus: !0,
              animation: {}
            }), r == null || r.focus(G.id, { animation: { duration: 220, easingFunction: "easeInOutQuad" } });
          zv(
            jt,
            G,
            e,
            Z,
            (F == null ? void 0 : F.name) ?? "Gregorian",
            M
          );
        };
        if (pt.length > 0 || _t.length > 0) {
          if (_t.length > 0) {
            const G = document.createElement("span");
            G.className = "timeline-outline-heading", G.textContent = "Eras", Je.append(G);
            for (const V of _t) {
              const _e = document.createElement("button");
              _e.type = "button", _e.className = "timeline-event-card", _e.dataset.eventId = V.id, _e.classList.toggle("is-selected", V.id === g), _e.textContent = V.entity.name, _e.onclick = () => lr(V), Je.append(_e);
            }
          }
          pt.length > 0 && JI(
            Je,
            pt,
            H,
            Z,
            g,
            lr,
            M
          );
        } else {
          const G = document.createElement("p");
          G.className = "timeline-empty", G.textContent = "No items match the current filters.", Je.append(G);
        }
        if (l)
          ee.append(wt, jt);
        else {
          const G = document.createElement("div");
          G.className = "timeline-outline-resize", G.setAttribute("role", "separator"), G.setAttribute("aria-label", "Resize chronological outline"), G.tabIndex = 0, G.onpointerdown = (V) => {
            V.preventDefault();
            const _e = V.clientX, oe = o, Or = (ci) => {
              o = Math.max(210, Math.min(520, oe + ci.clientX - _e)), ee.style.setProperty("--timeline-outline-width", `${o}px`), r == null || r.redraw();
            }, Oe = () => {
              window.removeEventListener("pointermove", Or), window.removeEventListener("pointerup", Oe), S = null;
            };
            S = Oe, window.addEventListener("pointermove", Or), window.addEventListener("pointerup", Oe, { once: !0 });
          }, ee.append(Je, G, wt, jt);
        }
        const Dr = g ? pt.find((G) => G.id === g) ?? _t.find((G) => G.id === g) : void 0;
        Dr && zv(
          jt,
          Dr,
          e,
          Z,
          (F == null ? void 0 : F.name) ?? "Gregorian",
          M
        ), Ct.append(L, de, ee), n.append(Ct);
        const { Timeline: Dn } = await Promise.resolve().then(() => yG);
        if (t) return;
        const ui = new Map([...pt, ..._t].map((G) => [G.id, G])), W = {
          stack: !0,
          stackSubgroups: !0,
          selectable: !0,
          multiselect: !1,
          zoomable: !0,
          moveable: !0,
          showCurrentTime: !1,
          orientation: "top",
          margin: { item: { horizontal: 8, vertical: 6 }, axis: 12 },
          tooltip: { followMouse: !0 },
          format: {
            minorLabels: (G, V) => Bv(G, H, V),
            majorLabels: (G, V) => Bv(G, H, V)
          }
        };
        pt.length >= 40 && (W.cluster = { titleTemplate: "{count} events", showStipes: !0, maxItems: 8 });
        const Ba = tM(pt);
        r = new Dn(
          wt,
          [
            ..._t.map((G) => ({
              id: G.id,
              content: G.entity.name,
              start: G.start,
              end: ra(G),
              type: "background",
              title: G.entity.name
            })),
            ...pt.map((G) => eM(G, Z))
          ],
          Ba,
          W
        ), r.fit({ animation: !1 }), g && r.setSelection([g]), Ee.onclick = () => r == null ? void 0 : r.zoomIn(0.4), De.onclick = () => r == null ? void 0 : r.zoomOut(0.4), ne.onclick = () => r == null ? void 0 : r.fit({ animation: { duration: 320, easingFunction: "easeInOutQuad" } }), r.on("select", (G) => {
          const V = G.items[0], _e = V != null ? ui.get(String(V)) : void 0;
          if (!_e) {
            g = null;
            for (const oe of Je.querySelectorAll(".timeline-event-card"))
              oe.classList.remove("is-selected");
            jt.replaceChildren(Fi);
            return;
          }
          lr(_e);
        }), r.on("click", (G) => {
          const V = G.item != null ? String(G.item) : "", _e = V ? ui.get(V) : void 0;
          if (_e) {
            lr(_e);
            return;
          }
          const oe = G.time instanceof Date ? G.time : null, Or = oe ? GI(_t, oe) : void 0;
          if (Or) {
            lr(Or);
            return;
          }
          if (G.item == null) {
            g = null, r == null || r.setSelection([]);
            for (const Oe of Je.querySelectorAll(".timeline-event-card"))
              Oe.classList.remove("is-selected");
            jt.replaceChildren(Fi);
          }
        }), s = new ResizeObserver(() => r == null ? void 0 : r.redraw()), s.observe(wt);
      }
      if (j.length > 0) {
        const L = document.createElement("div");
        L.className = "timeline-undated";
        const ue = document.createElement("small");
        ue.textContent = K ? "Unplaced in this era" : j.some((Te) => Te.relativeYear !== void 0) ? "Unplaced or relative items" : "Unplaced items";
        const fe = document.createElement("div");
        fe.className = "timeline-undated-list";
        for (const Te of j) {
          const Qe = document.createElement("button");
          Qe.type = "button", Qe.textContent = Te.relativeYear === void 0 ? Te.entity.name : `${Te.entity.name} · ${uD(Te.relativeYear)}`, Qe.onclick = () => {
            Jt && QI(Jt, Te, e);
          }, fe.append(Qe);
        }
        L.append(ue, fe), Ct.append(L);
      }
      n.contains(Ct) || n.append(Ct);
    } catch (C) {
      if (t) return;
      console.error("timeline projection failed", C), n.replaceChildren(), n.className = "timeline-projection";
      const P = document.createElement("div");
      P.className = "timeline-shell", P.append(k), rM(P, C), n.append(P);
    }
  };
  return D(), () => {
    t = !0, T !== null && window.clearTimeout(T), T = null, s == null || s.disconnect(), s = null, S == null || S(), S = null, r == null || r.destroy(), r = null, n.replaceChildren();
  };
}
const nM = dI, aa = document.querySelector("#chronology"), oa = document.querySelector("#chronology-status");
async function sM() {
  if (!aa) throw new Error("Chronology root is missing");
  const n = _I(yI()), e = await n.bootstrap();
  Xp(e.appearance, document.documentElement);
  const t = await AI(n, nM);
  t.reportSurfaceMeta = (r) => {
    oa && (oa.textContent = r.subtitle);
  }, iM(aa, t);
}
sM().catch((n) => {
  const e = n instanceof Error ? n.message : String(n);
  oa ? oa.textContent = e : aa && (aa.textContent = e);
});
/**
 * vis-timeline and vis-graph2d
 * https://visjs.github.io/vis-timeline/
 *
 * Create a fully customizable, interactive timeline with items and ranges.
 *
 * @version 8.5.3
 * @date    2026-08-05T15:55:32.472Z
 *
 * @copyright (c) 2011-2017 Almende B.V, http://almende.com
 * @copyright (c) 2017-2019 visjs contributors, https://github.com/visjs
 *
 * @license
 * vis.js is dual licensed under both
 *
 *   1. The Apache 2.0 License
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *   and
 *
 *   2. The MIT License
 *      http://opensource.org/licenses/MIT
 *
 * vis.js may be distributed under either license.
 */
var Uv = typeof globalThis < "u" ? globalThis : typeof window < "u" ? window : typeof global < "u" ? global : typeof self < "u" ? self : {};
function Q(n) {
  return n && n.__esModule && Object.prototype.hasOwnProperty.call(n, "default") ? n.default : n;
}
function fr(n) {
  throw new Error('Could not dynamically require "' + n + '". Please configure the dynamicRequireTargets or/and ignoreDynamicRequires option of @rollup/plugin-commonjs appropriately for this require call to work.');
}
var wp = { exports: {} }, ia = { exports: {} }, aM = ia.exports, Vv;
function pr() {
  return Vv || (Vv = 1, (function(n, e) {
    (function(t, r) {
      n.exports = r();
    })(aM, (function() {
      var t;
      function r() {
        return t.apply(null, arguments);
      }
      function s(u) {
        t = u;
      }
      function a(u) {
        return u instanceof Array || Object.prototype.toString.call(u) === "[object Array]";
      }
      function o(u) {
        return u != null && Object.prototype.toString.call(u) === "[object Object]";
      }
      function l(u, h) {
        return Object.prototype.hasOwnProperty.call(u, h);
      }
      function c(u) {
        if (Object.getOwnPropertyNames)
          return Object.getOwnPropertyNames(u).length === 0;
        var h;
        for (h in u)
          if (l(u, h))
            return !1;
        return !0;
      }
      function d(u) {
        return u === void 0;
      }
      function f(u) {
        return typeof u == "number" || Object.prototype.toString.call(u) === "[object Number]";
      }
      function p(u) {
        return u instanceof Date || Object.prototype.toString.call(u) === "[object Date]";
      }
      function m(u, h) {
        var v = [], y, w = u.length;
        for (y = 0; y < w; ++y)
          v.push(h(u[y], y));
        return v;
      }
      function g(u, h) {
        for (var v in h)
          l(h, v) && (u[v] = h[v]);
        return l(h, "toString") && (u.toString = h.toString), l(h, "valueOf") && (u.valueOf = h.valueOf), u;
      }
      function b(u, h, v, y) {
        return dv(u, h, v, y, !0).utc();
      }
      function _() {
        return {
          empty: !1,
          unusedTokens: [],
          unusedInput: [],
          overflow: -2,
          charsLeftOver: 0,
          nullInput: !1,
          invalidEra: null,
          invalidMonth: null,
          invalidFormat: !1,
          userInvalidated: !1,
          iso: !1,
          parsedDateParts: [],
          era: null,
          meridiem: null,
          rfc2822: !1,
          weekdayMismatch: !1
        };
      }
      function x(u) {
        return u._pf == null && (u._pf = _()), u._pf;
      }
      var T;
      Array.prototype.some ? T = Array.prototype.some : T = function(u) {
        var h = Object(this), v = h.length >>> 0, y;
        for (y = 0; y < v; y++)
          if (y in h && u.call(this, h[y], y, h))
            return !0;
        return !1;
      };
      function S(u) {
        var h = null, v = !1, y = u._d && !isNaN(u._d.getTime());
        if (y && (h = x(u), v = T.call(h.parsedDateParts, function(w) {
          return w != null;
        }), y = h.overflow < 0 && !h.empty && !h.invalidEra && !h.invalidMonth && !h.invalidWeekday && !h.weekdayMismatch && !h.nullInput && !h.invalidFormat && !h.userInvalidated && (!h.meridiem || h.meridiem && v), u._strict && (y = y && h.charsLeftOver === 0 && h.unusedTokens.length === 0 && h.bigHour === void 0)), Object.isFrozen == null || !Object.isFrozen(u))
          u._isValid = y;
        else
          return y;
        return u._isValid;
      }
      function k(u) {
        var h = b(NaN);
        return u != null ? g(x(h), u) : x(h).userInvalidated = !0, h;
      }
      var D = r.momentProperties = [], I = !1;
      function C(u, h) {
        var v, y, w, O = D.length;
        if (d(h._isAMomentObject) || (u._isAMomentObject = h._isAMomentObject), d(h._i) || (u._i = h._i), d(h._f) || (u._f = h._f), d(h._l) || (u._l = h._l), d(h._strict) || (u._strict = h._strict), d(h._tzm) || (u._tzm = h._tzm), d(h._isUTC) || (u._isUTC = h._isUTC), d(h._offset) || (u._offset = h._offset), d(h._pf) || (u._pf = x(h)), d(h._locale) || (u._locale = h._locale), O > 0)
          for (v = 0; v < O; v++)
            y = D[v], w = h[y], d(w) || (u[y] = w);
        return u;
      }
      function P(u) {
        C(this, u), this._d = new Date(u._d != null ? u._d.getTime() : NaN), this.isValid() || (this._d = /* @__PURE__ */ new Date(NaN)), I === !1 && (I = !0, r.updateOffset(this), I = !1);
      }
      function R(u) {
        return u instanceof P || u != null && u._isAMomentObject != null;
      }
      function M(u) {
        r.suppressDeprecationWarnings === !1 && typeof console < "u" && console.warn && console.warn("Deprecation warning: " + u);
      }
      function q(u, h) {
        var v = !0;
        return g(function() {
          if (r.deprecationHandler != null && r.deprecationHandler(null, u), v) {
            var y = [], w, O, A, U = arguments.length;
            for (O = 0; O < U; O++) {
              if (w = "", typeof arguments[O] == "object") {
                w += `
[` + O + "] ";
                for (A in arguments[0])
                  l(arguments[0], A) && (w += A + ": " + arguments[0][A] + ", ");
                w = w.slice(0, -2);
              } else
                w = arguments[O];
              y.push(w);
            }
            M(
              u + `
Arguments: ` + Array.prototype.slice.call(y).join("") + `
` + new Error().stack
            ), v = !1;
          }
          return h.apply(this, arguments);
        }, h);
      }
      var F = {};
      function H(u, h) {
        r.deprecationHandler != null && r.deprecationHandler(u, h), F[u] || (M(h), F[u] = !0);
      }
      r.suppressDeprecationWarnings = !1, r.deprecationHandler = null;
      function Z(u) {
        return typeof Function < "u" && u instanceof Function || Object.prototype.toString.call(u) === "[object Function]";
      }
      function ve(u) {
        var h, v;
        for (v in u)
          l(u, v) && (h = u[v], Z(h) ? this[v] = h : this["_" + v] = h);
        this._config = u, this._dayOfMonthOrdinalParseLenient = new RegExp(
          (this._dayOfMonthOrdinalParse.source || this._ordinalParse.source) + "|" + /\d{1,2}/.source
        );
      }
      function Me(u, h) {
        var v = g({}, u), y;
        for (y in h)
          l(h, y) && (o(u[y]) && o(h[y]) ? (v[y] = {}, g(v[y], u[y]), g(v[y], h[y])) : h[y] != null ? v[y] = h[y] : delete v[y]);
        for (y in u)
          l(u, y) && !l(h, y) && o(u[y]) && (v[y] = g({}, v[y]));
        return v;
      }
      function Y(u) {
        u != null && this.set(u);
      }
      var J;
      Object.keys ? J = Object.keys : J = function(u) {
        var h, v = [];
        for (h in u)
          l(u, h) && v.push(h);
        return v;
      };
      var z = {
        sameDay: "[Today at] LT",
        nextDay: "[Tomorrow at] LT",
        nextWeek: "dddd [at] LT",
        lastDay: "[Yesterday at] LT",
        lastWeek: "[Last] dddd [at] LT",
        sameElse: "L"
      };
      function te(u, h, v) {
        var y = this._calendar[u] || this._calendar.sameElse;
        return Z(y) ? y.call(h, v) : y;
      }
      function K(u, h, v) {
        var y = "" + Math.abs(u), w = h - y.length, O = u >= 0;
        return (O ? v ? "+" : "" : "-") + Math.pow(10, Math.max(0, w)).toString().substr(1) + y;
      }
      var re = /(\[[^\[]*\])|(\\)?([Hh]mm(ss)?|Mo|MM?M?M?|Do|DDDo|DD?D?D?|ddd?d?|do?|w[o|w]?|W[o|W]?|Qo?|N{1,5}|YYYYYY|YYYYY|YYYY|YY|y{2,4}|yo?|gg(ggg?)?|GG(GGG?)?|e|E|a|A|hh?|HH?|kk?|mm?|ss?|S{1,9}|x|X|zz?|ZZ?|.)/g, ae = /(\[[^\[]*\])|(\\)?(LTS|LT|LL?L?L?|l{1,4})/g, be = {}, se = {};
      function j(u, h, v, y) {
        var w = y;
        typeof y == "string" && (w = function() {
          return this[y]();
        }), u && (se[u] = w), h && (se[h[0]] = function() {
          return K(w.apply(this, arguments), h[1], h[2]);
        }), v && (se[v] = function() {
          return this.localeData().ordinal(
            w.apply(this, arguments),
            u
          );
        });
      }
      function xe(u) {
        return u.match(/\[[\s\S]/) ? u.replace(/^\[|\]$/g, "") : u.replace(/\\/g, "");
      }
      function lt(u) {
        var h = u.match(re), v, y;
        for (v = 0, y = h.length; v < y; v++)
          se[h[v]] ? h[v] = se[h[v]] : h[v] = xe(h[v]);
        return function(w) {
          var O = "", A;
          for (A = 0; A < y; A++)
            O += Z(h[A]) ? h[A].call(w, u) : h[A];
          return O;
        };
      }
      function st(u, h) {
        return u.isValid() ? (h = ht(h, u.localeData()), be[h] = be[h] || lt(h), be[h](u)) : u.localeData().invalidDate();
      }
      function ht(u, h) {
        var v = 5;
        function y(w) {
          return h.longDateFormat(w) || w;
        }
        for (ae.lastIndex = 0; v >= 0 && ae.test(u); )
          u = u.replace(
            ae,
            y
          ), ae.lastIndex = 0, v -= 1;
        return u;
      }
      var Er = {
        LTS: "h:mm:ss A",
        LT: "h:mm A",
        L: "MM/DD/YYYY",
        LL: "MMMM D, YYYY",
        LLL: "MMMM D, YYYY h:mm A",
        LLLL: "dddd, MMMM D, YYYY h:mm A"
      };
      function Lt(u) {
        var h = this._longDateFormat[u], v = this._longDateFormat[u.toUpperCase()];
        return h || !v ? h : (this._longDateFormat[u] = v.match(re).map(function(y) {
          return y === "MMMM" || y === "MM" || y === "DD" || y === "dddd" ? y.slice(1) : y;
        }).join(""), this._longDateFormat[u]);
      }
      var Qt = "Invalid date";
      function Ft() {
        return this._invalidDate;
      }
      var pt = "%d", _t = /\d{1,2}/;
      function Ct(u) {
        return this._ordinal.replace("%d", u);
      }
      var $t = {
        future: "in %s",
        past: "%s ago",
        s: "a few seconds",
        ss: "%d seconds",
        m: "a minute",
        mm: "%d minutes",
        h: "an hour",
        hh: "%d hours",
        d: "a day",
        dd: "%d days",
        w: "a week",
        ww: "%d weeks",
        M: "a month",
        MM: "%d months",
        y: "a year",
        yy: "%d years"
      };
      function Jt(u, h, v, y) {
        var w = this._relativeTime[v];
        return Z(w) ? w(u, h, v, y) : w.replace(/%d/i, u);
      }
      function L(u, h) {
        var v = this._relativeTime[u > 0 ? "future" : "past"];
        return Z(v) ? v(h) : v.replace(/%s/i, h);
      }
      var ue = {
        D: "date",
        dates: "date",
        date: "date",
        d: "day",
        days: "day",
        day: "day",
        e: "weekday",
        weekdays: "weekday",
        weekday: "weekday",
        E: "isoWeekday",
        isoweekdays: "isoWeekday",
        isoweekday: "isoWeekday",
        DDD: "dayOfYear",
        dayofyears: "dayOfYear",
        dayofyear: "dayOfYear",
        h: "hour",
        hours: "hour",
        hour: "hour",
        ms: "millisecond",
        milliseconds: "millisecond",
        millisecond: "millisecond",
        m: "minute",
        minutes: "minute",
        minute: "minute",
        M: "month",
        months: "month",
        month: "month",
        Q: "quarter",
        quarters: "quarter",
        quarter: "quarter",
        s: "second",
        seconds: "second",
        second: "second",
        gg: "weekYear",
        weekyears: "weekYear",
        weekyear: "weekYear",
        GG: "isoWeekYear",
        isoweekyears: "isoWeekYear",
        isoweekyear: "isoWeekYear",
        w: "week",
        weeks: "week",
        week: "week",
        W: "isoWeek",
        isoweeks: "isoWeek",
        isoweek: "isoWeek",
        y: "year",
        years: "year",
        year: "year"
      };
      function fe(u) {
        return typeof u == "string" ? ue[u] || ue[u.toLowerCase()] : void 0;
      }
      function Te(u) {
        var h = {}, v, y;
        for (y in u)
          l(u, y) && (v = fe(y), v && (h[v] = u[y]));
        return h;
      }
      var Qe = {
        date: 9,
        day: 11,
        weekday: 11,
        isoWeekday: 11,
        dayOfYear: 4,
        hour: 13,
        millisecond: 16,
        minute: 14,
        month: 8,
        quarter: 7,
        second: 15,
        weekYear: 1,
        isoWeekYear: 1,
        week: 5,
        isoWeek: 5,
        year: 1
      };
      function ft(u) {
        var h = [], v;
        for (v in u)
          l(u, v) && h.push({ unit: v, priority: Qe[v] });
        return h.sort(function(y, w) {
          return y.priority - w.priority;
        }), h;
      }
      var Rt = /\d/, Ie = /\d\d/, at = /\d{3}/, er = /\d{4}/, Bt = /[+-]?\d{6}/, Ee = /\d\d?/, De = /\d\d\d\d?/, ne = /\d\d\d\d\d\d?/, ce = /\d{1,3}/, de = /\d{1,4}/, ge = /[+-]?\d{1,6}/, ee = /\d+/, Je = /[+-]?\d+/, wt = /Z|[+-]\d\d:?\d\d/gi, jt = /Z|[+-]\d\d(?::?\d\d)?/gi, Fi = /[+-]?\d+(\.\d{1,3})?/, lr = /[0-9]{0,256}['a-z\u00A0-\u05FF\u0700-\uD7FF\uF900-\uFDCF\uFDF0-\uFF07\uFF10-\uFFEF]{1,256}|[\u0600-\u06FF\/]{1,256}(\s*?[\u0600-\u06FF]{1,256}){1,2}/i, Dr = /^[1-9]\d?/, Dn = /^([1-9]\d|\d)/, ui;
      ui = {};
      function W(u, h, v) {
        ui[u] = Z(h) ? h : function(y, w) {
          return y && v ? v : h;
        };
      }
      function Ba(u, h) {
        return l(ui, u) ? ui[u](h._strict, h._locale) : new RegExp(G(u));
      }
      function G(u) {
        return V(
          u.replace("\\", "").replace(
            /\\(\[)|\\(\])|\[([^\]\[]*)\]|\\(.)/g,
            function(h, v, y, w, O) {
              return v || y || w || O;
            }
          )
        );
      }
      function V(u) {
        return u.replace(/[-\/\\^$*+?.()|[\]{}]/g, "\\$&");
      }
      function _e(u) {
        return u < 0 ? Math.ceil(u) || 0 : Math.floor(u);
      }
      function oe(u) {
        var h = +u, v = 0;
        return h !== 0 && isFinite(h) && (v = _e(h)), v;
      }
      var Or = {};
      function Oe(u, h) {
        var v, y = h, w;
        for (typeof u == "string" && (u = [u]), f(h) && (y = function(O, A) {
          A[h] = oe(O);
        }), w = u.length, v = 0; v < w; v++)
          Or[u[v]] = y;
      }
      function ci(u, h) {
        Oe(u, function(v, y, w, O) {
          w._w = w._w || {}, h(v, w._w, w, O);
        });
      }
      function pk(u, h, v) {
        h != null && l(Or, u) && Or[u](h, v._a, v, u);
      }
      function Is(u) {
        return u % 4 === 0 && u % 100 !== 0 || u % 400 === 0;
      }
      var xt = 0, kr = 1, gr = 2, ut = 3, ur = 4, Cr = 5, di = 6, mk = 7, vk = 8;
      j("Y", 0, 0, function() {
        var u = this.year();
        return u <= 9999 ? K(u, 4) : "+" + u;
      }), j(0, ["YY", 2], 0, function() {
        return this.year() % 100;
      }), j(0, ["YYYY", 4], 0, "year"), j(0, ["YYYYY", 5], 0, "year"), j(0, ["YYYYYY", 6, !0], 0, "year"), W("Y", Je), W("YY", Ee, Ie), W("YYYY", de, er), W("YYYYY", ge, Bt), W("YYYYYY", ge, Bt), Oe(["YYYYY", "YYYYYY"], xt), Oe("YYYY", function(u, h) {
        h[xt] = u.length === 2 ? r.parseTwoDigitYear(u) : oe(u);
      }), Oe("YY", function(u, h) {
        h[xt] = r.parseTwoDigitYear(u);
      }), Oe("Y", function(u, h) {
        h[xt] = parseInt(u, 10);
      });
      function On(u) {
        return Is(u) ? 366 : 365;
      }
      r.parseTwoDigitYear = function(u) {
        return oe(u) + (oe(u) > 68 ? 1900 : 2e3);
      };
      var Km = $i("FullYear", !0);
      function gk() {
        return Is(this.year());
      }
      function $i(u, h) {
        return function(v) {
          return v != null ? (Xm(this, u, v), r.updateOffset(this, h), this) : kn(this, u);
        };
      }
      function kn(u, h) {
        if (!u.isValid())
          return NaN;
        var v = u._d, y = u._isUTC;
        switch (h) {
          case "Milliseconds":
            return y ? v.getUTCMilliseconds() : v.getMilliseconds();
          case "Seconds":
            return y ? v.getUTCSeconds() : v.getSeconds();
          case "Minutes":
            return y ? v.getUTCMinutes() : v.getMinutes();
          case "Hours":
            return y ? v.getUTCHours() : v.getHours();
          case "Date":
            return y ? v.getUTCDate() : v.getDate();
          case "Day":
            return y ? v.getUTCDay() : v.getDay();
          case "Month":
            return y ? v.getUTCMonth() : v.getMonth();
          case "FullYear":
            return y ? v.getUTCFullYear() : v.getFullYear();
          default:
            return NaN;
        }
      }
      function Xm(u, h, v) {
        var y, w, O, A, U;
        if (!(!u.isValid() || isNaN(v))) {
          switch (y = u._d, w = u._isUTC, h) {
            case "Milliseconds":
              return void (w ? y.setUTCMilliseconds(v) : y.setMilliseconds(v));
            case "Seconds":
              return void (w ? y.setUTCSeconds(v) : y.setSeconds(v));
            case "Minutes":
              return void (w ? y.setUTCMinutes(v) : y.setMinutes(v));
            case "Hours":
              return void (w ? y.setUTCHours(v) : y.setHours(v));
            case "Date":
              return void (w ? y.setUTCDate(v) : y.setDate(v));
            // case 'Day': // Not real
            //    return void (isUTC ? d.setUTCDay(value) : d.setDay(value));
            // case 'Month': // Not used because we need to pass two variables
            //     return void (isUTC ? d.setUTCMonth(value) : d.setMonth(value));
            case "FullYear":
              break;
            // See below ...
            default:
              return;
          }
          O = v, A = u.month(), U = u.date(), U = U === 29 && A === 1 && !Is(O) ? 28 : U, w ? y.setUTCFullYear(O, A, U) : y.setFullYear(O, A, U);
        }
      }
      function yk(u) {
        return u = fe(u), Z(this[u]) ? this[u]() : this;
      }
      function bk(u, h) {
        if (typeof u == "object") {
          u = Te(u);
          var v = ft(u), y, w = v.length;
          for (y = 0; y < w; y++)
            this[v[y].unit](u[v[y].unit]);
        } else if (u = fe(u), Z(this[u]))
          return this[u](h);
        return this;
      }
      function _k(u, h) {
        return (u % h + h) % h;
      }
      var et;
      Array.prototype.indexOf ? et = Array.prototype.indexOf : et = function(u) {
        var h;
        for (h = 0; h < this.length; ++h)
          if (this[h] === u)
            return h;
        return -1;
      };
      function Wa(u, h) {
        if (isNaN(u) || isNaN(h))
          return NaN;
        var v = _k(h, 12);
        return u += (h - v) / 12, v === 1 ? Is(u) ? 29 : 28 : 31 - v % 7 % 2;
      }
      j("M", ["MM", 2], "Mo", function() {
        return this.month() + 1;
      }), j("MMM", 0, 0, function(u) {
        return this.localeData().monthsShort(this, u);
      }), j("MMMM", 0, 0, function(u) {
        return this.localeData().months(this, u);
      }), W("M", Ee, Dr), W("MM", Ee, Ie), W("MMM", function(u, h) {
        return h.monthsShortRegex(u);
      }), W("MMMM", function(u, h) {
        return h.monthsRegex(u);
      }), Oe(["M", "MM"], function(u, h) {
        h[kr] = oe(u) - 1;
      }), Oe(["MMM", "MMMM"], function(u, h, v, y) {
        var w = v._locale.monthsParse(u, y, v._strict);
        w != null ? h[kr] = w : x(v).invalidMonth = u;
      });
      var wk = "January_February_March_April_May_June_July_August_September_October_November_December".split(
        "_"
      ), Zm = "Jan_Feb_Mar_Apr_May_Jun_Jul_Aug_Sep_Oct_Nov_Dec".split("_"), Qm = /D[oD]?(\[[^\[\]]*\]|\s)+MMMM?/, xk = lr, Sk = lr;
      function Tk(u, h) {
        return u ? a(this._months) ? this._months[u.month()] : this._months[(this._months.isFormat || Qm).test(h) ? "format" : "standalone"][u.month()] : a(this._months) ? this._months : this._months.standalone;
      }
      function Ek(u, h) {
        return u ? a(this._monthsShort) ? this._monthsShort[u.month()] : this._monthsShort[Qm.test(h) ? "format" : "standalone"][u.month()] : a(this._monthsShort) ? this._monthsShort : this._monthsShort.standalone;
      }
      function Dk(u, h, v) {
        var y, w, O, A = u.toLocaleLowerCase();
        if (!this._monthsParse)
          for (this._monthsParse = [], this._longMonthsParse = [], this._shortMonthsParse = [], y = 0; y < 12; ++y)
            O = b([2e3, y]), this._shortMonthsParse[y] = this.monthsShort(
              O,
              ""
            ).toLocaleLowerCase(), this._longMonthsParse[y] = this.months(O, "").toLocaleLowerCase();
        return v ? h === "MMM" ? (w = et.call(this._shortMonthsParse, A), w !== -1 ? w : null) : (w = et.call(this._longMonthsParse, A), w !== -1 ? w : null) : h === "MMM" ? (w = et.call(this._shortMonthsParse, A), w !== -1 ? w : (w = et.call(this._longMonthsParse, A), w !== -1 ? w : null)) : (w = et.call(this._longMonthsParse, A), w !== -1 ? w : (w = et.call(this._shortMonthsParse, A), w !== -1 ? w : null));
      }
      function Ok(u, h, v) {
        var y, w, O;
        if (this._monthsParseExact)
          return Dk.call(this, u, h, v);
        for (this._monthsParse || (this._monthsParse = [], this._longMonthsParse = [], this._shortMonthsParse = []), y = 0; y < 12; y++) {
          if (w = b([2e3, y]), v && !this._longMonthsParse[y] && (this._longMonthsParse[y] = new RegExp(
            "^" + this.months(w, "").replace(".", "") + "$",
            "i"
          ), this._shortMonthsParse[y] = new RegExp(
            "^" + this.monthsShort(w, "").replace(".", "") + "$",
            "i"
          )), !v && !this._monthsParse[y] && (O = "^" + this.months(w, "") + "|^" + this.monthsShort(w, ""), this._monthsParse[y] = new RegExp(O.replace(".", ""), "i")), v && h === "MMMM" && this._longMonthsParse[y].test(u))
            return y;
          if (v && h === "MMM" && this._shortMonthsParse[y].test(u))
            return y;
          if (!v && this._monthsParse[y].test(u))
            return y;
        }
      }
      function Jm(u, h) {
        if (!u.isValid())
          return u;
        if (typeof h == "string") {
          if (/^\d+$/.test(h))
            h = oe(h);
          else if (h = u.localeData().monthsParse(h), !f(h))
            return u;
        }
        var v = h, y = u.date();
        return y = y < 29 ? y : Math.min(y, Wa(u.year(), v)), u._isUTC ? u._d.setUTCMonth(v, y) : u._d.setMonth(v, y), u;
      }
      function ev(u) {
        return u != null ? (Jm(this, u), r.updateOffset(this, !0), this) : kn(this, "Month");
      }
      function kk() {
        return Wa(this.year(), this.month());
      }
      function Ck(u) {
        return this._monthsParseExact ? (l(this, "_monthsRegex") || tv.call(this), u ? this._monthsShortStrictRegex : this._monthsShortRegex) : (l(this, "_monthsShortRegex") || (this._monthsShortRegex = xk), this._monthsShortStrictRegex && u ? this._monthsShortStrictRegex : this._monthsShortRegex);
      }
      function Rk(u) {
        return this._monthsParseExact ? (l(this, "_monthsRegex") || tv.call(this), u ? this._monthsStrictRegex : this._monthsRegex) : (l(this, "_monthsRegex") || (this._monthsRegex = Sk), this._monthsStrictRegex && u ? this._monthsStrictRegex : this._monthsRegex);
      }
      function tv() {
        function u(le, Se) {
          return Se.length - le.length;
        }
        var h = [], v = [], y = [], w, O, A, U;
        for (w = 0; w < 12; w++)
          O = b([2e3, w]), A = V(this.monthsShort(O, "")), U = V(this.months(O, "")), h.push(A), v.push(U), y.push(U), y.push(A);
        h.sort(u), v.sort(u), y.sort(u), this._monthsRegex = new RegExp("^(" + y.join("|") + ")", "i"), this._monthsShortRegex = this._monthsRegex, this._monthsStrictRegex = new RegExp(
          "^(" + v.join("|") + ")",
          "i"
        ), this._monthsShortStrictRegex = new RegExp(
          "^(" + h.join("|") + ")",
          "i"
        );
      }
      function Pk(u, h, v, y, w, O, A) {
        var U;
        return u < 100 && u >= 0 ? (U = new Date(u + 400, h, v, y, w, O, A), isFinite(U.getFullYear()) && U.setFullYear(u)) : U = new Date(u, h, v, y, w, O, A), U;
      }
      function Cn(u) {
        var h, v;
        return u < 100 && u >= 0 ? (v = Array.prototype.slice.call(arguments), v[0] = u + 400, h = new Date(Date.UTC.apply(null, v)), isFinite(h.getUTCFullYear()) && h.setUTCFullYear(u)) : h = new Date(Date.UTC.apply(null, arguments)), h;
      }
      function Ms(u, h, v) {
        var y = 7 + h - v, w = (7 + Cn(u, 0, y).getUTCDay() - h) % 7;
        return -w + y - 1;
      }
      function rv(u, h, v, y, w) {
        var O = (7 + v - y) % 7, A = Ms(u, y, w), U = 1 + 7 * (h - 1) + O + A, le, Se;
        return U <= 0 ? (le = u - 1, Se = On(le) + U) : U > On(u) ? (le = u + 1, Se = U - On(u)) : (le = u, Se = U), {
          year: le,
          dayOfYear: Se
        };
      }
      function Rn(u, h, v) {
        var y = Ms(u.year(), h, v), w = Math.floor((u.dayOfYear() - y - 1) / 7) + 1, O, A;
        return w < 1 ? (A = u.year() - 1, O = w + Rr(A, h, v)) : w > Rr(u.year(), h, v) ? (O = w - Rr(u.year(), h, v), A = u.year() + 1) : (A = u.year(), O = w), {
          week: O,
          year: A
        };
      }
      function Rr(u, h, v) {
        var y = Ms(u, h, v), w = Ms(u + 1, h, v);
        return (On(u) - y + w) / 7;
      }
      j("w", ["ww", 2], "wo", "week"), j("W", ["WW", 2], "Wo", "isoWeek"), W("w", Ee, Dr), W("ww", Ee, Ie), W("W", Ee, Dr), W("WW", Ee, Ie), ci(
        ["w", "ww", "W", "WW"],
        function(u, h, v, y) {
          h[y.substr(0, 1)] = oe(u);
        }
      );
      function Ik(u) {
        return Rn(u, this._week.dow, this._week.doy).week;
      }
      var Mk = {
        dow: 0,
        // Sunday is the first day of the week.
        doy: 6
        // The week that contains Jan 6th is the first week of the year.
      };
      function Ak() {
        return this._week.dow;
      }
      function Nk() {
        return this._week.doy;
      }
      function qk(u) {
        var h = this.localeData().week(this);
        return u == null ? h : this.add((u - h) * 7, "d");
      }
      function Lk(u) {
        var h = Rn(this, 1, 4).week;
        return u == null ? h : this.add((u - h) * 7, "d");
      }
      j("d", 0, "do", "day"), j("dd", 0, 0, function(u) {
        return this.localeData().weekdaysMin(this, u);
      }), j("ddd", 0, 0, function(u) {
        return this.localeData().weekdaysShort(this, u);
      }), j("dddd", 0, 0, function(u) {
        return this.localeData().weekdays(this, u);
      }), j("e", 0, 0, "weekday"), j("E", 0, 0, "isoWeekday"), W("d", Ee), W("e", Ee), W("E", Ee), W("dd", function(u, h) {
        return h.weekdaysMinRegex(u);
      }), W("ddd", function(u, h) {
        return h.weekdaysShortRegex(u);
      }), W("dddd", function(u, h) {
        return h.weekdaysRegex(u);
      }), ci(["dd", "ddd", "dddd"], function(u, h, v, y) {
        var w = v._locale.weekdaysParse(u, y, v._strict);
        w != null ? h.d = w : x(v).invalidWeekday = u;
      }), ci(["d", "e", "E"], function(u, h, v, y) {
        h[y] = oe(u);
      });
      function Fk(u, h) {
        return typeof u != "string" ? u : isNaN(u) ? (u = h.weekdaysParse(u), typeof u == "number" ? u : null) : parseInt(u, 10);
      }
      function $k(u, h) {
        return typeof u == "string" ? h.weekdaysParse(u) % 7 || 7 : isNaN(u) ? null : u;
      }
      function za(u, h) {
        return u.slice(h, 7).concat(u.slice(0, h));
      }
      var jk = "Sunday_Monday_Tuesday_Wednesday_Thursday_Friday_Saturday".split("_"), iv = "Sun_Mon_Tue_Wed_Thu_Fri_Sat".split("_"), Yk = "Su_Mo_Tu_We_Th_Fr_Sa".split("_"), Hk = lr, Gk = lr, Bk = lr;
      function Wk(u, h) {
        var v = a(this._weekdays) ? this._weekdays : this._weekdays[u && u !== !0 && this._weekdays.isFormat.test(h) ? "format" : "standalone"];
        return u === !0 ? za(v, this._week.dow) : u ? v[u.day()] : v;
      }
      function zk(u) {
        return u === !0 ? za(this._weekdaysShort, this._week.dow) : u ? this._weekdaysShort[u.day()] : this._weekdaysShort;
      }
      function Uk(u) {
        return u === !0 ? za(this._weekdaysMin, this._week.dow) : u ? this._weekdaysMin[u.day()] : this._weekdaysMin;
      }
      function Vk(u, h, v) {
        var y, w, O, A = u.toLocaleLowerCase();
        if (!this._weekdaysParse)
          for (this._weekdaysParse = [], this._shortWeekdaysParse = [], this._minWeekdaysParse = [], y = 0; y < 7; ++y)
            O = b([2e3, 1]).day(y), this._minWeekdaysParse[y] = this.weekdaysMin(
              O,
              ""
            ).toLocaleLowerCase(), this._shortWeekdaysParse[y] = this.weekdaysShort(
              O,
              ""
            ).toLocaleLowerCase(), this._weekdaysParse[y] = this.weekdays(O, "").toLocaleLowerCase();
        return v ? h === "dddd" ? (w = et.call(this._weekdaysParse, A), w !== -1 ? w : null) : h === "ddd" ? (w = et.call(this._shortWeekdaysParse, A), w !== -1 ? w : null) : (w = et.call(this._minWeekdaysParse, A), w !== -1 ? w : null) : h === "dddd" ? (w = et.call(this._weekdaysParse, A), w !== -1 || (w = et.call(this._shortWeekdaysParse, A), w !== -1) ? w : (w = et.call(this._minWeekdaysParse, A), w !== -1 ? w : null)) : h === "ddd" ? (w = et.call(this._shortWeekdaysParse, A), w !== -1 || (w = et.call(this._weekdaysParse, A), w !== -1) ? w : (w = et.call(this._minWeekdaysParse, A), w !== -1 ? w : null)) : (w = et.call(this._minWeekdaysParse, A), w !== -1 || (w = et.call(this._weekdaysParse, A), w !== -1) ? w : (w = et.call(this._shortWeekdaysParse, A), w !== -1 ? w : null));
      }
      function Kk(u, h, v) {
        var y, w, O;
        if (this._weekdaysParseExact)
          return Vk.call(this, u, h, v);
        for (this._weekdaysParse || (this._weekdaysParse = [], this._minWeekdaysParse = [], this._shortWeekdaysParse = [], this._fullWeekdaysParse = []), y = 0; y < 7; y++) {
          if (w = b([2e3, 1]).day(y), v && !this._fullWeekdaysParse[y] && (this._fullWeekdaysParse[y] = new RegExp(
            "^" + this.weekdays(w, "").replace(".", "\\.?") + "$",
            "i"
          ), this._shortWeekdaysParse[y] = new RegExp(
            "^" + this.weekdaysShort(w, "").replace(".", "\\.?") + "$",
            "i"
          ), this._minWeekdaysParse[y] = new RegExp(
            "^" + this.weekdaysMin(w, "").replace(".", "\\.?") + "$",
            "i"
          )), this._weekdaysParse[y] || (O = "^" + this.weekdays(w, "") + "|^" + this.weekdaysShort(w, "") + "|^" + this.weekdaysMin(w, ""), this._weekdaysParse[y] = new RegExp(O.replace(".", ""), "i")), v && h === "dddd" && this._fullWeekdaysParse[y].test(u))
            return y;
          if (v && h === "ddd" && this._shortWeekdaysParse[y].test(u))
            return y;
          if (v && h === "dd" && this._minWeekdaysParse[y].test(u))
            return y;
          if (!v && this._weekdaysParse[y].test(u))
            return y;
        }
      }
      function Xk(u) {
        if (!this.isValid())
          return u != null ? this : NaN;
        var h = kn(this, "Day");
        return u != null ? (u = Fk(u, this.localeData()), this.add(u - h, "d")) : h;
      }
      function Zk(u) {
        if (!this.isValid())
          return u != null ? this : NaN;
        var h = (this.day() + 7 - this.localeData()._week.dow) % 7;
        return u == null ? h : this.add(u - h, "d");
      }
      function Qk(u) {
        if (!this.isValid())
          return u != null ? this : NaN;
        if (u != null) {
          var h = $k(u, this.localeData());
          return this.day(this.day() % 7 ? h : h - 7);
        } else
          return this.day() || 7;
      }
      function Jk(u) {
        return this._weekdaysParseExact ? (l(this, "_weekdaysRegex") || Ua.call(this), u ? this._weekdaysStrictRegex : this._weekdaysRegex) : (l(this, "_weekdaysRegex") || (this._weekdaysRegex = Hk), this._weekdaysStrictRegex && u ? this._weekdaysStrictRegex : this._weekdaysRegex);
      }
      function eC(u) {
        return this._weekdaysParseExact ? (l(this, "_weekdaysRegex") || Ua.call(this), u ? this._weekdaysShortStrictRegex : this._weekdaysShortRegex) : (l(this, "_weekdaysShortRegex") || (this._weekdaysShortRegex = Gk), this._weekdaysShortStrictRegex && u ? this._weekdaysShortStrictRegex : this._weekdaysShortRegex);
      }
      function tC(u) {
        return this._weekdaysParseExact ? (l(this, "_weekdaysRegex") || Ua.call(this), u ? this._weekdaysMinStrictRegex : this._weekdaysMinRegex) : (l(this, "_weekdaysMinRegex") || (this._weekdaysMinRegex = Bk), this._weekdaysMinStrictRegex && u ? this._weekdaysMinStrictRegex : this._weekdaysMinRegex);
      }
      function Ua() {
        function u(Pt, Nr) {
          return Nr.length - Pt.length;
        }
        var h = [], v = [], y = [], w = [], O, A, U, le, Se;
        for (O = 0; O < 7; O++)
          A = b([2e3, 1]).day(O), U = V(this.weekdaysMin(A, "")), le = V(this.weekdaysShort(A, "")), Se = V(this.weekdays(A, "")), h.push(U), v.push(le), y.push(Se), w.push(U), w.push(le), w.push(Se);
        h.sort(u), v.sort(u), y.sort(u), w.sort(u), this._weekdaysRegex = new RegExp("^(" + w.join("|") + ")", "i"), this._weekdaysShortRegex = this._weekdaysRegex, this._weekdaysMinRegex = this._weekdaysRegex, this._weekdaysStrictRegex = new RegExp(
          "^(" + y.join("|") + ")",
          "i"
        ), this._weekdaysShortStrictRegex = new RegExp(
          "^(" + v.join("|") + ")",
          "i"
        ), this._weekdaysMinStrictRegex = new RegExp(
          "^(" + h.join("|") + ")",
          "i"
        );
      }
      function Va() {
        return this.hours() % 12 || 12;
      }
      function rC() {
        return this.hours() || 24;
      }
      j("H", ["HH", 2], 0, "hour"), j("h", ["hh", 2], 0, Va), j("k", ["kk", 2], 0, rC), j("hmm", 0, 0, function() {
        return "" + Va.apply(this) + K(this.minutes(), 2);
      }), j("hmmss", 0, 0, function() {
        return "" + Va.apply(this) + K(this.minutes(), 2) + K(this.seconds(), 2);
      }), j("Hmm", 0, 0, function() {
        return "" + this.hours() + K(this.minutes(), 2);
      }), j("Hmmss", 0, 0, function() {
        return "" + this.hours() + K(this.minutes(), 2) + K(this.seconds(), 2);
      });
      function nv(u, h) {
        j(u, 0, 0, function() {
          return this.localeData().meridiem(
            this.hours(),
            this.minutes(),
            h
          );
        });
      }
      nv("a", !0), nv("A", !1);
      function sv(u, h) {
        return h._meridiemParse;
      }
      W("a", sv), W("A", sv), W("H", Ee, Dn), W("h", Ee, Dr), W("k", Ee, Dr), W("HH", Ee, Ie), W("hh", Ee, Ie), W("kk", Ee, Ie), W("hmm", De), W("hmmss", ne), W("Hmm", De), W("Hmmss", ne), Oe(["H", "HH"], ut), Oe(["k", "kk"], function(u, h, v) {
        var y = oe(u);
        h[ut] = y === 24 ? 0 : y;
      }), Oe(["a", "A"], function(u, h, v) {
        v._isPm = v._locale.isPM(u), v._meridiem = u;
      }), Oe(["h", "hh"], function(u, h, v) {
        h[ut] = oe(u), x(v).bigHour = !0;
      }), Oe("hmm", function(u, h, v) {
        var y = u.length - 2;
        h[ut] = oe(u.substr(0, y)), h[ur] = oe(u.substr(y)), x(v).bigHour = !0;
      }), Oe("hmmss", function(u, h, v) {
        var y = u.length - 4, w = u.length - 2;
        h[ut] = oe(u.substr(0, y)), h[ur] = oe(u.substr(y, 2)), h[Cr] = oe(u.substr(w)), x(v).bigHour = !0;
      }), Oe("Hmm", function(u, h, v) {
        var y = u.length - 2;
        h[ut] = oe(u.substr(0, y)), h[ur] = oe(u.substr(y));
      }), Oe("Hmmss", function(u, h, v) {
        var y = u.length - 4, w = u.length - 2;
        h[ut] = oe(u.substr(0, y)), h[ur] = oe(u.substr(y, 2)), h[Cr] = oe(u.substr(w));
      });
      function iC(u) {
        return (u + "").toLowerCase().charAt(0) === "p";
      }
      var nC = /[ap]\.?m?\.?/i, sC = $i("Hours", !0);
      function aC(u, h, v) {
        return u > 11 ? v ? "pm" : "PM" : v ? "am" : "AM";
      }
      var av = {
        calendar: z,
        longDateFormat: Er,
        invalidDate: Qt,
        ordinal: pt,
        dayOfMonthOrdinalParse: _t,
        relativeTime: $t,
        months: wk,
        monthsShort: Zm,
        week: Mk,
        weekdays: jk,
        weekdaysMin: Yk,
        weekdaysShort: iv,
        meridiemParse: nC
      }, Be = {}, Pn = {}, In;
      function oC(u, h) {
        var v, y = Math.min(u.length, h.length);
        for (v = 0; v < y; v += 1)
          if (u[v] !== h[v])
            return v;
        return y;
      }
      function ov(u) {
        return u && u.toLowerCase().replace("_", "-");
      }
      function lC(u) {
        for (var h = 0, v, y, w, O; h < u.length; ) {
          for (O = ov(u[h]).split("-"), v = O.length, y = ov(u[h + 1]), y = y ? y.split("-") : null; v > 0; ) {
            if (w = As(O.slice(0, v).join("-")), w)
              return w;
            if (y && y.length >= v && oC(O, y) >= v - 1)
              break;
            v--;
          }
          h++;
        }
        return In;
      }
      function uC(u) {
        return !!(u && u.match("^[^/\\\\]*$"));
      }
      function As(u) {
        var h = null, v;
        if (Be[u] === void 0 && n && n.exports && uC(u))
          try {
            h = In._abbr, v = fr, v("./locale/" + u), Vr(h);
          } catch {
            Be[u] = null;
          }
        return Be[u];
      }
      function Vr(u, h) {
        var v;
        return u && (d(h) ? v = Pr(u) : v = Ka(u, h), v ? In = v : typeof console < "u" && console.warn && console.warn(
          "Locale " + u + " not found. Did you forget to load it?"
        )), In._abbr;
      }
      function Ka(u, h) {
        if (h !== null) {
          var v, y = av;
          if (h.abbr = u, Be[u] != null)
            H(
              "defineLocaleOverride",
              "use moment.updateLocale(localeName, config) to change an existing locale. moment.defineLocale(localeName, config) should only be used for creating a new locale See http://momentjs.com/guides/#/warnings/define-locale/ for more info."
            ), y = Be[u]._config;
          else if (h.parentLocale != null)
            if (Be[h.parentLocale] != null)
              y = Be[h.parentLocale]._config;
            else if (v = As(h.parentLocale), v != null)
              y = v._config;
            else
              return Pn[h.parentLocale] || (Pn[h.parentLocale] = []), Pn[h.parentLocale].push({
                name: u,
                config: h
              }), null;
          return Be[u] = new Y(Me(y, h)), Pn[u] && Pn[u].forEach(function(w) {
            Ka(w.name, w.config);
          }), Vr(u), Be[u];
        } else
          return delete Be[u], null;
      }
      function cC(u, h) {
        if (h != null) {
          var v, y, w = av;
          Be[u] != null && Be[u].parentLocale != null ? Be[u].set(Me(Be[u]._config, h)) : (y = As(u), y != null && (w = y._config), h = Me(w, h), y == null && (h.abbr = u), v = new Y(h), v.parentLocale = Be[u], Be[u] = v), Vr(u);
        } else
          Be[u] != null && (Be[u].parentLocale != null ? (Be[u] = Be[u].parentLocale, u === Vr() && Vr(u)) : Be[u] != null && delete Be[u]);
        return Be[u];
      }
      function Pr(u) {
        var h;
        if (u && u._locale && u._locale._abbr && (u = u._locale._abbr), !u)
          return In;
        if (!a(u)) {
          if (h = As(u), h)
            return h;
          u = [u];
        }
        return lC(u);
      }
      function dC() {
        return J(Be);
      }
      function Xa(u) {
        var h, v = u._a;
        return v && x(u).overflow === -2 && (h = v[kr] < 0 || v[kr] > 11 ? kr : v[gr] < 1 || v[gr] > Wa(v[xt], v[kr]) ? gr : v[ut] < 0 || v[ut] > 24 || v[ut] === 24 && (v[ur] !== 0 || v[Cr] !== 0 || v[di] !== 0) ? ut : v[ur] < 0 || v[ur] > 59 ? ur : v[Cr] < 0 || v[Cr] > 59 ? Cr : v[di] < 0 || v[di] > 999 ? di : -1, x(u)._overflowDayOfYear && (h < xt || h > gr) && (h = gr), x(u)._overflowWeeks && h === -1 && (h = mk), x(u)._overflowWeekday && h === -1 && (h = vk), x(u).overflow = h), u;
      }
      var hC = /^\s*((?:[+-]\d{6}|\d{4})-(?:\d\d-\d\d|W\d\d-\d|W\d\d|\d\d\d|\d\d))(?:(T| )(\d\d(?::\d\d(?::\d\d(?:[.,]\d+)?)?)?)([+-]\d\d(?::?\d\d)?|\s*Z)?)?$/, fC = /^\s*((?:[+-]\d{6}|\d{4})(?:\d\d\d\d|W\d\d\d|W\d\d|\d\d\d|\d\d|))(?:(T| )(\d\d(?:\d\d(?:\d\d(?:[.,]\d+)?)?)?)([+-]\d\d(?::?\d\d)?|\s*Z)?)?$/, pC = /Z|[+-]\d\d(?::?\d\d)?/, Ns = [
        ["YYYYYY-MM-DD", /[+-]\d{6}-\d\d-\d\d/],
        ["YYYY-MM-DD", /\d{4}-\d\d-\d\d/],
        ["GGGG-[W]WW-E", /\d{4}-W\d\d-\d/],
        ["GGGG-[W]WW", /\d{4}-W\d\d/, !1],
        ["YYYY-DDD", /\d{4}-\d{3}/],
        ["YYYY-MM", /\d{4}-\d\d/, !1],
        ["YYYYYYMMDD", /[+-]\d{10}/],
        ["YYYYMMDD", /\d{8}/],
        ["GGGG[W]WWE", /\d{4}W\d{3}/],
        ["GGGG[W]WW", /\d{4}W\d{2}/, !1],
        ["YYYYDDD", /\d{7}/],
        ["YYYYMM", /\d{6}/, !1],
        ["YYYY", /\d{4}/, !1]
      ], Za = [
        ["HH:mm:ss.SSSS", /\d\d:\d\d:\d\d\.\d+/],
        ["HH:mm:ss,SSSS", /\d\d:\d\d:\d\d,\d+/],
        ["HH:mm:ss", /\d\d:\d\d:\d\d/],
        ["HH:mm", /\d\d:\d\d/],
        ["HHmmss.SSSS", /\d\d\d\d\d\d\.\d+/],
        ["HHmmss,SSSS", /\d\d\d\d\d\d,\d+/],
        ["HHmmss", /\d\d\d\d\d\d/],
        ["HHmm", /\d\d\d\d/],
        ["HH", /\d\d/]
      ], mC = /^\/?Date\((-?\d+)/i, vC = /^(?:(Mon|Tue|Wed|Thu|Fri|Sat|Sun),?\s)?(\d{1,2})\s(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s(\d{2,4})\s(\d\d):(\d\d)(?::(\d\d))?\s(?:(UT|GMT|[ECMP][SD]T)|([Zz])|([+-]\d{4}))$/, gC = {
        UT: 0,
        GMT: 0,
        EDT: -240,
        EST: -300,
        CDT: -300,
        CST: -360,
        MDT: -360,
        MST: -420,
        PDT: -420,
        PST: -480
      };
      function lv(u) {
        var h, v, y = u._i, w = hC.exec(y) || fC.exec(y), O, A, U, le, Se = Ns.length, Pt = Za.length;
        if (w) {
          for (x(u).iso = !0, h = 0, v = Se; h < v; h++)
            if (Ns[h][1].exec(w[1])) {
              A = Ns[h][0], O = Ns[h][2] !== !1;
              break;
            }
          if (A == null) {
            u._isValid = !1;
            return;
          }
          if (w[3]) {
            for (h = 0, v = Pt; h < v; h++)
              if (Za[h][1].exec(w[3])) {
                U = (w[2] || " ") + Za[h][0];
                break;
              }
            if (U == null) {
              u._isValid = !1;
              return;
            }
          }
          if (!O && U != null) {
            u._isValid = !1;
            return;
          }
          if (w[4])
            if (pC.exec(w[4]))
              le = "Z";
            else {
              u._isValid = !1;
              return;
            }
          u._f = A + (U || "") + (le || ""), Ja(u);
        } else
          u._isValid = !1;
      }
      function yC(u, h, v, y, w, O) {
        var A = [
          bC(u),
          Zm.indexOf(h),
          parseInt(v, 10),
          parseInt(y, 10),
          parseInt(w, 10)
        ];
        return O && A.push(parseInt(O, 10)), A;
      }
      function bC(u) {
        var h = parseInt(u, 10);
        return h <= 49 ? 2e3 + h : h <= 999 ? 1900 + h : h;
      }
      function _C(u) {
        return u.replace(/\([^()]*\)|[\n\t]/g, " ").replace(/(\s\s+)/g, " ").replace(/^\s\s*/, "").replace(/\s\s*$/, "");
      }
      function wC(u, h, v) {
        if (u) {
          var y = iv.indexOf(u), w = new Date(
            h[0],
            h[1],
            h[2]
          ).getDay();
          if (y !== w)
            return x(v).weekdayMismatch = !0, v._isValid = !1, !1;
        }
        return !0;
      }
      function xC(u, h, v) {
        if (u)
          return gC[u];
        if (h)
          return 0;
        var y = parseInt(v, 10), w = y % 100, O = (y - w) / 100;
        return O * 60 + w;
      }
      function uv(u) {
        var h = vC.exec(_C(u._i)), v;
        if (h) {
          if (v = yC(
            h[4],
            h[3],
            h[2],
            h[5],
            h[6],
            h[7]
          ), !wC(h[1], v, u))
            return;
          u._a = v, u._tzm = xC(h[8], h[9], h[10]), u._d = Cn.apply(null, u._a), u._d.setUTCMinutes(u._d.getUTCMinutes() - u._tzm), x(u).rfc2822 = !0;
        } else
          u._isValid = !1;
      }
      function SC(u) {
        var h = mC.exec(u._i);
        if (h !== null) {
          u._d = /* @__PURE__ */ new Date(+h[1]);
          return;
        }
        if (lv(u), u._isValid === !1)
          delete u._isValid;
        else
          return;
        if (uv(u), u._isValid === !1)
          delete u._isValid;
        else
          return;
        u._strict ? u._isValid = !1 : r.createFromInputFallback(u);
      }
      r.createFromInputFallback = q(
        "value provided is not in a recognized RFC2822 or ISO format. moment construction falls back to js Date(), which is not reliable across all browsers and versions. Non RFC2822/ISO date formats are discouraged. Please refer to http://momentjs.com/guides/#/warnings/js-date/ for more info.",
        function(u) {
          u._d = /* @__PURE__ */ new Date(u._i + (u._useUTC ? " UTC" : ""));
        }
      );
      function ji(u, h, v) {
        return u ?? h ?? v;
      }
      function TC(u) {
        var h = new Date(r.now());
        return u._useUTC ? [
          h.getUTCFullYear(),
          h.getUTCMonth(),
          h.getUTCDate()
        ] : [h.getFullYear(), h.getMonth(), h.getDate()];
      }
      function Qa(u) {
        var h, v, y = [], w, O, A;
        if (!u._d) {
          for (w = TC(u), u._w && u._a[gr] == null && u._a[kr] == null && EC(u), u._dayOfYear != null && (A = ji(u._a[xt], w[xt]), (u._dayOfYear > On(A) || u._dayOfYear === 0) && (x(u)._overflowDayOfYear = !0), v = Cn(A, 0, u._dayOfYear), u._a[kr] = v.getUTCMonth(), u._a[gr] = v.getUTCDate()), h = 0; h < 3 && u._a[h] == null; ++h)
            u._a[h] = y[h] = w[h];
          for (; h < 7; h++)
            u._a[h] = y[h] = u._a[h] == null ? h === 2 ? 1 : 0 : u._a[h];
          u._a[ut] === 24 && u._a[ur] === 0 && u._a[Cr] === 0 && u._a[di] === 0 && (u._nextDay = !0, u._a[ut] = 0), u._d = (u._useUTC ? Cn : Pk).apply(
            null,
            y
          ), O = u._useUTC ? u._d.getUTCDay() : u._d.getDay(), u._tzm != null && u._d.setUTCMinutes(u._d.getUTCMinutes() - u._tzm), u._nextDay && (u._a[ut] = 24), u._w && typeof u._w.d < "u" && u._w.d !== O && (x(u).weekdayMismatch = !0);
        }
      }
      function EC(u) {
        var h, v, y, w, O, A, U, le, Se;
        h = u._w, h.GG != null || h.W != null || h.E != null ? (O = 1, A = 4, v = ji(
          h.GG,
          u._a[xt],
          Rn(Ge(), 1, 4).year
        ), y = ji(h.W, 1), w = ji(h.E, 1), (w < 1 || w > 7) && (le = !0)) : (O = u._locale._week.dow, A = u._locale._week.doy, Se = Rn(Ge(), O, A), v = ji(h.gg, u._a[xt], Se.year), y = ji(h.w, Se.week), h.d != null ? (w = h.d, (w < 0 || w > 6) && (le = !0)) : h.e != null ? (w = h.e + O, (h.e < 0 || h.e > 6) && (le = !0)) : w = O), y < 1 || y > Rr(v, O, A) ? x(u)._overflowWeeks = !0 : le != null ? x(u)._overflowWeekday = !0 : (U = rv(v, y, w, O, A), u._a[xt] = U.year, u._dayOfYear = U.dayOfYear);
      }
      r.ISO_8601 = function() {
      }, r.RFC_2822 = function() {
      };
      function Ja(u) {
        if (u._f === r.ISO_8601) {
          lv(u);
          return;
        }
        if (u._f === r.RFC_2822) {
          uv(u);
          return;
        }
        u._a = [], x(u).empty = !0;
        var h = "" + u._i, v, y, w, O, A, U = h.length, le = 0, Se, Pt;
        for (w = ht(u._f, u._locale).match(re) || [], Pt = w.length, v = 0; v < Pt; v++)
          O = w[v], y = (h.match(Ba(O, u)) || [])[0], y && (A = h.substr(0, h.indexOf(y)), A.length > 0 && x(u).unusedInput.push(A), h = h.slice(
            h.indexOf(y) + y.length
          ), le += y.length), se[O] ? (y ? x(u).empty = !1 : x(u).unusedTokens.push(O), pk(O, y, u)) : u._strict && !y && x(u).unusedTokens.push(O);
        x(u).charsLeftOver = U - le, h.length > 0 && x(u).unusedInput.push(h), u._a[ut] <= 12 && x(u).bigHour === !0 && u._a[ut] > 0 && (x(u).bigHour = void 0), x(u).parsedDateParts = u._a.slice(0), x(u).meridiem = u._meridiem, u._a[ut] = DC(
          u._locale,
          u._a[ut],
          u._meridiem
        ), Se = x(u).era, Se !== null && (u._a[xt] = u._locale.erasConvertYear(Se, u._a[xt])), Qa(u), Xa(u);
      }
      function DC(u, h, v) {
        var y;
        return v == null ? h : u.meridiemHour != null ? u.meridiemHour(h, v) : (u.isPM != null && (y = u.isPM(v), y && h < 12 && (h += 12), !y && h === 12 && (h = 0)), h);
      }
      function OC(u) {
        var h, v, y, w, O, A, U = !1, le = u._f.length;
        if (le === 0) {
          x(u).invalidFormat = !0, u._d = /* @__PURE__ */ new Date(NaN);
          return;
        }
        for (w = 0; w < le; w++)
          O = 0, A = !1, h = C({}, u), u._useUTC != null && (h._useUTC = u._useUTC), h._f = u._f[w], Ja(h), S(h) && (A = !0), O += x(h).charsLeftOver, O += x(h).unusedTokens.length * 10, x(h).score = O, U ? O < y && (y = O, v = h) : (y == null || O < y || A) && (y = O, v = h, A && (U = !0));
        g(u, v || h);
      }
      function kC(u) {
        if (!u._d) {
          var h = Te(u._i), v = h.day === void 0 ? h.date : h.day;
          u._a = m(
            [h.year, h.month, v, h.hour, h.minute, h.second, h.millisecond],
            function(y) {
              return y && parseInt(y, 10);
            }
          ), Qa(u);
        }
      }
      function CC(u) {
        var h = new P(Xa(cv(u)));
        return h._nextDay && (h.add(1, "d"), h._nextDay = void 0), h;
      }
      function cv(u) {
        var h = u._i, v = u._f;
        return u._locale = u._locale || Pr(u._l), h === null || v === void 0 && h === "" ? k({ nullInput: !0 }) : (typeof h == "string" && (u._i = h = u._locale.preparse(h)), R(h) ? new P(Xa(h)) : (p(h) ? u._d = h : a(v) ? OC(u) : v ? Ja(u) : RC(u), S(u) || (u._d = null), u));
      }
      function RC(u) {
        var h = u._i;
        d(h) ? u._d = new Date(r.now()) : p(h) ? u._d = new Date(h.valueOf()) : typeof h == "string" ? SC(u) : a(h) ? (u._a = m(h.slice(0), function(v) {
          return parseInt(v, 10);
        }), Qa(u)) : o(h) ? kC(u) : f(h) ? u._d = new Date(h) : r.createFromInputFallback(u);
      }
      function dv(u, h, v, y, w) {
        var O = {};
        return (h === !0 || h === !1) && (y = h, h = void 0), (v === !0 || v === !1) && (y = v, v = void 0), (o(u) && c(u) || a(u) && u.length === 0) && (u = void 0), O._isAMomentObject = !0, O._useUTC = O._isUTC = w, O._l = v, O._i = u, O._f = h, O._strict = y, CC(O);
      }
      function Ge(u, h, v, y) {
        return dv(u, h, v, y, !1);
      }
      var PC = q(
        "moment().min is deprecated, use moment.max instead. http://momentjs.com/guides/#/warnings/min-max/",
        function() {
          var u = Ge.apply(null, arguments);
          return this.isValid() && u.isValid() ? u < this ? this : u : k();
        }
      ), IC = q(
        "moment().max is deprecated, use moment.min instead. http://momentjs.com/guides/#/warnings/min-max/",
        function() {
          var u = Ge.apply(null, arguments);
          return this.isValid() && u.isValid() ? u > this ? this : u : k();
        }
      );
      function hv(u, h) {
        var v, y;
        if (h.length === 1 && a(h[0]) && (h = h[0]), !h.length)
          return Ge();
        for (v = h[0], y = 1; y < h.length; ++y)
          (!h[y].isValid() || h[y][u](v)) && (v = h[y]);
        return v;
      }
      function MC() {
        var u = [].slice.call(arguments, 0);
        return hv("isBefore", u);
      }
      function AC() {
        var u = [].slice.call(arguments, 0);
        return hv("isAfter", u);
      }
      var NC = function() {
        return Date.now ? Date.now() : +/* @__PURE__ */ new Date();
      }, Mn = [
        "year",
        "quarter",
        "month",
        "week",
        "day",
        "hour",
        "minute",
        "second",
        "millisecond"
      ];
      function qC(u) {
        var h, v = !1, y, w = Mn.length;
        for (h in u)
          if (l(u, h) && !(et.call(Mn, h) !== -1 && (u[h] == null || !isNaN(u[h]))))
            return !1;
        for (y = 0; y < w; ++y)
          if (u[Mn[y]]) {
            if (v)
              return !1;
            parseFloat(u[Mn[y]]) !== oe(u[Mn[y]]) && (v = !0);
          }
        return !0;
      }
      function LC() {
        return this._isValid;
      }
      function FC() {
        return cr(NaN);
      }
      function qs(u) {
        var h = Te(u), v = h.year || 0, y = h.quarter || 0, w = h.month || 0, O = h.week || h.isoWeek || 0, A = h.day || 0, U = h.hour || 0, le = h.minute || 0, Se = h.second || 0, Pt = h.millisecond || 0;
        this._isValid = qC(h), this._milliseconds = +Pt + Se * 1e3 + // 1000
        le * 6e4 + // 1000 * 60
        U * 1e3 * 60 * 60, this._days = +A + O * 7, this._months = +w + y * 3 + v * 12, this._data = {}, this._locale = Pr(), this._bubble();
      }
      function Ls(u) {
        return u instanceof qs;
      }
      function eo(u) {
        return u < 0 ? Math.round(-1 * u) * -1 : Math.round(u);
      }
      function $C(u, h, v) {
        var y = Math.min(u.length, h.length), w = Math.abs(u.length - h.length), O = 0, A;
        for (A = 0; A < y; A++)
          oe(u[A]) !== oe(h[A]) && O++;
        return O + w;
      }
      function fv(u, h) {
        j(u, 0, 0, function() {
          var v = this.utcOffset(), y = "+";
          return v < 0 && (v = -v, y = "-"), y + K(~~(v / 60), 2) + h + K(~~v % 60, 2);
        });
      }
      fv("Z", ":"), fv("ZZ", ""), W("Z", jt), W("ZZ", jt), Oe(["Z", "ZZ"], function(u, h, v) {
        v._useUTC = !0, v._tzm = to(jt, u);
      });
      var jC = /([\+\-]|\d\d)/gi;
      function to(u, h) {
        var v = (h || "").match(u), y, w, O;
        return v === null ? null : (y = v[v.length - 1] || [], w = (y + "").match(jC) || ["-", 0, 0], O = +(w[1] * 60) + oe(w[2]), O === 0 ? 0 : w[0] === "+" ? O : -O);
      }
      function ro(u, h) {
        var v, y;
        return h._isUTC ? (v = h.clone(), y = (R(u) || p(u) ? u.valueOf() : Ge(u).valueOf()) - v.valueOf(), v._d.setTime(v._d.valueOf() + y), r.updateOffset(v, !1), v) : Ge(u).local();
      }
      function io(u) {
        return -Math.round(u._d.getTimezoneOffset());
      }
      r.updateOffset = function() {
      };
      function YC(u, h, v) {
        var y = this._offset || 0, w;
        if (!this.isValid())
          return u != null ? this : NaN;
        if (u != null) {
          if (typeof u == "string") {
            if (u = to(jt, u), u === null)
              return this;
          } else Math.abs(u) < 16 && !v && (u = u * 60);
          return !this._isUTC && h && (w = io(this)), this._offset = u, this._isUTC = !0, w != null && this.add(w, "m"), y !== u && (!h || this._changeInProgress ? gv(
            this,
            cr(u - y, "m"),
            1,
            !1
          ) : this._changeInProgress || (this._changeInProgress = !0, r.updateOffset(this, !0), this._changeInProgress = null)), this;
        } else
          return this._isUTC ? y : io(this);
      }
      function HC(u, h) {
        return u != null ? (typeof u != "string" && (u = -u), this.utcOffset(u, h), this) : -this.utcOffset();
      }
      function GC(u) {
        return this.utcOffset(0, u);
      }
      function BC(u) {
        return this._isUTC && (this.utcOffset(0, u), this._isUTC = !1, u && this.subtract(io(this), "m")), this;
      }
      function WC() {
        if (this._tzm != null)
          this.utcOffset(this._tzm, !1, !0);
        else if (typeof this._i == "string") {
          var u = to(wt, this._i);
          u != null ? this.utcOffset(u) : this.utcOffset(0, !0);
        }
        return this;
      }
      function zC(u) {
        return this.isValid() ? (u = u ? Ge(u).utcOffset() : 0, (this.utcOffset() - u) % 60 === 0) : !1;
      }
      function UC() {
        return this.utcOffset() > this.clone().month(0).utcOffset() || this.utcOffset() > this.clone().month(5).utcOffset();
      }
      function VC() {
        if (!d(this._isDSTShifted))
          return this._isDSTShifted;
        var u = {}, h;
        return C(u, this), u = cv(u), u._a ? (h = u._isUTC ? b(u._a) : Ge(u._a), this._isDSTShifted = this.isValid() && $C(u._a, h.toArray()) > 0) : this._isDSTShifted = !1, this._isDSTShifted;
      }
      function KC() {
        return this.isValid() ? !this._isUTC : !1;
      }
      function XC() {
        return this.isValid() ? this._isUTC : !1;
      }
      function pv() {
        return this.isValid() ? this._isUTC && this._offset === 0 : !1;
      }
      var ZC = /^(-|\+)?(?:(\d*)[. ])?(\d+):(\d+)(?::(\d+)(\.\d*)?)?$/, QC = /^(-|\+)?P(?:([-+]?[0-9,.]*)Y)?(?:([-+]?[0-9,.]*)M)?(?:([-+]?[0-9,.]*)W)?(?:([-+]?[0-9,.]*)D)?(?:T(?:([-+]?[0-9,.]*)H)?(?:([-+]?[0-9,.]*)M)?(?:([-+]?[0-9,.]*)S)?)?$/;
      function cr(u, h) {
        var v = u, y = null, w, O, A;
        return Ls(u) ? v = {
          ms: u._milliseconds,
          d: u._days,
          M: u._months
        } : f(u) || !isNaN(+u) ? (v = {}, h ? v[h] = +u : v.milliseconds = +u) : (y = ZC.exec(u)) ? (w = y[1] === "-" ? -1 : 1, v = {
          y: 0,
          d: oe(y[gr]) * w,
          h: oe(y[ut]) * w,
          m: oe(y[ur]) * w,
          s: oe(y[Cr]) * w,
          ms: oe(eo(y[di] * 1e3)) * w
          // the millisecond decimal point is included in the match
        }) : (y = QC.exec(u)) ? (w = y[1] === "-" ? -1 : 1, v = {
          y: hi(y[2], w),
          M: hi(y[3], w),
          w: hi(y[4], w),
          d: hi(y[5], w),
          h: hi(y[6], w),
          m: hi(y[7], w),
          s: hi(y[8], w)
        }) : v == null ? v = {} : typeof v == "object" && ("from" in v || "to" in v) && (A = JC(
          Ge(v.from),
          Ge(v.to)
        ), v = {}, v.ms = A.milliseconds, v.M = A.months), O = new qs(v), Ls(u) && l(u, "_locale") && (O._locale = u._locale), Ls(u) && l(u, "_isValid") && (O._isValid = u._isValid), O;
      }
      cr.fn = qs.prototype, cr.invalid = FC;
      function hi(u, h) {
        var v = u && parseFloat(u.replace(",", "."));
        return (isNaN(v) ? 0 : v) * h;
      }
      function mv(u, h) {
        var v = {};
        return v.months = h.month() - u.month() + (h.year() - u.year()) * 12, u.clone().add(v.months, "M").isAfter(h) && --v.months, v.milliseconds = +h - +u.clone().add(v.months, "M"), v;
      }
      function JC(u, h) {
        var v;
        return u.isValid() && h.isValid() ? (h = ro(h, u), u.isBefore(h) ? v = mv(u, h) : (v = mv(h, u), v.milliseconds = -v.milliseconds, v.months = -v.months), v) : { milliseconds: 0, months: 0 };
      }
      function vv(u, h) {
        return function(v, y) {
          var w, O;
          return y !== null && !isNaN(+y) && (H(
            h,
            "moment()." + h + "(period, number) is deprecated. Please use moment()." + h + "(number, period). See http://momentjs.com/guides/#/warnings/add-inverted-param/ for more info."
          ), O = v, v = y, y = O), w = cr(v, y), gv(this, w, u), this;
        };
      }
      function gv(u, h, v, y) {
        var w = h._milliseconds, O = eo(h._days), A = eo(h._months);
        u.isValid() && (y = y ?? !0, A && Jm(u, kn(u, "Month") + A * v), O && Xm(u, "Date", kn(u, "Date") + O * v), w && u._d.setTime(u._d.valueOf() + w * v), y && r.updateOffset(u, O || A));
      }
      var eR = vv(1, "add"), tR = vv(-1, "subtract");
      function yv(u) {
        return typeof u == "string" || u instanceof String;
      }
      function rR(u) {
        return R(u) || p(u) || yv(u) || f(u) || nR(u) || iR(u) || u === null || u === void 0;
      }
      function iR(u) {
        var h = o(u) && !c(u), v = !1, y = [
          "years",
          "year",
          "y",
          "months",
          "month",
          "M",
          "days",
          "day",
          "d",
          "dates",
          "date",
          "D",
          "hours",
          "hour",
          "h",
          "minutes",
          "minute",
          "m",
          "seconds",
          "second",
          "s",
          "milliseconds",
          "millisecond",
          "ms"
        ], w, O, A = y.length;
        for (w = 0; w < A; w += 1)
          O = y[w], v = v || l(u, O);
        return h && v;
      }
      function nR(u) {
        var h = a(u), v = !1;
        return h && (v = u.filter(function(y) {
          return !f(y) && yv(u);
        }).length === 0), h && v;
      }
      function sR(u) {
        var h = o(u) && !c(u), v = !1, y = [
          "sameDay",
          "nextDay",
          "lastDay",
          "nextWeek",
          "lastWeek",
          "sameElse"
        ], w, O;
        for (w = 0; w < y.length; w += 1)
          O = y[w], v = v || l(u, O);
        return h && v;
      }
      function aR(u, h) {
        var v = u.diff(h, "days", !0);
        return v < -6 ? "sameElse" : v < -1 ? "lastWeek" : v < 0 ? "lastDay" : v < 1 ? "sameDay" : v < 2 ? "nextDay" : v < 7 ? "nextWeek" : "sameElse";
      }
      function oR(u, h) {
        arguments.length === 1 && (arguments[0] ? rR(arguments[0]) ? (u = arguments[0], h = void 0) : sR(arguments[0]) && (h = arguments[0], u = void 0) : (u = void 0, h = void 0));
        var v = u || Ge(), y = ro(v, this).startOf("day"), w = r.calendarFormat(this, y) || "sameElse", O = h && (Z(h[w]) ? h[w].call(this, v) : h[w]);
        return this.format(
          O || this.localeData().calendar(w, this, Ge(v))
        );
      }
      function lR() {
        return new P(this);
      }
      function uR(u, h) {
        var v = R(u) ? u : Ge(u);
        return this.isValid() && v.isValid() ? (h = fe(h) || "millisecond", h === "millisecond" ? this.valueOf() > v.valueOf() : v.valueOf() < this.clone().startOf(h).valueOf()) : !1;
      }
      function cR(u, h) {
        var v = R(u) ? u : Ge(u);
        return this.isValid() && v.isValid() ? (h = fe(h) || "millisecond", h === "millisecond" ? this.valueOf() < v.valueOf() : this.clone().endOf(h).valueOf() < v.valueOf()) : !1;
      }
      function dR(u, h, v, y) {
        var w = R(u) ? u : Ge(u), O = R(h) ? h : Ge(h);
        return this.isValid() && w.isValid() && O.isValid() ? (y = y || "()", (y[0] === "(" ? this.isAfter(w, v) : !this.isBefore(w, v)) && (y[1] === ")" ? this.isBefore(O, v) : !this.isAfter(O, v))) : !1;
      }
      function hR(u, h) {
        var v = R(u) ? u : Ge(u), y;
        return this.isValid() && v.isValid() ? (h = fe(h) || "millisecond", h === "millisecond" ? this.valueOf() === v.valueOf() : (y = v.valueOf(), this.clone().startOf(h).valueOf() <= y && y <= this.clone().endOf(h).valueOf())) : !1;
      }
      function fR(u, h) {
        return this.isSame(u, h) || this.isAfter(u, h);
      }
      function pR(u, h) {
        return this.isSame(u, h) || this.isBefore(u, h);
      }
      function mR(u, h, v) {
        var y, w, O;
        if (!this.isValid())
          return NaN;
        if (y = ro(u, this), !y.isValid())
          return NaN;
        switch (w = (y.utcOffset() - this.utcOffset()) * 6e4, h = fe(h), h) {
          case "year":
            O = Fs(this, y) / 12;
            break;
          case "month":
            O = Fs(this, y);
            break;
          case "quarter":
            O = Fs(this, y) / 3;
            break;
          case "second":
            O = (this - y) / 1e3;
            break;
          // 1000
          case "minute":
            O = (this - y) / 6e4;
            break;
          // 1000 * 60
          case "hour":
            O = (this - y) / 36e5;
            break;
          // 1000 * 60 * 60
          case "day":
            O = (this - y - w) / 864e5;
            break;
          // 1000 * 60 * 60 * 24, negate dst
          case "week":
            O = (this - y - w) / 6048e5;
            break;
          // 1000 * 60 * 60 * 24 * 7, negate dst
          default:
            O = this - y;
        }
        return v ? O : _e(O);
      }
      function Fs(u, h) {
        if (u.date() < h.date())
          return -Fs(h, u);
        var v = (h.year() - u.year()) * 12 + (h.month() - u.month()), y = u.clone().add(v, "months"), w, O;
        return h - y < 0 ? (w = u.clone().add(v - 1, "months"), O = (h - y) / (y - w)) : (w = u.clone().add(v + 1, "months"), O = (h - y) / (w - y)), -(v + O) || 0;
      }
      r.defaultFormat = "YYYY-MM-DDTHH:mm:ssZ", r.defaultFormatUtc = "YYYY-MM-DDTHH:mm:ss[Z]";
      function vR() {
        return this.clone().locale("en").format("ddd MMM DD YYYY HH:mm:ss [GMT]ZZ");
      }
      function gR(u) {
        if (!this.isValid())
          return null;
        var h = u !== !0, v = h ? this.clone().utc() : this;
        return v.year() < 0 || v.year() > 9999 ? st(
          v,
          h ? "YYYYYY-MM-DD[T]HH:mm:ss.SSS[Z]" : "YYYYYY-MM-DD[T]HH:mm:ss.SSSZ"
        ) : Z(Date.prototype.toISOString) ? h ? this.toDate().toISOString() : new Date(this.valueOf() + this.utcOffset() * 60 * 1e3).toISOString().replace("Z", st(v, "Z")) : st(
          v,
          h ? "YYYY-MM-DD[T]HH:mm:ss.SSS[Z]" : "YYYY-MM-DD[T]HH:mm:ss.SSSZ"
        );
      }
      function yR() {
        if (!this.isValid())
          return "moment.invalid(/* " + this._i + " */)";
        var u = "moment", h = "", v, y, w, O;
        return this.isLocal() || (u = this.utcOffset() === 0 ? "moment.utc" : "moment.parseZone", h = "Z"), v = "[" + u + '("]', y = 0 <= this.year() && this.year() <= 9999 ? "YYYY" : "YYYYYY", w = "-MM-DD[T]HH:mm:ss.SSS", O = h + '[")]', this.format(v + y + w + O);
      }
      function bR(u) {
        u || (u = this.isUtc() ? r.defaultFormatUtc : r.defaultFormat);
        var h = st(this, u);
        return this.localeData().postformat(h);
      }
      function _R(u, h) {
        return this.isValid() && (R(u) && u.isValid() || Ge(u).isValid()) ? cr({ to: this, from: u }).locale(this.locale()).humanize(!h) : this.localeData().invalidDate();
      }
      function wR(u) {
        return this.from(Ge(), u);
      }
      function xR(u, h) {
        return this.isValid() && (R(u) && u.isValid() || Ge(u).isValid()) ? cr({ from: this, to: u }).locale(this.locale()).humanize(!h) : this.localeData().invalidDate();
      }
      function SR(u) {
        return this.to(Ge(), u);
      }
      function bv(u) {
        var h;
        return u === void 0 ? this._locale._abbr : (h = Pr(u), h != null && (this._locale = h), this);
      }
      var _v = q(
        "moment().lang() is deprecated. Instead, use moment().localeData() to get the language configuration. Use moment().locale() to change languages.",
        function(u) {
          return u === void 0 ? this.localeData() : this.locale(u);
        }
      );
      function wv() {
        return this._locale;
      }
      var $s = 1e3, Yi = 60 * $s, js = 60 * Yi, xv = (365 * 400 + 97) * 24 * js;
      function Hi(u, h) {
        return (u % h + h) % h;
      }
      function Sv(u, h, v) {
        return u < 100 && u >= 0 ? new Date(u + 400, h, v) - xv : new Date(u, h, v).valueOf();
      }
      function Tv(u, h, v) {
        return u < 100 && u >= 0 ? Date.UTC(u + 400, h, v) - xv : Date.UTC(u, h, v);
      }
      function TR(u) {
        var h, v;
        if (u = fe(u), u === void 0 || u === "millisecond" || !this.isValid())
          return this;
        switch (v = this._isUTC ? Tv : Sv, u) {
          case "year":
            h = v(this.year(), 0, 1);
            break;
          case "quarter":
            h = v(
              this.year(),
              this.month() - this.month() % 3,
              1
            );
            break;
          case "month":
            h = v(this.year(), this.month(), 1);
            break;
          case "week":
            h = v(
              this.year(),
              this.month(),
              this.date() - this.weekday()
            );
            break;
          case "isoWeek":
            h = v(
              this.year(),
              this.month(),
              this.date() - (this.isoWeekday() - 1)
            );
            break;
          case "day":
          case "date":
            h = v(this.year(), this.month(), this.date());
            break;
          case "hour":
            h = this._d.valueOf(), h -= Hi(
              h + (this._isUTC ? 0 : this.utcOffset() * Yi),
              js
            );
            break;
          case "minute":
            h = this._d.valueOf(), h -= Hi(h, Yi);
            break;
          case "second":
            h = this._d.valueOf(), h -= Hi(h, $s);
            break;
        }
        return this._d.setTime(h), r.updateOffset(this, !0), this;
      }
      function ER(u) {
        var h, v;
        if (u = fe(u), u === void 0 || u === "millisecond" || !this.isValid())
          return this;
        switch (v = this._isUTC ? Tv : Sv, u) {
          case "year":
            h = v(this.year() + 1, 0, 1) - 1;
            break;
          case "quarter":
            h = v(
              this.year(),
              this.month() - this.month() % 3 + 3,
              1
            ) - 1;
            break;
          case "month":
            h = v(this.year(), this.month() + 1, 1) - 1;
            break;
          case "week":
            h = v(
              this.year(),
              this.month(),
              this.date() - this.weekday() + 7
            ) - 1;
            break;
          case "isoWeek":
            h = v(
              this.year(),
              this.month(),
              this.date() - (this.isoWeekday() - 1) + 7
            ) - 1;
            break;
          case "day":
          case "date":
            h = v(this.year(), this.month(), this.date() + 1) - 1;
            break;
          case "hour":
            h = this._d.valueOf(), h += js - Hi(
              h + (this._isUTC ? 0 : this.utcOffset() * Yi),
              js
            ) - 1;
            break;
          case "minute":
            h = this._d.valueOf(), h += Yi - Hi(h, Yi) - 1;
            break;
          case "second":
            h = this._d.valueOf(), h += $s - Hi(h, $s) - 1;
            break;
        }
        return this._d.setTime(h), r.updateOffset(this, !0), this;
      }
      function DR() {
        return this._d.valueOf() - (this._offset || 0) * 6e4;
      }
      function OR() {
        return Math.floor(this.valueOf() / 1e3);
      }
      function kR() {
        return new Date(this.valueOf());
      }
      function CR() {
        var u = this;
        return [
          u.year(),
          u.month(),
          u.date(),
          u.hour(),
          u.minute(),
          u.second(),
          u.millisecond()
        ];
      }
      function RR() {
        var u = this;
        return {
          years: u.year(),
          months: u.month(),
          date: u.date(),
          hours: u.hours(),
          minutes: u.minutes(),
          seconds: u.seconds(),
          milliseconds: u.milliseconds()
        };
      }
      function PR() {
        return this.isValid() ? this.toISOString() : null;
      }
      function IR() {
        return S(this);
      }
      function MR() {
        return g({}, x(this));
      }
      function AR() {
        return x(this).overflow;
      }
      function NR() {
        return {
          input: this._i,
          format: this._f,
          locale: this._locale,
          isUTC: this._isUTC,
          strict: this._strict
        };
      }
      j("N", 0, 0, "eraAbbr"), j("NN", 0, 0, "eraAbbr"), j("NNN", 0, 0, "eraAbbr"), j("NNNN", 0, 0, "eraName"), j("NNNNN", 0, 0, "eraNarrow"), j("y", ["y", 1], "yo", "eraYear"), j("y", ["yy", 2], 0, "eraYear"), j("y", ["yyy", 3], 0, "eraYear"), j("y", ["yyyy", 4], 0, "eraYear"), W("N", no), W("NN", no), W("NNN", no), W("NNNN", zR), W("NNNNN", UR), Oe(
        ["N", "NN", "NNN", "NNNN", "NNNNN"],
        function(u, h, v, y) {
          var w = v._locale.erasParse(u, y, v._strict);
          w ? x(v).era = w : x(v).invalidEra = u;
        }
      ), W("y", ee), W("yy", ee), W("yyy", ee), W("yyyy", ee), W("yo", VR), Oe(["y", "yy", "yyy", "yyyy"], xt), Oe(["yo"], function(u, h, v, y) {
        var w;
        v._locale._eraYearOrdinalRegex && (w = u.match(v._locale._eraYearOrdinalRegex)), v._locale.eraYearOrdinalParse ? h[xt] = v._locale.eraYearOrdinalParse(u, w) : h[xt] = parseInt(u, 10);
      });
      function qR(u, h) {
        var v, y, w, O = this._eras || Pr("en")._eras;
        for (v = 0, y = O.length; v < y; ++v) {
          switch (typeof O[v].since) {
            case "string":
              w = r(O[v].since).startOf("day"), O[v].since = w.valueOf();
              break;
          }
          switch (typeof O[v].until) {
            case "undefined":
              O[v].until = 1 / 0;
              break;
            case "string":
              w = r(O[v].until).startOf("day").valueOf(), O[v].until = w.valueOf();
              break;
          }
        }
        return O;
      }
      function LR(u, h, v) {
        var y, w, O = this.eras(), A, U, le;
        for (u = u.toUpperCase(), y = 0, w = O.length; y < w; ++y)
          if (A = O[y].name.toUpperCase(), U = O[y].abbr.toUpperCase(), le = O[y].narrow.toUpperCase(), v)
            switch (h) {
              case "N":
              case "NN":
              case "NNN":
                if (U === u)
                  return O[y];
                break;
              case "NNNN":
                if (A === u)
                  return O[y];
                break;
              case "NNNNN":
                if (le === u)
                  return O[y];
                break;
            }
          else if ([A, U, le].indexOf(u) >= 0)
            return O[y];
      }
      function FR(u, h) {
        var v = u.since <= u.until ? 1 : -1;
        return h === void 0 ? r(u.since).year() : r(u.since).year() + (h - u.offset) * v;
      }
      function $R() {
        var u, h, v, y = this.localeData().eras();
        for (u = 0, h = y.length; u < h; ++u)
          if (v = this.clone().startOf("day").valueOf(), y[u].since <= v && v <= y[u].until || y[u].until <= v && v <= y[u].since)
            return y[u].name;
        return "";
      }
      function jR() {
        var u, h, v, y = this.localeData().eras();
        for (u = 0, h = y.length; u < h; ++u)
          if (v = this.clone().startOf("day").valueOf(), y[u].since <= v && v <= y[u].until || y[u].until <= v && v <= y[u].since)
            return y[u].narrow;
        return "";
      }
      function YR() {
        var u, h, v, y = this.localeData().eras();
        for (u = 0, h = y.length; u < h; ++u)
          if (v = this.clone().startOf("day").valueOf(), y[u].since <= v && v <= y[u].until || y[u].until <= v && v <= y[u].since)
            return y[u].abbr;
        return "";
      }
      function HR() {
        var u, h, v, y, w = this.localeData().eras();
        for (u = 0, h = w.length; u < h; ++u)
          if (v = w[u].since <= w[u].until ? 1 : -1, y = this.clone().startOf("day").valueOf(), w[u].since <= y && y <= w[u].until || w[u].until <= y && y <= w[u].since)
            return (this.year() - r(w[u].since).year()) * v + w[u].offset;
        return this.year();
      }
      function GR(u) {
        return l(this, "_erasNameRegex") || so.call(this), u ? this._erasNameRegex : this._erasRegex;
      }
      function BR(u) {
        return l(this, "_erasAbbrRegex") || so.call(this), u ? this._erasAbbrRegex : this._erasRegex;
      }
      function WR(u) {
        return l(this, "_erasNarrowRegex") || so.call(this), u ? this._erasNarrowRegex : this._erasRegex;
      }
      function no(u, h) {
        return h.erasAbbrRegex(u);
      }
      function zR(u, h) {
        return h.erasNameRegex(u);
      }
      function UR(u, h) {
        return h.erasNarrowRegex(u);
      }
      function VR(u, h) {
        return h._eraYearOrdinalRegex || ee;
      }
      function so() {
        var u = [], h = [], v = [], y = [], w, O, A, U, le, Se = this.eras();
        for (w = 0, O = Se.length; w < O; ++w)
          A = V(Se[w].name), U = V(Se[w].abbr), le = V(Se[w].narrow), h.push(A), u.push(U), v.push(le), y.push(A), y.push(U), y.push(le);
        this._erasRegex = new RegExp("^(" + y.join("|") + ")", "i"), this._erasNameRegex = new RegExp("^(" + h.join("|") + ")", "i"), this._erasAbbrRegex = new RegExp("^(" + u.join("|") + ")", "i"), this._erasNarrowRegex = new RegExp(
          "^(" + v.join("|") + ")",
          "i"
        );
      }
      j(0, ["gg", 2], 0, function() {
        return this.weekYear() % 100;
      }), j(0, ["GG", 2], 0, function() {
        return this.isoWeekYear() % 100;
      });
      function Ys(u, h) {
        j(0, [u, u.length], 0, h);
      }
      Ys("gggg", "weekYear"), Ys("ggggg", "weekYear"), Ys("GGGG", "isoWeekYear"), Ys("GGGGG", "isoWeekYear"), W("G", Je), W("g", Je), W("GG", Ee, Ie), W("gg", Ee, Ie), W("GGGG", de, er), W("gggg", de, er), W("GGGGG", ge, Bt), W("ggggg", ge, Bt), ci(
        ["gggg", "ggggg", "GGGG", "GGGGG"],
        function(u, h, v, y) {
          h[y.substr(0, 2)] = oe(u);
        }
      ), ci(["gg", "GG"], function(u, h, v, y) {
        h[y] = r.parseTwoDigitYear(u);
      });
      function KR(u) {
        return Ev.call(
          this,
          u,
          this.week(),
          this.weekday() + this.localeData()._week.dow,
          this.localeData()._week.dow,
          this.localeData()._week.doy
        );
      }
      function XR(u) {
        return Ev.call(
          this,
          u,
          this.isoWeek(),
          this.isoWeekday(),
          1,
          4
        );
      }
      function ZR() {
        return Rr(this.year(), 1, 4);
      }
      function QR() {
        return Rr(this.isoWeekYear(), 1, 4);
      }
      function JR() {
        var u = this.localeData()._week;
        return Rr(this.year(), u.dow, u.doy);
      }
      function eP() {
        var u = this.localeData()._week;
        return Rr(this.weekYear(), u.dow, u.doy);
      }
      function Ev(u, h, v, y, w) {
        var O;
        return u == null ? Rn(this, y, w).year : (O = Rr(u, y, w), h > O && (h = O), tP.call(this, u, h, v, y, w));
      }
      function tP(u, h, v, y, w) {
        var O = rv(u, h, v, y, w), A = Cn(O.year, 0, O.dayOfYear);
        return this.year(A.getUTCFullYear()), this.month(A.getUTCMonth()), this.date(A.getUTCDate()), this;
      }
      j("Q", 0, "Qo", "quarter"), W("Q", Rt), Oe("Q", function(u, h) {
        h[kr] = (oe(u) - 1) * 3;
      });
      function rP(u) {
        return u == null ? Math.ceil((this.month() + 1) / 3) : this.month((u - 1) * 3 + this.month() % 3);
      }
      j("D", ["DD", 2], "Do", "date"), W("D", Ee, Dr), W("DD", Ee, Ie), W("Do", function(u, h) {
        return u ? h._dayOfMonthOrdinalParse || h._ordinalParse : h._dayOfMonthOrdinalParseLenient;
      }), Oe(["D", "DD"], gr), Oe("Do", function(u, h) {
        h[gr] = oe(u.match(Ee)[0]);
      });
      var Dv = $i("Date", !0);
      j("DDD", ["DDDD", 3], "DDDo", "dayOfYear"), W("DDD", ce), W("DDDD", at), Oe(["DDD", "DDDD"], function(u, h, v) {
        v._dayOfYear = oe(u);
      });
      function iP(u) {
        var h = Math.round(
          (this.clone().startOf("day") - this.clone().startOf("year")) / 864e5
        ) + 1;
        return u == null ? h : this.add(u - h, "d");
      }
      j("m", ["mm", 2], 0, "minute"), W("m", Ee, Dn), W("mm", Ee, Ie), Oe(["m", "mm"], ur);
      var nP = $i("Minutes", !1);
      j("s", ["ss", 2], 0, "second"), W("s", Ee, Dn), W("ss", Ee, Ie), Oe(["s", "ss"], Cr);
      var sP = $i("Seconds", !1);
      j("S", 0, 0, function() {
        return ~~(this.millisecond() / 100);
      }), j(0, ["SS", 2], 0, function() {
        return ~~(this.millisecond() / 10);
      }), j(0, ["SSS", 3], 0, "millisecond"), j(0, ["SSSS", 4], 0, function() {
        return this.millisecond() * 10;
      }), j(0, ["SSSSS", 5], 0, function() {
        return this.millisecond() * 100;
      }), j(0, ["SSSSSS", 6], 0, function() {
        return this.millisecond() * 1e3;
      }), j(0, ["SSSSSSS", 7], 0, function() {
        return this.millisecond() * 1e4;
      }), j(0, ["SSSSSSSS", 8], 0, function() {
        return this.millisecond() * 1e5;
      }), j(0, ["SSSSSSSSS", 9], 0, function() {
        return this.millisecond() * 1e6;
      }), W("S", ce, Rt), W("SS", ce, Ie), W("SSS", ce, at);
      var Kr, Ov;
      for (Kr = "SSSS"; Kr.length <= 9; Kr += "S")
        W(Kr, ee);
      function aP(u, h) {
        h[di] = oe(("0." + u) * 1e3);
      }
      for (Kr = "S"; Kr.length <= 9; Kr += "S")
        Oe(Kr, aP);
      Ov = $i("Milliseconds", !1), j("z", 0, 0, "zoneAbbr"), j("zz", 0, 0, "zoneName");
      function oP() {
        return this._isUTC ? "UTC" : "";
      }
      function lP() {
        return this._isUTC ? "Coordinated Universal Time" : "";
      }
      var $ = P.prototype;
      $.add = eR, $.calendar = oR, $.clone = lR, $.diff = mR, $.endOf = ER, $.format = bR, $.from = _R, $.fromNow = wR, $.to = xR, $.toNow = SR, $.get = yk, $.invalidAt = AR, $.isAfter = uR, $.isBefore = cR, $.isBetween = dR, $.isSame = hR, $.isSameOrAfter = fR, $.isSameOrBefore = pR, $.isValid = IR, $.lang = _v, $.locale = bv, $.localeData = wv, $.max = IC, $.min = PC, $.parsingFlags = MR, $.set = bk, $.startOf = TR, $.subtract = tR, $.toArray = CR, $.toObject = RR, $.toDate = kR, $.toISOString = gR, $.inspect = yR, typeof Symbol < "u" && Symbol.for != null && ($[Symbol.for("nodejs.util.inspect.custom")] = function() {
        return "Moment<" + this.format() + ">";
      }), $.toJSON = PR, $.toString = vR, $.unix = OR, $.valueOf = DR, $.creationData = NR, $.eraName = $R, $.eraNarrow = jR, $.eraAbbr = YR, $.eraYear = HR, $.year = Km, $.isLeapYear = gk, $.weekYear = KR, $.isoWeekYear = XR, $.quarter = $.quarters = rP, $.month = ev, $.daysInMonth = kk, $.week = $.weeks = qk, $.isoWeek = $.isoWeeks = Lk, $.weeksInYear = JR, $.weeksInWeekYear = eP, $.isoWeeksInYear = ZR, $.isoWeeksInISOWeekYear = QR, $.date = Dv, $.day = $.days = Xk, $.weekday = Zk, $.isoWeekday = Qk, $.dayOfYear = iP, $.hour = $.hours = sC, $.minute = $.minutes = nP, $.second = $.seconds = sP, $.millisecond = $.milliseconds = Ov, $.utcOffset = YC, $.utc = GC, $.local = BC, $.parseZone = WC, $.hasAlignedHourOffset = zC, $.isDST = UC, $.isLocal = KC, $.isUtcOffset = XC, $.isUtc = pv, $.isUTC = pv, $.zoneAbbr = oP, $.zoneName = lP, $.dates = q(
        "dates accessor is deprecated. Use date instead.",
        Dv
      ), $.months = q(
        "months accessor is deprecated. Use month instead",
        ev
      ), $.years = q(
        "years accessor is deprecated. Use year instead",
        Km
      ), $.zone = q(
        "moment().zone is deprecated, use moment().utcOffset instead. http://momentjs.com/guides/#/warnings/zone/",
        HC
      ), $.isDSTShifted = q(
        "isDSTShifted is deprecated. See http://momentjs.com/guides/#/warnings/dst-shifted/ for more information",
        VC
      );
      function uP(u) {
        return Ge(u * 1e3);
      }
      function cP() {
        return Ge.apply(null, arguments).parseZone();
      }
      function kv(u) {
        return u;
      }
      var Re = Y.prototype;
      Re.calendar = te, Re.longDateFormat = Lt, Re.invalidDate = Ft, Re.ordinal = Ct, Re.preparse = kv, Re.postformat = kv, Re.relativeTime = Jt, Re.pastFuture = L, Re.set = ve, Re.eras = qR, Re.erasParse = LR, Re.erasConvertYear = FR, Re.erasAbbrRegex = BR, Re.erasNameRegex = GR, Re.erasNarrowRegex = WR, Re.months = Tk, Re.monthsShort = Ek, Re.monthsParse = Ok, Re.monthsRegex = Rk, Re.monthsShortRegex = Ck, Re.week = Ik, Re.firstDayOfYear = Nk, Re.firstDayOfWeek = Ak, Re.weekdays = Wk, Re.weekdaysMin = Uk, Re.weekdaysShort = zk, Re.weekdaysParse = Kk, Re.weekdaysRegex = Jk, Re.weekdaysShortRegex = eC, Re.weekdaysMinRegex = tC, Re.isPM = iC, Re.meridiem = aC;
      function Hs(u, h, v, y) {
        var w = Pr(), O = b().set(y, h);
        return w[v](O, u);
      }
      function Cv(u, h, v) {
        if (f(u) && (h = u, u = void 0), u = u || "", h != null)
          return Hs(u, h, v, "month");
        var y, w = [];
        for (y = 0; y < 12; y++)
          w[y] = Hs(u, y, v, "month");
        return w;
      }
      function ao(u, h, v, y) {
        typeof u == "boolean" ? (f(h) && (v = h, h = void 0), h = h || "") : (h = u, v = h, u = !1, f(h) && (v = h, h = void 0), h = h || "");
        var w = Pr(), O = u ? w._week.dow : 0, A, U = [];
        if (v != null)
          return Hs(h, (v + O) % 7, y, "day");
        for (A = 0; A < 7; A++)
          U[A] = Hs(h, (A + O) % 7, y, "day");
        return U;
      }
      function dP(u, h) {
        return Cv(u, h, "months");
      }
      function hP(u, h) {
        return Cv(u, h, "monthsShort");
      }
      function fP(u, h, v) {
        return ao(u, h, v, "weekdays");
      }
      function pP(u, h, v) {
        return ao(u, h, v, "weekdaysShort");
      }
      function mP(u, h, v) {
        return ao(u, h, v, "weekdaysMin");
      }
      Vr("en", {
        eras: [
          {
            since: "0001-01-01",
            until: 1 / 0,
            offset: 1,
            name: "Anno Domini",
            narrow: "AD",
            abbr: "AD"
          },
          {
            since: "0000-12-31",
            until: -1 / 0,
            offset: 1,
            name: "Before Christ",
            narrow: "BC",
            abbr: "BC"
          }
        ],
        dayOfMonthOrdinalParse: /\d{1,2}(th|st|nd|rd)/,
        ordinal: function(u) {
          var h = u % 10, v = oe(u % 100 / 10) === 1 ? "th" : h === 1 ? "st" : h === 2 ? "nd" : h === 3 ? "rd" : "th";
          return u + v;
        }
      }), r.lang = q(
        "moment.lang is deprecated. Use moment.locale instead.",
        Vr
      ), r.langData = q(
        "moment.langData is deprecated. Use moment.localeData instead.",
        Pr
      );
      var Ir = Math.abs;
      function vP() {
        var u = this._data;
        return this._milliseconds = Ir(this._milliseconds), this._days = Ir(this._days), this._months = Ir(this._months), u.milliseconds = Ir(u.milliseconds), u.seconds = Ir(u.seconds), u.minutes = Ir(u.minutes), u.hours = Ir(u.hours), u.months = Ir(u.months), u.years = Ir(u.years), this;
      }
      function Rv(u, h, v, y) {
        var w = cr(h, v);
        return u._milliseconds += y * w._milliseconds, u._days += y * w._days, u._months += y * w._months, u._bubble();
      }
      function gP(u, h) {
        return Rv(this, u, h, 1);
      }
      function yP(u, h) {
        return Rv(this, u, h, -1);
      }
      function Pv(u) {
        return u < 0 ? Math.floor(u) : Math.ceil(u);
      }
      function bP() {
        var u = this._milliseconds, h = this._days, v = this._months, y = this._data, w, O, A, U, le;
        return u >= 0 && h >= 0 && v >= 0 || u <= 0 && h <= 0 && v <= 0 || (u += Pv(oo(v) + h) * 864e5, h = 0, v = 0), y.milliseconds = u % 1e3, w = _e(u / 1e3), y.seconds = w % 60, O = _e(w / 60), y.minutes = O % 60, A = _e(O / 60), y.hours = A % 24, h += _e(A / 24), le = _e(Iv(h)), v += le, h -= Pv(oo(le)), U = _e(v / 12), v %= 12, y.days = h, y.months = v, y.years = U, this;
      }
      function Iv(u) {
        return u * 4800 / 146097;
      }
      function oo(u) {
        return u * 146097 / 4800;
      }
      function _P(u) {
        if (!this.isValid())
          return NaN;
        var h, v, y = this._milliseconds;
        if (u = fe(u), u === "month" || u === "quarter" || u === "year")
          switch (h = this._days + y / 864e5, v = this._months + Iv(h), u) {
            case "month":
              return v;
            case "quarter":
              return v / 3;
            case "year":
              return v / 12;
          }
        else
          switch (h = this._days + Math.round(oo(this._months)), u) {
            case "week":
              return h / 7 + y / 6048e5;
            case "day":
              return h + y / 864e5;
            case "hour":
              return h * 24 + y / 36e5;
            case "minute":
              return h * 1440 + y / 6e4;
            case "second":
              return h * 86400 + y / 1e3;
            // Math.floor prevents floating point math errors here
            case "millisecond":
              return Math.floor(h * 864e5) + y;
            default:
              throw new Error("Unknown unit " + u);
          }
      }
      function Mr(u) {
        return function() {
          return this.as(u);
        };
      }
      var Mv = Mr("ms"), wP = Mr("s"), xP = Mr("m"), SP = Mr("h"), TP = Mr("d"), EP = Mr("w"), DP = Mr("M"), OP = Mr("Q"), kP = Mr("y"), CP = Mv;
      function RP() {
        return cr(this);
      }
      function PP(u) {
        return u = fe(u), this.isValid() ? this[u + "s"]() : NaN;
      }
      function fi(u) {
        return function() {
          return this.isValid() ? this._data[u] : NaN;
        };
      }
      var IP = fi("milliseconds"), MP = fi("seconds"), AP = fi("minutes"), NP = fi("hours"), qP = fi("days"), LP = fi("months"), FP = fi("years");
      function $P() {
        return _e(this.days() / 7);
      }
      var Ar = Math.round, Gi = {
        ss: 44,
        // a few seconds to seconds
        s: 45,
        // seconds to minute
        m: 45,
        // minutes to hour
        h: 22,
        // hours to day
        d: 26,
        // days to month/week
        w: null,
        // weeks to month
        M: 11
        // months to year
      };
      function jP(u, h, v, y, w) {
        return w.relativeTime(h || 1, !!v, u, y);
      }
      function YP(u, h, v, y) {
        var w = cr(u).abs(), O = Ar(w.as("s")), A = Ar(w.as("m")), U = Ar(w.as("h")), le = Ar(w.as("d")), Se = Ar(w.as("M")), Pt = Ar(w.as("w")), Nr = Ar(w.as("y")), Xr = O <= v.ss && ["s", O] || O < v.s && ["ss", O] || A <= 1 && ["m"] || A < v.m && ["mm", A] || U <= 1 && ["h"] || U < v.h && ["hh", U] || le <= 1 && ["d"] || le < v.d && ["dd", le];
        return v.w != null && (Xr = Xr || Pt <= 1 && ["w"] || Pt < v.w && ["ww", Pt]), Xr = Xr || Se <= 1 && ["M"] || Se < v.M && ["MM", Se] || Nr <= 1 && ["y"] || ["yy", Nr], Xr[2] = h, Xr[3] = +u > 0, Xr[4] = y, jP.apply(null, Xr);
      }
      function HP(u) {
        return u === void 0 ? Ar : typeof u == "function" ? (Ar = u, !0) : !1;
      }
      function GP(u, h) {
        return Gi[u] === void 0 ? !1 : h === void 0 ? Gi[u] : (Gi[u] = h, u === "s" && (Gi.ss = h - 1), !0);
      }
      function BP(u, h) {
        if (!this.isValid())
          return this.localeData().invalidDate();
        var v = !1, y = Gi, w, O;
        return typeof u == "object" && (h = u, u = !1), typeof u == "boolean" && (v = u), typeof h == "object" && (y = Object.assign({}, Gi, h), h.s != null && h.ss == null && (y.ss = h.s - 1)), w = this.localeData(), O = YP(this, !v, y, w), v && (O = w.pastFuture(+this, O)), w.postformat(O);
      }
      var lo = Math.abs;
      function Bi(u) {
        return (u > 0) - (u < 0) || +u;
      }
      function Gs() {
        if (!this.isValid())
          return this.localeData().invalidDate();
        var u = lo(this._milliseconds) / 1e3, h = lo(this._days), v = lo(this._months), y, w, O, A, U = this.asSeconds(), le, Se, Pt, Nr;
        return U ? (y = _e(u / 60), w = _e(y / 60), u %= 60, y %= 60, O = _e(v / 12), v %= 12, A = u ? u.toFixed(3).replace(/\.?0+$/, "") : "", le = U < 0 ? "-" : "", Se = Bi(this._months) !== Bi(U) ? "-" : "", Pt = Bi(this._days) !== Bi(U) ? "-" : "", Nr = Bi(this._milliseconds) !== Bi(U) ? "-" : "", le + "P" + (O ? Se + O + "Y" : "") + (v ? Se + v + "M" : "") + (h ? Pt + h + "D" : "") + (w || y || u ? "T" : "") + (w ? Nr + w + "H" : "") + (y ? Nr + y + "M" : "") + (u ? Nr + A + "S" : "")) : "P0D";
      }
      var ke = qs.prototype;
      ke.isValid = LC, ke.abs = vP, ke.add = gP, ke.subtract = yP, ke.as = _P, ke.asMilliseconds = Mv, ke.asSeconds = wP, ke.asMinutes = xP, ke.asHours = SP, ke.asDays = TP, ke.asWeeks = EP, ke.asMonths = DP, ke.asQuarters = OP, ke.asYears = kP, ke.valueOf = CP, ke._bubble = bP, ke.clone = RP, ke.get = PP, ke.milliseconds = IP, ke.seconds = MP, ke.minutes = AP, ke.hours = NP, ke.days = qP, ke.weeks = $P, ke.months = LP, ke.years = FP, ke.humanize = BP, ke.toISOString = Gs, ke.toString = Gs, ke.toJSON = Gs, ke.locale = bv, ke.localeData = wv, ke.toIsoString = q(
        "toIsoString() is deprecated. Please use toISOString() instead (notice the capitals)",
        Gs
      ), ke.lang = _v, j("X", 0, 0, "unix"), j("x", 0, 0, "valueOf"), W("x", Je), W("X", Fi), Oe("X", function(u, h, v) {
        v._d = new Date(parseFloat(u) * 1e3);
      }), Oe("x", function(u, h, v) {
        v._d = new Date(oe(u));
      });
      //! moment.js
      return r.version = "2.30.1", s(Ge), r.fn = $, r.min = MC, r.max = AC, r.now = NC, r.utc = b, r.unix = uP, r.months = dP, r.isDate = p, r.locale = Vr, r.invalid = k, r.duration = cr, r.isMoment = R, r.weekdays = fP, r.parseZone = cP, r.localeData = Pr, r.isDuration = Ls, r.monthsShort = hP, r.weekdaysMin = mP, r.defineLocale = Ka, r.updateLocale = cC, r.locales = dC, r.weekdaysShort = pP, r.normalizeUnits = fe, r.relativeTimeRounding = HP, r.relativeTimeThreshold = GP, r.calendarFormat = aR, r.prototype = $, r.HTML5_FMT = {
        DATETIME_LOCAL: "YYYY-MM-DDTHH:mm",
        // <input type="datetime-local" />
        DATETIME_LOCAL_SECONDS: "YYYY-MM-DDTHH:mm:ss",
        // <input type="datetime-local" step="1" />
        DATETIME_LOCAL_MS: "YYYY-MM-DDTHH:mm:ss.SSS",
        // <input type="datetime-local" step="0.001" />
        DATE: "YYYY-MM-DD",
        // <input type="date" />
        TIME: "HH:mm",
        // <input type="time" />
        TIME_SECONDS: "HH:mm:ss",
        // <input type="time" step="1" />
        TIME_MS: "HH:mm:ss.SSS",
        // <input type="time" step="0.001" />
        WEEK: "GGGG-[W]WW",
        // <input type="week" />
        MONTH: "YYYY-MM"
        // <input type="month" />
      }, r;
    }));
  })(ia)), ia.exports;
}
var oM = wp.exports, Kv;
function lM() {
  return Kv || (Kv = 1, (function(n, e) {
    (function(t, r) {
      r(typeof fr == "function" ? pr() : t.moment);
    })(oM, (function(t) {
      //! moment.js locale configuration
      function r(a, o, l, c) {
        var d = {
          m: ["eine Minute", "einer Minute"],
          h: ["eine Stunde", "einer Stunde"],
          d: ["ein Tag", "einem Tag"],
          dd: [a + " Tage", a + " Tagen"],
          w: ["eine Woche", "einer Woche"],
          M: ["ein Monat", "einem Monat"],
          MM: [a + " Monate", a + " Monaten"],
          y: ["ein Jahr", "einem Jahr"],
          yy: [a + " Jahre", a + " Jahren"]
        };
        return o ? d[l][0] : d[l][1];
      }
      var s = t.defineLocale("de", {
        months: "Januar_Februar_März_April_Mai_Juni_Juli_August_September_Oktober_November_Dezember".split(
          "_"
        ),
        monthsShort: "Jan._Feb._März_Apr._Mai_Juni_Juli_Aug._Sep._Okt._Nov._Dez.".split("_"),
        monthsParseExact: !0,
        weekdays: "Sonntag_Montag_Dienstag_Mittwoch_Donnerstag_Freitag_Samstag".split(
          "_"
        ),
        weekdaysShort: "So._Mo._Di._Mi._Do._Fr._Sa.".split("_"),
        weekdaysMin: "So_Mo_Di_Mi_Do_Fr_Sa".split("_"),
        weekdaysParseExact: !0,
        longDateFormat: {
          LT: "HH:mm",
          LTS: "HH:mm:ss",
          L: "DD.MM.YYYY",
          LL: "D. MMMM YYYY",
          LLL: "D. MMMM YYYY HH:mm",
          LLLL: "dddd, D. MMMM YYYY HH:mm"
        },
        calendar: {
          sameDay: "[heute um] LT [Uhr]",
          sameElse: "L",
          nextDay: "[morgen um] LT [Uhr]",
          nextWeek: "dddd [um] LT [Uhr]",
          lastDay: "[gestern um] LT [Uhr]",
          lastWeek: "[letzten] dddd [um] LT [Uhr]"
        },
        relativeTime: {
          future: "in %s",
          past: "vor %s",
          s: "ein paar Sekunden",
          ss: "%d Sekunden",
          m: r,
          mm: "%d Minuten",
          h: r,
          hh: "%d Stunden",
          d: r,
          dd: r,
          w: r,
          ww: "%d Wochen",
          M: r,
          MM: r,
          y: r,
          yy: r
        },
        dayOfMonthOrdinalParse: /\d{1,2}\./,
        ordinal: "%d.",
        week: {
          dow: 1,
          // Monday is the first day of the week.
          doy: 4
          // The week that contains Jan 4th is the first week of the year.
        }
      });
      return s;
    }));
  })()), wp.exports;
}
lM();
var xp = { exports: {} }, uM = xp.exports, Xv;
function cM() {
  return Xv || (Xv = 1, (function(n, e) {
    (function(t, r) {
      r(typeof fr == "function" ? pr() : t.moment);
    })(uM, (function(t) {
      //! moment.js locale configuration
      var r = "ene._feb._mar._abr._may._jun._jul._ago._sep._oct._nov._dic.".split(
        "_"
      ), s = "ene_feb_mar_abr_may_jun_jul_ago_sep_oct_nov_dic".split("_"), a = [
        /^ene/i,
        /^feb/i,
        /^mar/i,
        /^abr/i,
        /^may/i,
        /^jun/i,
        /^jul/i,
        /^ago/i,
        /^sep/i,
        /^oct/i,
        /^nov/i,
        /^dic/i
      ], o = /^(enero|febrero|marzo|abril|mayo|junio|julio|agosto|septiembre|octubre|noviembre|diciembre|ene\.?|feb\.?|mar\.?|abr\.?|may\.?|jun\.?|jul\.?|ago\.?|sep\.?|oct\.?|nov\.?|dic\.?)/i, l = t.defineLocale("es", {
        months: "enero_febrero_marzo_abril_mayo_junio_julio_agosto_septiembre_octubre_noviembre_diciembre".split(
          "_"
        ),
        monthsShort: function(c, d) {
          return c ? /-MMM-/.test(d) ? s[c.month()] : r[c.month()] : r;
        },
        monthsRegex: o,
        monthsShortRegex: o,
        monthsStrictRegex: /^(enero|febrero|marzo|abril|mayo|junio|julio|agosto|septiembre|octubre|noviembre|diciembre)/i,
        monthsShortStrictRegex: /^(ene\.?|feb\.?|mar\.?|abr\.?|may\.?|jun\.?|jul\.?|ago\.?|sep\.?|oct\.?|nov\.?|dic\.?)/i,
        monthsParse: a,
        longMonthsParse: a,
        shortMonthsParse: a,
        weekdays: "domingo_lunes_martes_miércoles_jueves_viernes_sábado".split("_"),
        weekdaysShort: "dom._lun._mar._mié._jue._vie._sáb.".split("_"),
        weekdaysMin: "do_lu_ma_mi_ju_vi_sá".split("_"),
        weekdaysParseExact: !0,
        longDateFormat: {
          LT: "H:mm",
          LTS: "H:mm:ss",
          L: "DD/MM/YYYY",
          LL: "D [de] MMMM [de] YYYY",
          LLL: "D [de] MMMM [de] YYYY H:mm",
          LLLL: "dddd, D [de] MMMM [de] YYYY H:mm"
        },
        calendar: {
          sameDay: function() {
            return "[hoy a la" + (this.hours() !== 1 ? "s" : "") + "] LT";
          },
          nextDay: function() {
            return "[mañana a la" + (this.hours() !== 1 ? "s" : "") + "] LT";
          },
          nextWeek: function() {
            return "dddd [a la" + (this.hours() !== 1 ? "s" : "") + "] LT";
          },
          lastDay: function() {
            return "[ayer a la" + (this.hours() !== 1 ? "s" : "") + "] LT";
          },
          lastWeek: function() {
            return "[el] dddd [pasado a la" + (this.hours() !== 1 ? "s" : "") + "] LT";
          },
          sameElse: "L"
        },
        relativeTime: {
          future: "en %s",
          past: "hace %s",
          s: "unos segundos",
          ss: "%d segundos",
          m: "un minuto",
          mm: "%d minutos",
          h: "una hora",
          hh: "%d horas",
          d: "un día",
          dd: "%d días",
          w: "una semana",
          ww: "%d semanas",
          M: "un mes",
          MM: "%d meses",
          y: "un año",
          yy: "%d años"
        },
        dayOfMonthOrdinalParse: /\d{1,2}º/,
        ordinal: "%dº",
        week: {
          dow: 1,
          // Monday is the first day of the week.
          doy: 4
          // The week that contains Jan 4th is the first week of the year.
        },
        invalidDate: "Fecha inválida"
      });
      return l;
    }));
  })()), xp.exports;
}
cM();
var Sp = { exports: {} }, dM = Sp.exports, Zv;
function hM() {
  return Zv || (Zv = 1, (function(n, e) {
    (function(t, r) {
      r(typeof fr == "function" ? pr() : t.moment);
    })(dM, (function(t) {
      //! moment.js locale configuration
      var r = /^(janvier|février|mars|avril|mai|juin|juillet|août|septembre|octobre|novembre|décembre)/i, s = /(janv\.?|févr\.?|mars|avr\.?|mai|juin|juil\.?|août|sept\.?|oct\.?|nov\.?|déc\.?)/i, a = /(janv\.?|févr\.?|mars|avr\.?|mai|juin|juil\.?|août|sept\.?|oct\.?|nov\.?|déc\.?|janvier|février|mars|avril|mai|juin|juillet|août|septembre|octobre|novembre|décembre)/i, o = [
        /^janv/i,
        /^févr/i,
        /^mars/i,
        /^avr/i,
        /^mai/i,
        /^juin/i,
        /^juil/i,
        /^août/i,
        /^sept/i,
        /^oct/i,
        /^nov/i,
        /^déc/i
      ], l = t.defineLocale("fr", {
        months: "janvier_février_mars_avril_mai_juin_juillet_août_septembre_octobre_novembre_décembre".split(
          "_"
        ),
        monthsShort: "janv._févr._mars_avr._mai_juin_juil._août_sept._oct._nov._déc.".split(
          "_"
        ),
        monthsRegex: a,
        monthsShortRegex: a,
        monthsStrictRegex: r,
        monthsShortStrictRegex: s,
        monthsParse: o,
        longMonthsParse: o,
        shortMonthsParse: o,
        weekdays: "dimanche_lundi_mardi_mercredi_jeudi_vendredi_samedi".split("_"),
        weekdaysShort: "dim._lun._mar._mer._jeu._ven._sam.".split("_"),
        weekdaysMin: "di_lu_ma_me_je_ve_sa".split("_"),
        weekdaysParseExact: !0,
        longDateFormat: {
          LT: "HH:mm",
          LTS: "HH:mm:ss",
          L: "DD/MM/YYYY",
          LL: "D MMMM YYYY",
          LLL: "D MMMM YYYY HH:mm",
          LLLL: "dddd D MMMM YYYY HH:mm"
        },
        calendar: {
          sameDay: "[Aujourd’hui à] LT",
          nextDay: "[Demain à] LT",
          nextWeek: "dddd [à] LT",
          lastDay: "[Hier à] LT",
          lastWeek: "dddd [dernier à] LT",
          sameElse: "L"
        },
        relativeTime: {
          future: "dans %s",
          past: "il y a %s",
          s: "quelques secondes",
          ss: "%d secondes",
          m: "une minute",
          mm: "%d minutes",
          h: "une heure",
          hh: "%d heures",
          d: "un jour",
          dd: "%d jours",
          w: "une semaine",
          ww: "%d semaines",
          M: "un mois",
          MM: "%d mois",
          y: "un an",
          yy: "%d ans"
        },
        dayOfMonthOrdinalParse: /\d{1,2}(er|)/,
        ordinal: function(c, d) {
          switch (d) {
            // TODO: Return 'e' when day of month > 1. Move this case inside
            // block for masculine words below.
            // See https://github.com/moment/moment/issues/3375
            case "D":
              return c + (c === 1 ? "er" : "");
            // Words with masculine grammatical gender: mois, trimestre, jour
            default:
            case "M":
            case "Q":
            case "DDD":
            case "d":
              return c + (c === 1 ? "er" : "e");
            // Words with feminine grammatical gender: semaine
            case "w":
            case "W":
              return c + (c === 1 ? "re" : "e");
          }
        },
        week: {
          dow: 1,
          // Monday is the first day of the week.
          doy: 4
          // The week that contains Jan 4th is the first week of the year.
        }
      });
      return l;
    }));
  })()), Sp.exports;
}
hM();
var Tp = { exports: {} }, fM = Tp.exports, Qv;
function pM() {
  return Qv || (Qv = 1, (function(n, e) {
    (function(t, r) {
      r(typeof fr == "function" ? pr() : t.moment);
    })(fM, (function(t) {
      //! moment.js locale configuration
      var r = t.defineLocale("it", {
        months: "gennaio_febbraio_marzo_aprile_maggio_giugno_luglio_agosto_settembre_ottobre_novembre_dicembre".split(
          "_"
        ),
        monthsShort: "gen_feb_mar_apr_mag_giu_lug_ago_set_ott_nov_dic".split("_"),
        weekdays: "domenica_lunedì_martedì_mercoledì_giovedì_venerdì_sabato".split(
          "_"
        ),
        weekdaysShort: "dom_lun_mar_mer_gio_ven_sab".split("_"),
        weekdaysMin: "do_lu_ma_me_gi_ve_sa".split("_"),
        longDateFormat: {
          LT: "HH:mm",
          LTS: "HH:mm:ss",
          L: "DD/MM/YYYY",
          LL: "D MMMM YYYY",
          LLL: "D MMMM YYYY HH:mm",
          LLLL: "dddd D MMMM YYYY HH:mm"
        },
        calendar: {
          sameDay: function() {
            return "[Oggi a" + (this.hours() > 1 ? "lle " : this.hours() === 0 ? " " : "ll'") + "]LT";
          },
          nextDay: function() {
            return "[Domani a" + (this.hours() > 1 ? "lle " : this.hours() === 0 ? " " : "ll'") + "]LT";
          },
          nextWeek: function() {
            return "dddd [a" + (this.hours() > 1 ? "lle " : this.hours() === 0 ? " " : "ll'") + "]LT";
          },
          lastDay: function() {
            return "[Ieri a" + (this.hours() > 1 ? "lle " : this.hours() === 0 ? " " : "ll'") + "]LT";
          },
          lastWeek: function() {
            switch (this.day()) {
              case 0:
                return "[La scorsa] dddd [a" + (this.hours() > 1 ? "lle " : this.hours() === 0 ? " " : "ll'") + "]LT";
              default:
                return "[Lo scorso] dddd [a" + (this.hours() > 1 ? "lle " : this.hours() === 0 ? " " : "ll'") + "]LT";
            }
          },
          sameElse: "L"
        },
        relativeTime: {
          future: "tra %s",
          past: "%s fa",
          s: "alcuni secondi",
          ss: "%d secondi",
          m: "un minuto",
          mm: "%d minuti",
          h: "un'ora",
          hh: "%d ore",
          d: "un giorno",
          dd: "%d giorni",
          w: "una settimana",
          ww: "%d settimane",
          M: "un mese",
          MM: "%d mesi",
          y: "un anno",
          yy: "%d anni"
        },
        dayOfMonthOrdinalParse: /\d{1,2}º/,
        ordinal: "%dº",
        week: {
          dow: 1,
          // Monday is the first day of the week.
          doy: 4
          // The week that contains Jan 4th is the first week of the year.
        }
      });
      return r;
    }));
  })()), Tp.exports;
}
pM();
var Ep = { exports: {} }, mM = Ep.exports, Jv;
function vM() {
  return Jv || (Jv = 1, (function(n, e) {
    (function(t, r) {
      r(typeof fr == "function" ? pr() : t.moment);
    })(mM, (function(t) {
      //! moment.js locale configuration
      var r = t.defineLocale("ja", {
        eras: [
          {
            since: "2019-05-01",
            offset: 1,
            name: "令和",
            narrow: "㋿",
            abbr: "R"
          },
          {
            since: "1989-01-08",
            until: "2019-04-30",
            offset: 1,
            name: "平成",
            narrow: "㍻",
            abbr: "H"
          },
          {
            since: "1926-12-25",
            until: "1989-01-07",
            offset: 1,
            name: "昭和",
            narrow: "㍼",
            abbr: "S"
          },
          {
            since: "1912-07-30",
            until: "1926-12-24",
            offset: 1,
            name: "大正",
            narrow: "㍽",
            abbr: "T"
          },
          {
            since: "1873-01-01",
            until: "1912-07-29",
            offset: 6,
            name: "明治",
            narrow: "㍾",
            abbr: "M"
          },
          {
            since: "0001-01-01",
            until: "1873-12-31",
            offset: 1,
            name: "西暦",
            narrow: "AD",
            abbr: "AD"
          },
          {
            since: "0000-12-31",
            until: -1 / 0,
            offset: 1,
            name: "紀元前",
            narrow: "BC",
            abbr: "BC"
          }
        ],
        eraYearOrdinalRegex: /(元|\d+)年/,
        eraYearOrdinalParse: function(s, a) {
          return a[1] === "元" ? 1 : parseInt(a[1] || s, 10);
        },
        months: "1月_2月_3月_4月_5月_6月_7月_8月_9月_10月_11月_12月".split("_"),
        monthsShort: "1月_2月_3月_4月_5月_6月_7月_8月_9月_10月_11月_12月".split(
          "_"
        ),
        weekdays: "日曜日_月曜日_火曜日_水曜日_木曜日_金曜日_土曜日".split("_"),
        weekdaysShort: "日_月_火_水_木_金_土".split("_"),
        weekdaysMin: "日_月_火_水_木_金_土".split("_"),
        longDateFormat: {
          LT: "HH:mm",
          LTS: "HH:mm:ss",
          L: "YYYY/MM/DD",
          LL: "YYYY年M月D日",
          LLL: "YYYY年M月D日 HH:mm",
          LLLL: "YYYY年M月D日 dddd HH:mm",
          l: "YYYY/MM/DD",
          ll: "YYYY年M月D日",
          lll: "YYYY年M月D日 HH:mm",
          llll: "YYYY年M月D日(ddd) HH:mm"
        },
        meridiemParse: /午前|午後/i,
        isPM: function(s) {
          return s === "午後";
        },
        meridiem: function(s, a, o) {
          return s < 12 ? "午前" : "午後";
        },
        calendar: {
          sameDay: "[今日] LT",
          nextDay: "[明日] LT",
          nextWeek: function(s) {
            return s.week() !== this.week() ? "[来週]dddd LT" : "dddd LT";
          },
          lastDay: "[昨日] LT",
          lastWeek: function(s) {
            return this.week() !== s.week() ? "[先週]dddd LT" : "dddd LT";
          },
          sameElse: "L"
        },
        dayOfMonthOrdinalParse: /\d{1,2}日/,
        ordinal: function(s, a) {
          switch (a) {
            case "y":
              return s === 1 ? "元年" : s + "年";
            case "d":
            case "D":
            case "DDD":
              return s + "日";
            default:
              return s;
          }
        },
        relativeTime: {
          future: "%s後",
          past: "%s前",
          s: "数秒",
          ss: "%d秒",
          m: "1分",
          mm: "%d分",
          h: "1時間",
          hh: "%d時間",
          d: "1日",
          dd: "%d日",
          M: "1ヶ月",
          MM: "%dヶ月",
          y: "1年",
          yy: "%d年"
        }
      });
      return r;
    }));
  })()), Ep.exports;
}
vM();
var Dp = { exports: {} }, gM = Dp.exports, eg;
function yM() {
  return eg || (eg = 1, (function(n, e) {
    (function(t, r) {
      r(typeof fr == "function" ? pr() : t.moment);
    })(gM, (function(t) {
      //! moment.js locale configuration
      var r = "jan._feb._mrt._apr._mei_jun._jul._aug._sep._okt._nov._dec.".split("_"), s = "jan_feb_mrt_apr_mei_jun_jul_aug_sep_okt_nov_dec".split("_"), a = [
        /^jan/i,
        /^feb/i,
        /^(maart|mrt\.?)$/i,
        /^apr/i,
        /^mei$/i,
        /^jun[i.]?$/i,
        /^jul[i.]?$/i,
        /^aug/i,
        /^sep/i,
        /^okt/i,
        /^nov/i,
        /^dec/i
      ], o = /^(januari|februari|maart|april|mei|ju[nl]i|augustus|september|oktober|november|december|jan\.?|feb\.?|mrt\.?|apr\.?|ju[nl]\.?|aug\.?|sep\.?|okt\.?|nov\.?|dec\.?)/i, l = t.defineLocale("nl", {
        months: "januari_februari_maart_april_mei_juni_juli_augustus_september_oktober_november_december".split(
          "_"
        ),
        monthsShort: function(c, d) {
          return c ? /-MMM-/.test(d) ? s[c.month()] : r[c.month()] : r;
        },
        monthsRegex: o,
        monthsShortRegex: o,
        monthsStrictRegex: /^(januari|februari|maart|april|mei|ju[nl]i|augustus|september|oktober|november|december)/i,
        monthsShortStrictRegex: /^(jan\.?|feb\.?|mrt\.?|apr\.?|mei|ju[nl]\.?|aug\.?|sep\.?|okt\.?|nov\.?|dec\.?)/i,
        monthsParse: a,
        longMonthsParse: a,
        shortMonthsParse: a,
        weekdays: "zondag_maandag_dinsdag_woensdag_donderdag_vrijdag_zaterdag".split("_"),
        weekdaysShort: "zo._ma._di._wo._do._vr._za.".split("_"),
        weekdaysMin: "zo_ma_di_wo_do_vr_za".split("_"),
        weekdaysParseExact: !0,
        longDateFormat: {
          LT: "HH:mm",
          LTS: "HH:mm:ss",
          L: "DD-MM-YYYY",
          LL: "D MMMM YYYY",
          LLL: "D MMMM YYYY HH:mm",
          LLLL: "dddd D MMMM YYYY HH:mm"
        },
        calendar: {
          sameDay: "[vandaag om] LT",
          nextDay: "[morgen om] LT",
          nextWeek: "dddd [om] LT",
          lastDay: "[gisteren om] LT",
          lastWeek: "[afgelopen] dddd [om] LT",
          sameElse: "L"
        },
        relativeTime: {
          future: "over %s",
          past: "%s geleden",
          s: "een paar seconden",
          ss: "%d seconden",
          m: "één minuut",
          mm: "%d minuten",
          h: "één uur",
          hh: "%d uur",
          d: "één dag",
          dd: "%d dagen",
          w: "één week",
          ww: "%d weken",
          M: "één maand",
          MM: "%d maanden",
          y: "één jaar",
          yy: "%d jaar"
        },
        dayOfMonthOrdinalParse: /\d{1,2}(ste|de)/,
        ordinal: function(c) {
          return c + (c === 1 || c === 8 || c >= 20 ? "ste" : "de");
        },
        week: {
          dow: 1,
          // Monday is the first day of the week.
          doy: 4
          // The week that contains Jan 4th is the first week of the year.
        }
      });
      return l;
    }));
  })()), Dp.exports;
}
yM();
var Op = { exports: {} }, bM = Op.exports, tg;
function _M() {
  return tg || (tg = 1, (function(n, e) {
    (function(t, r) {
      r(typeof fr == "function" ? pr() : t.moment);
    })(bM, (function(t) {
      //! moment.js locale configuration
      var r = "styczeń_luty_marzec_kwiecień_maj_czerwiec_lipiec_sierpień_wrzesień_październik_listopad_grudzień".split(
        "_"
      ), s = "stycznia_lutego_marca_kwietnia_maja_czerwca_lipca_sierpnia_września_października_listopada_grudnia".split(
        "_"
      ), a = [
        /^sty/i,
        /^lut/i,
        /^mar/i,
        /^kwi/i,
        /^maj/i,
        /^cze/i,
        /^lip/i,
        /^sie/i,
        /^wrz/i,
        /^paź/i,
        /^lis/i,
        /^gru/i
      ];
      function o(d) {
        return d % 10 < 5 && d % 10 > 1 && ~~(d / 10) % 10 !== 1;
      }
      function l(d, f, p) {
        var m = d + " ";
        switch (p) {
          case "ss":
            return m + (o(d) ? "sekundy" : "sekund");
          case "m":
            return f ? "minuta" : "minutę";
          case "mm":
            return m + (o(d) ? "minuty" : "minut");
          case "h":
            return f ? "godzina" : "godzinę";
          case "hh":
            return m + (o(d) ? "godziny" : "godzin");
          case "ww":
            return m + (o(d) ? "tygodnie" : "tygodni");
          case "MM":
            return m + (o(d) ? "miesiące" : "miesięcy");
          case "yy":
            return m + (o(d) ? "lata" : "lat");
        }
      }
      var c = t.defineLocale("pl", {
        months: function(d, f) {
          return d ? /D MMMM/.test(f) ? s[d.month()] : r[d.month()] : r;
        },
        monthsShort: "sty_lut_mar_kwi_maj_cze_lip_sie_wrz_paź_lis_gru".split("_"),
        monthsParse: a,
        longMonthsParse: a,
        shortMonthsParse: a,
        weekdays: "niedziela_poniedziałek_wtorek_środa_czwartek_piątek_sobota".split("_"),
        weekdaysShort: "ndz_pon_wt_śr_czw_pt_sob".split("_"),
        weekdaysMin: "Nd_Pn_Wt_Śr_Cz_Pt_So".split("_"),
        longDateFormat: {
          LT: "HH:mm",
          LTS: "HH:mm:ss",
          L: "DD.MM.YYYY",
          LL: "D MMMM YYYY",
          LLL: "D MMMM YYYY HH:mm",
          LLLL: "dddd, D MMMM YYYY HH:mm"
        },
        calendar: {
          sameDay: "[Dziś o] LT",
          nextDay: "[Jutro o] LT",
          nextWeek: function() {
            switch (this.day()) {
              case 0:
                return "[W niedzielę o] LT";
              case 2:
                return "[We wtorek o] LT";
              case 3:
                return "[W środę o] LT";
              case 6:
                return "[W sobotę o] LT";
              default:
                return "[W] dddd [o] LT";
            }
          },
          lastDay: "[Wczoraj o] LT",
          lastWeek: function() {
            switch (this.day()) {
              case 0:
                return "[W zeszłą niedzielę o] LT";
              case 3:
                return "[W zeszłą środę o] LT";
              case 6:
                return "[W zeszłą sobotę o] LT";
              default:
                return "[W zeszły] dddd [o] LT";
            }
          },
          sameElse: "L"
        },
        relativeTime: {
          future: "za %s",
          past: "%s temu",
          s: "kilka sekund",
          ss: l,
          m: l,
          mm: l,
          h: l,
          hh: l,
          d: "1 dzień",
          dd: "%d dni",
          w: "tydzień",
          ww: l,
          M: "miesiąc",
          MM: l,
          y: "rok",
          yy: l
        },
        dayOfMonthOrdinalParse: /\d{1,2}\./,
        ordinal: "%d.",
        week: {
          dow: 1,
          // Monday is the first day of the week.
          doy: 4
          // The week that contains Jan 4th is the first week of the year.
        }
      });
      return c;
    }));
  })()), Op.exports;
}
_M();
var kp = { exports: {} }, wM = kp.exports, rg;
function xM() {
  return rg || (rg = 1, (function(n, e) {
    (function(t, r) {
      r(typeof fr == "function" ? pr() : t.moment);
    })(wM, (function(t) {
      //! moment.js locale configuration
      function r(l, c) {
        var d = l.split("_");
        return c % 10 === 1 && c % 100 !== 11 ? d[0] : c % 10 >= 2 && c % 10 <= 4 && (c % 100 < 10 || c % 100 >= 20) ? d[1] : d[2];
      }
      function s(l, c, d) {
        var f = {
          ss: c ? "секунда_секунды_секунд" : "секунду_секунды_секунд",
          mm: c ? "минута_минуты_минут" : "минуту_минуты_минут",
          hh: "час_часа_часов",
          dd: "день_дня_дней",
          ww: "неделя_недели_недель",
          MM: "месяц_месяца_месяцев",
          yy: "год_года_лет"
        };
        return d === "m" ? c ? "минута" : "минуту" : l + " " + r(f[d], +l);
      }
      var a = [
        /^янв/i,
        /^фев/i,
        /^мар/i,
        /^апр/i,
        /^ма[йя]/i,
        /^июн/i,
        /^июл/i,
        /^авг/i,
        /^сен/i,
        /^окт/i,
        /^ноя/i,
        /^дек/i
      ], o = t.defineLocale("ru", {
        months: {
          format: "января_февраля_марта_апреля_мая_июня_июля_августа_сентября_октября_ноября_декабря".split(
            "_"
          ),
          standalone: "январь_февраль_март_апрель_май_июнь_июль_август_сентябрь_октябрь_ноябрь_декабрь".split(
            "_"
          )
        },
        monthsShort: {
          // по CLDR именно "июл." и "июн.", но какой смысл менять букву на точку?
          format: "янв._февр._мар._апр._мая_июня_июля_авг._сент._окт._нояб._дек.".split(
            "_"
          ),
          standalone: "янв._февр._март_апр._май_июнь_июль_авг._сент._окт._нояб._дек.".split(
            "_"
          )
        },
        weekdays: {
          standalone: "воскресенье_понедельник_вторник_среда_четверг_пятница_суббота".split(
            "_"
          ),
          format: "воскресенье_понедельник_вторник_среду_четверг_пятницу_субботу".split(
            "_"
          ),
          isFormat: /\[ ?[Вв] ?(?:прошлую|следующую|эту)? ?] ?dddd/
        },
        weekdaysShort: "вс_пн_вт_ср_чт_пт_сб".split("_"),
        weekdaysMin: "вс_пн_вт_ср_чт_пт_сб".split("_"),
        monthsParse: a,
        longMonthsParse: a,
        shortMonthsParse: a,
        // полные названия с падежами, по три буквы, для некоторых, по 4 буквы, сокращения с точкой и без точки
        monthsRegex: /^(январ[ья]|янв\.?|феврал[ья]|февр?\.?|марта?|мар\.?|апрел[ья]|апр\.?|ма[йя]|июн[ья]|июн\.?|июл[ья]|июл\.?|августа?|авг\.?|сентябр[ья]|сент?\.?|октябр[ья]|окт\.?|ноябр[ья]|нояб?\.?|декабр[ья]|дек\.?)/i,
        // копия предыдущего
        monthsShortRegex: /^(январ[ья]|янв\.?|феврал[ья]|февр?\.?|марта?|мар\.?|апрел[ья]|апр\.?|ма[йя]|июн[ья]|июн\.?|июл[ья]|июл\.?|августа?|авг\.?|сентябр[ья]|сент?\.?|октябр[ья]|окт\.?|ноябр[ья]|нояб?\.?|декабр[ья]|дек\.?)/i,
        // полные названия с падежами
        monthsStrictRegex: /^(январ[яь]|феврал[яь]|марта?|апрел[яь]|ма[яй]|июн[яь]|июл[яь]|августа?|сентябр[яь]|октябр[яь]|ноябр[яь]|декабр[яь])/i,
        // Выражение, которое соответствует только сокращённым формам
        monthsShortStrictRegex: /^(янв\.|февр?\.|мар[т.]|апр\.|ма[яй]|июн[ья.]|июл[ья.]|авг\.|сент?\.|окт\.|нояб?\.|дек\.)/i,
        longDateFormat: {
          LT: "H:mm",
          LTS: "H:mm:ss",
          L: "DD.MM.YYYY",
          LL: "D MMMM YYYY г.",
          LLL: "D MMMM YYYY г., H:mm",
          LLLL: "dddd, D MMMM YYYY г., H:mm"
        },
        calendar: {
          sameDay: "[Сегодня, в] LT",
          nextDay: "[Завтра, в] LT",
          lastDay: "[Вчера, в] LT",
          nextWeek: function(l) {
            if (l.week() !== this.week())
              switch (this.day()) {
                case 0:
                  return "[В следующее] dddd, [в] LT";
                case 1:
                case 2:
                case 4:
                  return "[В следующий] dddd, [в] LT";
                case 3:
                case 5:
                case 6:
                  return "[В следующую] dddd, [в] LT";
              }
            else
              return this.day() === 2 ? "[Во] dddd, [в] LT" : "[В] dddd, [в] LT";
          },
          lastWeek: function(l) {
            if (l.week() !== this.week())
              switch (this.day()) {
                case 0:
                  return "[В прошлое] dddd, [в] LT";
                case 1:
                case 2:
                case 4:
                  return "[В прошлый] dddd, [в] LT";
                case 3:
                case 5:
                case 6:
                  return "[В прошлую] dddd, [в] LT";
              }
            else
              return this.day() === 2 ? "[Во] dddd, [в] LT" : "[В] dddd, [в] LT";
          },
          sameElse: "L"
        },
        relativeTime: {
          future: "через %s",
          past: "%s назад",
          s: "несколько секунд",
          ss: s,
          m: s,
          mm: s,
          h: "час",
          hh: s,
          d: "день",
          dd: s,
          w: "неделя",
          ww: s,
          M: "месяц",
          MM: s,
          y: "год",
          yy: s
        },
        meridiemParse: /ночи|утра|дня|вечера/i,
        isPM: function(l) {
          return /^(дня|вечера)$/.test(l);
        },
        meridiem: function(l, c, d) {
          return l < 4 ? "ночи" : l < 12 ? "утра" : l < 17 ? "дня" : "вечера";
        },
        dayOfMonthOrdinalParse: /\d{1,2}-(й|го|я)/,
        ordinal: function(l, c) {
          switch (c) {
            case "M":
            case "d":
            case "DDD":
              return l + "-й";
            case "D":
              return l + "-го";
            case "w":
            case "W":
              return l + "-я";
            default:
              return l;
          }
        },
        week: {
          dow: 1,
          // Monday is the first day of the week.
          doy: 4
          // The week that contains Jan 4th is the first week of the year.
        }
      });
      return o;
    }));
  })()), kp.exports;
}
xM();
var Cp = { exports: {} }, SM = Cp.exports, ig;
function TM() {
  return ig || (ig = 1, (function(n, e) {
    (function(t, r) {
      r(typeof fr == "function" ? pr() : t.moment);
    })(SM, (function(t) {
      //! moment.js locale configuration
      var r = {
        1: "'inci",
        5: "'inci",
        8: "'inci",
        70: "'inci",
        80: "'inci",
        2: "'nci",
        7: "'nci",
        20: "'nci",
        50: "'nci",
        3: "'üncü",
        4: "'üncü",
        100: "'üncü",
        6: "'ncı",
        9: "'uncu",
        10: "'uncu",
        30: "'uncu",
        60: "'ıncı",
        90: "'ıncı"
      }, s = t.defineLocale("tr", {
        months: "Ocak_Şubat_Mart_Nisan_Mayıs_Haziran_Temmuz_Ağustos_Eylül_Ekim_Kasım_Aralık".split(
          "_"
        ),
        monthsShort: "Oca_Şub_Mar_Nis_May_Haz_Tem_Ağu_Eyl_Eki_Kas_Ara".split("_"),
        weekdays: "Pazar_Pazartesi_Salı_Çarşamba_Perşembe_Cuma_Cumartesi".split(
          "_"
        ),
        weekdaysShort: "Paz_Pzt_Sal_Çar_Per_Cum_Cmt".split("_"),
        weekdaysMin: "Pz_Pt_Sa_Ça_Pe_Cu_Ct".split("_"),
        meridiem: function(a, o, l) {
          return a < 12 ? l ? "öö" : "ÖÖ" : l ? "ös" : "ÖS";
        },
        meridiemParse: /öö|ÖÖ|ös|ÖS/,
        isPM: function(a) {
          return a === "ös" || a === "ÖS";
        },
        longDateFormat: {
          LT: "HH:mm",
          LTS: "HH:mm:ss",
          L: "DD.MM.YYYY",
          LL: "D MMMM YYYY",
          LLL: "D MMMM YYYY HH:mm",
          LLLL: "dddd, D MMMM YYYY HH:mm"
        },
        calendar: {
          sameDay: "[bugün saat] LT",
          nextDay: "[yarın saat] LT",
          nextWeek: "[gelecek] dddd [saat] LT",
          lastDay: "[dün] LT",
          lastWeek: "[geçen] dddd [saat] LT",
          sameElse: "L"
        },
        relativeTime: {
          future: "%s sonra",
          past: "%s önce",
          s: "birkaç saniye",
          ss: "%d saniye",
          m: "bir dakika",
          mm: "%d dakika",
          h: "bir saat",
          hh: "%d saat",
          d: "bir gün",
          dd: "%d gün",
          w: "bir hafta",
          ww: "%d hafta",
          M: "bir ay",
          MM: "%d ay",
          y: "bir yıl",
          yy: "%d yıl"
        },
        ordinal: function(a, o) {
          switch (o) {
            case "d":
            case "D":
            case "Do":
            case "DD":
              return a;
            default:
              if (a === 0)
                return a + "'ıncı";
              var l = a % 10, c = a % 100 - l, d = a >= 100 ? 100 : null;
              return a + (r[l] || r[c] || r[d]);
          }
        },
        week: {
          dow: 1,
          // Monday is the first day of the week.
          doy: 7
          // The week that contains Jan 7th is the first week of the year.
        }
      });
      return s;
    }));
  })()), Cp.exports;
}
TM();
var Rp = { exports: {} }, EM = Rp.exports, ng;
function DM() {
  return ng || (ng = 1, (function(n, e) {
    (function(t, r) {
      r(typeof fr == "function" ? pr() : t.moment);
    })(EM, (function(t) {
      //! moment.js locale configuration
      function r(c, d) {
        var f = c.split("_");
        return d % 10 === 1 && d % 100 !== 11 ? f[0] : d % 10 >= 2 && d % 10 <= 4 && (d % 100 < 10 || d % 100 >= 20) ? f[1] : f[2];
      }
      function s(c, d, f) {
        var p = {
          ss: d ? "секунда_секунди_секунд" : "секунду_секунди_секунд",
          mm: d ? "хвилина_хвилини_хвилин" : "хвилину_хвилини_хвилин",
          hh: d ? "година_години_годин" : "годину_години_годин",
          dd: "день_дні_днів",
          MM: "місяць_місяці_місяців",
          yy: "рік_роки_років"
        };
        return f === "m" ? d ? "хвилина" : "хвилину" : f === "h" ? d ? "година" : "годину" : c + " " + r(p[f], +c);
      }
      function a(c, d) {
        var f = {
          nominative: "неділя_понеділок_вівторок_середа_четвер_п’ятниця_субота".split(
            "_"
          ),
          accusative: "неділю_понеділок_вівторок_середу_четвер_п’ятницю_суботу".split(
            "_"
          ),
          genitive: "неділі_понеділка_вівторка_середи_четверга_п’ятниці_суботи".split(
            "_"
          )
        }, p;
        return c === !0 ? f.nominative.slice(1, 7).concat(f.nominative.slice(0, 1)) : c ? (p = /(\[[ВвУу]\]) ?dddd/.test(d) ? "accusative" : /\[?(?:минулої|наступної)? ?\] ?dddd/.test(d) ? "genitive" : "nominative", f[p][c.day()]) : f.nominative;
      }
      function o(c) {
        return function() {
          return c + "о" + (this.hours() === 11 ? "б" : "") + "] LT";
        };
      }
      var l = t.defineLocale("uk", {
        months: {
          format: "січня_лютого_березня_квітня_травня_червня_липня_серпня_вересня_жовтня_листопада_грудня".split(
            "_"
          ),
          standalone: "січень_лютий_березень_квітень_травень_червень_липень_серпень_вересень_жовтень_листопад_грудень".split(
            "_"
          )
        },
        monthsShort: "січ_лют_бер_квіт_трав_черв_лип_серп_вер_жовт_лист_груд".split(
          "_"
        ),
        weekdays: a,
        weekdaysShort: "нд_пн_вт_ср_чт_пт_сб".split("_"),
        weekdaysMin: "нд_пн_вт_ср_чт_пт_сб".split("_"),
        longDateFormat: {
          LT: "HH:mm",
          LTS: "HH:mm:ss",
          L: "DD.MM.YYYY",
          LL: "D MMMM YYYY р.",
          LLL: "D MMMM YYYY р., HH:mm",
          LLLL: "dddd, D MMMM YYYY р., HH:mm"
        },
        calendar: {
          sameDay: o("[Сьогодні "),
          nextDay: o("[Завтра "),
          lastDay: o("[Вчора "),
          nextWeek: o("[У] dddd ["),
          lastWeek: function() {
            switch (this.day()) {
              case 0:
              case 3:
              case 5:
              case 6:
                return o("[Минулої] dddd [").call(this);
              case 1:
              case 2:
              case 4:
                return o("[Минулого] dddd [").call(this);
            }
          },
          sameElse: "L"
        },
        relativeTime: {
          future: "за %s",
          past: "%s тому",
          s: "декілька секунд",
          ss: s,
          m: s,
          mm: s,
          h: "годину",
          hh: s,
          d: "день",
          dd: s,
          M: "місяць",
          MM: s,
          y: "рік",
          yy: s
        },
        // M. E.: those two are virtually unused but a user might want to implement them for his/her website for some reason
        meridiemParse: /ночі|ранку|дня|вечора/,
        isPM: function(c) {
          return /^(дня|вечора)$/.test(c);
        },
        meridiem: function(c, d, f) {
          return c < 4 ? "ночі" : c < 12 ? "ранку" : c < 17 ? "дня" : "вечора";
        },
        dayOfMonthOrdinalParse: /\d{1,2}-(й|го)/,
        ordinal: function(c, d) {
          switch (d) {
            case "M":
            case "d":
            case "DDD":
            case "w":
            case "W":
              return c + "-й";
            case "D":
              return c + "-го";
            default:
              return c;
          }
        },
        week: {
          dow: 1,
          // Monday is the first day of the week.
          doy: 7
          // The week that contains Jan 7th is the first week of the year.
        }
      });
      return l;
    }));
  })()), Rp.exports;
}
DM();
function qt(n, e) {
  e === void 0 && (e = {});
  var t = e.insertAt;
  if (!(!n || typeof document > "u")) {
    var r = document.head || document.getElementsByTagName("head")[0], s = document.createElement("style");
    s.type = "text/css", t === "top" && r.firstChild ? r.insertBefore(s, r.firstChild) : r.appendChild(s), s.styleSheet ? s.styleSheet.cssText = n : s.appendChild(document.createTextNode(n));
  }
}
var OM = `.vis .overlay {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;

  /* Must be displayed above for example selected Timeline items */
  z-index: 10;
}

.vis-active {
  box-shadow: 0 0 10px #86d5f8;
}
`;
qt(OM);
var kM = `/* override some bootstrap styles screwing up the timelines css */

.vis [class*="span"] {
  min-height: 0;
  width: auto;
}
`;
qt(kM);
var CM = `div.vis-configuration {
  position: relative;
  display: block;
  float: left;
  font-size: 12px;
}

div.vis-configuration-wrapper {
  display: block;
  width: 700px;
}

div.vis-configuration-wrapper::after {
  clear: both;
  content: "";
  display: block;
}

div.vis-configuration.vis-config-option-container {
  display: block;
  width: 495px;
  background-color: #ffffff;
  border: 2px solid #f7f8fa;
  border-radius: 4px;
  margin-top: 20px;
  left: 10px;
  padding-left: 5px;
}

div.vis-configuration.vis-config-button {
  display: block;
  width: 495px;
  height: 25px;
  vertical-align: middle;
  line-height: 25px;
  background-color: #f7f8fa;
  border: 2px solid #ceced0;
  border-radius: 4px;
  margin-top: 20px;
  left: 10px;
  padding-left: 5px;
  cursor: pointer;
  margin-bottom: 30px;
}

div.vis-configuration.vis-config-button.hover {
  background-color: #4588e6;
  border: 2px solid #214373;
  color: #ffffff;
}

div.vis-configuration.vis-config-item {
  display: block;
  float: left;
  width: 495px;
  height: 25px;
  vertical-align: middle;
  line-height: 25px;
}

div.vis-configuration.vis-config-item.vis-config-s2 {
  left: 10px;
  background-color: #f7f8fa;
  padding-left: 5px;
  border-radius: 3px;
}
div.vis-configuration.vis-config-item.vis-config-s3 {
  left: 20px;
  background-color: #e4e9f0;
  padding-left: 5px;
  border-radius: 3px;
}
div.vis-configuration.vis-config-item.vis-config-s4 {
  left: 30px;
  background-color: #cfd8e6;
  padding-left: 5px;
  border-radius: 3px;
}

div.vis-configuration.vis-config-header {
  font-size: 18px;
  font-weight: bold;
}

div.vis-configuration.vis-config-label {
  width: 120px;
  height: 25px;
  line-height: 25px;
}

div.vis-configuration.vis-config-label.vis-config-s3 {
  width: 110px;
}
div.vis-configuration.vis-config-label.vis-config-s4 {
  width: 100px;
}

div.vis-configuration.vis-config-colorBlock {
  top: 1px;
  width: 30px;
  height: 19px;
  border: 1px solid #444444;
  border-radius: 2px;
  padding: 0px;
  margin: 0px;
  cursor: pointer;
}

input.vis-configuration.vis-config-checkbox {
  left: -5px;
}

input.vis-configuration.vis-config-rangeinput {
  position: relative;
  top: -5px;
  width: 60px;
  /*height:13px;*/
  padding: 1px;
  margin: 0;
  pointer-events: none;
}

input.vis-configuration.vis-config-range {
  /*removes default webkit styles*/
  -webkit-appearance: none;

  /*fix for FF unable to apply focus style bug */
  border: 0px solid white;
  background-color: rgba(0, 0, 0, 0);

  /*required for proper track sizing in FF*/
  width: 300px;
  height: 20px;
}
input.vis-configuration.vis-config-range::-webkit-slider-runnable-track {
  width: 300px;
  height: 5px;
  background: #dedede; /* Old browsers */
  background: -moz-linear-gradient(top, #dedede 0%, #c8c8c8 99%); /* FF3.6+ */
  background: -webkit-gradient(
    linear,
    left top,
    left bottom,
    color-stop(0%, #dedede),
    color-stop(99%, #c8c8c8)
  ); /* Chrome,Safari4+ */
  background: -webkit-linear-gradient(
    top,
    #dedede 0%,
    #c8c8c8 99%
  ); /* Chrome10+,Safari5.1+ */
  background: -o-linear-gradient(
    top,
    #dedede 0%,
    #c8c8c8 99%
  ); /* Opera 11.10+ */
  background: -ms-linear-gradient(top, #dedede 0%, #c8c8c8 99%); /* IE10+ */
  background: linear-gradient(to bottom, #dedede 0%, #c8c8c8 99%); /* W3C */
  filter: progid:DXImageTransform.Microsoft.gradient( startColorstr='#dedede', endColorstr='#c8c8c8',GradientType=0 ); /* IE6-9 */

  border: 1px solid #999999;
  box-shadow: #aaaaaa 0px 0px 3px 0px;
  border-radius: 3px;
}
input.vis-configuration.vis-config-range::-webkit-slider-thumb {
  -webkit-appearance: none;
  border: 1px solid #14334b;
  height: 17px;
  width: 17px;
  border-radius: 50%;
  background: #3876c2; /* Old browsers */
  background: -moz-linear-gradient(top, #3876c2 0%, #385380 100%); /* FF3.6+ */
  background: -webkit-gradient(
    linear,
    left top,
    left bottom,
    color-stop(0%, #3876c2),
    color-stop(100%, #385380)
  ); /* Chrome,Safari4+ */
  background: -webkit-linear-gradient(
    top,
    #3876c2 0%,
    #385380 100%
  ); /* Chrome10+,Safari5.1+ */
  background: -o-linear-gradient(
    top,
    #3876c2 0%,
    #385380 100%
  ); /* Opera 11.10+ */
  background: -ms-linear-gradient(top, #3876c2 0%, #385380 100%); /* IE10+ */
  background: linear-gradient(to bottom, #3876c2 0%, #385380 100%); /* W3C */
  filter: progid:DXImageTransform.Microsoft.gradient( startColorstr='#3876c2', endColorstr='#385380',GradientType=0 ); /* IE6-9 */
  box-shadow: #111927 0px 0px 1px 0px;
  margin-top: -7px;
}
input.vis-configuration.vis-config-range:focus {
  outline: none;
}
input.vis-configuration.vis-config-range:focus::-webkit-slider-runnable-track {
  background: #9d9d9d; /* Old browsers */
  background: -moz-linear-gradient(top, #9d9d9d 0%, #c8c8c8 99%); /* FF3.6+ */
  background: -webkit-gradient(
    linear,
    left top,
    left bottom,
    color-stop(0%, #9d9d9d),
    color-stop(99%, #c8c8c8)
  ); /* Chrome,Safari4+ */
  background: -webkit-linear-gradient(
    top,
    #9d9d9d 0%,
    #c8c8c8 99%
  ); /* Chrome10+,Safari5.1+ */
  background: -o-linear-gradient(
    top,
    #9d9d9d 0%,
    #c8c8c8 99%
  ); /* Opera 11.10+ */
  background: -ms-linear-gradient(top, #9d9d9d 0%, #c8c8c8 99%); /* IE10+ */
  background: linear-gradient(to bottom, #9d9d9d 0%, #c8c8c8 99%); /* W3C */
  filter: progid:DXImageTransform.Microsoft.gradient( startColorstr='#9d9d9d', endColorstr='#c8c8c8',GradientType=0 ); /* IE6-9 */
}

input.vis-configuration.vis-config-range::-moz-range-track {
  width: 300px;
  height: 10px;
  background: #dedede; /* Old browsers */
  background: -moz-linear-gradient(top, #dedede 0%, #c8c8c8 99%); /* FF3.6+ */
  background: -webkit-gradient(
    linear,
    left top,
    left bottom,
    color-stop(0%, #dedede),
    color-stop(99%, #c8c8c8)
  ); /* Chrome,Safari4+ */
  background: -webkit-linear-gradient(
    top,
    #dedede 0%,
    #c8c8c8 99%
  ); /* Chrome10+,Safari5.1+ */
  background: -o-linear-gradient(
    top,
    #dedede 0%,
    #c8c8c8 99%
  ); /* Opera 11.10+ */
  background: -ms-linear-gradient(top, #dedede 0%, #c8c8c8 99%); /* IE10+ */
  background: linear-gradient(to bottom, #dedede 0%, #c8c8c8 99%); /* W3C */
  filter: progid:DXImageTransform.Microsoft.gradient( startColorstr='#dedede', endColorstr='#c8c8c8',GradientType=0 ); /* IE6-9 */

  border: 1px solid #999999;
  box-shadow: #aaaaaa 0px 0px 3px 0px;
  border-radius: 3px;
}
input.vis-configuration.vis-config-range::-moz-range-thumb {
  border: none;
  height: 16px;
  width: 16px;

  border-radius: 50%;
  background: #385380;
}

/*hide the outline behind the border*/
input.vis-configuration.vis-config-range:-moz-focusring {
  outline: 1px solid white;
  outline-offset: -1px;
}

input.vis-configuration.vis-config-range::-ms-track {
  width: 300px;
  height: 5px;

  /*remove bg colour from the track, we'll use ms-fill-lower and ms-fill-upper instead */
  background: transparent;

  /*leave room for the larger thumb to overflow with a transparent border */
  border-color: transparent;
  border-width: 6px 0;

  /*remove default tick marks*/
  color: transparent;
}
input.vis-configuration.vis-config-range::-ms-fill-lower {
  background: #777;
  border-radius: 10px;
}
input.vis-configuration.vis-config-range::-ms-fill-upper {
  background: #ddd;
  border-radius: 10px;
}
input.vis-configuration.vis-config-range::-ms-thumb {
  border: none;
  height: 16px;
  width: 16px;
  border-radius: 50%;
  background: #385380;
}
input.vis-configuration.vis-config-range:focus::-ms-fill-lower {
  background: #888;
}
input.vis-configuration.vis-config-range:focus::-ms-fill-upper {
  background: #ccc;
}

.vis-configuration-popup {
  position: absolute;
  background: rgba(57, 76, 89, 0.85);
  border: 2px solid #f2faff;
  line-height: 30px;
  height: 30px;
  width: 150px;
  text-align: center;
  color: #ffffff;
  font-size: 14px;
  border-radius: 4px;
  -webkit-transition: opacity 0.3s ease-in-out;
  -moz-transition: opacity 0.3s ease-in-out;
  transition: opacity 0.3s ease-in-out;
}
.vis-configuration-popup:after,
.vis-configuration-popup:before {
  left: 100%;
  top: 50%;
  border: solid transparent;
  content: " ";
  height: 0;
  width: 0;
  position: absolute;
  pointer-events: none;
}

.vis-configuration-popup:after {
  border-color: rgba(136, 183, 213, 0);
  border-left-color: rgba(57, 76, 89, 0.85);
  border-width: 8px;
  margin-top: -8px;
}
.vis-configuration-popup:before {
  border-color: rgba(194, 225, 245, 0);
  border-left-color: #f2faff;
  border-width: 12px;
  margin-top: -12px;
}
`;
qt(CM);
var RM = `div.vis-tooltip {
  position: absolute;
  visibility: hidden;
  padding: 5px;
  white-space: nowrap;

  font-family: verdana;
  font-size: 14px;
  color: #000000;
  background-color: #f5f4ed;

  -moz-border-radius: 3px;
  -webkit-border-radius: 3px;
  border-radius: 3px;
  border: 1px solid #808074;

  box-shadow: 3px 3px 10px rgba(0, 0, 0, 0.2);
  pointer-events: none;

  z-index: 5;
}
`;
qt(RM);
var PM = `.vis-timeline {
  /*
  -webkit-transition: height .4s ease-in-out;
  transition:         height .4s ease-in-out;
  */
}

.vis-panel {
  /*
  -webkit-transition: height .4s ease-in-out, top .4s ease-in-out;
  transition:         height .4s ease-in-out, top .4s ease-in-out;
  */
}

.vis-axis {
  /*
  -webkit-transition: top .4s ease-in-out;
  transition:         top .4s ease-in-out;
  */
}

/* TODO: get animation working nicely

.vis-item {
  -webkit-transition: top .4s ease-in-out;
  transition:         top .4s ease-in-out;
}

.vis-item.line {
  -webkit-transition: height .4s ease-in-out, top .4s ease-in-out;
  transition:         height .4s ease-in-out, top .4s ease-in-out;
}
/**/
`;
qt(PM);
var IM = `.vis-current-time {
  background-color: #ff7f6e;
  width: 2px;
  z-index: 1;
  pointer-events: none;
}

.vis-rolling-mode-btn {
  height: 40px;
  width: 40px;
  position: absolute;
  top: 7px;
  right: 20px;
  border-radius: 50%;
  font-size: 28px;
  cursor: pointer;
  opacity: 0.8;
  color: white;
  font-weight: bold;
  text-align: center;
  background: #3876c2;
}
.vis-rolling-mode-btn:before {
  content: "\\26F6";
}

.vis-rolling-mode-btn:hover {
  opacity: 1;
}
`;
qt(IM);
var MM = `.vis-custom-time {
  background-color: #6e94ff;
  width: 2px;
  cursor: move;
  z-index: 1;
}

.vis-custom-time > .vis-custom-time-marker {
  background-color: inherit;
  color: white;
  font-size: 12px;
  white-space: nowrap;
  padding: 3px 5px;
  top: 0px;
  cursor: initial;
  z-index: inherit;
}
`;
qt(MM);
var AM = `.vis-panel.vis-background.vis-horizontal .vis-grid.vis-horizontal {
  position: absolute;
  width: 100%;
  height: 0;
  border-bottom: 1px solid;
}

.vis-panel.vis-background.vis-horizontal .vis-grid.vis-minor {
  border-color: #e5e5e5;
}

.vis-panel.vis-background.vis-horizontal .vis-grid.vis-major {
  border-color: #bfbfbf;
}

.vis-data-axis .vis-y-axis.vis-major {
  width: 100%;
  position: absolute;
  color: #4d4d4d;
  white-space: nowrap;
}

.vis-data-axis .vis-y-axis.vis-major.vis-measure {
  padding: 0;
  margin: 0;
  border: 0;
  visibility: hidden;
  width: auto;
}

.vis-data-axis .vis-y-axis.vis-minor {
  position: absolute;
  width: 100%;
  color: #bebebe;
  white-space: nowrap;
}

.vis-data-axis .vis-y-axis.vis-minor.vis-measure {
  padding: 0;
  margin: 0;
  border: 0;
  visibility: hidden;
  width: auto;
}

.vis-data-axis .vis-y-axis.vis-title {
  position: absolute;
  color: #4d4d4d;
  white-space: nowrap;
  bottom: 20px;
  text-align: center;
}

.vis-data-axis .vis-y-axis.vis-title.vis-measure {
  padding: 0;
  margin: 0;
  visibility: hidden;
  width: auto;
}

.vis-data-axis .vis-y-axis.vis-title.vis-left {
  bottom: 0;
  -webkit-transform-origin: left top;
  -moz-transform-origin: left top;
  -ms-transform-origin: left top;
  -o-transform-origin: left top;
  transform-origin: left bottom;
  -webkit-transform: rotate(-90deg);
  -moz-transform: rotate(-90deg);
  -ms-transform: rotate(-90deg);
  -o-transform: rotate(-90deg);
  transform: rotate(-90deg);
}

.vis-data-axis .vis-y-axis.vis-title.vis-right {
  bottom: 0;
  -webkit-transform-origin: right bottom;
  -moz-transform-origin: right bottom;
  -ms-transform-origin: right bottom;
  -o-transform-origin: right bottom;
  transform-origin: right bottom;
  -webkit-transform: rotate(90deg);
  -moz-transform: rotate(90deg);
  -ms-transform: rotate(90deg);
  -o-transform: rotate(90deg);
  transform: rotate(90deg);
}

.vis-legend {
  background-color: rgba(247, 252, 255, 0.65);
  padding: 5px;
  border: 1px solid #b3b3b3;
  box-shadow: 2px 2px 10px rgba(154, 154, 154, 0.55);
}

.vis-legend-text {
  /*font-size: 10px;*/
  white-space: nowrap;
  display: inline-block;
}
`;
qt(AM);
var NM = `.vis-item {
  position: absolute;
  color: #1a1a1a;
  border-color: #97b0f8;
  border-width: 1px;
  background-color: #d5ddf6;
  display: inline-block;
  z-index: 1;
  /*overflow: hidden;*/
}

.vis-item.vis-selected {
  border-color: #ffc200;
  background-color: #fff785;

  /* z-index must be higher than the z-index of custom time bar and current time bar */
  z-index: 2;
}

.vis-editable.vis-selected {
  cursor: move;
}

.vis-item.vis-point.vis-selected {
  background-color: #fff785;
}

.vis-item.vis-box {
  text-align: center;
  border-style: solid;
  border-radius: 2px;
}

.vis-item.vis-point {
  background: none;
}

.vis-item.vis-dot {
  position: absolute;
  padding: 0;
  border-width: 4px;
  border-style: solid;
  border-radius: 4px;
}

.vis-item.vis-range {
  border-style: solid;
  border-radius: 2px;
  box-sizing: border-box;
}

.vis-item.vis-background {
  border: none;
  background-color: rgba(213, 221, 246, 0.4);
  box-sizing: border-box;
  padding: 0;
  margin: 0;
}

.vis-item .vis-item-overflow {
  position: relative;
  width: 100%;
  height: 100%;
  padding: 0;
  margin: 0;
  overflow: hidden;
}

.vis-item-visible-frame {
  white-space: nowrap;
}

.vis-item.vis-range .vis-item-content {
  position: relative;
  display: inline-block;
}

.vis-item.vis-background .vis-item-content {
  position: absolute;
  display: inline-block;
}

.vis-item.vis-line {
  padding: 0;
  position: absolute;
  width: 0;
  border-left-width: 1px;
  border-left-style: solid;
}

.vis-item .vis-item-content {
  white-space: nowrap;
  box-sizing: border-box;
  padding: 5px;
}

.vis-item .vis-onUpdateTime-tooltip {
  position: absolute;
  background: #4f81bd;
  color: white;
  width: 200px;
  text-align: center;
  white-space: nowrap;
  padding: 5px;
  border-radius: 1px;
  transition: 0.4s;
  -o-transition: 0.4s;
  -moz-transition: 0.4s;
  -webkit-transition: 0.4s;
}

.vis-item .vis-delete,
.vis-item .vis-delete-rtl {
  position: absolute;
  top: 0px;
  width: 24px;
  height: 24px;
  box-sizing: border-box;
  padding: 0px 5px;
  cursor: pointer;

  -webkit-transition: background 0.2s linear;
  -moz-transition: background 0.2s linear;
  -ms-transition: background 0.2s linear;
  -o-transition: background 0.2s linear;
  transition: background 0.2s linear;
}

.vis-item .vis-delete {
  right: -24px;
}

.vis-item .vis-delete-rtl {
  left: -24px;
}

.vis-item .vis-delete:after,
.vis-item .vis-delete-rtl:after {
  content: "\\00D7"; /* MULTIPLICATION SIGN */
  color: red;
  font-family: arial, sans-serif;
  font-size: 22px;
  font-weight: bold;

  -webkit-transition: color 0.2s linear;
  -moz-transition: color 0.2s linear;
  -ms-transition: color 0.2s linear;
  -o-transition: color 0.2s linear;
  transition: color 0.2s linear;
}

.vis-item .vis-delete:hover,
.vis-item .vis-delete-rtl:hover {
  background: red;
}

.vis-item .vis-delete:hover:after,
.vis-item .vis-delete-rtl:hover:after {
  color: white;
}

.vis-item .vis-drag-center {
  position: absolute;
  width: 100%;
  height: 100%;
  top: 0;
  left: 0px;
  cursor: move;
}

.vis-item.vis-range .vis-drag-left {
  position: absolute;
  width: 24px;
  max-width: 20%;
  min-width: 2px;
  height: 100%;
  top: 0;
  left: -4px;

  cursor: w-resize;
}

.vis-item.vis-range .vis-drag-right {
  position: absolute;
  width: 24px;
  max-width: 20%;
  min-width: 2px;
  height: 100%;
  top: 0;
  right: -4px;

  cursor: e-resize;
}

.vis-range.vis-item.vis-readonly .vis-drag-left,
.vis-range.vis-item.vis-readonly .vis-drag-right {
  cursor: auto;
}

.vis-item.vis-cluster {
  vertical-align: center;
  text-align: center;
  border-style: solid;
  border-radius: 2px;
}

.vis-item.vis-cluster-line {
  padding: 0;
  position: absolute;
  width: 0;
  border-left-width: 1px;
  border-left-style: solid;
}

.vis-item.vis-cluster-dot {
  position: absolute;
  padding: 0;
  border-width: 4px;
  border-style: solid;
  border-radius: 4px;
}
`;
qt(NM);
var qM = `.vis-itemset {
  position: relative;
  padding: 0;
  margin: 0;

  box-sizing: border-box;
}

.vis-itemset .vis-background,
.vis-itemset .vis-foreground {
  position: absolute;
  width: 100%;
  height: 100%;
  overflow: visible;
}

.vis-axis {
  position: absolute;
  width: 100%;
  height: 0;
  left: 0;
  z-index: 1;
}

.vis-foreground .vis-group {
  position: relative;
  box-sizing: border-box;
  border-bottom: 1px solid #bfbfbf;
}

.vis-foreground .vis-group:last-child {
  border-bottom: none;
}

.vis-nesting-group {
  cursor: pointer;
}

.vis-label.vis-nested-group.vis-group-level-unknown-but-gte1 {
  background: #f5f5f5;
}
.vis-label.vis-nested-group.vis-group-level-0 {
  background-color: #ffffff;
}
.vis-ltr .vis-label.vis-nested-group.vis-group-level-0 .vis-inner {
  padding-left: 0;
}
.vis-rtl .vis-label.vis-nested-group.vis-group-level-0 .vis-inner {
  padding-right: 0;
}
.vis-label.vis-nested-group.vis-group-level-1 {
  background-color: rgba(0, 0, 0, 0.05);
}
.vis-ltr .vis-label.vis-nested-group.vis-group-level-1 .vis-inner {
  padding-left: 15px;
}
.vis-rtl .vis-label.vis-nested-group.vis-group-level-1 .vis-inner {
  padding-right: 15px;
}
.vis-label.vis-nested-group.vis-group-level-2 {
  background-color: rgba(0, 0, 0, 0.1);
}
.vis-ltr .vis-label.vis-nested-group.vis-group-level-2 .vis-inner {
  padding-left: 30px;
}
.vis-rtl .vis-label.vis-nested-group.vis-group-level-2 .vis-inner {
  padding-right: 30px;
}
.vis-label.vis-nested-group.vis-group-level-3 {
  background-color: rgba(0, 0, 0, 0.15);
}
.vis-ltr .vis-label.vis-nested-group.vis-group-level-3 .vis-inner {
  padding-left: 45px;
}
.vis-rtl .vis-label.vis-nested-group.vis-group-level-3 .vis-inner {
  padding-right: 45px;
}
.vis-label.vis-nested-group.vis-group-level-4 {
  background-color: rgba(0, 0, 0, 0.2);
}
.vis-ltr .vis-label.vis-nested-group.vis-group-level-4 .vis-inner {
  padding-left: 60px;
}
.vis-rtl .vis-label.vis-nested-group.vis-group-level-4 .vis-inner {
  padding-right: 60px;
}
.vis-label.vis-nested-group.vis-group-level-5 {
  background-color: rgba(0, 0, 0, 0.25);
}
.vis-ltr .vis-label.vis-nested-group.vis-group-level-5 .vis-inner {
  padding-left: 75px;
}
.vis-rtl .vis-label.vis-nested-group.vis-group-level-5 .vis-inner {
  padding-right: 75px;
}
.vis-label.vis-nested-group.vis-group-level-6 {
  background-color: rgba(0, 0, 0, 0.3);
}
.vis-ltr .vis-label.vis-nested-group.vis-group-level-6 .vis-inner {
  padding-left: 90px;
}
.vis-rtl .vis-label.vis-nested-group.vis-group-level-6 .vis-inner {
  padding-right: 90px;
}
.vis-label.vis-nested-group.vis-group-level-7 {
  background-color: rgba(0, 0, 0, 0.35);
}
.vis-ltr .vis-label.vis-nested-group.vis-group-level-7 .vis-inner {
  padding-left: 105px;
}
.vis-rtl .vis-label.vis-nested-group.vis-group-level-7 .vis-inner {
  padding-right: 105px;
}
.vis-label.vis-nested-group.vis-group-level-8 {
  background-color: rgba(0, 0, 0, 0.4);
}
.vis-ltr .vis-label.vis-nested-group.vis-group-level-8 .vis-inner {
  padding-left: 120px;
}
.vis-rtl .vis-label.vis-nested-group.vis-group-level-8 .vis-inner {
  padding-right: 120px;
}
.vis-label.vis-nested-group.vis-group-level-9 {
  background-color: rgba(0, 0, 0, 0.45);
}
.vis-ltr .vis-label.vis-nested-group.vis-group-level-9 .vis-inner {
  padding-left: 135px;
}
.vis-rtl .vis-label.vis-nested-group.vis-group-level-9 .vis-inner {
  padding-right: 135px;
}
/* default takes over beginning with level-10 (thats why we add .vis-nested-group
  to the selectors above, to have higher specifity than these rules for the defaults) */
.vis-label.vis-nested-group {
  background-color: rgba(0, 0, 0, 0.5);
}
.vis-ltr .vis-label.vis-nested-group .vis-inner {
  padding-left: 150px;
}
.vis-rtl .vis-label.vis-nested-group .vis-inner {
  padding-right: 150px;
}

.vis-group-level-unknown-but-gte1 {
  border: 1px solid red;
}

/* expanded/collapsed indicators */
.vis-label.vis-nesting-group:before,
.vis-label.vis-nesting-group:before {
  display: inline-block;
  width: 15px;
}
.vis-label.vis-nesting-group.expanded:before {
  content: "\\25BC";
}
.vis-label.vis-nesting-group.collapsed:before {
  content: "\\25B6";
}
.vis-rtl .vis-label.vis-nesting-group.collapsed:before {
  content: "\\25C0";
}
/* compensate missing expanded/collapsed indicator, but only at levels > 0 */
.vis-ltr .vis-label:not(.vis-nesting-group):not(.vis-group-level-0) {
  padding-left: 15px;
}
.vis-rtl .vis-label:not(.vis-nesting-group):not(.vis-group-level-0) {
  padding-right: 15px;
}

.vis-overlay {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  z-index: 10;
}
`;
qt(qM);
var LM = `.vis-labelset {
  position: relative;

  overflow: hidden;

  box-sizing: border-box;
}

.vis-labelset .vis-label {
  position: relative;
  left: 0;
  top: 0;
  width: 100%;
  color: #4d4d4d;

  box-sizing: border-box;
}

.vis-labelset .vis-label {
  border-bottom: 1px solid #bfbfbf;
}

.vis-labelset .vis-label.draggable {
  cursor: pointer;
}

.vis-group-is-dragging {
  background: rgba(0, 0, 0, 0.1);
}

.vis-labelset .vis-label:last-child {
  border-bottom: none;
}

.vis-labelset .vis-label .vis-inner {
  display: inline-block;
  padding: 5px;
}

.vis-labelset .vis-label .vis-inner.vis-hidden {
  padding: 0;
}
`;
qt(LM);
var FM = `.vis-panel {
  position: absolute;

  padding: 0;
  margin: 0;

  box-sizing: border-box;
}

.vis-panel.vis-center,
.vis-panel.vis-left,
.vis-panel.vis-right,
.vis-panel.vis-top,
.vis-panel.vis-bottom {
  border: 1px #bfbfbf;
}

.vis-panel.vis-center,
.vis-panel.vis-left,
.vis-panel.vis-right {
  border-top-style: solid;
  border-bottom-style: solid;
  overflow: hidden;
}

.vis-left.vis-panel.vis-vertical-scroll,
.vis-right.vis-panel.vis-vertical-scroll {
  height: 100%;
  overflow-x: hidden;
  overflow-y: scroll;
}

.vis-left.vis-panel.vis-vertical-scroll {
  direction: rtl;
}

.vis-left.vis-panel.vis-vertical-scroll .vis-content {
  direction: ltr;
}

.vis-right.vis-panel.vis-vertical-scroll {
  direction: ltr;
}

.vis-right.vis-panel.vis-vertical-scroll .vis-content {
  direction: rtl;
}

.vis-panel.vis-center,
.vis-panel.vis-top,
.vis-panel.vis-bottom {
  border-left-style: solid;
  border-right-style: solid;
}

.vis-background {
  overflow: hidden;
}

.vis-panel > .vis-content {
  position: relative;
}

.vis-panel .vis-shadow {
  position: absolute;
  width: 100%;
  height: 1px;
  box-shadow: 0 0 10px rgba(0, 0, 0, 0.8);
  /* TODO: find a nice way to ensure vis-shadows are drawn on top of items
  z-index: 1;
  */
}

.vis-panel .vis-shadow.vis-top {
  top: -1px;
  left: 0;
}

.vis-panel .vis-shadow.vis-bottom {
  bottom: -1px;
  left: 0;
}
`;
qt(FM);
var $M = `.vis-graph-group0 {
  fill: #4f81bd;
  fill-opacity: 0;
  stroke-width: 2px;
  stroke: #4f81bd;
}

.vis-graph-group1 {
  fill: #f79646;
  fill-opacity: 0;
  stroke-width: 2px;
  stroke: #f79646;
}

.vis-graph-group2 {
  fill: #8c51cf;
  fill-opacity: 0;
  stroke-width: 2px;
  stroke: #8c51cf;
}

.vis-graph-group3 {
  fill: #75c841;
  fill-opacity: 0;
  stroke-width: 2px;
  stroke: #75c841;
}

.vis-graph-group4 {
  fill: #ff0100;
  fill-opacity: 0;
  stroke-width: 2px;
  stroke: #ff0100;
}

.vis-graph-group5 {
  fill: #37d8e6;
  fill-opacity: 0;
  stroke-width: 2px;
  stroke: #37d8e6;
}

.vis-graph-group6 {
  fill: #042662;
  fill-opacity: 0;
  stroke-width: 2px;
  stroke: #042662;
}

.vis-graph-group7 {
  fill: #00ff26;
  fill-opacity: 0;
  stroke-width: 2px;
  stroke: #00ff26;
}

.vis-graph-group8 {
  fill: #ff00ff;
  fill-opacity: 0;
  stroke-width: 2px;
  stroke: #ff00ff;
}

.vis-graph-group9 {
  fill: #8f3938;
  fill-opacity: 0;
  stroke-width: 2px;
  stroke: #8f3938;
}

.vis-timeline .vis-fill {
  fill-opacity: 0.1;
  stroke: none;
}

.vis-timeline .vis-bar {
  fill-opacity: 0.5;
  stroke-width: 1px;
}

.vis-timeline .vis-point {
  stroke-width: 2px;
  fill-opacity: 1;
}

.vis-timeline .vis-legend-background {
  stroke-width: 1px;
  fill-opacity: 0.9;
  fill: #ffffff;
  stroke: #c2c2c2;
}

.vis-timeline .vis-outline {
  stroke-width: 1px;
  fill-opacity: 1;
  fill: #ffffff;
  stroke: #e5e5e5;
}

.vis-timeline .vis-icon-fill {
  fill-opacity: 0.3;
  stroke: none;
}
`;
qt($M);
var jM = `.vis-time-axis {
  position: relative;
  overflow: hidden;
}

.vis-time-axis.vis-foreground {
  top: 0;
  left: 0;
  width: 100%;
}

.vis-time-axis.vis-background {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
}

.vis-time-axis .vis-text {
  position: absolute;
  color: #4d4d4d;
  padding: 3px;
  overflow: hidden;
  box-sizing: border-box;

  white-space: nowrap;
}

.vis-time-axis .vis-text.vis-measure {
  position: absolute;
  padding-left: 0;
  padding-right: 0;
  margin-left: 0;
  margin-right: 0;
  visibility: hidden;
}

.vis-time-axis .vis-grid.vis-vertical {
  position: absolute;
  border-left: 1px solid;
}

.vis-time-axis .vis-grid.vis-vertical-rtl {
  position: absolute;
  border-right: 1px solid;
}

.vis-time-axis .vis-grid.vis-minor {
  border-color: #e5e5e5;
}

.vis-time-axis .vis-grid.vis-major {
  border-color: #bfbfbf;
}
`;
qt(jM);
var YM = `.vis-timeline {
  position: relative;
  border: 1px solid #bfbfbf;
  overflow: hidden;
  padding: 0;
  margin: 0;
  box-sizing: border-box;
}

.vis-loading-screen {
  width: 100%;
  height: 100%;
  position: absolute;
  top: 0;
  left: 0;
}
`;
qt(YM);
function fD(n) {
  var e = n && n.preventDefault || !1, t = n && n.container || window, r = {}, s = { keydown: {}, keyup: {} }, a = {}, o;
  for (o = 97; o <= 122; o++)
    a[String.fromCharCode(o)] = { code: 65 + (o - 97), shift: !1 };
  for (o = 65; o <= 90; o++)
    a[String.fromCharCode(o)] = { code: o, shift: !0 };
  for (o = 0; o <= 9; o++)
    a["" + o] = { code: 48 + o, shift: !1 };
  for (o = 1; o <= 12; o++)
    a["F" + o] = { code: 111 + o, shift: !1 };
  for (o = 0; o <= 9; o++)
    a["num" + o] = { code: 96 + o, shift: !1 };
  a["num*"] = { code: 106, shift: !1 }, a["num+"] = { code: 107, shift: !1 }, a["num-"] = { code: 109, shift: !1 }, a["num/"] = { code: 111, shift: !1 }, a["num."] = { code: 110, shift: !1 }, a.left = { code: 37, shift: !1 }, a.up = { code: 38, shift: !1 }, a.right = { code: 39, shift: !1 }, a.down = { code: 40, shift: !1 }, a.space = { code: 32, shift: !1 }, a.enter = { code: 13, shift: !1 }, a.shift = { code: 16, shift: void 0 }, a.esc = { code: 27, shift: !1 }, a.backspace = { code: 8, shift: !1 }, a.tab = { code: 9, shift: !1 }, a.ctrl = { code: 17, shift: !1 }, a.alt = { code: 18, shift: !1 }, a.delete = { code: 46, shift: !1 }, a.pageup = { code: 33, shift: !1 }, a.pagedown = { code: 34, shift: !1 }, a["="] = { code: 187, shift: !1 }, a["-"] = { code: 189, shift: !1 }, a["]"] = { code: 221, shift: !1 }, a["["] = { code: 219, shift: !1 };
  var l = function(f) {
    d(f, "keydown");
  }, c = function(f) {
    d(f, "keyup");
  }, d = function(f, p) {
    if (s[p][f.keyCode] !== void 0) {
      for (var m = s[p][f.keyCode], g = 0; g < m.length; g++)
        (m[g].shift === void 0 || m[g].shift == !0 && f.shiftKey == !0 || m[g].shift == !1 && f.shiftKey == !1) && m[g].fn(f);
      e == !0 && f.preventDefault();
    }
  };
  return r.bind = function(f, p, m) {
    if (m === void 0 && (m = "keydown"), a[f] === void 0)
      throw new Error("unsupported key: " + f);
    s[m][a[f].code] === void 0 && (s[m][a[f].code] = []), s[m][a[f].code].push({ fn: p, shift: a[f].shift });
  }, r.bindAll = function(f, p) {
    p === void 0 && (p = "keydown");
    for (var m in a)
      a.hasOwnProperty(m) && r.bind(m, f, p);
  }, r.getKey = function(f) {
    for (var p in a)
      if (a.hasOwnProperty(p)) {
        if (f.shiftKey == !0 && a[p].shift == !0 && f.keyCode == a[p].code)
          return p;
        if (f.shiftKey == !1 && a[p].shift == !1 && f.keyCode == a[p].code)
          return p;
        if (f.keyCode == a[p].code && p == "shift")
          return p;
      }
    return "unknown key, currently not supported";
  }, r.unbind = function(f, p, m) {
    if (m === void 0 && (m = "keydown"), a[f] === void 0)
      throw new Error("unsupported key: " + f);
    if (p !== void 0) {
      var g = [], b = s[m][a[f].code];
      if (b !== void 0)
        for (var _ = 0; _ < b.length; _++)
          b[_].fn == p && b[_].shift == a[f].shift || g.push(s[m][a[f].code][_]);
      s[m][a[f].code] = g;
    } else
      s[m][a[f].code] = [];
  }, r.reset = function() {
    s = { keydown: {}, keyup: {} };
  }, r.destroy = function() {
    s = { keydown: {}, keyup: {} }, t.removeEventListener("keydown", l, !0), t.removeEventListener("keyup", c, !0);
  }, t.addEventListener("keydown", l, !0), t.addEventListener("keyup", c, !0), r;
}
var sg = {}, ag = {}, Nn, og;
function Ze() {
  if (og) return Nn;
  og = 1;
  var n = function(e) {
    return e && e.Math === Math && e;
  };
  return Nn = // eslint-disable-next-line es/no-global-this -- safe
  n(typeof globalThis == "object" && globalThis) || n(typeof window == "object" && window) || // eslint-disable-next-line no-restricted-globals -- safe
  n(typeof self == "object" && self) || n(typeof Uv == "object" && Uv) || n(typeof Nn == "object" && Nn) || // eslint-disable-next-line no-new-func -- fallback
  /* @__PURE__ */ (function() {
    return this;
  })() || Function("return this")(), Nn;
}
var mo, lg;
function we() {
  return lg || (lg = 1, mo = function(n) {
    try {
      return !!n();
    } catch {
      return !0;
    }
  }), mo;
}
var vo, ug;
function as() {
  if (ug) return vo;
  ug = 1;
  var n = /* @__PURE__ */ we();
  return vo = !n(function() {
    var e = (function() {
    }).bind();
    return typeof e != "function" || e.hasOwnProperty("prototype");
  }), vo;
}
var go, cg;
function _a() {
  if (cg) return go;
  cg = 1;
  var n = /* @__PURE__ */ as(), e = Function.prototype, t = e.apply, r = e.call;
  return go = typeof Reflect == "object" && Reflect.apply || (n ? r.bind(t) : function() {
    return r.apply(t, arguments);
  }), go;
}
var yo, dg;
function Pe() {
  if (dg) return yo;
  dg = 1;
  var n = /* @__PURE__ */ as(), e = Function.prototype, t = e.call, r = n && e.bind.bind(t, t);
  return yo = n ? r : function(s) {
    return function() {
      return t.apply(s, arguments);
    };
  }, yo;
}
var bo, hg;
function xr() {
  if (hg) return bo;
  hg = 1;
  var n = /* @__PURE__ */ Pe(), e = n({}.toString), t = n("".slice);
  return bo = function(r) {
    return t(e(r), 8, -1);
  }, bo;
}
var _o, fg;
function im() {
  if (fg) return _o;
  fg = 1;
  var n = /* @__PURE__ */ xr(), e = /* @__PURE__ */ Pe();
  return _o = function(t) {
    if (n(t) === "Function") return e(t);
  }, _o;
}
var wo, pg;
function yt() {
  if (pg) return wo;
  pg = 1;
  var n = typeof document == "object" && document.all;
  return wo = typeof n > "u" && n !== void 0 ? function(e) {
    return typeof e == "function" || e === n;
  } : function(e) {
    return typeof e == "function";
  }, wo;
}
var xo = {}, So, mg;
function it() {
  if (mg) return So;
  mg = 1;
  var n = /* @__PURE__ */ we();
  return So = !n(function() {
    return Object.defineProperty({}, 1, { get: function() {
      return 7;
    } })[1] !== 7;
  }), So;
}
var To, vg;
function Ot() {
  if (vg) return To;
  vg = 1;
  var n = /* @__PURE__ */ as(), e = Function.prototype.call;
  return To = n ? e.bind(e) : function() {
    return e.apply(e, arguments);
  }, To;
}
var Eo = {}, gg;
function wa() {
  if (gg) return Eo;
  gg = 1;
  var n = {}.propertyIsEnumerable, e = Object.getOwnPropertyDescriptor, t = e && !n.call({ 1: 2 }, 1);
  return Eo.f = t ? function(s) {
    var a = e(this, s);
    return !!a && a.enumerable;
  } : n, Eo;
}
var Do, yg;
function os() {
  return yg || (yg = 1, Do = function(n, e) {
    return {
      enumerable: !(n & 1),
      configurable: !(n & 2),
      writable: !(n & 4),
      value: e
    };
  }), Do;
}
var Oo, bg;
function xa() {
  if (bg) return Oo;
  bg = 1;
  var n = /* @__PURE__ */ Pe(), e = /* @__PURE__ */ we(), t = /* @__PURE__ */ xr(), r = Object, s = n("".split);
  return Oo = e(function() {
    return !r("z").propertyIsEnumerable(0);
  }) ? function(a) {
    return t(a) === "String" ? s(a, "") : r(a);
  } : r, Oo;
}
var ko, _g;
function ls() {
  return _g || (_g = 1, ko = function(n) {
    return n == null;
  }), ko;
}
var Co, wg;
function Gr() {
  if (wg) return Co;
  wg = 1;
  var n = /* @__PURE__ */ ls(), e = TypeError;
  return Co = function(t) {
    if (n(t)) throw new e("Can't call method on " + t);
    return t;
  }, Co;
}
var Ro, xg;
function mr() {
  if (xg) return Ro;
  xg = 1;
  var n = /* @__PURE__ */ xa(), e = /* @__PURE__ */ Gr();
  return Ro = function(t) {
    return n(e(t));
  }, Ro;
}
var Po, Sg;
function bt() {
  if (Sg) return Po;
  Sg = 1;
  var n = /* @__PURE__ */ yt();
  return Po = function(e) {
    return typeof e == "object" ? e !== null : n(e);
  }, Po;
}
var Io, Tg;
function je() {
  return Tg || (Tg = 1, Io = {}), Io;
}
var Mo, Eg;
function Xt() {
  if (Eg) return Mo;
  Eg = 1;
  var n = /* @__PURE__ */ je(), e = /* @__PURE__ */ Ze(), t = /* @__PURE__ */ yt(), r = function(s) {
    return t(s) ? s : void 0;
  };
  return Mo = function(s, a) {
    return arguments.length < 2 ? r(n[s]) || r(e[s]) : n[s] && n[s][a] || e[s] && e[s][a];
  }, Mo;
}
var Ao, Dg;
function He() {
  if (Dg) return Ao;
  Dg = 1;
  var n = /* @__PURE__ */ Pe();
  return Ao = n({}.isPrototypeOf), Ao;
}
var No, Og;
function hn() {
  if (Og) return No;
  Og = 1;
  var n = /* @__PURE__ */ Ze(), e = n.navigator, t = e && e.userAgent;
  return No = t ? String(t) : "", No;
}
var qo, kg;
function us() {
  if (kg) return qo;
  kg = 1;
  var n = /* @__PURE__ */ Ze(), e = /* @__PURE__ */ hn(), t = n.process, r = n.Deno, s = t && t.versions || r && r.version, a = s && s.v8, o, l;
  return a && (o = a.split("."), l = o[0] > 0 && o[0] < 4 ? 1 : +(o[0] + o[1])), !l && e && (o = e.match(/Edge\/(\d+)/), (!o || o[1] >= 74) && (o = e.match(/Chrome\/(\d+)/), o && (l = +o[1]))), qo = l, qo;
}
var Lo, Cg;
function fn() {
  if (Cg) return Lo;
  Cg = 1;
  var n = /* @__PURE__ */ us(), e = /* @__PURE__ */ we(), t = /* @__PURE__ */ Ze(), r = t.String;
  return Lo = !!Object.getOwnPropertySymbols && !e(function() {
    var s = Symbol("symbol detection");
    return !r(s) || !(Object(s) instanceof Symbol) || // Chrome 38-40 symbols are not inherited from DOM collections prototypes to instances
    !Symbol.sham && n && n < 41;
  }), Lo;
}
var Fo, Rg;
function pD() {
  if (Rg) return Fo;
  Rg = 1;
  var n = /* @__PURE__ */ fn();
  return Fo = n && !Symbol.sham && typeof Symbol.iterator == "symbol", Fo;
}
var $o, Pg;
function cs() {
  if (Pg) return $o;
  Pg = 1;
  var n = /* @__PURE__ */ Xt(), e = /* @__PURE__ */ yt(), t = /* @__PURE__ */ He(), r = /* @__PURE__ */ pD(), s = Object;
  return $o = r ? function(a) {
    return typeof a == "symbol";
  } : function(a) {
    var o = n("Symbol");
    return e(o) && t(o.prototype, s(a));
  }, $o;
}
var jo, Ig;
function pn() {
  if (Ig) return jo;
  Ig = 1;
  var n = String;
  return jo = function(e) {
    try {
      return n(e);
    } catch {
      return "Object";
    }
  }, jo;
}
var Yo, Mg;
function vr() {
  if (Mg) return Yo;
  Mg = 1;
  var n = /* @__PURE__ */ yt(), e = /* @__PURE__ */ pn(), t = TypeError;
  return Yo = function(r) {
    if (n(r)) return r;
    throw new t(e(r) + " is not a function");
  }, Yo;
}
var Ho, Ag;
function nm() {
  if (Ag) return Ho;
  Ag = 1;
  var n = /* @__PURE__ */ vr(), e = /* @__PURE__ */ ls();
  return Ho = function(t, r) {
    var s = t[r];
    return e(s) ? void 0 : n(s);
  }, Ho;
}
var Go, Ng;
function HM() {
  if (Ng) return Go;
  Ng = 1;
  var n = /* @__PURE__ */ Ot(), e = /* @__PURE__ */ yt(), t = /* @__PURE__ */ bt(), r = TypeError;
  return Go = function(s, a) {
    var o, l;
    if (a === "string" && e(o = s.toString) && !t(l = n(o, s)) || e(o = s.valueOf) && !t(l = n(o, s)) || a !== "string" && e(o = s.toString) && !t(l = n(o, s))) return l;
    throw new r("Can't convert object to primitive value");
  }, Go;
}
var Bo = { exports: {} }, Wo, qg;
function ni() {
  return qg || (qg = 1, Wo = !0), Wo;
}
var zo, Lg;
function GM() {
  if (Lg) return zo;
  Lg = 1;
  var n = /* @__PURE__ */ Ze(), e = Object.defineProperty;
  return zo = function(t, r) {
    try {
      e(n, t, { value: r, configurable: !0, writable: !0 });
    } catch {
      n[t] = r;
    }
    return r;
  }, zo;
}
var Fg;
function sm() {
  if (Fg) return Bo.exports;
  Fg = 1;
  var n = /* @__PURE__ */ ni(), e = /* @__PURE__ */ Ze(), t = /* @__PURE__ */ GM(), r = "__core-js_shared__", s = Bo.exports = e[r] || t(r, {});
  return (s.versions || (s.versions = [])).push({
    version: "3.49.0",
    mode: n ? "pure" : "global",
    copyright: "© 2013–2025 Denis Pushkarev (zloirock.ru), 2025–2026 CoreJS Company (core-js.io). All rights reserved.",
    license: "https://github.com/zloirock/core-js/blob/v3.49.0/LICENSE",
    source: "https://github.com/zloirock/core-js"
  }), Bo.exports;
}
var Uo, $g;
function mn() {
  if ($g) return Uo;
  $g = 1;
  var n = /* @__PURE__ */ sm();
  return Uo = function(e, t) {
    return n[e] || (n[e] = t || {});
  }, Uo;
}
var Vo, jg;
function kt() {
  if (jg) return Vo;
  jg = 1;
  var n = /* @__PURE__ */ Gr(), e = Object;
  return Vo = function(t) {
    return e(n(t));
  }, Vo;
}
var Ko, Yg;
function dt() {
  if (Yg) return Ko;
  Yg = 1;
  var n = /* @__PURE__ */ Pe(), e = /* @__PURE__ */ kt(), t = n({}.hasOwnProperty);
  return Ko = Object.hasOwn || function(s, a) {
    return t(e(s), a);
  }, Ko;
}
var Xo, Hg;
function ds() {
  if (Hg) return Xo;
  Hg = 1;
  var n = /* @__PURE__ */ Pe(), e = 0, t = Math.random(), r = n(1.1.toString);
  return Xo = function(s) {
    return "Symbol(" + (s === void 0 ? "" : s) + ")_" + r(++e + t, 36);
  }, Xo;
}
var Zo, Gg;
function nt() {
  if (Gg) return Zo;
  Gg = 1;
  var n = /* @__PURE__ */ Ze(), e = /* @__PURE__ */ mn(), t = /* @__PURE__ */ dt(), r = /* @__PURE__ */ ds(), s = /* @__PURE__ */ fn(), a = /* @__PURE__ */ pD(), o = n.Symbol, l = e("wks"), c = a ? o.for || o : o && o.withoutSetter || r;
  return Zo = function(d) {
    return t(l, d) || (l[d] = s && t(o, d) ? o[d] : c("Symbol." + d)), l[d];
  }, Zo;
}
var Qo, Bg;
function mD() {
  if (Bg) return Qo;
  Bg = 1;
  var n = /* @__PURE__ */ Ot(), e = /* @__PURE__ */ bt(), t = /* @__PURE__ */ cs(), r = /* @__PURE__ */ nm(), s = /* @__PURE__ */ HM(), a = /* @__PURE__ */ nt(), o = TypeError, l = a("toPrimitive");
  return Qo = function(c, d) {
    if (!e(c) || t(c)) return c;
    var f = r(c, l), p;
    if (f) {
      if (d === void 0 && (d = "default"), p = n(f, c, d), !e(p) || t(p)) return p;
      throw new o("Can't convert object to primitive value");
    }
    return d === void 0 && (d = "number"), s(c, d);
  }, Qo;
}
var Jo, Wg;
function am() {
  if (Wg) return Jo;
  Wg = 1;
  var n = /* @__PURE__ */ mD(), e = /* @__PURE__ */ cs();
  return Jo = function(t) {
    var r = n(t, "string");
    return e(r) ? r : r + "";
  }, Jo;
}
var el, zg;
function vD() {
  if (zg) return el;
  zg = 1;
  var n = /* @__PURE__ */ Ze(), e = /* @__PURE__ */ bt(), t = n.document, r = e(t) && e(t.createElement);
  return el = function(s) {
    return r ? t.createElement(s) : {};
  }, el;
}
var tl, Ug;
function gD() {
  if (Ug) return tl;
  Ug = 1;
  var n = /* @__PURE__ */ it(), e = /* @__PURE__ */ we(), t = /* @__PURE__ */ vD();
  return tl = !n && !e(function() {
    return Object.defineProperty(t("div"), "a", {
      get: function() {
        return 7;
      }
    }).a !== 7;
  }), tl;
}
var Vg;
function Sa() {
  if (Vg) return xo;
  Vg = 1;
  var n = /* @__PURE__ */ it(), e = /* @__PURE__ */ Ot(), t = /* @__PURE__ */ wa(), r = /* @__PURE__ */ os(), s = /* @__PURE__ */ mr(), a = /* @__PURE__ */ am(), o = /* @__PURE__ */ dt(), l = /* @__PURE__ */ gD(), c = Object.getOwnPropertyDescriptor;
  return xo.f = n ? c : function(f, p) {
    if (f = s(f), p = a(p), l) try {
      return c(f, p);
    } catch {
    }
    if (o(f, p)) return r(!e(t.f, f, p), f[p]);
  }, xo;
}
var rl, Kg;
function BM() {
  if (Kg) return rl;
  Kg = 1;
  var n = /* @__PURE__ */ we(), e = /* @__PURE__ */ yt(), t = /#|\.prototype\./, r = function(c, d) {
    var f = a[s(c)];
    return f === l ? !0 : f === o ? !1 : e(d) ? n(d) : !!d;
  }, s = r.normalize = function(c) {
    return String(c).replace(t, ".").toLowerCase();
  }, a = r.data = {}, o = r.NATIVE = "N", l = r.POLYFILL = "P";
  return rl = r, rl;
}
var il, Xg;
function vn() {
  if (Xg) return il;
  Xg = 1;
  var n = /* @__PURE__ */ im(), e = /* @__PURE__ */ vr(), t = /* @__PURE__ */ as(), r = n(n.bind);
  return il = function(s, a) {
    return e(s), a === void 0 ? s : t ? r(s, a) : function() {
      return s.apply(a, arguments);
    };
  }, il;
}
var nl = {}, sl, Zg;
function yD() {
  if (Zg) return sl;
  Zg = 1;
  var n = /* @__PURE__ */ it(), e = /* @__PURE__ */ we();
  return sl = n && e(function() {
    return Object.defineProperty(function() {
    }, "prototype", {
      value: 42,
      writable: !1
    }).prototype !== 42;
  }), sl;
}
var al, Qg;
function Sr() {
  if (Qg) return al;
  Qg = 1;
  var n = /* @__PURE__ */ bt(), e = String, t = TypeError;
  return al = function(r) {
    if (n(r)) return r;
    throw new t(e(r) + " is not an object");
  }, al;
}
var Jg;
function sr() {
  if (Jg) return nl;
  Jg = 1;
  var n = /* @__PURE__ */ it(), e = /* @__PURE__ */ gD(), t = /* @__PURE__ */ yD(), r = /* @__PURE__ */ Sr(), s = /* @__PURE__ */ am(), a = TypeError, o = Object.defineProperty, l = Object.getOwnPropertyDescriptor, c = "enumerable", d = "configurable", f = "writable";
  return nl.f = n ? t ? function(m, g, b) {
    if (r(m), g = s(g), r(b), typeof m == "function" && g === "prototype" && "value" in b && f in b && !b[f]) {
      var _ = l(m, g);
      _ && _[f] && (m[g] = b.value, b = {
        configurable: d in b ? b[d] : _[d],
        enumerable: c in b ? b[c] : _[c],
        writable: !1
      });
    }
    return o(m, g, b);
  } : o : function(m, g, b) {
    if (r(m), g = s(g), r(b), e) try {
      return o(m, g, b);
    } catch {
    }
    if ("get" in b || "set" in b) throw new a("Accessors not supported");
    return "value" in b && (m[g] = b.value), m;
  }, nl;
}
var ol, ey;
function gn() {
  if (ey) return ol;
  ey = 1;
  var n = /* @__PURE__ */ it(), e = /* @__PURE__ */ sr(), t = /* @__PURE__ */ os();
  return ol = n ? function(r, s, a) {
    return e.f(r, s, t(1, a));
  } : function(r, s, a) {
    return r[s] = a, r;
  }, ol;
}
var ll, ty;
function X() {
  if (ty) return ll;
  ty = 1;
  var n = /* @__PURE__ */ Ze(), e = /* @__PURE__ */ _a(), t = /* @__PURE__ */ im(), r = /* @__PURE__ */ yt(), s = Sa().f, a = /* @__PURE__ */ BM(), o = /* @__PURE__ */ je(), l = /* @__PURE__ */ vn(), c = /* @__PURE__ */ gn(), d = /* @__PURE__ */ dt(), f = function(p) {
    var m = function(g, b, _) {
      if (this instanceof m) {
        switch (arguments.length) {
          case 0:
            return new p();
          case 1:
            return new p(g);
          case 2:
            return new p(g, b);
        }
        return new p(g, b, _);
      }
      return e(p, this, arguments);
    };
    return m.prototype = p.prototype, m;
  };
  return ll = function(p, m) {
    var g = p.target, b = p.global, _ = p.stat, x = p.proto, T = b ? n : _ ? n[g] : n[g] && n[g].prototype, S = b ? o : o[g] || c(o, g, {})[g], k = S.prototype, D, I, C, P, R, M, q, F, H;
    for (P in m)
      D = a(b ? P : g + (_ ? "." : "#") + P, p.forced), I = !D && T && d(T, P), M = S[P], I && (p.dontCallGetSet ? (H = s(T, P), q = H && H.value) : q = T[P]), R = I && q ? q : m[P], !(!D && !x && typeof M == typeof R) && (p.bind && I ? F = l(R, n) : p.wrap && I ? F = f(R) : x && r(R) ? F = t(R) : F = R, (p.sham || R && R.sham || M && M.sham) && c(F, "sham", !0), c(S, P, F), x && (C = g + "Prototype", d(o, C) || c(o, C, {}), c(o[C], P, R), p.real && k && (D || !k[P]) && c(k, P, R)));
  }, ll;
}
var ul, ry;
function om() {
  if (ry) return ul;
  ry = 1;
  var n = /* @__PURE__ */ nt(), e = n("toStringTag"), t = {};
  return t[e] = "z", ul = String(t) === "[object z]", ul;
}
var cl, iy;
function si() {
  if (iy) return cl;
  iy = 1;
  var n = /* @__PURE__ */ om(), e = /* @__PURE__ */ yt(), t = /* @__PURE__ */ xr(), r = /* @__PURE__ */ nt(), s = r("toStringTag"), a = Object, o = t(/* @__PURE__ */ (function() {
    return arguments;
  })()) === "Arguments", l = function(c, d) {
    try {
      return c[d];
    } catch {
    }
  };
  return cl = n ? t : function(c) {
    var d, f, p;
    return c === void 0 ? "Undefined" : c === null ? "Null" : typeof (f = l(d = a(c), s)) == "string" ? f : o ? t(d) : (p = t(d)) === "Object" && e(d.callee) ? "Arguments" : p;
  }, cl;
}
var dl, ny;
function ar() {
  if (ny) return dl;
  ny = 1;
  var n = /* @__PURE__ */ si(), e = String;
  return dl = function(t) {
    if (n(t) === "Symbol") throw new TypeError("Cannot convert a Symbol value to a string");
    return e(t);
  }, dl;
}
var hl = {}, fl, sy;
function WM() {
  if (sy) return fl;
  sy = 1;
  var n = Math.ceil, e = Math.floor;
  return fl = Math.trunc || function(r) {
    var s = +r;
    return (s > 0 ? e : n)(s);
  }, fl;
}
var pl, ay;
function yn() {
  if (ay) return pl;
  ay = 1;
  var n = /* @__PURE__ */ WM();
  return pl = function(e) {
    var t = +e;
    return t !== t || t === 0 ? 0 : n(t);
  }, pl;
}
var ml, oy;
function Ta() {
  if (oy) return ml;
  oy = 1;
  var n = /* @__PURE__ */ yn(), e = Math.max, t = Math.min;
  return ml = function(r, s) {
    var a = n(r);
    return a < 0 ? e(a + s, 0) : t(a, s);
  }, ml;
}
var vl, ly;
function bD() {
  if (ly) return vl;
  ly = 1;
  var n = /* @__PURE__ */ yn(), e = Math.min;
  return vl = function(t) {
    var r = n(t);
    return r > 0 ? e(r, 9007199254740991) : 0;
  }, vl;
}
var gl, uy;
function Zt() {
  if (uy) return gl;
  uy = 1;
  var n = /* @__PURE__ */ bD();
  return gl = function(e) {
    return n(e.length);
  }, gl;
}
var yl, cy;
function lm() {
  if (cy) return yl;
  cy = 1;
  var n = /* @__PURE__ */ mr(), e = /* @__PURE__ */ Ta(), t = /* @__PURE__ */ Zt(), r = function(s) {
    return function(a, o, l) {
      var c = n(a), d = t(c);
      if (d === 0) return !s && -1;
      var f = e(l, d), p;
      if (s && o !== o) {
        for (; d > f; )
          if (p = c[f++], p !== p) return !0;
      } else for (; d > f; f++)
        if ((s || f in c) && c[f] === o) return s || f || 0;
      return !s && -1;
    };
  };
  return yl = {
    // `Array.prototype.includes` method
    // https://tc39.es/ecma262/#sec-array.prototype.includes
    includes: r(!0),
    // `Array.prototype.indexOf` method
    // https://tc39.es/ecma262/#sec-array.prototype.indexof
    indexOf: r(!1)
  }, yl;
}
var bl, dy;
function hs() {
  return dy || (dy = 1, bl = {}), bl;
}
var _l, hy;
function _D() {
  if (hy) return _l;
  hy = 1;
  var n = /* @__PURE__ */ Pe(), e = /* @__PURE__ */ dt(), t = /* @__PURE__ */ mr(), r = lm().indexOf, s = /* @__PURE__ */ hs(), a = n([].push);
  return _l = function(o, l) {
    var c = t(o), d = 0, f = [], p;
    for (p in c) !e(s, p) && e(c, p) && a(f, p);
    for (; l.length > d; ) e(c, p = l[d++]) && (~r(f, p) || a(f, p));
    return f;
  }, _l;
}
var wl, fy;
function um() {
  return fy || (fy = 1, wl = [
    "constructor",
    "hasOwnProperty",
    "isPrototypeOf",
    "propertyIsEnumerable",
    "toLocaleString",
    "toString",
    "valueOf"
  ]), wl;
}
var xl, py;
function fs() {
  if (py) return xl;
  py = 1;
  var n = /* @__PURE__ */ _D(), e = /* @__PURE__ */ um();
  return xl = Object.keys || function(r) {
    return n(r, e);
  }, xl;
}
var my;
function cm() {
  if (my) return hl;
  my = 1;
  var n = /* @__PURE__ */ it(), e = /* @__PURE__ */ yD(), t = /* @__PURE__ */ sr(), r = /* @__PURE__ */ Sr(), s = /* @__PURE__ */ mr(), a = /* @__PURE__ */ fs();
  return hl.f = n && !e ? Object.defineProperties : function(l, c) {
    r(l);
    for (var d = s(c), f = a(c), p = f.length, m = 0, g; p > m; ) t.f(l, g = f[m++], d[g]);
    return l;
  }, hl;
}
var Sl, vy;
function zM() {
  if (vy) return Sl;
  vy = 1;
  var n = /* @__PURE__ */ Xt();
  return Sl = n("document", "documentElement"), Sl;
}
var Tl, gy;
function Ea() {
  if (gy) return Tl;
  gy = 1;
  var n = /* @__PURE__ */ mn(), e = /* @__PURE__ */ ds(), t = n("keys");
  return Tl = function(r) {
    return t[r] || (t[r] = e(r));
  }, Tl;
}
var El, yy;
function ps() {
  if (yy) return El;
  yy = 1;
  var n = /* @__PURE__ */ Sr(), e = /* @__PURE__ */ cm(), t = /* @__PURE__ */ um(), r = /* @__PURE__ */ hs(), s = /* @__PURE__ */ zM(), a = /* @__PURE__ */ vD(), o = /* @__PURE__ */ Ea(), l = ">", c = "<", d = "prototype", f = "script", p = o("IE_PROTO"), m = function() {
  }, g = function(S) {
    return c + f + l + S + c + "/" + f + l;
  }, b = function(S) {
    S.write(g("")), S.close();
    var k = S.parentWindow.Object;
    return S = null, k;
  }, _ = function() {
    var S = a("iframe"), k = "java" + f + ":", D;
    return S.style.display = "none", s.appendChild(S), S.src = String(k), D = S.contentWindow.document, D.open(), D.write(g("document.F=Object")), D.close(), D.F;
  }, x, T = function() {
    try {
      x = new ActiveXObject("htmlfile");
    } catch {
    }
    T = typeof document < "u" ? document.domain && x ? b(x) : _() : b(x);
    for (var S = t.length; S--; ) delete T[d][t[S]];
    return T();
  };
  return r[p] = !0, El = Object.create || function(k, D) {
    var I;
    return k !== null ? (m[d] = n(k), I = new m(), m[d] = null, I[p] = k) : I = T(), D === void 0 ? I : e.f(I, D);
  }, El;
}
var Dl = {}, by;
function Da() {
  if (by) return Dl;
  by = 1;
  var n = /* @__PURE__ */ _D(), e = /* @__PURE__ */ um(), t = e.concat("length", "prototype");
  return Dl.f = Object.getOwnPropertyNames || function(s) {
    return n(s, t);
  }, Dl;
}
var Ol = {}, kl, _y;
function bn() {
  if (_y) return kl;
  _y = 1;
  var n = /* @__PURE__ */ Pe();
  return kl = n([].slice), kl;
}
var wy;
function wD() {
  if (wy) return Ol;
  wy = 1;
  var n = /* @__PURE__ */ xr(), e = /* @__PURE__ */ mr(), t = Da().f, r = /* @__PURE__ */ bn(), s = typeof window == "object" && window && Object.getOwnPropertyNames ? Object.getOwnPropertyNames(window) : [], a = function(o) {
    try {
      return t(o);
    } catch {
      return r(s);
    }
  };
  return Ol.f = function(l) {
    return s && n(l) === "Window" ? a(l) : t(e(l));
  }, Ol;
}
var Cl = {}, xy;
function Oa() {
  return xy || (xy = 1, Cl.f = Object.getOwnPropertySymbols), Cl;
}
var Rl, Sy;
function ms() {
  if (Sy) return Rl;
  Sy = 1;
  var n = /* @__PURE__ */ gn();
  return Rl = function(e, t, r, s) {
    return s && s.enumerable ? e[t] = r : n(e, t, r), e;
  }, Rl;
}
var Pl, Ty;
function dm() {
  if (Ty) return Pl;
  Ty = 1;
  var n = /* @__PURE__ */ sr();
  return Pl = function(e, t, r) {
    return n.f(e, t, r);
  }, Pl;
}
var Il = {}, Ey;
function ka() {
  if (Ey) return Il;
  Ey = 1;
  var n = /* @__PURE__ */ nt();
  return Il.f = n, Il;
}
var Ml, Dy;
function Ve() {
  if (Dy) return Ml;
  Dy = 1;
  var n = /* @__PURE__ */ je(), e = /* @__PURE__ */ dt(), t = /* @__PURE__ */ ka(), r = sr().f;
  return Ml = function(s) {
    var a = n.Symbol || (n.Symbol = {});
    e(a, s) || r(a, s, {
      value: t.f(s)
    });
  }, Ml;
}
var Al, Oy;
function xD() {
  if (Oy) return Al;
  Oy = 1;
  var n = /* @__PURE__ */ Ot(), e = /* @__PURE__ */ Xt(), t = /* @__PURE__ */ nt(), r = /* @__PURE__ */ ms();
  return Al = function() {
    var s = e("Symbol"), a = s && s.prototype, o = a && a.valueOf, l = t("toPrimitive");
    a && !a[l] && r(a, l, function(c) {
      return n(o, this);
    }, { arity: 1 });
  }, Al;
}
var Nl, ky;
function UM() {
  if (ky) return Nl;
  ky = 1;
  var n = /* @__PURE__ */ om(), e = /* @__PURE__ */ si();
  return Nl = n ? {}.toString : function() {
    return "[object " + e(this) + "]";
  }, Nl;
}
var ql, Cy;
function Ci() {
  if (Cy) return ql;
  Cy = 1;
  var n = /* @__PURE__ */ om(), e = sr().f, t = /* @__PURE__ */ gn(), r = /* @__PURE__ */ dt(), s = /* @__PURE__ */ UM(), a = /* @__PURE__ */ nt(), o = a("toStringTag");
  return ql = function(l, c, d, f) {
    var p = d ? l : l && l.prototype;
    p && (r(p, o) || e(p, o, { configurable: !0, value: c }), f && !n && t(p, "toString", s));
  }, ql;
}
var Ll, Ry;
function VM() {
  if (Ry) return Ll;
  Ry = 1;
  var n = /* @__PURE__ */ Ze(), e = /* @__PURE__ */ yt(), t = n.WeakMap;
  return Ll = e(t) && /native code/.test(String(t)), Ll;
}
var Fl, Py;
function _n() {
  if (Py) return Fl;
  Py = 1;
  var n = /* @__PURE__ */ VM(), e = /* @__PURE__ */ Ze(), t = /* @__PURE__ */ bt(), r = /* @__PURE__ */ gn(), s = /* @__PURE__ */ dt(), a = /* @__PURE__ */ sm(), o = /* @__PURE__ */ Ea(), l = /* @__PURE__ */ hs(), c = "Object already initialized", d = e.TypeError, f = e.WeakMap, p, m, g, b = function(S) {
    return g(S) ? m(S) : p(S, {});
  }, _ = function(S) {
    return function(k) {
      var D;
      if (!t(k) || (D = m(k)).type !== S)
        throw new d("Incompatible receiver, " + S + " required");
      return D;
    };
  };
  if (n || a.state) {
    var x = a.state || (a.state = new f());
    x.get = x.get, x.has = x.has, x.set = x.set, p = function(S, k) {
      if (x.has(S)) throw new d(c);
      return k.facade = S, x.set(S, k), k;
    }, m = function(S) {
      return x.get(S) || {};
    }, g = function(S) {
      return x.has(S);
    };
  } else {
    var T = o("state");
    l[T] = !0, p = function(S, k) {
      if (s(S, T)) throw new d(c);
      return k.facade = S, r(S, T, k), k;
    }, m = function(S) {
      return s(S, T) ? S[T] : {};
    }, g = function(S) {
      return s(S, T);
    };
  }
  return Fl = {
    set: p,
    get: m,
    has: g,
    enforce: b,
    getterFor: _
  }, Fl;
}
var $l, Iy;
function ai() {
  if (Iy) return $l;
  Iy = 1;
  var n = /* @__PURE__ */ xr();
  return $l = Array.isArray || function(t) {
    return n(t) === "Array";
  }, $l;
}
var jl, My;
function KM() {
  if (My) return jl;
  My = 1;
  var n = /* @__PURE__ */ Pe(), e = /* @__PURE__ */ yt(), t = /* @__PURE__ */ sm(), r = n(Function.toString);
  return e(t.inspectSource) || (t.inspectSource = function(s) {
    return r(s);
  }), jl = t.inspectSource, jl;
}
var Yl, Ay;
function hm() {
  if (Ay) return Yl;
  Ay = 1;
  var n = /* @__PURE__ */ Pe(), e = /* @__PURE__ */ we(), t = /* @__PURE__ */ yt(), r = /* @__PURE__ */ si(), s = /* @__PURE__ */ Xt(), a = /* @__PURE__ */ KM(), o = function() {
  }, l = s("Reflect", "construct"), c = /^\s*(?:class|function)\b/, d = n(c.exec), f = !c.test(o), p = function(b) {
    if (!t(b)) return !1;
    try {
      return l(o, [], b), !0;
    } catch {
      return !1;
    }
  }, m = function(b) {
    if (!t(b)) return !1;
    switch (r(b)) {
      case "AsyncFunction":
      case "GeneratorFunction":
      case "AsyncGeneratorFunction":
        return !1;
    }
    try {
      return f || !!d(c, a(b));
    } catch {
      return !0;
    }
  };
  return m.sham = !0, Yl = !l || e(function() {
    var g;
    return p(p.call) || !p(Object) || !p(function() {
      g = !0;
    }) || g;
  }) ? m : p, Yl;
}
var Hl, Ny;
function XM() {
  if (Ny) return Hl;
  Ny = 1;
  var n = /* @__PURE__ */ ai(), e = /* @__PURE__ */ hm(), t = /* @__PURE__ */ bt(), r = /* @__PURE__ */ nt(), s = r("species"), a = Array;
  return Hl = function(o) {
    var l;
    return n(o) && (l = o.constructor, e(l) && (l === a || n(l.prototype)) ? l = void 0 : t(l) && (l = l[s], l === null && (l = void 0))), l === void 0 ? a : l;
  }, Hl;
}
var Gl, qy;
function Ca() {
  if (qy) return Gl;
  qy = 1;
  var n = /* @__PURE__ */ XM();
  return Gl = function(e, t) {
    return new (n(e))(t === 0 ? 0 : t);
  }, Gl;
}
var Bl, Ly;
function Ri() {
  if (Ly) return Bl;
  Ly = 1;
  var n = /* @__PURE__ */ it(), e = /* @__PURE__ */ sr(), t = /* @__PURE__ */ os();
  return Bl = function(r, s, a) {
    n ? e.f(r, s, t(0, a)) : r[s] = a;
  }, Bl;
}
var Wl, Fy;
function oi() {
  if (Fy) return Wl;
  Fy = 1;
  var n = /* @__PURE__ */ vn(), e = /* @__PURE__ */ xa(), t = /* @__PURE__ */ kt(), r = /* @__PURE__ */ Zt(), s = /* @__PURE__ */ Ca(), a = /* @__PURE__ */ Ri(), o = function(l) {
    var c = l === 1, d = l === 2, f = l === 3, p = l === 4, m = l === 6, g = l === 7, b = l === 5 || m;
    return function(_, x, T) {
      for (var S = t(_), k = e(S), D = r(k), I = n(x, T), C = 0, P = 0, R = c ? s(_, D) : d || g ? s(_, 0) : void 0, M, q; D > C; C++) if ((b || C in k) && (M = k[C], q = I(M, C, S), l))
        if (c) a(R, C, q);
        else if (q) switch (l) {
          case 3:
            return !0;
          // some
          case 5:
            return M;
          // find
          case 6:
            return C;
          // findIndex
          case 2:
            a(R, P++, M);
        }
        else switch (l) {
          case 4:
            return !1;
          // every
          case 7:
            a(R, P++, M);
        }
      return m ? -1 : f || p ? p : R;
    };
  };
  return Wl = {
    // `Array.prototype.forEach` method
    // https://tc39.es/ecma262/#sec-array.prototype.foreach
    forEach: o(0),
    // `Array.prototype.map` method
    // https://tc39.es/ecma262/#sec-array.prototype.map
    map: o(1),
    // `Array.prototype.filter` method
    // https://tc39.es/ecma262/#sec-array.prototype.filter
    filter: o(2),
    // `Array.prototype.some` method
    // https://tc39.es/ecma262/#sec-array.prototype.some
    some: o(3),
    // `Array.prototype.every` method
    // https://tc39.es/ecma262/#sec-array.prototype.every
    every: o(4),
    // `Array.prototype.find` method
    // https://tc39.es/ecma262/#sec-array.prototype.find
    find: o(5),
    // `Array.prototype.findIndex` method
    // https://tc39.es/ecma262/#sec-array.prototype.findIndex
    findIndex: o(6),
    // `Array.prototype.filterReject` method
    // https://github.com/tc39/proposal-array-filtering
    filterReject: o(7)
  }, Wl;
}
var $y;
function ZM() {
  if ($y) return ag;
  $y = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ Ze(), t = /* @__PURE__ */ Ot(), r = /* @__PURE__ */ Pe(), s = /* @__PURE__ */ ni(), a = /* @__PURE__ */ it(), o = /* @__PURE__ */ fn(), l = /* @__PURE__ */ we(), c = /* @__PURE__ */ dt(), d = /* @__PURE__ */ He(), f = /* @__PURE__ */ Sr(), p = /* @__PURE__ */ mr(), m = /* @__PURE__ */ am(), g = /* @__PURE__ */ ar(), b = /* @__PURE__ */ os(), _ = /* @__PURE__ */ ps(), x = /* @__PURE__ */ fs(), T = /* @__PURE__ */ Da(), S = /* @__PURE__ */ wD(), k = /* @__PURE__ */ Oa(), D = /* @__PURE__ */ Sa(), I = /* @__PURE__ */ sr(), C = /* @__PURE__ */ cm(), P = /* @__PURE__ */ wa(), R = /* @__PURE__ */ ms(), M = /* @__PURE__ */ dm(), q = /* @__PURE__ */ mn(), F = /* @__PURE__ */ Ea(), H = /* @__PURE__ */ hs(), Z = /* @__PURE__ */ ds(), ve = /* @__PURE__ */ nt(), Me = /* @__PURE__ */ ka(), Y = /* @__PURE__ */ Ve(), J = /* @__PURE__ */ xD(), z = /* @__PURE__ */ Ci(), te = /* @__PURE__ */ _n(), K = oi().forEach, re = F("hidden"), ae = "Symbol", be = "prototype", se = te.set, j = te.getterFor(ae), xe = Object[be], lt = e.Symbol, st = lt && lt[be], ht = e.RangeError, Er = e.TypeError, Lt = e.QObject, Qt = D.f, Ft = I.f, pt = S.f, _t = P.f, Ct = r([].push), $t = q("symbols"), Jt = q("op-symbols"), L = q("wks"), ue = !Lt || !Lt[be] || !Lt[be].findChild, fe = function(De, ne, ce) {
    var de = Qt(xe, ne);
    return de && delete xe[ne], Ft(De, ne, ce), de && De !== xe && Ft(xe, ne, de), De;
  }, Te = a && l(function() {
    return _(Ft({}, "a", {
      get: function() {
        return Ft(this, "a", { value: 7 }).a;
      }
    })).a !== 7;
  }) ? fe : Ft, Qe = function(De, ne) {
    var ce = $t[De] = _(st);
    return se(ce, {
      type: ae,
      tag: De,
      description: ne
    }), a || (ce.description = ne), ce;
  }, ft = function(ne, ce, de) {
    ne === xe && ft(Jt, ce, de), f(ne);
    var ge = m(ce);
    return f(de), c($t, ge) ? (("enumerable" in de ? !de.enumerable : !c(ne, ge) || c(ne, re) && ne[re][ge]) ? (c(ne, re) || Ft(ne, re, b(1, _(null))), ne[re][ge] = !0) : (c(ne, re) && ne[re][ge] && (ne[re][ge] = !1), de = _(de, { enumerable: b(0, !1) })), Te(ne, ge, de)) : Ft(ne, ge, de);
  }, Rt = function(ne, ce) {
    f(ne);
    var de = p(ce), ge = x(de).concat(Ee(de));
    return K(ge, function(ee) {
      (!a || t(at, de, ee)) && ft(ne, ee, de[ee]);
    }), ne;
  }, Ie = function(ne, ce) {
    return ce === void 0 ? _(ne) : Rt(_(ne), ce);
  }, at = function(ne) {
    var ce = m(ne), de = t(_t, this, ce);
    return this === xe && c($t, ce) && !c(Jt, ce) ? !1 : de || !c(this, ce) || !c($t, ce) || c(this, re) && this[re][ce] ? de : !0;
  }, er = function(ne, ce) {
    var de = p(ne), ge = m(ce);
    if (!(de === xe && c($t, ge) && !c(Jt, ge))) {
      var ee = Qt(de, ge);
      return ee && c($t, ge) && !(c(de, re) && de[re][ge]) && (ee.enumerable = !0), ee;
    }
  }, Bt = function(ne) {
    var ce = pt(p(ne)), de = [];
    return K(ce, function(ge) {
      !c($t, ge) && !c(H, ge) && Ct(de, ge);
    }), de;
  }, Ee = function(De) {
    var ne = De === xe, ce = pt(ne ? Jt : p(De)), de = [];
    return K(ce, function(ge) {
      c($t, ge) && (!ne || c(xe, ge)) && Ct(de, $t[ge]);
    }), de;
  };
  return o || (lt = function() {
    if (d(st, this)) throw new Er("Symbol is not a constructor");
    var ne = !arguments.length || arguments[0] === void 0 ? void 0 : g(arguments[0]), ce = Z(ne), de = function(ge) {
      var ee = this === void 0 ? e : this;
      ee === xe && t(de, Jt, ge), c(ee, re) && c(ee[re], ce) && (ee[re][ce] = !1);
      var Je = b(1, ge);
      try {
        Te(ee, ce, Je);
      } catch (wt) {
        if (!(wt instanceof ht)) throw wt;
        fe(ee, ce, Je);
      }
    };
    return a && ue && Te(xe, ce, { configurable: !0, set: de }), Qe(ce, ne);
  }, st = lt[be], R(st, "toString", function() {
    return j(this).tag;
  }), R(lt, "withoutSetter", function(De) {
    return Qe(Z(De), De);
  }), P.f = at, I.f = ft, C.f = Rt, D.f = er, T.f = S.f = Bt, k.f = Ee, Me.f = function(De) {
    return Qe(ve(De), De);
  }, a && (M(st, "description", {
    configurable: !0,
    get: function() {
      return j(this).description;
    }
  }), s || R(xe, "propertyIsEnumerable", at, { unsafe: !0 }))), n({ global: !0, constructor: !0, wrap: !0, forced: !o, sham: !o }, {
    Symbol: lt
  }), K(x(L), function(De) {
    Y(De);
  }), n({ target: ae, stat: !0, forced: !o }, {
    useSetter: function() {
      ue = !0;
    },
    useSimple: function() {
      ue = !1;
    }
  }), n({ target: "Object", stat: !0, forced: !o, sham: !a }, {
    // `Object.create` method
    // https://tc39.es/ecma262/#sec-object.create
    create: Ie,
    // `Object.defineProperty` method
    // https://tc39.es/ecma262/#sec-object.defineproperty
    defineProperty: ft,
    // `Object.defineProperties` method
    // https://tc39.es/ecma262/#sec-object.defineproperties
    defineProperties: Rt,
    // `Object.getOwnPropertyDescriptor` method
    // https://tc39.es/ecma262/#sec-object.getownpropertydescriptors
    getOwnPropertyDescriptor: er
  }), n({ target: "Object", stat: !0, forced: !o }, {
    // `Object.getOwnPropertyNames` method
    // https://tc39.es/ecma262/#sec-object.getownpropertynames
    getOwnPropertyNames: Bt
  }), J(), z(lt, ae), H[re] = !0, ag;
}
var jy = {}, zl, Yy;
function SD() {
  if (Yy) return zl;
  Yy = 1;
  var n = /* @__PURE__ */ fn();
  return zl = n && !!Symbol.for && !!Symbol.keyFor, zl;
}
var Hy;
function QM() {
  if (Hy) return jy;
  Hy = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ Xt(), t = /* @__PURE__ */ dt(), r = /* @__PURE__ */ ar(), s = /* @__PURE__ */ mn(), a = /* @__PURE__ */ SD(), o = s("string-to-symbol-registry"), l = s("symbol-to-string-registry");
  return n({ target: "Symbol", stat: !0, forced: !a }, {
    for: function(c) {
      var d = r(c);
      if (t(o, d)) return o[d];
      var f = e("Symbol")(d);
      return o[d] = f, l[f] = d, f;
    }
  }), jy;
}
var Gy = {}, By;
function JM() {
  if (By) return Gy;
  By = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ dt(), t = /* @__PURE__ */ cs(), r = /* @__PURE__ */ pn(), s = /* @__PURE__ */ mn(), a = /* @__PURE__ */ SD(), o = s("symbol-to-string-registry");
  return n({ target: "Symbol", stat: !0, forced: !a }, {
    keyFor: function(c) {
      if (!t(c)) throw new TypeError(r(c) + " is not a symbol");
      if (e(o, c)) return o[c];
    }
  }), Gy;
}
var Wy = {}, Ul, zy;
function eA() {
  if (zy) return Ul;
  zy = 1;
  var n = /* @__PURE__ */ bt(), e = _n().get;
  return Ul = function(r) {
    if (!n(r)) return !1;
    var s = e(r);
    return !!s && s.type === "RawJSON";
  }, Ul;
}
var Vl, Uy;
function tA() {
  if (Uy) return Vl;
  Uy = 1;
  var n = /* @__PURE__ */ Pe(), e = /* @__PURE__ */ dt(), t = SyntaxError, r = parseInt, s = String.fromCharCode, a = n("".charAt), o = n("".slice), l = n(/./.exec), c = {
    '\\"': '"',
    "\\\\": "\\",
    "\\/": "/",
    "\\b": "\b",
    "\\f": "\f",
    "\\n": `
`,
    "\\r": "\r",
    "\\t": "	"
  }, d = /^[\da-f]{4}$/i, f = /^[\u0000-\u001F]$/;
  return Vl = function(p, m) {
    for (var g = !0, b = ""; m < p.length; ) {
      var _ = a(p, m);
      if (_ === "\\") {
        var x = o(p, m, m + 2);
        if (e(c, x))
          b += c[x], m += 2;
        else if (x === "\\u") {
          m += 2;
          var T = o(p, m, m + 4);
          if (!l(d, T)) throw new t("Bad Unicode escape at: " + m);
          b += s(r(T, 16)), m += 4;
        } else throw new t('Unknown escape sequence: "' + x + '"');
      } else if (_ === '"') {
        g = !1, m++;
        break;
      } else {
        if (l(f, _)) throw new t("Bad control character in string literal at: " + m);
        b += _, m++;
      }
    }
    if (g) throw new t("Unterminated string at: " + m);
    return { value: b, end: m };
  }, Vl;
}
var Kl, Vy;
function rA() {
  if (Vy) return Kl;
  Vy = 1;
  var n = /* @__PURE__ */ we();
  return Kl = !n(function() {
    var e = "9007199254740993", t = JSON.rawJSON(e);
    return !JSON.isRawJSON(t) || JSON.stringify(t) !== e;
  }), Kl;
}
var Ky;
function TD() {
  if (Ky) return Wy;
  Ky = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ Xt(), t = /* @__PURE__ */ _a(), r = /* @__PURE__ */ Ot(), s = /* @__PURE__ */ Pe(), a = /* @__PURE__ */ we(), o = /* @__PURE__ */ ai(), l = /* @__PURE__ */ yt(), c = /* @__PURE__ */ eA(), d = /* @__PURE__ */ cs(), f = /* @__PURE__ */ xr(), p = /* @__PURE__ */ ar(), m = /* @__PURE__ */ bn(), g = /* @__PURE__ */ tA(), b = /* @__PURE__ */ ds(), _ = /* @__PURE__ */ fn(), x = /* @__PURE__ */ rA(), T = String, S = e("JSON", "stringify"), k = s(/./.exec), D = s("".charAt), I = s("".charCodeAt), C = s("".replace), P = s("".slice), R = s([].push), M = s(1.1.toString), q = /[\uD800-\uDFFF]/g, F = /^[\uD800-\uDBFF]$/, H = /^[\uDC00-\uDFFF]$/, Z = b(), ve = Z.length, Me = !_ || a(function() {
    var K = e("Symbol")("stringify detection");
    return S([K]) !== "[null]" || S({ a: K }) !== "{}" || S(Object(K)) !== "{}";
  }), Y = a(function() {
    return S("\uDF06\uD834") !== '"\\udf06\\ud834"' || S("\uDEAD") !== '"\\udead"';
  }), J = Me ? function(K, re) {
    var ae = m(arguments), be = te(re);
    if (!(!l(be) && (K === void 0 || d(K))))
      return ae[1] = function(se, j) {
        if (l(be) && (j = r(be, this, T(se), j)), !d(j)) return j;
      }, t(S, null, ae);
  } : S, z = function(K, re, ae) {
    var be = D(ae, re - 1), se = D(ae, re + 1);
    return k(F, K) && !k(H, se) || k(H, K) && !k(F, be) ? "\\u" + M(I(K, 0), 16) : K;
  }, te = function(K) {
    if (l(K)) return K;
    if (o(K)) {
      for (var re = K.length, ae = [], be = 0; be < re; be++) {
        var se = K[be];
        typeof se == "string" ? R(ae, se) : (typeof se == "number" || f(se) === "Number" || f(se) === "String") && R(ae, p(se));
      }
      var j = ae.length, xe = !0;
      return function(lt, st) {
        if (xe)
          return xe = !1, st;
        if (o(this)) return st;
        for (var ht = 0; ht < j; ht++) if (ae[ht] === lt) return st;
      };
    }
  };
  return S && n({ target: "JSON", stat: !0, arity: 3, forced: Me || Y || !x }, {
    stringify: function(re, ae, be) {
      var se = te(ae), j = [], xe = J(re, function(Ft, pt) {
        var _t = l(se) ? r(se, this, T(Ft), pt) : pt;
        return !x && c(_t) ? Z + (R(j, _t.rawJSON) - 1) : _t;
      }, be);
      if (typeof xe != "string" || (Y && (xe = C(xe, q, z)), x)) return xe;
      for (var lt = "", st = xe.length, ht = 0; ht < st; ht++) {
        var Er = D(xe, ht);
        if (Er === '"') {
          var Lt = g(xe, ++ht).end - 1, Qt = P(xe, ht, Lt);
          lt += P(Qt, 0, ve) === Z ? j[P(Qt, ve)] : '"' + Qt + '"', ht = Lt;
        } else lt += Er;
      }
      return lt;
    }
  }), Wy;
}
var Xy = {}, Zy;
function iA() {
  if (Zy) return Xy;
  Zy = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ fn(), t = /* @__PURE__ */ we(), r = /* @__PURE__ */ Oa(), s = /* @__PURE__ */ kt(), a = !e || t(function() {
    r.f(1);
  });
  return n({ target: "Object", stat: !0, forced: a }, {
    getOwnPropertySymbols: function(l) {
      var c = r.f;
      return c ? c(s(l)) : [];
    }
  }), Xy;
}
var Qy;
function ED() {
  return Qy || (Qy = 1, ZM(), QM(), JM(), TD(), iA()), sg;
}
var Xl, Jy;
function nA() {
  if (Jy) return Xl;
  Jy = 1, ED();
  var n = /* @__PURE__ */ je();
  return Xl = n.Object.getOwnPropertySymbols, Xl;
}
var Zl, eb;
function sA() {
  if (eb) return Zl;
  eb = 1;
  var n = /* @__PURE__ */ nA();
  return Zl = n, Zl;
}
var Ql, tb;
function aA() {
  return tb || (tb = 1, Ql = /* @__PURE__ */ sA()), Ql;
}
var oA = aA(), la = /* @__PURE__ */ Q(oA), Jl = { exports: {} }, rb = {}, ib;
function lA() {
  if (ib) return rb;
  ib = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ we(), t = /* @__PURE__ */ mr(), r = Sa().f, s = /* @__PURE__ */ it(), a = !s || e(function() {
    r(1);
  });
  return n({ target: "Object", stat: !0, forced: a, sham: !s }, {
    getOwnPropertyDescriptor: function(l, c) {
      return r(t(l), c);
    }
  }), rb;
}
var nb;
function uA() {
  if (nb) return Jl.exports;
  nb = 1, lA();
  var n = /* @__PURE__ */ je(), e = n.Object, t = Jl.exports = function(s, a) {
    return e.getOwnPropertyDescriptor(s, a);
  };
  return e.getOwnPropertyDescriptor.sham && (t.sham = !0), Jl.exports;
}
var eu, sb;
function cA() {
  if (sb) return eu;
  sb = 1;
  var n = /* @__PURE__ */ uA();
  return eu = n, eu;
}
var tu, ab;
function dA() {
  return ab || (ab = 1, tu = /* @__PURE__ */ cA()), tu;
}
var hA = dA(), Ra = /* @__PURE__ */ Q(hA), ob = {}, ru, lb;
function DD() {
  if (lb) return ru;
  lb = 1;
  var n = /* @__PURE__ */ Xt(), e = /* @__PURE__ */ Pe(), t = /* @__PURE__ */ Da(), r = /* @__PURE__ */ Oa(), s = /* @__PURE__ */ Sr(), a = e([].concat);
  return ru = n("Reflect", "ownKeys") || function(l) {
    var c = t.f(s(l)), d = r.f;
    return d ? a(c, d(l)) : c;
  }, ru;
}
var ub;
function fA() {
  if (ub) return ob;
  ub = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ it(), t = /* @__PURE__ */ DD(), r = /* @__PURE__ */ mr(), s = /* @__PURE__ */ Sa(), a = /* @__PURE__ */ Ri();
  return n({ target: "Object", stat: !0, sham: !e }, {
    getOwnPropertyDescriptors: function(l) {
      for (var c = r(l), d = s.f, f = t(c), p = {}, m = 0, g, b; f.length > m; )
        b = d(c, g = f[m++]), b !== void 0 && a(p, g, b);
      return p;
    }
  }), ob;
}
var iu, cb;
function pA() {
  if (cb) return iu;
  cb = 1, fA();
  var n = /* @__PURE__ */ je();
  return iu = n.Object.getOwnPropertyDescriptors, iu;
}
var nu, db;
function mA() {
  if (db) return nu;
  db = 1;
  var n = /* @__PURE__ */ pA();
  return nu = n, nu;
}
var su, hb;
function vA() {
  return hb || (hb = 1, su = /* @__PURE__ */ mA()), su;
}
var gA = vA(), ua = /* @__PURE__ */ Q(gA), au = { exports: {} }, fb = {}, pb;
function yA() {
  if (pb) return fb;
  pb = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ it(), t = cm().f;
  return n({ target: "Object", stat: !0, forced: Object.defineProperties !== t, sham: !e }, {
    defineProperties: t
  }), fb;
}
var mb;
function bA() {
  if (mb) return au.exports;
  mb = 1, yA();
  var n = /* @__PURE__ */ je(), e = n.Object, t = au.exports = function(s, a) {
    return e.defineProperties(s, a);
  };
  return e.defineProperties.sham && (t.sham = !0), au.exports;
}
var ou, vb;
function _A() {
  if (vb) return ou;
  vb = 1;
  var n = /* @__PURE__ */ bA();
  return ou = n, ou;
}
var lu, gb;
function wA() {
  return gb || (gb = 1, lu = /* @__PURE__ */ _A()), lu;
}
var xA = wA(), OD = /* @__PURE__ */ Q(xA), yb = {}, bb;
function SA() {
  if (bb) return yb;
  bb = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ ai();
  return n({ target: "Array", stat: !0 }, {
    isArray: e
  }), yb;
}
var uu, _b;
function TA() {
  if (_b) return uu;
  _b = 1, SA();
  var n = /* @__PURE__ */ je();
  return uu = n.Array.isArray, uu;
}
var cu, wb;
function kD() {
  if (wb) return cu;
  wb = 1;
  var n = /* @__PURE__ */ TA();
  return cu = n, cu;
}
var du, xb;
function EA() {
  if (xb) return du;
  xb = 1;
  var n = /* @__PURE__ */ kD();
  return du = n, du;
}
var hu, Sb;
function DA() {
  if (Sb) return hu;
  Sb = 1;
  var n = /* @__PURE__ */ EA();
  return hu = n, hu;
}
var fu, Tb;
function OA() {
  return Tb || (Tb = 1, fu = /* @__PURE__ */ DA()), fu;
}
var kA = /* @__PURE__ */ OA(), CA = /* @__PURE__ */ Q(kA);
function RA(n) {
  if (CA(n)) return n;
}
var Eb = {}, pu, Db;
function Pa() {
  if (Db) return pu;
  Db = 1;
  var n = TypeError, e = 9007199254740991;
  return pu = function(t) {
    if (t > e) throw new n("Maximum allowed index exceeded");
    return t;
  }, pu;
}
var mu, Ob;
function vs() {
  if (Ob) return mu;
  Ob = 1;
  var n = /* @__PURE__ */ it(), e = /* @__PURE__ */ ai(), t = TypeError, r = Object.getOwnPropertyDescriptor, s = n && !(function() {
    if (this !== void 0) return !0;
    try {
      Object.defineProperty([], "length", { writable: !1 }).length = 1;
    } catch (a) {
      return a instanceof TypeError;
    }
  })();
  return mu = s ? function(a, o) {
    if (e(a) && !r(a, "length").writable)
      throw new t("Cannot set read only .length");
    return a.length = o;
  } : function(a, o) {
    return a.length = o;
  }, mu;
}
var vu, kb;
function gs() {
  if (kb) return vu;
  kb = 1;
  var n = /* @__PURE__ */ we(), e = /* @__PURE__ */ nt(), t = /* @__PURE__ */ us(), r = e("species");
  return vu = function(s) {
    return t >= 51 || !n(function() {
      var a = [], o = a.constructor = {};
      return o[r] = function() {
        return { foo: 1 };
      }, a[s](Boolean).foo !== 1;
    });
  }, vu;
}
var Cb;
function CD() {
  if (Cb) return Eb;
  Cb = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ we(), t = /* @__PURE__ */ ai(), r = /* @__PURE__ */ bt(), s = /* @__PURE__ */ kt(), a = /* @__PURE__ */ Zt(), o = /* @__PURE__ */ Pa(), l = /* @__PURE__ */ Ri(), c = /* @__PURE__ */ vs(), d = /* @__PURE__ */ Ca(), f = /* @__PURE__ */ gs(), p = /* @__PURE__ */ nt(), m = /* @__PURE__ */ us(), g = p("isConcatSpreadable"), b = m >= 51 || !e(function() {
    var T = [];
    return T[g] = !1, T.concat()[0] !== T;
  }), _ = function(T) {
    if (!r(T)) return !1;
    var S = T[g];
    return S !== void 0 ? !!S : t(T);
  }, x = !b || !f("concat");
  return n({ target: "Array", proto: !0, arity: 1, forced: x }, {
    // eslint-disable-next-line no-unused-vars -- required for `.length`
    concat: function(S) {
      var k = s(this), D = d(k, 0), I = 0, C, P, R, M, q;
      for (C = -1, R = arguments.length; C < R; C++)
        if (q = C === -1 ? k : arguments[C], _(q))
          for (M = a(q), o(I + M), P = 0; P < M; P++, I++) P in q && l(D, I, q[P]);
        else
          o(I + 1), l(D, I++, q);
      return c(D, I), D;
    }
  }), Eb;
}
var Rb = {}, Pb;
function RD() {
  if (Pb) return Rb;
  Pb = 1;
  var n = /* @__PURE__ */ Ve();
  return n("asyncDispose"), Rb;
}
var Ib = {}, Mb;
function PA() {
  if (Mb) return Ib;
  Mb = 1;
  var n = /* @__PURE__ */ Ve();
  return n("asyncIterator"), Ib;
}
var Ab = {}, Nb;
function PD() {
  if (Nb) return Ab;
  Nb = 1;
  var n = /* @__PURE__ */ Ve();
  return n("dispose"), Ab;
}
var qb = {}, Lb;
function IA() {
  if (Lb) return qb;
  Lb = 1;
  var n = /* @__PURE__ */ Ve();
  return n("hasInstance"), qb;
}
var Fb = {}, $b;
function MA() {
  if ($b) return Fb;
  $b = 1;
  var n = /* @__PURE__ */ Ve();
  return n("isConcatSpreadable"), Fb;
}
var jb = {}, Yb;
function ID() {
  if (Yb) return jb;
  Yb = 1;
  var n = /* @__PURE__ */ Ve();
  return n("iterator"), jb;
}
var Hb = {}, Gb;
function AA() {
  if (Gb) return Hb;
  Gb = 1;
  var n = /* @__PURE__ */ Ve();
  return n("match"), Hb;
}
var Bb = {}, Wb;
function NA() {
  if (Wb) return Bb;
  Wb = 1;
  var n = /* @__PURE__ */ Ve();
  return n("matchAll"), Bb;
}
var zb = {}, Ub;
function qA() {
  if (Ub) return zb;
  Ub = 1;
  var n = /* @__PURE__ */ Ve();
  return n("replace"), zb;
}
var Vb = {}, Kb;
function LA() {
  if (Kb) return Vb;
  Kb = 1;
  var n = /* @__PURE__ */ Ve();
  return n("search"), Vb;
}
var Xb = {}, Zb;
function FA() {
  if (Zb) return Xb;
  Zb = 1;
  var n = /* @__PURE__ */ Ve();
  return n("species"), Xb;
}
var Qb = {}, Jb;
function $A() {
  if (Jb) return Qb;
  Jb = 1;
  var n = /* @__PURE__ */ Ve();
  return n("split"), Qb;
}
var e_ = {}, t_;
function MD() {
  if (t_) return e_;
  t_ = 1;
  var n = /* @__PURE__ */ Ve(), e = /* @__PURE__ */ xD();
  return n("toPrimitive"), e(), e_;
}
var r_ = {}, i_;
function jA() {
  if (i_) return r_;
  i_ = 1;
  var n = /* @__PURE__ */ Xt(), e = /* @__PURE__ */ Ve(), t = /* @__PURE__ */ Ci();
  return e("toStringTag"), t(n("Symbol"), "Symbol"), r_;
}
var n_ = {}, s_;
function YA() {
  if (s_) return n_;
  s_ = 1;
  var n = /* @__PURE__ */ Ve();
  return n("unscopables"), n_;
}
var a_ = {}, o_;
function HA() {
  if (o_) return a_;
  o_ = 1;
  var n = /* @__PURE__ */ Ze(), e = /* @__PURE__ */ Ci();
  return e(n.JSON, "JSON", !0), a_;
}
var gu, l_;
function GA() {
  if (l_) return gu;
  l_ = 1, CD(), ED(), RD(), PA(), PD(), IA(), MA(), ID(), AA(), NA(), qA(), LA(), FA(), $A(), MD(), jA(), YA(), HA();
  var n = /* @__PURE__ */ je();
  return gu = n.Symbol, gu;
}
var u_ = {}, yu, c_;
function ys() {
  return c_ || (c_ = 1, yu = function() {
  }), yu;
}
var bu, d_;
function wn() {
  return d_ || (d_ = 1, bu = {}), bu;
}
var _u, h_;
function BA() {
  if (h_) return _u;
  h_ = 1;
  var n = /* @__PURE__ */ it(), e = /* @__PURE__ */ dt(), t = Function.prototype, r = n && Object.getOwnPropertyDescriptor, s = e(t, "name"), a = s && (function() {
  }).name === "something", o = s && (!n || n && r(t, "name").configurable);
  return _u = {
    EXISTS: s,
    PROPER: a,
    CONFIGURABLE: o
  }, _u;
}
var wu, f_;
function AD() {
  if (f_) return wu;
  f_ = 1;
  var n = /* @__PURE__ */ we();
  return wu = !n(function() {
    function e() {
    }
    return e.prototype.constructor = null, Object.getPrototypeOf(new e()) !== e.prototype;
  }), wu;
}
var xu, p_;
function Ia() {
  if (p_) return xu;
  p_ = 1;
  var n = /* @__PURE__ */ dt(), e = /* @__PURE__ */ yt(), t = /* @__PURE__ */ kt(), r = /* @__PURE__ */ Ea(), s = /* @__PURE__ */ AD(), a = r("IE_PROTO"), o = Object, l = o.prototype;
  return xu = s ? o.getPrototypeOf : function(c) {
    var d = t(c);
    if (n(d, a)) return d[a];
    var f = d.constructor;
    return e(f) && d instanceof f ? f.prototype : d instanceof o ? l : null;
  }, xu;
}
var Su, m_;
function ND() {
  if (m_) return Su;
  m_ = 1;
  var n = /* @__PURE__ */ we(), e = /* @__PURE__ */ yt(), t = /* @__PURE__ */ bt(), r = /* @__PURE__ */ ps(), s = /* @__PURE__ */ Ia(), a = /* @__PURE__ */ ms(), o = /* @__PURE__ */ nt(), l = /* @__PURE__ */ ni(), c = o("iterator"), d = !1, f, p, m;
  [].keys && (m = [].keys(), "next" in m ? (p = s(s(m)), p !== Object.prototype && (f = p)) : d = !0);
  var g = !t(f) || n(function() {
    var b = {};
    return f[c].call(b) !== b;
  });
  return g ? f = {} : l && (f = r(f)), e(f[c]) || a(f, c, function() {
    return this;
  }), Su = {
    IteratorPrototype: f,
    BUGGY_SAFARI_ITERATORS: d
  }, Su;
}
var Tu, v_;
function WA() {
  if (v_) return Tu;
  v_ = 1;
  var n = ND().IteratorPrototype, e = /* @__PURE__ */ ps(), t = /* @__PURE__ */ os(), r = /* @__PURE__ */ Ci(), s = /* @__PURE__ */ wn(), a = function() {
    return this;
  };
  return Tu = function(o, l, c, d) {
    var f = l + " Iterator";
    return o.prototype = e(n, { next: t(+!d, c) }), r(o, f, !1, !0), s[f] = a, o;
  }, Tu;
}
var Eu, g_;
function zA() {
  if (g_) return Eu;
  g_ = 1;
  var n = /* @__PURE__ */ Pe(), e = /* @__PURE__ */ vr();
  return Eu = function(t, r, s) {
    try {
      return n(e(Object.getOwnPropertyDescriptor(t, r)[s]));
    } catch {
    }
  }, Eu;
}
var Du, y_;
function UA() {
  if (y_) return Du;
  y_ = 1;
  var n = /* @__PURE__ */ bt();
  return Du = function(e) {
    return n(e) || e === null;
  }, Du;
}
var Ou, b_;
function VA() {
  if (b_) return Ou;
  b_ = 1;
  var n = /* @__PURE__ */ UA(), e = String, t = TypeError;
  return Ou = function(r) {
    if (n(r)) return r;
    throw new t("Can't set " + e(r) + " as a prototype");
  }, Ou;
}
var ku, __;
function KA() {
  if (__) return ku;
  __ = 1;
  var n = /* @__PURE__ */ zA(), e = /* @__PURE__ */ bt(), t = /* @__PURE__ */ Gr(), r = /* @__PURE__ */ VA();
  return ku = Object.setPrototypeOf || ("__proto__" in {} ? (function() {
    var s = !1, a = {}, o;
    try {
      o = n(Object.prototype, "__proto__", "set"), o(a, []), s = a instanceof Array;
    } catch {
    }
    return function(c, d) {
      return t(c), r(d), e(c) && (s ? o(c, d) : c.__proto__ = d), c;
    };
  })() : void 0), ku;
}
var Cu, w_;
function fm() {
  if (w_) return Cu;
  w_ = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ Ot(), t = /* @__PURE__ */ ni(), r = /* @__PURE__ */ BA(), s = /* @__PURE__ */ yt(), a = /* @__PURE__ */ WA(), o = /* @__PURE__ */ Ia(), l = /* @__PURE__ */ KA(), c = /* @__PURE__ */ Ci(), d = /* @__PURE__ */ gn(), f = /* @__PURE__ */ ms(), p = /* @__PURE__ */ nt(), m = /* @__PURE__ */ wn(), g = /* @__PURE__ */ ND(), b = r.PROPER, _ = r.CONFIGURABLE, x = g.IteratorPrototype, T = g.BUGGY_SAFARI_ITERATORS, S = p("iterator"), k = "keys", D = "values", I = "entries", C = function() {
    return this;
  };
  return Cu = function(P, R, M, q, F, H, Z) {
    a(M, R, q);
    var ve = function(se) {
      if (se === F && te) return te;
      if (!T && se && se in J) return J[se];
      switch (se) {
        case k:
          return function() {
            return new M(this, se);
          };
        case D:
          return function() {
            return new M(this, se);
          };
        case I:
          return function() {
            return new M(this, se);
          };
      }
      return function() {
        return new M(this);
      };
    }, Me = R + " Iterator", Y = !1, J = P.prototype, z = J[S] || J["@@iterator"] || F && J[F], te = !T && z || ve(F), K = R === "Array" && J.entries || z, re, ae, be;
    if (K && (re = o(K.call(new P())), re !== Object.prototype && re.next && (!t && o(re) !== x && (l ? l(re, x) : s(re[S]) || f(re, S, C)), c(re, Me, !0, !0), t && (m[Me] = C))), b && F === D && z && z.name !== D && (!t && _ ? d(J, "name", D) : (Y = !0, te = function() {
      return e(z, this);
    })), F)
      if (ae = {
        values: ve(D),
        keys: H ? te : ve(k),
        entries: ve(I)
      }, Z) for (be in ae)
        (T || Y || !(be in J)) && f(J, be, ae[be]);
      else n({ target: R, proto: !0, forced: T || Y }, ae);
    return (!t || Z) && J[S] !== te && f(J, S, te, { name: F }), m[R] = te, ae;
  }, Cu;
}
var Ru, x_;
function pm() {
  return x_ || (x_ = 1, Ru = function(n, e) {
    return { value: n, done: e };
  }), Ru;
}
var Pu, S_;
function Br() {
  if (S_) return Pu;
  S_ = 1;
  var n = /* @__PURE__ */ mr(), e = /* @__PURE__ */ ys(), t = /* @__PURE__ */ wn(), r = /* @__PURE__ */ _n(), s = sr().f, a = /* @__PURE__ */ fm(), o = /* @__PURE__ */ pm(), l = /* @__PURE__ */ ni(), c = /* @__PURE__ */ it(), d = "Array Iterator", f = r.set, p = r.getterFor(d);
  Pu = a(Array, "Array", function(g, b) {
    f(this, {
      type: d,
      target: n(g),
      // target
      index: 0,
      // next index
      kind: b
      // kind
    });
  }, function() {
    var g = p(this), b = g.target, _ = g.index++;
    if (!b || _ >= b.length)
      return g.target = null, o(void 0, !0);
    switch (g.kind) {
      case "keys":
        return o(_, !1);
      case "values":
        return o(b[_], !1);
    }
    return o([_, b[_]], !1);
  }, "values");
  var m = t.Arguments = t.Array;
  if (e("keys"), e("values"), e("entries"), !l && c && m.name !== "values") try {
    s(m, "name", { value: "values" });
  } catch {
  }
  return Pu;
}
var Iu, T_;
function XA() {
  return T_ || (T_ = 1, Iu = {
    CSSRuleList: 0,
    CSSStyleDeclaration: 0,
    CSSValueList: 0,
    ClientRectList: 0,
    DOMRectList: 0,
    DOMStringList: 0,
    DOMTokenList: 1,
    DataTransferItemList: 0,
    FileList: 0,
    HTMLAllCollection: 0,
    HTMLCollection: 0,
    HTMLFormElement: 0,
    HTMLSelectElement: 0,
    MediaList: 0,
    MimeTypeArray: 0,
    NamedNodeMap: 0,
    NodeList: 1,
    PaintRequestList: 0,
    Plugin: 0,
    PluginArray: 0,
    SVGLengthList: 0,
    SVGNumberList: 0,
    SVGPathSegList: 0,
    SVGPointList: 0,
    SVGStringList: 0,
    SVGTransformList: 0,
    SourceBufferList: 0,
    StyleSheetList: 0,
    TextTrackCueList: 0,
    TextTrackList: 0,
    TouchList: 0
  }), Iu;
}
var E_;
function Wr() {
  if (E_) return u_;
  E_ = 1, Br();
  var n = /* @__PURE__ */ XA(), e = /* @__PURE__ */ Ze(), t = /* @__PURE__ */ Ci(), r = /* @__PURE__ */ wn();
  for (var s in n)
    t(e[s], s), r[s] = r.Array;
  return u_;
}
var Mu, D_;
function qD() {
  if (D_) return Mu;
  D_ = 1;
  var n = /* @__PURE__ */ GA();
  return Wr(), Mu = n, Mu;
}
var O_ = {}, k_;
function ZA() {
  if (k_) return O_;
  k_ = 1;
  var n = /* @__PURE__ */ nt(), e = sr().f, t = n("metadata"), r = Function.prototype;
  return r[t] === void 0 && e(r, t, {
    value: null
  }), O_;
}
var C_ = {}, R_;
function QA() {
  return R_ || (R_ = 1, RD()), C_;
}
var P_ = {}, I_;
function JA() {
  return I_ || (I_ = 1, PD()), P_;
}
var M_ = {}, A_;
function eN() {
  if (A_) return M_;
  A_ = 1;
  var n = /* @__PURE__ */ Ve();
  return n("metadata"), M_;
}
var Au, N_;
function tN() {
  if (N_) return Au;
  N_ = 1;
  var n = /* @__PURE__ */ qD();
  return ZA(), QA(), JA(), eN(), Au = n, Au;
}
var q_ = {}, Nu, L_;
function LD() {
  if (L_) return Nu;
  L_ = 1;
  var n = /* @__PURE__ */ Xt(), e = /* @__PURE__ */ Pe(), t = n("Symbol"), r = t.keyFor, s = e(t.prototype.valueOf);
  return Nu = t.isRegisteredSymbol || function(o) {
    try {
      return r(s(o)) !== void 0;
    } catch {
      return !1;
    }
  }, Nu;
}
var F_;
function rN() {
  if (F_) return q_;
  F_ = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ LD();
  return n({ target: "Symbol", stat: !0 }, {
    isRegisteredSymbol: e
  }), q_;
}
var $_ = {}, qu, j_;
function FD() {
  if (j_) return qu;
  j_ = 1;
  for (var n = /* @__PURE__ */ mn(), e = /* @__PURE__ */ Xt(), t = /* @__PURE__ */ Pe(), r = /* @__PURE__ */ cs(), s = /* @__PURE__ */ nt(), a = e("Symbol"), o = a.isWellKnownSymbol, l = e("Object", "getOwnPropertyNames"), c = t(a.prototype.valueOf), d = n("wks"), f = 0, p = l(a), m = p.length; f < m; f++)
    try {
      var g = p[f];
      r(a[g]) && s(g);
    } catch {
    }
  return qu = function(_) {
    if (o && o(_)) return !0;
    try {
      for (var x = c(_), T = 0, S = l(d), k = S.length; T < k; T++)
        if (d[S[T]] == x) return !0;
    } catch {
    }
    return !1;
  }, qu;
}
var Y_;
function iN() {
  if (Y_) return $_;
  Y_ = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ FD();
  return n({ target: "Symbol", stat: !0, forced: !0 }, {
    isWellKnownSymbol: e
  }), $_;
}
var H_ = {}, G_;
function nN() {
  if (G_) return H_;
  G_ = 1;
  var n = /* @__PURE__ */ Ve();
  return n("customMatcher"), H_;
}
var B_ = {}, W_;
function sN() {
  if (W_) return B_;
  W_ = 1;
  var n = /* @__PURE__ */ Ve();
  return n("observable"), B_;
}
var z_ = {}, U_;
function aN() {
  if (U_) return z_;
  U_ = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ LD();
  return n({ target: "Symbol", stat: !0, name: "isRegisteredSymbol" }, {
    isRegistered: e
  }), z_;
}
var V_ = {}, K_;
function oN() {
  if (K_) return V_;
  K_ = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ FD();
  return n({ target: "Symbol", stat: !0, name: "isWellKnownSymbol", forced: !0 }, {
    isWellKnown: e
  }), V_;
}
var X_ = {}, Z_;
function lN() {
  if (Z_) return X_;
  Z_ = 1;
  var n = /* @__PURE__ */ Ve();
  return n("matcher"), X_;
}
var Q_ = {}, J_;
function uN() {
  if (J_) return Q_;
  J_ = 1;
  var n = /* @__PURE__ */ Ve();
  return n("metadataKey"), Q_;
}
var e0 = {}, t0;
function cN() {
  if (t0) return e0;
  t0 = 1;
  var n = /* @__PURE__ */ Ve();
  return n("patternMatch"), e0;
}
var r0 = {}, i0;
function dN() {
  if (i0) return r0;
  i0 = 1;
  var n = /* @__PURE__ */ Ve();
  return n("replaceAll"), r0;
}
var Lu, n0;
function hN() {
  if (n0) return Lu;
  n0 = 1;
  var n = /* @__PURE__ */ tN();
  return rN(), iN(), nN(), sN(), aN(), oN(), lN(), uN(), cN(), dN(), Lu = n, Lu;
}
var Fu, s0;
function fN() {
  return s0 || (s0 = 1, Fu = /* @__PURE__ */ hN()), Fu;
}
var pN = /* @__PURE__ */ fN(), Yn = /* @__PURE__ */ Q(pN), a0 = {}, $u, o0;
function mN() {
  if (o0) return $u;
  o0 = 1;
  var n = /* @__PURE__ */ Pe(), e = /* @__PURE__ */ yn(), t = /* @__PURE__ */ ar(), r = /* @__PURE__ */ Gr(), s = n("".charAt), a = n("".charCodeAt), o = n("".slice), l = function(c) {
    return function(d, f) {
      var p = t(r(d)), m = e(f), g = p.length, b, _;
      return m < 0 || m >= g ? c ? "" : void 0 : (b = a(p, m), b < 55296 || b > 56319 || m + 1 === g || (_ = a(p, m + 1)) < 56320 || _ > 57343 ? c ? s(p, m) : b : c ? o(p, m, m + 2) : (b - 55296 << 10) + (_ - 56320) + 65536);
    };
  };
  return $u = {
    // `String.prototype.codePointAt` method
    // https://tc39.es/ecma262/#sec-string.prototype.codepointat
    codeAt: l(!1),
    // `String.prototype.at` method
    // https://github.com/mathiasbynens/String.prototype.at
    charAt: l(!0)
  }, $u;
}
var l0;
function xn() {
  if (l0) return a0;
  l0 = 1;
  var n = mN().charAt, e = /* @__PURE__ */ ar(), t = /* @__PURE__ */ _n(), r = /* @__PURE__ */ fm(), s = /* @__PURE__ */ pm(), a = "String Iterator", o = t.set, l = t.getterFor(a);
  return r(String, "String", function(c) {
    o(this, {
      type: a,
      string: e(c),
      index: 0
    });
  }, function() {
    var d = l(this), f = d.string, p = d.index, m;
    return p >= f.length ? s(void 0, !0) : (m = n(f, p), d.index += m.length, s(m, !1));
  }), a0;
}
var ju, u0;
function Ma() {
  if (u0) return ju;
  u0 = 1;
  var n = /* @__PURE__ */ si(), e = /* @__PURE__ */ nm(), t = /* @__PURE__ */ ls(), r = /* @__PURE__ */ wn(), s = /* @__PURE__ */ nt(), a = s("iterator");
  return ju = function(o) {
    if (!t(o)) return e(o, a) || e(o, "@@iterator") || r[n(o)];
  }, ju;
}
var Yu, c0;
function vN() {
  if (c0) return Yu;
  c0 = 1, Br(), xn();
  var n = /* @__PURE__ */ Ma();
  return Yu = n, Yu;
}
var Hu, d0;
function gN() {
  if (d0) return Hu;
  d0 = 1;
  var n = /* @__PURE__ */ vN();
  return Wr(), Hu = n, Hu;
}
var Gu, h0;
function yN() {
  if (h0) return Gu;
  h0 = 1;
  var n = /* @__PURE__ */ gN();
  return Gu = n, Gu;
}
var Bu, f0;
function bN() {
  if (f0) return Bu;
  f0 = 1;
  var n = /* @__PURE__ */ yN();
  return Bu = n, Bu;
}
var Wu, p0;
function _N() {
  return p0 || (p0 = 1, Wu = /* @__PURE__ */ bN()), Wu;
}
var wN = /* @__PURE__ */ _N(), xN = /* @__PURE__ */ Q(wN), m0 = {}, v0;
function SN() {
  if (v0) return m0;
  v0 = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ kt(), t = /* @__PURE__ */ Zt(), r = /* @__PURE__ */ vs(), s = /* @__PURE__ */ Pa(), a = /* @__PURE__ */ we(), o = a(function() {
    return [].push.call({ length: 4294967296 }, 1) !== 4294967297;
  }), l = function() {
    try {
      Object.defineProperty([], "length", { writable: !1 }).push();
    } catch (d) {
      return d instanceof TypeError;
    }
  }, c = o || !l();
  return n({ target: "Array", proto: !0, arity: 1, forced: c }, {
    // eslint-disable-next-line no-unused-vars -- required for `.length`
    push: function(f) {
      var p = e(this), m = t(p), g = arguments.length;
      s(m + g);
      for (var b = 0; b < g; b++)
        p[m] = arguments[b], m++;
      return r(p, m), m;
    }
  }), m0;
}
var zu, g0;
function Ke() {
  if (g0) return zu;
  g0 = 1;
  var n = /* @__PURE__ */ Ze(), e = /* @__PURE__ */ je();
  return zu = function(t, r) {
    var s = e[t + "Prototype"], a = s && s[r];
    if (a) return a;
    var o = n[t], l = o && o.prototype;
    return l && l[r];
  }, zu;
}
var Uu, y0;
function TN() {
  if (y0) return Uu;
  y0 = 1, SN();
  var n = /* @__PURE__ */ Ke();
  return Uu = n("Array", "push"), Uu;
}
var Vu, b0;
function EN() {
  if (b0) return Vu;
  b0 = 1;
  var n = /* @__PURE__ */ He(), e = /* @__PURE__ */ TN(), t = Array.prototype;
  return Vu = function(r) {
    var s = r.push;
    return r === t || n(t, r) && s === t.push ? e : s;
  }, Vu;
}
var Ku, _0;
function DN() {
  if (_0) return Ku;
  _0 = 1;
  var n = /* @__PURE__ */ EN();
  return Ku = n, Ku;
}
var Xu, w0;
function ON() {
  if (w0) return Xu;
  w0 = 1;
  var n = /* @__PURE__ */ DN();
  return Xu = n, Xu;
}
var Zu, x0;
function kN() {
  if (x0) return Zu;
  x0 = 1;
  var n = /* @__PURE__ */ ON();
  return Zu = n, Zu;
}
var Qu, S0;
function CN() {
  return S0 || (S0 = 1, Qu = /* @__PURE__ */ kN()), Qu;
}
var RN = /* @__PURE__ */ CN(), PN = /* @__PURE__ */ Q(RN);
function IN(n, e) {
  var t = n == null ? null : typeof Yn < "u" && xN(n) || n["@@iterator"];
  if (t != null) {
    var r, s, a, o, l = [], c = !0, d = !1;
    try {
      if (a = (t = t.call(n)).next, e === 0) {
        if (Object(t) !== t) return;
        c = !1;
      } else for (; !(c = (r = a.call(t)).done) && (PN(l).call(l, r.value), l.length !== e); c = !0) ;
    } catch (f) {
      d = !0, s = f;
    } finally {
      try {
        if (!c && t.return != null && (o = t.return(), Object(o) !== o)) return;
      } finally {
        if (d) throw s;
      }
    }
    return l;
  }
}
var T0 = {}, E0;
function MN() {
  if (E0) return T0;
  E0 = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ ai(), t = /* @__PURE__ */ hm(), r = /* @__PURE__ */ bt(), s = /* @__PURE__ */ Ta(), a = /* @__PURE__ */ Zt(), o = /* @__PURE__ */ mr(), l = /* @__PURE__ */ Ri(), c = /* @__PURE__ */ vs(), d = /* @__PURE__ */ nt(), f = /* @__PURE__ */ gs(), p = /* @__PURE__ */ bn(), m = f("slice"), g = d("species"), b = Array, _ = Math.max;
  return n({ target: "Array", proto: !0, forced: !m }, {
    slice: function(T, S) {
      var k = o(this), D = a(k), I = s(T, D), C = s(S === void 0 ? D : S, D), P, R, M;
      if (e(k) && (P = k.constructor, t(P) && (P === b || e(P.prototype)) ? P = void 0 : r(P) && (P = P[g], P === null && (P = void 0)), P === b || P === void 0))
        return p(k, I, C);
      for (R = new (P === void 0 ? b : P)(_(C - I, 0)), M = 0; I < C; I++, M++) I in k && l(R, M, k[I]);
      return c(R, M), R;
    }
  }), T0;
}
var Ju, D0;
function AN() {
  if (D0) return Ju;
  D0 = 1, MN();
  var n = /* @__PURE__ */ Ke();
  return Ju = n("Array", "slice"), Ju;
}
var ec, O0;
function NN() {
  if (O0) return ec;
  O0 = 1;
  var n = /* @__PURE__ */ He(), e = /* @__PURE__ */ AN(), t = Array.prototype;
  return ec = function(r) {
    var s = r.slice;
    return r === t || n(t, r) && s === t.slice ? e : s;
  }, ec;
}
var tc, k0;
function $D() {
  if (k0) return tc;
  k0 = 1;
  var n = /* @__PURE__ */ NN();
  return tc = n, tc;
}
var rc, C0;
function qN() {
  if (C0) return rc;
  C0 = 1;
  var n = /* @__PURE__ */ $D();
  return rc = n, rc;
}
var ic, R0;
function LN() {
  if (R0) return ic;
  R0 = 1;
  var n = /* @__PURE__ */ qN();
  return ic = n, ic;
}
var nc, P0;
function FN() {
  return P0 || (P0 = 1, nc = /* @__PURE__ */ LN()), nc;
}
var $N = /* @__PURE__ */ FN(), jN = /* @__PURE__ */ Q($N), I0 = {}, sc, M0;
function bs() {
  if (M0) return sc;
  M0 = 1;
  var n = /* @__PURE__ */ Ot(), e = /* @__PURE__ */ Sr(), t = /* @__PURE__ */ nm();
  return sc = function(r, s, a) {
    var o, l;
    e(r);
    try {
      if (o = t(r, "return"), !o) {
        if (s === "throw") throw a;
        return a;
      }
      o = n(o, r);
    } catch (c) {
      l = !0, o = c;
    }
    if (s === "throw") throw a;
    if (l) throw o;
    return e(o), a;
  }, sc;
}
var ac, A0;
function YN() {
  if (A0) return ac;
  A0 = 1;
  var n = /* @__PURE__ */ Sr(), e = /* @__PURE__ */ bs();
  return ac = function(t, r, s, a) {
    try {
      return a ? r(n(s)[0], s[1]) : r(s);
    } catch (o) {
      e(t, "throw", o);
    }
  }, ac;
}
var oc, N0;
function jD() {
  if (N0) return oc;
  N0 = 1;
  var n = /* @__PURE__ */ nt(), e = /* @__PURE__ */ wn(), t = n("iterator"), r = Array.prototype;
  return oc = function(s) {
    return s !== void 0 && (e.Array === s || r[t] === s);
  }, oc;
}
var lc, q0;
function mm() {
  if (q0) return lc;
  q0 = 1;
  var n = /* @__PURE__ */ Ot(), e = /* @__PURE__ */ vr(), t = /* @__PURE__ */ Sr(), r = /* @__PURE__ */ pn(), s = /* @__PURE__ */ Ma(), a = TypeError;
  return lc = function(o, l) {
    var c = arguments.length < 2 ? s(o) : l;
    if (e(c)) return t(n(c, o));
    throw new a(r(o) + " is not iterable");
  }, lc;
}
var uc, L0;
function HN() {
  if (L0) return uc;
  L0 = 1;
  var n = /* @__PURE__ */ vn(), e = /* @__PURE__ */ Ot(), t = /* @__PURE__ */ kt(), r = /* @__PURE__ */ YN(), s = /* @__PURE__ */ jD(), a = /* @__PURE__ */ hm(), o = /* @__PURE__ */ Zt(), l = /* @__PURE__ */ Ri(), c = /* @__PURE__ */ vs(), d = /* @__PURE__ */ mm(), f = /* @__PURE__ */ Ma(), p = /* @__PURE__ */ bs(), m = Array;
  return uc = function(b) {
    var _ = a(this), x = arguments.length, T = x > 1 ? arguments[1] : void 0, S = T !== void 0;
    S && (T = n(T, x > 2 ? arguments[2] : void 0));
    var k = t(b), D = f(k), I = 0, C, P, R, M, q, F;
    if (D && !(this === m && s(D)))
      for (P = _ ? new this() : [], M = d(k, D), q = M.next; !(R = e(q, M)).done; I++) {
        F = S ? r(M, T, [R.value, I], !0) : R.value;
        try {
          l(P, I, F);
        } catch (H) {
          p(M, "throw", H);
        }
      }
    else
      for (C = o(k), P = _ ? new this(C) : m(C); C > I; I++)
        F = S ? T(k[I], I) : k[I], l(P, I, F);
    return c(P, I), P;
  }, uc;
}
var cc, F0;
function GN() {
  if (F0) return cc;
  F0 = 1;
  var n = /* @__PURE__ */ nt(), e = n("iterator"), t = !1;
  try {
    var r = 0, s = {
      next: function() {
        return { done: !!r++ };
      },
      return: function() {
        t = !0;
      }
    };
    s[e] = function() {
      return this;
    }, Array.from(s, function() {
      throw 2;
    });
  } catch {
  }
  return cc = function(a, o) {
    try {
      if (!o && !t) return !1;
    } catch {
      return !1;
    }
    var l = !1;
    try {
      var c = {};
      c[e] = function() {
        return {
          next: function() {
            return { done: l = !0 };
          }
        };
      }, a(c);
    } catch {
    }
    return l;
  }, cc;
}
var $0;
function BN() {
  if ($0) return I0;
  $0 = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ HN(), t = /* @__PURE__ */ GN(), r = !t(function(s) {
    Array.from(s);
  });
  return n({ target: "Array", stat: !0, forced: r }, {
    from: e
  }), I0;
}
var dc, j0;
function WN() {
  if (j0) return dc;
  j0 = 1, xn(), BN();
  var n = /* @__PURE__ */ je();
  return dc = n.Array.from, dc;
}
var hc, Y0;
function zN() {
  if (Y0) return hc;
  Y0 = 1;
  var n = /* @__PURE__ */ WN();
  return hc = n, hc;
}
var fc, H0;
function UN() {
  if (H0) return fc;
  H0 = 1;
  var n = /* @__PURE__ */ zN();
  return fc = n, fc;
}
var pc, G0;
function VN() {
  if (G0) return pc;
  G0 = 1;
  var n = /* @__PURE__ */ UN();
  return pc = n, pc;
}
var mc, B0;
function KN() {
  return B0 || (B0 = 1, mc = /* @__PURE__ */ VN()), mc;
}
var XN = /* @__PURE__ */ KN(), ZN = /* @__PURE__ */ Q(XN);
function W0(n, e) {
  (e == null || e > n.length) && (e = n.length);
  for (var t = 0, r = Array(e); t < e; t++) r[t] = n[t];
  return r;
}
function QN(n, e) {
  if (n) {
    var t;
    if (typeof n == "string") return W0(n, e);
    var r = jN(t = {}.toString.call(n)).call(t, 8, -1);
    return r === "Object" && n.constructor && (r = n.constructor.name), r === "Map" || r === "Set" ? ZN(n) : r === "Arguments" || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(r) ? W0(n, e) : void 0;
  }
}
function JN() {
  throw new TypeError(`Invalid attempt to destructure non-iterable instance.
In order to be iterable, non-array objects must have a [Symbol.iterator]() method.`);
}
function St(n, e) {
  return RA(n) || IN(n, e) || QN(n, e) || JN();
}
var vc = { exports: {} }, z0 = {}, U0;
function eq() {
  if (U0) return z0;
  U0 = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ it(), t = sr().f;
  return n({ target: "Object", stat: !0, forced: Object.defineProperty !== t, sham: !e }, {
    defineProperty: t
  }), z0;
}
var V0;
function tq() {
  if (V0) return vc.exports;
  V0 = 1, eq();
  var n = /* @__PURE__ */ je(), e = n.Object, t = vc.exports = function(s, a, o) {
    return e.defineProperty(s, a, o);
  };
  return e.defineProperty.sham && (t.sham = !0), vc.exports;
}
var gc, K0;
function YD() {
  if (K0) return gc;
  K0 = 1;
  var n = /* @__PURE__ */ tq();
  return gc = n, gc;
}
var yc, X0;
function rq() {
  if (X0) return yc;
  X0 = 1;
  var n = /* @__PURE__ */ YD();
  return yc = n, yc;
}
var bc, Z0;
function iq() {
  if (Z0) return bc;
  Z0 = 1;
  var n = /* @__PURE__ */ rq();
  return bc = n, bc;
}
var _c, Q0;
function nq() {
  return Q0 || (Q0 = 1, _c = /* @__PURE__ */ iq()), _c;
}
var sq = /* @__PURE__ */ nq(), aq = /* @__PURE__ */ Q(sq), wc, J0;
function oq() {
  if (J0) return wc;
  J0 = 1, Br(), xn(), ID();
  var n = /* @__PURE__ */ ka();
  return wc = n.f("iterator"), wc;
}
var xc, ew;
function HD() {
  if (ew) return xc;
  ew = 1;
  var n = /* @__PURE__ */ oq();
  return Wr(), xc = n, xc;
}
var Sc, tw;
function lq() {
  if (tw) return Sc;
  tw = 1;
  var n = /* @__PURE__ */ HD();
  return Sc = n, Sc;
}
var Tc, rw;
function uq() {
  if (rw) return Tc;
  rw = 1;
  var n = /* @__PURE__ */ lq();
  return Tc = n, Tc;
}
var Ec, iw;
function cq() {
  return iw || (iw = 1, Ec = /* @__PURE__ */ uq()), Ec;
}
var dq = /* @__PURE__ */ cq(), hq = /* @__PURE__ */ Q(dq);
function Un(n) {
  "@babel/helpers - typeof";
  return Un = typeof Yn == "function" && typeof hq == "symbol" ? function(e) {
    return typeof e;
  } : function(e) {
    return e && typeof Yn == "function" && e.constructor === Yn && e !== Yn.prototype ? "symbol" : typeof e;
  }, Un(n);
}
var Dc, nw;
function fq() {
  if (nw) return Dc;
  nw = 1, MD();
  var n = /* @__PURE__ */ ka();
  return Dc = n.f("toPrimitive"), Dc;
}
var Oc, sw;
function pq() {
  if (sw) return Oc;
  sw = 1;
  var n = /* @__PURE__ */ fq();
  return Oc = n, Oc;
}
var kc, aw;
function mq() {
  if (aw) return kc;
  aw = 1;
  var n = /* @__PURE__ */ pq();
  return kc = n, kc;
}
var Cc, ow;
function vq() {
  if (ow) return Cc;
  ow = 1;
  var n = /* @__PURE__ */ mq();
  return Cc = n, Cc;
}
var Rc, lw;
function gq() {
  return lw || (lw = 1, Rc = /* @__PURE__ */ vq()), Rc;
}
var yq = /* @__PURE__ */ gq(), bq = /* @__PURE__ */ Q(yq);
function _q(n, e) {
  if (Un(n) != "object" || !n) return n;
  var t = n[bq];
  if (t !== void 0) {
    var r = t.call(n, e);
    if (Un(r) != "object") return r;
    throw new TypeError("@@toPrimitive must return a primitive value.");
  }
  return (e === "string" ? String : Number)(n);
}
function wq(n) {
  var e = _q(n, "string");
  return Un(e) == "symbol" ? e : e + "";
}
function Le(n, e, t) {
  return (e = wq(e)) in n ? aq(n, e, {
    value: t,
    enumerable: !0,
    configurable: !0,
    writable: !0
  }) : n[e] = t, n;
}
var uw = {}, Pc, cw;
function xq() {
  if (cw) return Pc;
  cw = 1;
  var n = /* @__PURE__ */ Pe(), e = /* @__PURE__ */ vr(), t = /* @__PURE__ */ bt(), r = /* @__PURE__ */ dt(), s = /* @__PURE__ */ bn(), a = /* @__PURE__ */ as(), o = Function, l = n([].concat), c = n([].join), d = {}, f = function(p, m, g) {
    if (!r(d, m)) {
      for (var b = [], _ = 0; _ < m; _++) b[_] = "a[" + _ + "]";
      d[m] = o("C,a", "return new C(" + c(b, ",") + ")");
    }
    return d[m](p, g);
  };
  return Pc = a ? o.bind : function(m) {
    var g = e(this), b = g.prototype, _ = s(arguments, 1), x = function() {
      var S = l(_, s(arguments));
      return this instanceof x ? f(g, S.length, S) : g.apply(m, S);
    };
    return t(b) && (x.prototype = b), x;
  }, Pc;
}
var dw;
function Sq() {
  if (dw) return uw;
  dw = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ xq();
  return n({ target: "Function", proto: !0, forced: Function.bind !== e }, {
    bind: e
  }), uw;
}
var Ic, hw;
function Tq() {
  if (hw) return Ic;
  hw = 1, Sq();
  var n = /* @__PURE__ */ Ke();
  return Ic = n("Function", "bind"), Ic;
}
var Mc, fw;
function Eq() {
  if (fw) return Mc;
  fw = 1;
  var n = /* @__PURE__ */ He(), e = /* @__PURE__ */ Tq(), t = Function.prototype;
  return Mc = function(r) {
    var s = r.bind;
    return r === t || n(t, r) && s === t.bind ? e : s;
  }, Mc;
}
var Ac, pw;
function Dq() {
  if (pw) return Ac;
  pw = 1;
  var n = /* @__PURE__ */ Eq();
  return Ac = n, Ac;
}
var Nc, mw;
function Oq() {
  return mw || (mw = 1, Nc = /* @__PURE__ */ Dq()), Nc;
}
var kq = Oq(), N = /* @__PURE__ */ Q(kq), vw = {}, qc, gw;
function Cq() {
  if (gw) return qc;
  gw = 1;
  var n = /* @__PURE__ */ vr(), e = /* @__PURE__ */ kt(), t = /* @__PURE__ */ xa(), r = /* @__PURE__ */ Zt(), s = TypeError, a = "Reduce of empty array with no initial value", o = function(l) {
    return function(c, d, f, p) {
      var m = e(c), g = t(m), b = r(m);
      if (n(d), b === 0 && f < 2) throw new s(a);
      var _ = l ? b - 1 : 0, x = l ? -1 : 1;
      if (f < 2) for (; ; ) {
        if (_ in g) {
          p = g[_], _ += x;
          break;
        }
        if (_ += x, l ? _ < 0 : b <= _)
          throw new s(a);
      }
      for (; l ? _ >= 0 : b > _; _ += x) _ in g && (p = d(p, g[_], _, m));
      return p;
    };
  };
  return qc = {
    // `Array.prototype.reduce` method
    // https://tc39.es/ecma262/#sec-array.prototype.reduce
    left: o(!1),
    // `Array.prototype.reduceRight` method
    // https://tc39.es/ecma262/#sec-array.prototype.reduceright
    right: o(!0)
  }, qc;
}
var Lc, yw;
function Sn() {
  if (yw) return Lc;
  yw = 1;
  var n = /* @__PURE__ */ we();
  return Lc = function(e, t) {
    var r = [][e];
    return !!r && n(function() {
      r.call(null, t || function() {
        return 1;
      }, 1);
    });
  }, Lc;
}
var Fc, bw;
function GD() {
  if (bw) return Fc;
  bw = 1;
  var n = /* @__PURE__ */ Ze(), e = /* @__PURE__ */ hn(), t = /* @__PURE__ */ xr(), r = function(s) {
    return e.slice(0, s.length) === s;
  };
  return Fc = (function() {
    return r("Bun/") ? "BUN" : r("Cloudflare-Workers") ? "CLOUDFLARE" : r("Deno/") ? "DENO" : r("Node.js/") ? "NODE" : n.Bun && typeof Bun.version == "string" ? "BUN" : n.Deno && typeof Deno.version == "object" ? "DENO" : t(n.process) === "process" ? "NODE" : n.window && n.document ? "BROWSER" : "REST";
  })(), Fc;
}
var $c, _w;
function Rq() {
  if (_w) return $c;
  _w = 1;
  var n = /* @__PURE__ */ GD();
  return $c = n === "NODE", $c;
}
var ww;
function Pq() {
  if (ww) return vw;
  ww = 1;
  var n = /* @__PURE__ */ X(), e = Cq().left, t = /* @__PURE__ */ Sn(), r = /* @__PURE__ */ us(), s = /* @__PURE__ */ Rq(), a = !s && r > 79 && r < 83, o = a || !t("reduce");
  return n({ target: "Array", proto: !0, forced: o }, {
    reduce: function(c) {
      var d = arguments.length;
      return e(this, c, d, d > 1 ? arguments[1] : void 0);
    }
  }), vw;
}
var jc, xw;
function Iq() {
  if (xw) return jc;
  xw = 1, Pq();
  var n = /* @__PURE__ */ Ke();
  return jc = n("Array", "reduce"), jc;
}
var Yc, Sw;
function Mq() {
  if (Sw) return Yc;
  Sw = 1;
  var n = /* @__PURE__ */ He(), e = /* @__PURE__ */ Iq(), t = Array.prototype;
  return Yc = function(r) {
    var s = r.reduce;
    return r === t || n(t, r) && s === t.reduce ? e : s;
  }, Yc;
}
var Hc, Tw;
function Aq() {
  if (Tw) return Hc;
  Tw = 1;
  var n = /* @__PURE__ */ Mq();
  return Hc = n, Hc;
}
var Gc, Ew;
function Nq() {
  return Ew || (Ew = 1, Gc = /* @__PURE__ */ Aq()), Gc;
}
var qq = Nq(), Aa = /* @__PURE__ */ Q(qq), Dw = {}, Ow;
function Lq() {
  if (Ow) return Dw;
  Ow = 1;
  var n = /* @__PURE__ */ X(), e = oi().filter, t = /* @__PURE__ */ gs(), r = t("filter");
  return n({ target: "Array", proto: !0, forced: !r }, {
    filter: function(a) {
      return e(this, a, arguments.length > 1 ? arguments[1] : void 0);
    }
  }), Dw;
}
var Bc, kw;
function Fq() {
  if (kw) return Bc;
  kw = 1, Lq();
  var n = /* @__PURE__ */ Ke();
  return Bc = n("Array", "filter"), Bc;
}
var Wc, Cw;
function $q() {
  if (Cw) return Wc;
  Cw = 1;
  var n = /* @__PURE__ */ He(), e = /* @__PURE__ */ Fq(), t = Array.prototype;
  return Wc = function(r) {
    var s = r.filter;
    return r === t || n(t, r) && s === t.filter ? e : s;
  }, Wc;
}
var zc, Rw;
function jq() {
  if (Rw) return zc;
  Rw = 1;
  var n = /* @__PURE__ */ $q();
  return zc = n, zc;
}
var Uc, Pw;
function Yq() {
  return Pw || (Pw = 1, Uc = /* @__PURE__ */ jq()), Uc;
}
var Hq = Yq(), ie = /* @__PURE__ */ Q(Hq), Iw = {}, Mw;
function Gq() {
  if (Mw) return Iw;
  Mw = 1;
  var n = /* @__PURE__ */ X(), e = oi().map, t = /* @__PURE__ */ gs(), r = t("map");
  return n({ target: "Array", proto: !0, forced: !r }, {
    map: function(a) {
      return e(this, a, arguments.length > 1 ? arguments[1] : void 0);
    }
  }), Iw;
}
var Vc, Aw;
function Bq() {
  if (Aw) return Vc;
  Aw = 1, Gq();
  var n = /* @__PURE__ */ Ke();
  return Vc = n("Array", "map"), Vc;
}
var Kc, Nw;
function Wq() {
  if (Nw) return Kc;
  Nw = 1;
  var n = /* @__PURE__ */ He(), e = /* @__PURE__ */ Bq(), t = Array.prototype;
  return Kc = function(r) {
    var s = r.map;
    return r === t || n(t, r) && s === t.map ? e : s;
  }, Kc;
}
var Xc, qw;
function zq() {
  if (qw) return Xc;
  qw = 1;
  var n = /* @__PURE__ */ Wq();
  return Xc = n, Xc;
}
var Zc, Lw;
function Uq() {
  return Lw || (Lw = 1, Zc = /* @__PURE__ */ zq()), Zc;
}
var Vq = Uq(), Ae = /* @__PURE__ */ Q(Vq), Fw = {}, Qc, $w;
function Kq() {
  if ($w) return Qc;
  $w = 1;
  var n = /* @__PURE__ */ ai(), e = /* @__PURE__ */ Zt(), t = /* @__PURE__ */ Pa(), r = /* @__PURE__ */ vn(), s = /* @__PURE__ */ Ri(), a = function(o, l, c, d, f, p, m, g) {
    for (var b = f, _ = 0, x = m ? r(m, g) : !1, T, S; _ < d; )
      _ in c && (T = x ? x(c[_], _, l) : c[_], p > 0 && n(T) ? (S = e(T), b = a(o, l, T, S, b, p - 1) - 1) : (t(b + 1), s(o, b, T)), b++), _++;
    return b;
  };
  return Qc = a, Qc;
}
var jw;
function Xq() {
  if (jw) return Fw;
  jw = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ Kq(), t = /* @__PURE__ */ vr(), r = /* @__PURE__ */ kt(), s = /* @__PURE__ */ Zt(), a = /* @__PURE__ */ Ca();
  return n({ target: "Array", proto: !0 }, {
    flatMap: function(l) {
      var c = r(this), d = s(c), f;
      return t(l), f = a(c, 0), e(f, c, c, d, 0, 1, l, arguments.length > 1 ? arguments[1] : void 0), f;
    }
  }), Fw;
}
var Yw = {}, Hw;
function Zq() {
  if (Hw) return Yw;
  Hw = 1;
  var n = /* @__PURE__ */ ys();
  return n("flatMap"), Yw;
}
var Jc, Gw;
function Qq() {
  if (Gw) return Jc;
  Gw = 1, Xq(), Zq();
  var n = /* @__PURE__ */ Ke();
  return Jc = n("Array", "flatMap"), Jc;
}
var ed, Bw;
function Jq() {
  if (Bw) return ed;
  Bw = 1;
  var n = /* @__PURE__ */ He(), e = /* @__PURE__ */ Qq(), t = Array.prototype;
  return ed = function(r) {
    var s = r.flatMap;
    return r === t || n(t, r) && s === t.flatMap ? e : s;
  }, ed;
}
var td, Ww;
function eL() {
  if (Ww) return td;
  Ww = 1;
  var n = /* @__PURE__ */ Jq();
  return td = n, td;
}
var rd, zw;
function tL() {
  return zw || (zw = 1, rd = /* @__PURE__ */ eL()), rd;
}
var rL = tL(), iL = /* @__PURE__ */ Q(rL), Uw = {}, Vw = {}, id, Kw;
function nL() {
  if (Kw) return id;
  Kw = 1;
  var n = TypeError;
  return id = function(e, t) {
    if (e < t) throw new n("Not enough arguments");
    return e;
  }, id;
}
var nd, Xw;
function BD() {
  if (Xw) return nd;
  Xw = 1;
  var n = /* @__PURE__ */ Ze(), e = /* @__PURE__ */ _a(), t = /* @__PURE__ */ yt(), r = /* @__PURE__ */ GD(), s = /* @__PURE__ */ hn(), a = /* @__PURE__ */ bn(), o = /* @__PURE__ */ nL(), l = n.Function, c = /MSIE .\./.test(s) || r === "BUN" && (function() {
    var d = n.Bun.version.split(".");
    return d.length < 3 || d[0] === "0" && (d[1] < 3 || d[1] === "3" && d[2] === "0");
  })();
  return nd = function(d, f) {
    var p = f ? 2 : 1;
    return c ? function(m, g) {
      var b = o(arguments.length, 1) > p, _ = t(m) ? m : l(m), x = b ? a(arguments, p) : [], T = b ? function() {
        e(_, this, x);
      } : _;
      return f ? d(T, g) : d(T);
    } : d;
  }, nd;
}
var Zw;
function sL() {
  if (Zw) return Vw;
  Zw = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ Ze(), t = /* @__PURE__ */ BD(), r = t(e.setInterval, !0);
  return n({ global: !0, bind: !0, forced: e.setInterval !== r }, {
    setInterval: r
  }), Vw;
}
var Qw = {}, Jw;
function aL() {
  if (Jw) return Qw;
  Jw = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ Ze(), t = /* @__PURE__ */ BD(), r = t(e.setTimeout, !0);
  return n({ global: !0, bind: !0, forced: e.setTimeout !== r }, {
    setTimeout: r
  }), Qw;
}
var ex;
function WD() {
  return ex || (ex = 1, sL(), aL()), Uw;
}
var sd, tx;
function oL() {
  if (tx) return sd;
  tx = 1, WD();
  var n = /* @__PURE__ */ je();
  return sd = n.setTimeout, sd;
}
var ad, rx;
function lL() {
  return rx || (rx = 1, ad = /* @__PURE__ */ oL()), ad;
}
var uL = lL(), Dt = /* @__PURE__ */ Q(uL), ix = {}, od, nx;
function cL() {
  if (nx) return od;
  nx = 1;
  var n = oi().forEach, e = /* @__PURE__ */ Sn(), t = e("forEach");
  return od = t ? [].forEach : function(s) {
    return n(this, s, arguments.length > 1 ? arguments[1] : void 0);
  }, od;
}
var sx;
function dL() {
  if (sx) return ix;
  sx = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ cL();
  return n({ target: "Array", proto: !0, forced: [].forEach !== e }, {
    forEach: e
  }), ix;
}
var ld, ax;
function hL() {
  if (ax) return ld;
  ax = 1, dL();
  var n = /* @__PURE__ */ Ke();
  return ld = n("Array", "forEach"), ld;
}
var ud, ox;
function fL() {
  if (ox) return ud;
  ox = 1;
  var n = /* @__PURE__ */ hL();
  return ud = n, ud;
}
var cd, lx;
function pL() {
  if (lx) return cd;
  lx = 1;
  var n = /* @__PURE__ */ si(), e = /* @__PURE__ */ dt(), t = /* @__PURE__ */ He(), r = /* @__PURE__ */ fL(), s = Array.prototype, a = {
    DOMTokenList: !0,
    NodeList: !0
  };
  return cd = function(o) {
    var l = o.forEach;
    return o === s || t(s, o) && l === s.forEach || e(a, n(o)) ? r : l;
  }, cd;
}
var dd, ux;
function mL() {
  return ux || (ux = 1, dd = /* @__PURE__ */ pL()), dd;
}
var vL = mL(), B = /* @__PURE__ */ Q(vL), cx = {}, hd, dx;
function zD() {
  if (dx) return hd;
  dx = 1;
  var n = /* @__PURE__ */ pn(), e = TypeError;
  return hd = function(t, r) {
    if (!delete t[r]) throw new e("Cannot delete property " + n(r) + " of " + n(t));
  }, hd;
}
var hx;
function gL() {
  if (hx) return cx;
  hx = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ kt(), t = /* @__PURE__ */ Ta(), r = /* @__PURE__ */ yn(), s = /* @__PURE__ */ Zt(), a = /* @__PURE__ */ vs(), o = /* @__PURE__ */ Pa(), l = /* @__PURE__ */ Ca(), c = /* @__PURE__ */ Ri(), d = /* @__PURE__ */ zD(), f = /* @__PURE__ */ gs(), p = f("splice"), m = Math.max, g = Math.min;
  return n({ target: "Array", proto: !0, forced: !p }, {
    splice: function(_, x) {
      var T = e(this), S = s(T), k = t(_, S), D = arguments.length, I, C, P, R, M, q;
      for (D === 0 ? I = C = 0 : D === 1 ? (I = 0, C = S - k) : (I = D - 2, C = g(m(r(x), 0), S - k)), o(S + I - C), P = l(T, C), R = 0; R < C; R++)
        M = k + R, M in T && c(P, R, T[M]);
      if (a(P, C), I < C) {
        for (R = k; R < S - C; R++)
          M = R + C, q = R + I, M in T ? T[q] = T[M] : d(T, q);
        for (R = S; R > S - C + I; R--) d(T, R - 1);
      } else if (I > C)
        for (R = S - C; R > k; R--)
          M = R + C - 1, q = R + I - 1, M in T ? T[q] = T[M] : d(T, q);
      for (R = 0; R < I; R++)
        T[R + k] = arguments[R + 2];
      return a(T, S - C + I), P;
    }
  }), cx;
}
var fd, fx;
function yL() {
  if (fx) return fd;
  fx = 1, gL();
  var n = /* @__PURE__ */ Ke();
  return fd = n("Array", "splice"), fd;
}
var pd, px;
function bL() {
  if (px) return pd;
  px = 1;
  var n = /* @__PURE__ */ He(), e = /* @__PURE__ */ yL(), t = Array.prototype;
  return pd = function(r) {
    var s = r.splice;
    return r === t || n(t, r) && s === t.splice ? e : s;
  }, pd;
}
var md, mx;
function _L() {
  if (mx) return md;
  mx = 1;
  var n = /* @__PURE__ */ bL();
  return md = n, md;
}
var vd, vx;
function wL() {
  return vx || (vx = 1, vd = /* @__PURE__ */ _L()), vd;
}
var xL = wL(), Mt = /* @__PURE__ */ Q(xL), gd, gx;
function SL() {
  return gx || (gx = 1, gd = /* @__PURE__ */ HD()), gd;
}
var TL = SL(), vi = /* @__PURE__ */ Q(TL), yx = {}, bx;
function EL() {
  if (bx) return yx;
  bx = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ it(), t = /* @__PURE__ */ ps();
  return n({ target: "Object", stat: !0, sham: !e }, {
    create: t
  }), yx;
}
var yd, _x;
function DL() {
  if (_x) return yd;
  _x = 1, EL();
  var n = /* @__PURE__ */ je(), e = n.Object;
  return yd = function(r, s) {
    return e.create(r, s);
  }, yd;
}
var bd, wx;
function OL() {
  if (wx) return bd;
  wx = 1;
  var n = /* @__PURE__ */ DL();
  return bd = n, bd;
}
var _d, xx;
function kL() {
  return xx || (xx = 1, _d = /* @__PURE__ */ OL()), _d;
}
var CL = kL(), _s = /* @__PURE__ */ Q(CL), Sx = {}, Tx = {}, wd = { exports: {} }, xd, Ex;
function RL() {
  if (Ex) return xd;
  Ex = 1;
  var n = /* @__PURE__ */ we();
  return xd = n(function() {
    if (typeof ArrayBuffer == "function") {
      var e = new ArrayBuffer(8);
      Object.isExtensible(e) && Object.defineProperty(e, "a", { value: 8 });
    }
  }), xd;
}
var Sd, Dx;
function PL() {
  if (Dx) return Sd;
  Dx = 1;
  var n = /* @__PURE__ */ we(), e = /* @__PURE__ */ bt(), t = /* @__PURE__ */ xr(), r = /* @__PURE__ */ RL(), s = Object.isExtensible, a = n(function() {
  });
  return Sd = a || r ? function(l) {
    return !e(l) || r && t(l) === "ArrayBuffer" ? !1 : s ? s(l) : !0;
  } : s, Sd;
}
var Td, Ox;
function IL() {
  if (Ox) return Td;
  Ox = 1;
  var n = /* @__PURE__ */ we();
  return Td = !n(function() {
    return Object.isExtensible(Object.preventExtensions({}));
  }), Td;
}
var kx;
function UD() {
  if (kx) return wd.exports;
  kx = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ Pe(), t = /* @__PURE__ */ hs(), r = /* @__PURE__ */ bt(), s = /* @__PURE__ */ dt(), a = sr().f, o = /* @__PURE__ */ Da(), l = /* @__PURE__ */ wD(), c = /* @__PURE__ */ PL(), d = /* @__PURE__ */ ds(), f = /* @__PURE__ */ IL(), p = !1, m = d("meta"), g = 0, b = function(D) {
    a(D, m, { value: {
      objectID: "O" + g++,
      // object ID
      weakData: {}
      // weak collections IDs
    } });
  }, _ = function(D, I) {
    if (!r(D)) return typeof D == "symbol" ? D : (typeof D == "string" ? "S" : "P") + D;
    if (!s(D, m)) {
      if (!c(D)) return "F";
      if (!I) return "E";
      b(D);
    }
    return D[m].objectID;
  }, x = function(D, I) {
    if (!s(D, m)) {
      if (!c(D)) return !0;
      if (!I) return !1;
      b(D);
    }
    return D[m].weakData;
  }, T = function(D) {
    return f && p && c(D) && !s(D, m) && b(D), D;
  }, S = function() {
    k.enable = function() {
    }, p = !0;
    var D = o.f, I = e([].splice), C = {};
    C[m] = 1, D(C).length && (o.f = function(P) {
      for (var R = D(P), M = 0, q = R.length; M < q; M++)
        if (R[M] === m) {
          I(R, M, 1);
          break;
        }
      return R;
    }, n({ target: "Object", stat: !0, forced: !0 }, {
      getOwnPropertyNames: l.f
    }));
  }, k = wd.exports = {
    enable: S,
    fastKey: _,
    getWeakData: x,
    onFreeze: T
  };
  return t[m] = !0, wd.exports;
}
var Ed, Cx;
function vm() {
  if (Cx) return Ed;
  Cx = 1;
  var n = /* @__PURE__ */ vn(), e = /* @__PURE__ */ Ot(), t = /* @__PURE__ */ Sr(), r = /* @__PURE__ */ pn(), s = /* @__PURE__ */ jD(), a = /* @__PURE__ */ Zt(), o = /* @__PURE__ */ He(), l = /* @__PURE__ */ mm(), c = /* @__PURE__ */ Ma(), d = /* @__PURE__ */ bs(), f = TypeError, p = function(g, b) {
    this.stopped = g, this.result = b;
  }, m = p.prototype;
  return Ed = function(g, b, _) {
    var x = _ && _.that, T = !!(_ && _.AS_ENTRIES), S = !!(_ && _.IS_RECORD), k = !!(_ && _.IS_ITERATOR), D = !!(_ && _.INTERRUPTED), I = n(b, x), C, P, R, M, q, F, H, Z = function(Y) {
      var J = C;
      return C = void 0, J && d(J, "normal"), new p(!0, Y);
    }, ve = function(Y) {
      return T ? (t(Y), D ? I(Y[0], Y[1], Z) : I(Y[0], Y[1])) : D ? I(Y, Z) : I(Y);
    };
    if (S)
      C = g.iterator;
    else if (k)
      C = g;
    else {
      if (P = c(g), !P) throw new f(r(g) + " is not iterable");
      if (s(P)) {
        for (R = 0, M = a(g); M > R; R++)
          if (q = ve(g[R]), q && o(m, q)) return q;
        return new p(!1);
      }
      C = l(g, P);
    }
    for (F = S ? g.next : C.next; !(H = e(F, C)).done; ) {
      var Me = H.value;
      try {
        q = ve(Me);
      } catch (Y) {
        if (C) d(C, "throw", Y);
        else throw Y;
      }
      if (typeof q == "object" && q && o(m, q)) return q;
    }
    return new p(!1);
  }, Ed;
}
var Dd, Rx;
function VD() {
  if (Rx) return Dd;
  Rx = 1;
  var n = /* @__PURE__ */ He(), e = TypeError;
  return Dd = function(t, r) {
    if (n(r, t)) return t;
    throw new e("Incorrect invocation");
  }, Dd;
}
var Od, Px;
function KD() {
  if (Px) return Od;
  Px = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ Ze(), t = /* @__PURE__ */ UD(), r = /* @__PURE__ */ Ot(), s = /* @__PURE__ */ we(), a = /* @__PURE__ */ gn(), o = /* @__PURE__ */ vm(), l = /* @__PURE__ */ VD(), c = /* @__PURE__ */ yt(), d = /* @__PURE__ */ bt(), f = /* @__PURE__ */ ls(), p = /* @__PURE__ */ Ci(), m = sr().f, g = oi().forEach, b = /* @__PURE__ */ it(), _ = /* @__PURE__ */ _n(), x = _.set, T = _.getterFor;
  return Od = function(S, k, D) {
    var I = S.indexOf("Map") !== -1, C = S.indexOf("Weak") !== -1, P = I ? "set" : "add", R = e[S], M = R && R.prototype, q = {}, F;
    if (!b || !c(R) || !(C || M.forEach && !s(function() {
      new R().entries().next();
    })))
      F = D.getConstructor(k, S, I, P), t.enable();
    else {
      F = k(function(ve, Me) {
        x(l(ve, H), {
          type: S,
          collection: new R()
        }), f(Me) || o(Me, ve[P], { that: ve, AS_ENTRIES: I });
      });
      var H = F.prototype, Z = T(S);
      g(["add", "clear", "delete", "forEach", "get", "has", "set", "keys", "values", "entries"], function(ve) {
        var Me = ve === "add" || ve === "set";
        ve in M && !(C && ve === "clear") && a(H, ve, function(Y, J) {
          var z = this, te = Z(z).collection;
          if (!Me && C && !d(Y)) return ve === "get" ? void 0 : !1;
          var K = te[ve](ve === "forEach" ? function(re, ae) {
            r(Y, J, re, ae, z);
          } : Y === 0 ? 0 : Y, J);
          return Me ? z : K;
        });
      }), C || m(H, "size", {
        configurable: !0,
        get: function() {
          return Z(this).collection.size;
        }
      });
    }
    return p(F, S, !1, !0), q[S] = F, n({ global: !0, forced: !0 }, q), C || D.setStrong(F, S, I), F;
  }, Od;
}
var kd, Ix;
function ML() {
  if (Ix) return kd;
  Ix = 1;
  var n = /* @__PURE__ */ ms();
  return kd = function(e, t, r) {
    for (var s in t)
      r && r.unsafe && e[s] ? e[s] = t[s] : n(e, s, t[s], r);
    return e;
  }, kd;
}
var Cd, Mx;
function AL() {
  if (Mx) return Cd;
  Mx = 1;
  var n = /* @__PURE__ */ Xt(), e = /* @__PURE__ */ dm(), t = /* @__PURE__ */ nt(), r = /* @__PURE__ */ it(), s = t("species");
  return Cd = function(a) {
    var o = n(a);
    r && o && !o[s] && e(o, s, {
      configurable: !0,
      get: function() {
        return this;
      }
    });
  }, Cd;
}
var Rd, Ax;
function XD() {
  if (Ax) return Rd;
  Ax = 1;
  var n = /* @__PURE__ */ ps(), e = /* @__PURE__ */ dm(), t = /* @__PURE__ */ ML(), r = /* @__PURE__ */ vn(), s = /* @__PURE__ */ VD(), a = /* @__PURE__ */ ls(), o = /* @__PURE__ */ vm(), l = /* @__PURE__ */ fm(), c = /* @__PURE__ */ pm(), d = /* @__PURE__ */ AL(), f = /* @__PURE__ */ it(), p = UD().fastKey, m = /* @__PURE__ */ _n(), g = m.set, b = m.getterFor;
  return Rd = {
    getConstructor: function(_, x, T, S) {
      var k = _(function(R, M) {
        s(R, D), g(R, {
          type: x,
          index: n(null),
          first: null,
          last: null,
          size: 0
        }), f || (R.size = 0), a(M) || o(M, R[S], { that: R, AS_ENTRIES: T });
      }), D = k.prototype, I = b(x), C = function(R, M, q) {
        var F = I(R), H = P(R, M), Z, ve;
        return H ? H.value = q : (F.last = H = {
          index: ve = p(M, !0),
          key: M,
          value: q,
          previous: Z = F.last,
          next: null,
          removed: !1
        }, F.first || (F.first = H), Z && (Z.next = H), f ? F.size++ : R.size++, ve !== "F" && (F.index[ve] = H)), R;
      }, P = function(R, M) {
        var q = I(R), F = p(M), H;
        if (F !== "F") return q.index[F];
        for (H = q.first; H; H = H.next)
          if (H.key === M) return H;
      };
      return t(D, {
        // `{ Map, Set }.prototype.clear()` methods
        // https://tc39.es/ecma262/#sec-map.prototype.clear
        // https://tc39.es/ecma262/#sec-set.prototype.clear
        clear: function() {
          for (var M = this, q = I(M), F = q.first; F; )
            F.removed = !0, F.previous && (F.previous = F.previous.next = null), F = F.next;
          q.first = q.last = null, q.index = n(null), f ? q.size = 0 : M.size = 0;
        },
        // `{ Map, Set }.prototype.delete(key)` methods
        // https://tc39.es/ecma262/#sec-map.prototype.delete
        // https://tc39.es/ecma262/#sec-set.prototype.delete
        delete: function(R) {
          var M = this, q = I(M), F = P(M, R);
          if (F) {
            var H = F.next, Z = F.previous;
            delete q.index[F.index], F.removed = !0, Z && (Z.next = H), H && (H.previous = Z), q.first === F && (q.first = H), q.last === F && (q.last = Z), f ? q.size-- : M.size--;
          }
          return !!F;
        },
        // `{ Map, Set }.prototype.forEach(callbackfn, thisArg = undefined)` methods
        // https://tc39.es/ecma262/#sec-map.prototype.foreach
        // https://tc39.es/ecma262/#sec-set.prototype.foreach
        forEach: function(M) {
          for (var q = I(this), F = r(M, arguments.length > 1 ? arguments[1] : void 0), H; H = H ? H.next : q.first; )
            for (F(H.value, H.key, this); H && H.removed; ) H = H.previous;
        },
        // `{ Map, Set}.prototype.has(key)` methods
        // https://tc39.es/ecma262/#sec-map.prototype.has
        // https://tc39.es/ecma262/#sec-set.prototype.has
        has: function(M) {
          return !!P(this, M);
        }
      }), t(D, T ? {
        // `Map.prototype.get(key)` method
        // https://tc39.es/ecma262/#sec-map.prototype.get
        get: function(M) {
          var q = P(this, M);
          return q && q.value;
        },
        // `Map.prototype.set(key, value)` method
        // https://tc39.es/ecma262/#sec-map.prototype.set
        set: function(M, q) {
          return C(this, M === 0 ? 0 : M, q);
        }
      } : {
        // `Set.prototype.add(value)` method
        // https://tc39.es/ecma262/#sec-set.prototype.add
        add: function(M) {
          return C(this, M = M === 0 ? 0 : M, M);
        }
      }), f && e(D, "size", {
        configurable: !0,
        get: function() {
          return I(this).size;
        }
      }), k;
    },
    setStrong: function(_, x, T) {
      var S = x + " Iterator", k = b(x), D = b(S);
      l(_, x, function(I, C) {
        g(this, {
          type: S,
          target: I,
          state: k(I),
          kind: C,
          last: null
        });
      }, function() {
        for (var I = D(this), C = I.kind, P = I.last; P && P.removed; ) P = P.previous;
        return !I.target || !(I.last = P = P ? P.next : I.state.first) ? (I.target = null, c(void 0, !0)) : c(C === "keys" ? P.key : C === "values" ? P.value : [P.key, P.value], !1);
      }, T ? "entries" : "values", !T, !0), d(x);
    }
  }, Rd;
}
var Nx;
function NL() {
  if (Nx) return Tx;
  Nx = 1;
  var n = /* @__PURE__ */ KD(), e = /* @__PURE__ */ XD();
  return n("Map", function(t) {
    return function() {
      return t(this, arguments.length ? arguments[0] : void 0);
    };
  }, e), Tx;
}
var qx;
function qL() {
  return qx || (qx = 1, NL()), Sx;
}
var Lx = {}, Pd, Fx;
function ZD() {
  return Fx || (Fx = 1, Pd = function(n, e) {
    return e === 1 ? function(t, r) {
      return t[n](r);
    } : function(t, r, s) {
      return t[n](r, s);
    };
  }), Pd;
}
var Id, $x;
function gm() {
  if ($x) return Id;
  $x = 1;
  var n = /* @__PURE__ */ Xt(), e = /* @__PURE__ */ ZD(), t = n("Map");
  return Id = {
    Map: t,
    set: e("set", 2),
    get: e("get", 1),
    has: e("has", 1),
    remove: e("delete", 1),
    proto: t.prototype
  }, Id;
}
var jx;
function LL() {
  if (jx) return Lx;
  jx = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ Pe(), t = /* @__PURE__ */ vr(), r = /* @__PURE__ */ Gr(), s = /* @__PURE__ */ vm(), a = /* @__PURE__ */ gm(), o = /* @__PURE__ */ ni(), l = /* @__PURE__ */ we(), c = a.Map, d = a.has, f = a.get, p = a.set, m = e([].push), g = o || l(function() {
    return c.groupBy("ab", function(b) {
      return b;
    }).get("a").length !== 1;
  });
  return n({ target: "Map", stat: !0, forced: o || g }, {
    groupBy: function(_, x) {
      r(_), t(x);
      var T = new c(), S = 0;
      return s(_, function(k) {
        var D = x(k, S++);
        d(T, D) ? m(f(T, D), k) : p(T, D, [k]);
      }), T;
    }
  }), Lx;
}
var Yx = {}, Hx;
function FL() {
  if (Hx) return Yx;
  Hx = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ gm(), t = /* @__PURE__ */ ni(), r = e.get, s = e.has, a = e.set;
  return n({ target: "Map", proto: !0, real: !0, forced: t }, {
    getOrInsert: function(l, c) {
      return s(this, l) ? r(this, l) : (a(this, l, c), c);
    }
  }), Yx;
}
var Gx = {}, Bx;
function $L() {
  if (Bx) return Gx;
  Bx = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ vr(), t = /* @__PURE__ */ gm(), r = /* @__PURE__ */ ni(), s = t.get, a = t.has, o = t.set;
  return n({ target: "Map", proto: !0, real: !0, forced: r }, {
    getOrInsertComputed: function(c, d) {
      var f = a(this, c);
      if (e(d), f) return s(this, c);
      c === 0 && 1 / c === -1 / 0 && (c = 0);
      var p = d(c);
      return o(this, c, p), p;
    }
  }), Gx;
}
var Md, Wx;
function jL() {
  if (Wx) return Md;
  Wx = 1, Br(), qL(), LL(), FL(), $L(), xn();
  var n = /* @__PURE__ */ je();
  return Md = n.Map, Md;
}
var Ad, zx;
function YL() {
  if (zx) return Ad;
  zx = 1;
  var n = /* @__PURE__ */ jL();
  return Wr(), Ad = n, Ad;
}
var Nd, Ux;
function HL() {
  return Ux || (Ux = 1, Nd = /* @__PURE__ */ YL()), Nd;
}
var GL = HL(), QD = /* @__PURE__ */ Q(GL), Vx = {}, Kx = {}, Xx;
function BL() {
  if (Xx) return Kx;
  Xx = 1;
  var n = /* @__PURE__ */ KD(), e = /* @__PURE__ */ XD();
  return n("Set", function(t) {
    return function() {
      return t(this, arguments.length ? arguments[0] : void 0);
    };
  }, e), Kx;
}
var Zx;
function WL() {
  return Zx || (Zx = 1, BL()), Vx;
}
var Qx = {}, qd, Jx;
function Pi() {
  if (Jx) return qd;
  Jx = 1;
  var n = /* @__PURE__ */ pn(), e = TypeError;
  return qd = function(t) {
    if (typeof t == "object" && "size" in t && "has" in t && "add" in t && "delete" in t && "keys" in t) return t;
    throw new e(n(t) + " is not a set");
  }, qd;
}
var Ld, e1;
function Ii() {
  if (e1) return Ld;
  e1 = 1;
  var n = /* @__PURE__ */ Xt(), e = /* @__PURE__ */ ZD(), t = n("Set"), r = t.prototype;
  return Ld = {
    Set: t,
    add: e("add", 1),
    has: e("has", 1),
    remove: e("delete", 1),
    proto: r
  }, Ld;
}
var Fd, t1;
function Mi() {
  if (t1) return Fd;
  t1 = 1;
  var n = /* @__PURE__ */ Ot();
  return Fd = function(e, t, r) {
    for (var s = r ? e : e.iterator, a = e.next, o, l; !(o = n(a, s)).done; )
      if (l = t(o.value), l !== void 0) return l;
  }, Fd;
}
var $d, r1;
function ws() {
  if (r1) return $d;
  r1 = 1;
  var n = /* @__PURE__ */ Mi();
  return $d = function(e, t, r) {
    return r ? n(e.keys(), t, !0) : e.forEach(t);
  }, $d;
}
var jd, i1;
function ym() {
  if (i1) return jd;
  i1 = 1;
  var n = /* @__PURE__ */ Ii(), e = /* @__PURE__ */ ws(), t = n.Set, r = n.add;
  return jd = function(s) {
    var a = new t();
    return e(s, function(o) {
      r(a, o);
    }), a;
  }, jd;
}
var Yd, n1;
function xs() {
  return n1 || (n1 = 1, Yd = function(n) {
    return n.size;
  }), Yd;
}
var Hd, s1;
function zL() {
  return s1 || (s1 = 1, Hd = function(n) {
    return {
      iterator: n,
      next: n.next,
      done: !1
    };
  }), Hd;
}
var Gd, a1;
function Ai() {
  if (a1) return Gd;
  a1 = 1;
  var n = /* @__PURE__ */ vr(), e = /* @__PURE__ */ Sr(), t = /* @__PURE__ */ Ot(), r = /* @__PURE__ */ yn(), s = /* @__PURE__ */ zL(), a = "Invalid size", o = RangeError, l = TypeError, c = Math.max, d = function(f, p) {
    this.set = f, this.size = c(p, 0), this.has = n(f.has), this.keys = n(f.keys);
  };
  return d.prototype = {
    getIterator: function() {
      return s(e(t(this.keys, this.set)));
    },
    includes: function(f) {
      return t(this.has, this.set, f);
    }
  }, Gd = function(f) {
    e(f);
    var p = +f.size;
    if (p !== p) throw new l(a);
    var m = r(p);
    if (m < 0) throw new o(a);
    return new d(f, m);
  }, Gd;
}
var Bd, o1;
function UL() {
  if (o1) return Bd;
  o1 = 1;
  var n = /* @__PURE__ */ Pi(), e = /* @__PURE__ */ Ii(), t = /* @__PURE__ */ ym(), r = /* @__PURE__ */ xs(), s = /* @__PURE__ */ Ai(), a = /* @__PURE__ */ ws(), o = /* @__PURE__ */ Mi(), l = e.has, c = e.remove;
  return Bd = function(f) {
    var p = n(this), m = s(f), g = t(p);
    return r(g) <= m.size ? a(g, function(b) {
      m.includes(b) && c(g, b);
    }) : o(m.getIterator(), function(b) {
      l(g, b) && c(g, b);
    }), g;
  }, Bd;
}
var Wd, l1;
function Ni() {
  return l1 || (l1 = 1, Wd = function() {
    return !1;
  }), Wd;
}
var u1;
function VL() {
  if (u1) return Qx;
  u1 = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ UL(), t = /* @__PURE__ */ we(), r = /* @__PURE__ */ Ni(), s = !r("difference", function(o) {
    return o.size === 0;
  }), a = s || t(function() {
    var o = {
      size: 1,
      has: function() {
        return !0;
      },
      keys: function() {
        var c = 0;
        return {
          next: function() {
            var d = c++ > 1;
            return l.has(1) && l.clear(), { done: d, value: 2 };
          }
        };
      }
    }, l = /* @__PURE__ */ new Set([1, 2, 3, 4]);
    return l.difference(o).size !== 3;
  });
  return n({ target: "Set", proto: !0, real: !0, forced: a }, {
    difference: e
  }), Qx;
}
var c1 = {}, zd, d1;
function KL() {
  if (d1) return zd;
  d1 = 1;
  var n = /* @__PURE__ */ Pi(), e = /* @__PURE__ */ Ii(), t = /* @__PURE__ */ xs(), r = /* @__PURE__ */ Ai(), s = /* @__PURE__ */ ws(), a = /* @__PURE__ */ Mi(), o = e.Set, l = e.add, c = e.has;
  return zd = function(f) {
    var p = n(this), m = r(f), g = new o();
    return t(p) > m.size ? a(m.getIterator(), function(b) {
      c(p, b) && l(g, b);
    }) : s(p, function(b) {
      m.includes(b) && l(g, b);
    }), g;
  }, zd;
}
var h1;
function XL() {
  if (h1) return c1;
  h1 = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ we(), t = /* @__PURE__ */ KL(), r = /* @__PURE__ */ Ni(), s = !r("intersection", function(a) {
    return a.size === 2 && a.has(1) && a.has(2);
  }) || e(function() {
    return String(Array.from((/* @__PURE__ */ new Set([1, 2, 3])).intersection(/* @__PURE__ */ new Set([3, 2])))) !== "3,2";
  });
  return n({ target: "Set", proto: !0, real: !0, forced: s }, {
    intersection: t
  }), c1;
}
var f1 = {}, Ud, p1;
function ZL() {
  if (p1) return Ud;
  p1 = 1;
  var n = /* @__PURE__ */ Pi(), e = Ii().has, t = /* @__PURE__ */ xs(), r = /* @__PURE__ */ Ai(), s = /* @__PURE__ */ ws(), a = /* @__PURE__ */ Mi(), o = /* @__PURE__ */ bs();
  return Ud = function(c) {
    var d = n(this), f = r(c);
    if (t(d) <= f.size) return s(d, function(m) {
      if (f.includes(m)) return !1;
    }, !0) !== !1;
    var p = f.getIterator();
    return a(p, function(m) {
      if (e(d, m)) return o(p.iterator, "normal", !1);
    }) !== !1;
  }, Ud;
}
var m1;
function QL() {
  if (m1) return f1;
  m1 = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ ZL(), t = /* @__PURE__ */ Ni(), r = !t("isDisjointFrom", function(s) {
    return !s;
  });
  return n({ target: "Set", proto: !0, real: !0, forced: r }, {
    isDisjointFrom: e
  }), f1;
}
var v1 = {}, Vd, g1;
function JL() {
  if (g1) return Vd;
  g1 = 1;
  var n = /* @__PURE__ */ Pi(), e = /* @__PURE__ */ xs(), t = /* @__PURE__ */ ws(), r = /* @__PURE__ */ Ai();
  return Vd = function(a) {
    var o = n(this), l = r(a);
    return e(o) > l.size ? !1 : t(o, function(c) {
      if (!l.includes(c)) return !1;
    }, !0) !== !1;
  }, Vd;
}
var y1;
function eF() {
  if (y1) return v1;
  y1 = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ JL(), t = /* @__PURE__ */ Ni(), r = !t("isSubsetOf", function(s) {
    return s;
  });
  return n({ target: "Set", proto: !0, real: !0, forced: r }, {
    isSubsetOf: e
  }), v1;
}
var b1 = {}, Kd, _1;
function tF() {
  if (_1) return Kd;
  _1 = 1;
  var n = /* @__PURE__ */ Pi(), e = Ii().has, t = /* @__PURE__ */ xs(), r = /* @__PURE__ */ Ai(), s = /* @__PURE__ */ Mi(), a = /* @__PURE__ */ bs();
  return Kd = function(l) {
    var c = n(this), d = r(l);
    if (t(c) < d.size) return !1;
    var f = d.getIterator();
    return s(f, function(p) {
      if (!e(c, p)) return a(f.iterator, "normal", !1);
    }) !== !1;
  }, Kd;
}
var w1;
function rF() {
  if (w1) return b1;
  w1 = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ tF(), t = /* @__PURE__ */ Ni(), r = !t("isSupersetOf", function(s) {
    return !s;
  });
  return n({ target: "Set", proto: !0, real: !0, forced: r }, {
    isSupersetOf: e
  }), b1;
}
var x1 = {}, Xd, S1;
function iF() {
  if (S1) return Xd;
  S1 = 1;
  var n = /* @__PURE__ */ Pi(), e = /* @__PURE__ */ Ii(), t = /* @__PURE__ */ ym(), r = /* @__PURE__ */ Ai(), s = /* @__PURE__ */ Mi(), a = e.add, o = e.has, l = e.remove;
  return Xd = function(d) {
    var f = n(this), p = r(d).getIterator(), m = t(f);
    return s(p, function(g) {
      o(f, g) ? l(m, g) : a(m, g);
    }), m;
  }, Xd;
}
var Zd, T1;
function JD() {
  return T1 || (T1 = 1, Zd = function(n) {
    try {
      var e = /* @__PURE__ */ new Set(), t = {
        size: 0,
        has: function() {
          return !0;
        },
        keys: function() {
          return Object.defineProperty({}, "next", {
            get: function() {
              return e.clear(), e.add(4), function() {
                return { done: !0 };
              };
            }
          });
        }
      }, r = e[n](t);
      return r.size === 1 && r.values().next().value === 4;
    } catch {
      return !1;
    }
  }), Zd;
}
var E1;
function nF() {
  if (E1) return x1;
  E1 = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ iF(), t = /* @__PURE__ */ JD(), r = /* @__PURE__ */ Ni(), s = !r("symmetricDifference") || !t("symmetricDifference");
  return n({ target: "Set", proto: !0, real: !0, forced: s }, {
    symmetricDifference: e
  }), x1;
}
var D1 = {}, Qd, O1;
function sF() {
  if (O1) return Qd;
  O1 = 1;
  var n = /* @__PURE__ */ Pi(), e = Ii().add, t = /* @__PURE__ */ ym(), r = /* @__PURE__ */ Ai(), s = /* @__PURE__ */ Mi();
  return Qd = function(o) {
    var l = n(this), c = r(o).getIterator(), d = t(l);
    return s(c, function(f) {
      e(d, f);
    }), d;
  }, Qd;
}
var k1;
function aF() {
  if (k1) return D1;
  k1 = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ sF(), t = /* @__PURE__ */ JD(), r = /* @__PURE__ */ Ni(), s = !r("union") || !t("union");
  return n({ target: "Set", proto: !0, real: !0, forced: s }, {
    union: e
  }), D1;
}
var Jd, C1;
function oF() {
  if (C1) return Jd;
  C1 = 1, Br(), WL(), VL(), XL(), QL(), eF(), rF(), nF(), aF(), xn();
  var n = /* @__PURE__ */ je();
  return Jd = n.Set, Jd;
}
var eh, R1;
function lF() {
  if (R1) return eh;
  R1 = 1;
  var n = /* @__PURE__ */ oF();
  return Wr(), eh = n, eh;
}
var th, P1;
function uF() {
  return P1 || (P1 = 1, th = /* @__PURE__ */ lF()), th;
}
var cF = uF(), Jr = /* @__PURE__ */ Q(cF), rh, I1;
function dF() {
  if (I1) return rh;
  I1 = 1, Br(), xn();
  var n = /* @__PURE__ */ mm();
  return rh = n, rh;
}
var ih, M1;
function hF() {
  if (M1) return ih;
  M1 = 1;
  var n = /* @__PURE__ */ dF();
  return Wr(), ih = n, ih;
}
var nh, A1;
function fF() {
  if (A1) return nh;
  A1 = 1;
  var n = /* @__PURE__ */ hF();
  return nh = n, nh;
}
var sh, N1;
function pF() {
  if (N1) return sh;
  N1 = 1;
  var n = /* @__PURE__ */ fF();
  return sh = n, sh;
}
var ah, q1;
function mF() {
  return q1 || (q1 = 1, ah = /* @__PURE__ */ pF()), ah;
}
var oh, L1;
function vF() {
  return L1 || (L1 = 1, oh = /* @__PURE__ */ mF()), oh;
}
var gF = vF(), lh = /* @__PURE__ */ Q(gF), F1 = {}, uh, $1;
function yF() {
  if ($1) return uh;
  $1 = 1;
  var n = /* @__PURE__ */ bn(), e = Math.floor, t = function(r, s) {
    var a = r.length;
    if (a < 8)
      for (var o = 1, l, c; o < a; ) {
        for (c = o, l = r[o]; c && s(r[c - 1], l) > 0; )
          r[c] = r[--c];
        c !== o++ && (r[c] = l);
      }
    else
      for (var d = e(a / 2), f = t(n(r, 0, d), s), p = t(n(r, d), s), m = f.length, g = p.length, b = 0, _ = 0; b < m || _ < g; )
        r[b + _] = b < m && _ < g ? s(f[b], p[_]) <= 0 ? f[b++] : p[_++] : b < m ? f[b++] : p[_++];
    return r;
  };
  return uh = t, uh;
}
var ch, j1;
function bF() {
  if (j1) return ch;
  j1 = 1;
  var n = /* @__PURE__ */ hn(), e = n.match(/firefox\/(\d+)/i);
  return ch = !!e && +e[1], ch;
}
var dh, Y1;
function _F() {
  if (Y1) return dh;
  Y1 = 1;
  var n = /* @__PURE__ */ hn();
  return dh = /MSIE|Trident/.test(n), dh;
}
var hh, H1;
function wF() {
  if (H1) return hh;
  H1 = 1;
  var n = /* @__PURE__ */ hn(), e = n.match(/AppleWebKit\/(\d+)\./);
  return hh = !!e && +e[1], hh;
}
var G1;
function xF() {
  if (G1) return F1;
  G1 = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ Pe(), t = /* @__PURE__ */ vr(), r = /* @__PURE__ */ kt(), s = /* @__PURE__ */ Zt(), a = /* @__PURE__ */ zD(), o = /* @__PURE__ */ ar(), l = /* @__PURE__ */ we(), c = /* @__PURE__ */ yF(), d = /* @__PURE__ */ Sn(), f = /* @__PURE__ */ bF(), p = /* @__PURE__ */ _F(), m = /* @__PURE__ */ us(), g = /* @__PURE__ */ wF(), b = [], _ = e(b.sort), x = e(b.push), T = l(function() {
    b.sort(void 0);
  }), S = l(function() {
    b.sort(null);
  }), k = d("sort"), D = !l(function() {
    if (m) return m < 70;
    if (!(f && f > 3)) {
      if (p) return !0;
      if (g) return g < 603;
      var P = "", R, M, q, F;
      for (R = 65; R < 76; R++) {
        switch (M = String.fromCharCode(R), R) {
          case 66:
          case 69:
          case 70:
          case 72:
            q = 3;
            break;
          case 68:
          case 71:
            q = 4;
            break;
          default:
            q = 2;
        }
        for (F = 0; F < 47; F++)
          b.push({ k: M + F, v: q });
      }
      for (b.sort(function(H, Z) {
        return Z.v - H.v;
      }), F = 0; F < b.length; F++)
        M = b[F].k.charAt(0), P.charAt(P.length - 1) !== M && (P += M);
      return P !== "DGBEFHACIJK";
    }
  }), I = T || !S || !k || !D, C = function(P) {
    return function(R, M) {
      if (M === void 0) return -1;
      if (R === void 0) return 1;
      if (P !== void 0) return +P(R, M) || 0;
      var q = o(R), F = o(M);
      return q === F ? 0 : q > F ? 1 : -1;
    };
  };
  return n({ target: "Array", proto: !0, forced: I }, {
    sort: function(R) {
      R !== void 0 && t(R);
      var M = r(this);
      if (D) return R === void 0 ? _(M) : _(M, R);
      var q = [], F = s(M), H, Z;
      for (Z = 0; Z < F; Z++)
        Z in M && x(q, M[Z]);
      for (c(q, C(R)), H = s(q), Z = 0; Z < H; ) M[Z] = q[Z++];
      for (; Z < F; ) a(M, Z++);
      return M;
    }
  }), F1;
}
var fh, B1;
function SF() {
  if (B1) return fh;
  B1 = 1, xF();
  var n = /* @__PURE__ */ Ke();
  return fh = n("Array", "sort"), fh;
}
var ph, W1;
function TF() {
  if (W1) return ph;
  W1 = 1;
  var n = /* @__PURE__ */ He(), e = /* @__PURE__ */ SF(), t = Array.prototype;
  return ph = function(r) {
    var s = r.sort;
    return r === t || n(t, r) && s === t.sort ? e : s;
  }, ph;
}
var mh, z1;
function EF() {
  if (z1) return mh;
  z1 = 1;
  var n = /* @__PURE__ */ TF();
  return mh = n, mh;
}
var vh, U1;
function DF() {
  return U1 || (U1 = 1, vh = /* @__PURE__ */ EF()), vh;
}
var OF = DF(), gt = /* @__PURE__ */ Q(OF), gh, V1;
function kF() {
  return V1 || (V1 = 1, gh = /* @__PURE__ */ kD()), gh;
}
var CF = kF(), ye = /* @__PURE__ */ Q(CF), K1 = {}, X1;
function RF() {
  if (X1) return K1;
  X1 = 1;
  var n = /* @__PURE__ */ X(), e = oi().some, t = /* @__PURE__ */ Sn(), r = t("some");
  return n({ target: "Array", proto: !0, forced: !r }, {
    some: function(a) {
      return e(this, a, arguments.length > 1 ? arguments[1] : void 0);
    }
  }), K1;
}
var yh, Z1;
function PF() {
  if (Z1) return yh;
  Z1 = 1, RF();
  var n = /* @__PURE__ */ Ke();
  return yh = n("Array", "some"), yh;
}
var bh, Q1;
function IF() {
  if (Q1) return bh;
  Q1 = 1;
  var n = /* @__PURE__ */ He(), e = /* @__PURE__ */ PF(), t = Array.prototype;
  return bh = function(r) {
    var s = r.some;
    return r === t || n(t, r) && s === t.some ? e : s;
  }, bh;
}
var _h, J1;
function MF() {
  if (J1) return _h;
  J1 = 1;
  var n = /* @__PURE__ */ IF();
  return _h = n, _h;
}
var wh, eS;
function AF() {
  return eS || (eS = 1, wh = /* @__PURE__ */ MF()), wh;
}
var NF = AF(), bm = /* @__PURE__ */ Q(NF), tS = {}, xh, rS;
function qF() {
  if (rS) return xh;
  rS = 1;
  var n = /* @__PURE__ */ it(), e = /* @__PURE__ */ Pe(), t = /* @__PURE__ */ Ot(), r = /* @__PURE__ */ we(), s = /* @__PURE__ */ fs(), a = /* @__PURE__ */ Oa(), o = /* @__PURE__ */ wa(), l = /* @__PURE__ */ kt(), c = /* @__PURE__ */ xa(), d = Object.assign, f = Object.defineProperty, p = e([].concat);
  return xh = !d || r(function() {
    if (n && d({ b: 1 }, d(f({}, "a", {
      enumerable: !0,
      get: function() {
        f(this, "b", {
          value: 3,
          enumerable: !1
        });
      }
    }), { b: 2 })).b !== 1) return !0;
    var m = {}, g = {}, b = Symbol("assign detection"), _ = "abcdefghijklmnopqrst";
    return m[b] = 7, _.split("").forEach(function(x) {
      g[x] = x;
    }), d({}, m)[b] !== 7 || s(d({}, g)).join("") !== _;
  }) ? function(g, b) {
    for (var _ = l(g), x = arguments.length, T = 1, S = a.f, k = o.f; x > T; )
      for (var D = c(arguments[T++]), I = S ? p(s(D), S(D)) : s(D), C = I.length, P = 0, R; C > P; )
        R = I[P++], (!n || t(k, D, R)) && (_[R] = D[R]);
    return _;
  } : d, xh;
}
var iS;
function LF() {
  if (iS) return tS;
  iS = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ qF();
  return n({ target: "Object", stat: !0, arity: 2, forced: Object.assign !== e }, {
    assign: e
  }), tS;
}
var Sh, nS;
function FF() {
  if (nS) return Sh;
  nS = 1, LF();
  var n = /* @__PURE__ */ je();
  return Sh = n.Object.assign, Sh;
}
var Th, sS;
function $F() {
  if (sS) return Th;
  sS = 1;
  var n = /* @__PURE__ */ FF();
  return Th = n, Th;
}
var Eh, aS;
function jF() {
  return aS || (aS = 1, Eh = /* @__PURE__ */ $F()), Eh;
}
var YF = jF(), Yr = /* @__PURE__ */ Q(YF), Dh, oS;
function HF() {
  if (oS) return Dh;
  oS = 1, CD();
  var n = /* @__PURE__ */ Ke();
  return Dh = n("Array", "concat"), Dh;
}
var Oh, lS;
function GF() {
  if (lS) return Oh;
  lS = 1;
  var n = /* @__PURE__ */ He(), e = /* @__PURE__ */ HF(), t = Array.prototype;
  return Oh = function(r) {
    var s = r.concat;
    return r === t || n(t, r) && s === t.concat ? e : s;
  }, Oh;
}
var kh, uS;
function BF() {
  if (uS) return kh;
  uS = 1;
  var n = /* @__PURE__ */ GF();
  return kh = n, kh;
}
var Ch, cS;
function WF() {
  return cS || (cS = 1, Ch = /* @__PURE__ */ BF()), Ch;
}
var zF = WF(), rt = /* @__PURE__ */ Q(zF), Rh, dS;
function UF() {
  if (dS) return Rh;
  dS = 1, Br();
  var n = /* @__PURE__ */ Ke();
  return Rh = n("Array", "keys"), Rh;
}
var Ph, hS;
function VF() {
  if (hS) return Ph;
  hS = 1;
  var n = /* @__PURE__ */ UF();
  return Ph = n, Ph;
}
var Ih, fS;
function KF() {
  if (fS) return Ih;
  fS = 1, Wr();
  var n = /* @__PURE__ */ si(), e = /* @__PURE__ */ dt(), t = /* @__PURE__ */ He(), r = /* @__PURE__ */ VF(), s = Array.prototype, a = {
    DOMTokenList: !0,
    NodeList: !0
  };
  return Ih = function(o) {
    var l = o.keys;
    return o === s || t(s, o) && l === s.keys || e(a, n(o)) ? r : l;
  }, Ih;
}
var Mh, pS;
function XF() {
  return pS || (pS = 1, Mh = /* @__PURE__ */ KF()), Mh;
}
var ZF = XF(), mi = /* @__PURE__ */ Q(ZF), mS = {}, vS;
function QF() {
  if (vS) return mS;
  vS = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ kt(), t = /* @__PURE__ */ fs(), r = /* @__PURE__ */ we(), s = r(function() {
    t(1);
  });
  return n({ target: "Object", stat: !0, forced: s }, {
    keys: function(o) {
      return t(e(o));
    }
  }), mS;
}
var Ah, gS;
function JF() {
  if (gS) return Ah;
  gS = 1, QF();
  var n = /* @__PURE__ */ je();
  return Ah = n.Object.keys, Ah;
}
var Nh, yS;
function e2() {
  if (yS) return Nh;
  yS = 1;
  var n = /* @__PURE__ */ JF();
  return Nh = n, Nh;
}
var qh, bS;
function t2() {
  return bS || (bS = 1, qh = /* @__PURE__ */ e2()), qh;
}
var r2 = t2(), ct = /* @__PURE__ */ Q(r2), Lh, _S;
function i2() {
  if (_S) return Lh;
  _S = 1, Br();
  var n = /* @__PURE__ */ Ke();
  return Lh = n("Array", "values"), Lh;
}
var Fh, wS;
function n2() {
  if (wS) return Fh;
  wS = 1;
  var n = /* @__PURE__ */ i2();
  return Fh = n, Fh;
}
var $h, xS;
function s2() {
  if (xS) return $h;
  xS = 1, Wr();
  var n = /* @__PURE__ */ si(), e = /* @__PURE__ */ dt(), t = /* @__PURE__ */ He(), r = /* @__PURE__ */ n2(), s = Array.prototype, a = {
    DOMTokenList: !0,
    NodeList: !0
  };
  return $h = function(o) {
    var l = o.values;
    return o === s || t(s, o) && l === s.values || e(a, n(o)) ? r : l;
  }, $h;
}
var jh, SS;
function a2() {
  return SS || (SS = 1, jh = /* @__PURE__ */ s2()), jh;
}
var o2 = a2(), TS = /* @__PURE__ */ Q(o2), ES = {}, Yh, DS;
function eO() {
  if (DS) return Yh;
  DS = 1;
  var n = /* @__PURE__ */ yn(), e = /* @__PURE__ */ ar(), t = /* @__PURE__ */ Gr(), r = RangeError, s = Math.floor;
  return Yh = function(o) {
    var l = e(t(this)), c = "", d = n(o);
    if (d < 0 || d === 1 / 0) throw new r("Wrong number of repetitions");
    for (; d > 0; (d = s(d / 2)) && (l += l)) d % 2 && (c += l);
    return c;
  }, Yh;
}
var Hh, OS;
function l2() {
  if (OS) return Hh;
  OS = 1;
  var n = /* @__PURE__ */ Pe(), e = /* @__PURE__ */ bD(), t = /* @__PURE__ */ ar(), r = /* @__PURE__ */ eO(), s = /* @__PURE__ */ Gr(), a = n(r), o = n("".slice), l = Math.ceil, c = function(d) {
    return function(f, p, m) {
      var g = t(s(f)), b = e(p), _ = g.length;
      if (b <= _) return g;
      var x = m === void 0 ? " " : t(m), T, S;
      return x === "" ? g : (T = b - _, S = a(x, l(T / x.length)), S.length > T && (S = o(S, 0, T)), d ? g + S : S + g);
    };
  };
  return Hh = {
    // `String.prototype.padStart` method
    // https://tc39.es/ecma262/#sec-string.prototype.padstart
    start: c(!1),
    // `String.prototype.padEnd` method
    // https://tc39.es/ecma262/#sec-string.prototype.padend
    end: c(!0)
  }, Hh;
}
var Gh, kS;
function u2() {
  if (kS) return Gh;
  kS = 1;
  var n = /* @__PURE__ */ Pe(), e = /* @__PURE__ */ we(), t = l2().start, r = RangeError, s = isFinite, a = Math.abs, o = Date.prototype, l = o.toISOString, c = n(o.getTime), d = n(o.getUTCDate), f = n(o.getUTCFullYear), p = n(o.getUTCHours), m = n(o.getUTCMilliseconds), g = n(o.getUTCMinutes), b = n(o.getUTCMonth), _ = n(o.getUTCSeconds);
  return Gh = e(function() {
    return l.call(new Date(-5e13 - 1)) !== "0385-07-25T07:06:39.999Z";
  }) || !e(function() {
    l.call(/* @__PURE__ */ new Date(NaN));
  }) ? function() {
    if (!s(c(this))) throw new r("Invalid time value");
    var T = this, S = f(T), k = m(T), D = S < 0 ? "-" : S > 9999 ? "+" : "";
    return D + t(a(S), D ? 6 : 4, 0) + "-" + t(b(T) + 1, 2, 0) + "-" + t(d(T), 2, 0) + "T" + t(p(T), 2, 0) + ":" + t(g(T), 2, 0) + ":" + t(_(T), 2, 0) + "." + t(k, 3, 0) + "Z";
  } : l, Gh;
}
var CS;
function c2() {
  if (CS) return ES;
  CS = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ Ot(), t = /* @__PURE__ */ kt(), r = /* @__PURE__ */ mD(), s = /* @__PURE__ */ u2(), a = /* @__PURE__ */ xr(), o = /* @__PURE__ */ we(), l = o(function() {
    return (/* @__PURE__ */ new Date(NaN)).toJSON() !== null || e(Date.prototype.toJSON, { toISOString: function() {
      return 1;
    } }) !== 1;
  });
  return n({ target: "Date", proto: !0, forced: l }, {
    // eslint-disable-next-line no-unused-vars -- required for `.length`
    toJSON: function(d) {
      var f = t(this), p = r(f, "number");
      return typeof p == "number" && !isFinite(p) ? null : !("toISOString" in f) && a(f) === "Date" ? e(s, f) : f.toISOString();
    }
  }), ES;
}
var Bh, RS;
function d2() {
  if (RS) return Bh;
  RS = 1, c2(), TD();
  var n = /* @__PURE__ */ je(), e = /* @__PURE__ */ _a();
  return n.JSON || (n.JSON = { stringify: JSON.stringify }), Bh = function(r, s, a) {
    return e(n.JSON.stringify, null, arguments);
  }, Bh;
}
var Wh, PS;
function h2() {
  if (PS) return Wh;
  PS = 1;
  var n = /* @__PURE__ */ d2();
  return Wh = n, Wh;
}
var zh, IS;
function f2() {
  return IS || (IS = 1, zh = /* @__PURE__ */ h2()), zh;
}
var p2 = f2(), Yt = /* @__PURE__ */ Q(p2), Uh, MS;
function m2() {
  if (MS) return Uh;
  MS = 1, Br();
  var n = /* @__PURE__ */ Ke();
  return Uh = n("Array", "entries"), Uh;
}
var Vh, AS;
function v2() {
  if (AS) return Vh;
  AS = 1;
  var n = /* @__PURE__ */ m2();
  return Vh = n, Vh;
}
var Kh, NS;
function g2() {
  if (NS) return Kh;
  NS = 1, Wr();
  var n = /* @__PURE__ */ si(), e = /* @__PURE__ */ dt(), t = /* @__PURE__ */ He(), r = /* @__PURE__ */ v2(), s = Array.prototype, a = {
    DOMTokenList: !0,
    NodeList: !0
  };
  return Kh = function(o) {
    var l = o.entries;
    return o === s || t(s, o) && l === s.entries || e(a, n(o)) ? r : l;
  }, Kh;
}
var Xh, qS;
function y2() {
  return qS || (qS = 1, Xh = /* @__PURE__ */ g2()), Xh;
}
var b2 = y2(), _2 = /* @__PURE__ */ Q(b2), LS = {}, FS;
function w2() {
  if (FS) return LS;
  FS = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ DD();
  return n({ target: "Reflect", stat: !0 }, {
    ownKeys: e
  }), LS;
}
var Zh, $S;
function x2() {
  if ($S) return Zh;
  $S = 1, w2();
  var n = /* @__PURE__ */ je();
  return Zh = n.Reflect.ownKeys, Zh;
}
var Qh, jS;
function S2() {
  if (jS) return Qh;
  jS = 1;
  var n = /* @__PURE__ */ x2();
  return Qh = n, Qh;
}
var Jh, YS;
function T2() {
  return YS || (YS = 1, Jh = /* @__PURE__ */ S2()), Jh;
}
var E2 = T2(), tO = /* @__PURE__ */ Q(E2), ef, HS;
function D2() {
  return HS || (HS = 1, ef = /* @__PURE__ */ YD()), ef;
}
var O2 = D2(), Na = /* @__PURE__ */ Q(O2), tf, GS;
function k2() {
  return GS || (GS = 1, tf = /* @__PURE__ */ qD()), tf;
}
var C2 = k2(), R2 = /* @__PURE__ */ Q(C2), rf, BS;
function P2() {
  return BS || (BS = 1, rf = /* @__PURE__ */ $D()), rf;
}
var I2 = P2(), Tn = /* @__PURE__ */ Q(I2), WS = {}, zS;
function M2() {
  if (zS) return WS;
  zS = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ Pe(), t = Date, r = e(t.prototype.getTime);
  return n({ target: "Date", stat: !0 }, {
    now: function() {
      return r(new t());
    }
  }), WS;
}
var nf, US;
function A2() {
  if (US) return nf;
  US = 1, M2();
  var n = /* @__PURE__ */ je();
  return nf = n.Date.now, nf;
}
var sf, VS;
function N2() {
  if (VS) return sf;
  VS = 1;
  var n = /* @__PURE__ */ A2();
  return sf = n, sf;
}
var af, KS;
function q2() {
  return KS || (KS = 1, af = /* @__PURE__ */ N2()), af;
}
var L2 = q2(), Ut = /* @__PURE__ */ Q(L2), XS = {}, ZS;
function F2() {
  if (ZS) return XS;
  ZS = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ Pe(), t = /* @__PURE__ */ ai(), r = e([].reverse), s = [1, 2];
  return n({ target: "Array", proto: !0, forced: String(s) === String(s.reverse()) }, {
    reverse: function() {
      return t(this) && (this.length = this.length), r(this);
    }
  }), XS;
}
var of, QS;
function $2() {
  if (QS) return of;
  QS = 1, F2();
  var n = /* @__PURE__ */ Ke();
  return of = n("Array", "reverse"), of;
}
var lf, JS;
function j2() {
  if (JS) return lf;
  JS = 1;
  var n = /* @__PURE__ */ He(), e = /* @__PURE__ */ $2(), t = Array.prototype;
  return lf = function(r) {
    var s = r.reverse;
    return r === t || n(t, r) && s === t.reverse ? e : s;
  }, lf;
}
var uf, eT;
function Y2() {
  if (eT) return uf;
  eT = 1;
  var n = /* @__PURE__ */ j2();
  return uf = n, uf;
}
var cf, tT;
function H2() {
  return tT || (tT = 1, cf = /* @__PURE__ */ Y2()), cf;
}
var G2 = H2(), B2 = /* @__PURE__ */ Q(G2), rT = {}, iT;
function W2() {
  if (iT) return rT;
  iT = 1;
  var n = /* @__PURE__ */ X(), e = lm().includes, t = /* @__PURE__ */ we(), r = /* @__PURE__ */ ys(), s = t(function() {
    return !Array(1).includes();
  }), a = t(function() {
    return [, 1].includes(void 0, 1);
  });
  return n({ target: "Array", proto: !0, forced: s || a }, {
    includes: function(l) {
      return e(this, l, arguments.length > 1 ? arguments[1] : void 0);
    }
  }), r("includes"), rT;
}
var df, nT;
function z2() {
  if (nT) return df;
  nT = 1, W2();
  var n = /* @__PURE__ */ Ke();
  return df = n("Array", "includes"), df;
}
var sT = {}, hf, aT;
function U2() {
  if (aT) return hf;
  aT = 1;
  var n = /* @__PURE__ */ bt(), e = /* @__PURE__ */ xr(), t = /* @__PURE__ */ nt(), r = t("match");
  return hf = function(s) {
    var a;
    return n(s) && ((a = s[r]) !== void 0 ? !!a : e(s) === "RegExp");
  }, hf;
}
var ff, oT;
function V2() {
  if (oT) return ff;
  oT = 1;
  var n = /* @__PURE__ */ U2(), e = TypeError;
  return ff = function(t) {
    if (n(t))
      throw new e("The method doesn't accept regular expressions");
    return t;
  }, ff;
}
var pf, lT;
function K2() {
  if (lT) return pf;
  lT = 1;
  var n = /* @__PURE__ */ nt(), e = n("match");
  return pf = function(t) {
    var r = /./;
    try {
      "/./"[t](r);
    } catch {
      try {
        return r[e] = !1, "/./"[t](r);
      } catch {
      }
    }
    return !1;
  }, pf;
}
var uT;
function X2() {
  if (uT) return sT;
  uT = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ Pe(), t = /* @__PURE__ */ V2(), r = /* @__PURE__ */ Gr(), s = /* @__PURE__ */ ar(), a = /* @__PURE__ */ K2(), o = e("".indexOf);
  return n({ target: "String", proto: !0, forced: !a("includes") }, {
    includes: function(c) {
      return !!~o(
        s(r(this)),
        s(t(c)),
        arguments.length > 1 ? arguments[1] : void 0
      );
    }
  }), sT;
}
var mf, cT;
function Z2() {
  if (cT) return mf;
  cT = 1, X2();
  var n = /* @__PURE__ */ Ke();
  return mf = n("String", "includes"), mf;
}
var vf, dT;
function Q2() {
  if (dT) return vf;
  dT = 1;
  var n = /* @__PURE__ */ He(), e = /* @__PURE__ */ z2(), t = /* @__PURE__ */ Z2(), r = Array.prototype, s = String.prototype;
  return vf = function(a) {
    var o = a.includes;
    return a === r || n(r, a) && o === r.includes ? e : typeof a == "string" || a === s || n(s, a) && o === s.includes ? t : o;
  }, vf;
}
var gf, hT;
function J2() {
  if (hT) return gf;
  hT = 1;
  var n = /* @__PURE__ */ Q2();
  return gf = n, gf;
}
var yf, fT;
function e$() {
  return fT || (fT = 1, yf = /* @__PURE__ */ J2()), yf;
}
var t$ = e$(), an = /* @__PURE__ */ Q(t$), pT = {}, mT;
function r$() {
  if (mT) return pT;
  mT = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ we(), t = /* @__PURE__ */ kt(), r = /* @__PURE__ */ Ia(), s = /* @__PURE__ */ AD(), a = e(function() {
    r(1);
  });
  return n({ target: "Object", stat: !0, forced: a, sham: !s }, {
    getPrototypeOf: function(l) {
      return r(t(l));
    }
  }), pT;
}
var bf, vT;
function i$() {
  if (vT) return bf;
  vT = 1, r$();
  var n = /* @__PURE__ */ je();
  return bf = n.Object.getPrototypeOf, bf;
}
var _f, gT;
function n$() {
  if (gT) return _f;
  gT = 1;
  var n = /* @__PURE__ */ i$();
  return _f = n, _f;
}
var wf, yT;
function s$() {
  return yT || (yT = 1, wf = /* @__PURE__ */ n$()), wf;
}
var a$ = s$(), bT = /* @__PURE__ */ Q(a$), _T = {}, xf, wT;
function rO() {
  if (wT) return xf;
  wT = 1;
  var n = /* @__PURE__ */ it(), e = /* @__PURE__ */ we(), t = /* @__PURE__ */ Pe(), r = /* @__PURE__ */ Ia(), s = /* @__PURE__ */ fs(), a = /* @__PURE__ */ mr(), o = wa().f, l = t(o), c = t([].push), d = n && e(function() {
    var p = /* @__PURE__ */ Object.create(null);
    return p[2] = 2, !l(p, 2);
  }), f = function(p) {
    return function(m) {
      for (var g = a(m), b = s(g), _ = d && r(g) === null, x = b.length, T = 0, S = [], k; x > T; )
        k = b[T++], (!n || (_ ? k in g : l(g, k))) && c(S, p ? [k, g[k]] : g[k]);
      return S;
    };
  };
  return xf = {
    // `Object.entries` method
    // https://tc39.es/ecma262/#sec-object.entries
    entries: f(!0),
    // `Object.values` method
    // https://tc39.es/ecma262/#sec-object.values
    values: f(!1)
  }, xf;
}
var xT;
function o$() {
  if (xT) return _T;
  xT = 1;
  var n = /* @__PURE__ */ X(), e = rO().values;
  return n({ target: "Object", stat: !0 }, {
    values: function(r) {
      return e(r);
    }
  }), _T;
}
var Sf, ST;
function l$() {
  if (ST) return Sf;
  ST = 1, o$();
  var n = /* @__PURE__ */ je();
  return Sf = n.Object.values, Sf;
}
var Tf, TT;
function u$() {
  if (TT) return Tf;
  TT = 1;
  var n = /* @__PURE__ */ l$();
  return Tf = n, Tf;
}
var Ef, ET;
function c$() {
  return ET || (ET = 1, Ef = /* @__PURE__ */ u$()), Ef;
}
var d$ = c$(), _m = /* @__PURE__ */ Q(d$), DT = {}, Df, OT;
function wm() {
  return OT || (OT = 1, Df = `	
\v\f\r                　\u2028\u2029\uFEFF`), Df;
}
var Of, kT;
function iO() {
  if (kT) return Of;
  kT = 1;
  var n = /* @__PURE__ */ Pe(), e = /* @__PURE__ */ Gr(), t = /* @__PURE__ */ ar(), r = /* @__PURE__ */ wm(), s = n("".replace), a = RegExp("^[" + r + "]+"), o = RegExp("(^|[^" + r + "])[" + r + "]+$"), l = function(c) {
    return function(d) {
      var f = t(e(d));
      return c & 1 && (f = s(f, a, "")), c & 2 && (f = s(f, o, "$1")), f;
    };
  };
  return Of = {
    // `String.prototype.{ trimLeft, trimStart }` methods
    // https://tc39.es/ecma262/#sec-string.prototype.trimstart
    start: l(1),
    // `String.prototype.{ trimRight, trimEnd }` methods
    // https://tc39.es/ecma262/#sec-string.prototype.trimend
    end: l(2),
    // `String.prototype.trim` method
    // https://tc39.es/ecma262/#sec-string.prototype.trim
    trim: l(3)
  }, Of;
}
var kf, CT;
function h$() {
  if (CT) return kf;
  CT = 1;
  var n = /* @__PURE__ */ Ze(), e = /* @__PURE__ */ we(), t = /* @__PURE__ */ Pe(), r = /* @__PURE__ */ ar(), s = iO().trim, a = /* @__PURE__ */ wm(), o = n.parseInt, l = n.Symbol, c = l && l.iterator, d = /^[+-]?0x/i, f = t(d.exec), p = o(a + "08") !== 8 || o(a + "0x16") !== 22 || c && !e(function() {
    o(Object(c));
  });
  return kf = p ? function(g, b) {
    var _ = s(r(g));
    return o(_, b >>> 0 || (f(d, _) ? 16 : 10));
  } : o, kf;
}
var RT;
function f$() {
  if (RT) return DT;
  RT = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ h$();
  return n({ global: !0, forced: parseInt !== e }, {
    parseInt: e
  }), DT;
}
var Cf, PT;
function p$() {
  if (PT) return Cf;
  PT = 1, f$();
  var n = /* @__PURE__ */ je();
  return Cf = n.parseInt, Cf;
}
var Rf, IT;
function m$() {
  if (IT) return Rf;
  IT = 1;
  var n = /* @__PURE__ */ p$();
  return Rf = n, Rf;
}
var Pf, MT;
function v$() {
  return MT || (MT = 1, Pf = /* @__PURE__ */ m$()), Pf;
}
var g$ = v$(), Vt = /* @__PURE__ */ Q(g$), AT = {}, NT;
function y$() {
  if (NT) return AT;
  NT = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ im(), t = lm().indexOf, r = /* @__PURE__ */ Sn(), s = e([].indexOf), a = !!s && 1 / s([1], 1, -0) < 0, o = a || !r("indexOf");
  return n({ target: "Array", proto: !0, forced: o }, {
    indexOf: function(c) {
      var d = arguments.length > 1 ? arguments[1] : void 0;
      return a ? s(this, c, d) || 0 : t(this, c, d);
    }
  }), AT;
}
var If, qT;
function b$() {
  if (qT) return If;
  qT = 1, y$();
  var n = /* @__PURE__ */ Ke();
  return If = n("Array", "indexOf"), If;
}
var Mf, LT;
function _$() {
  if (LT) return Mf;
  LT = 1;
  var n = /* @__PURE__ */ He(), e = /* @__PURE__ */ b$(), t = Array.prototype;
  return Mf = function(r) {
    var s = r.indexOf;
    return r === t || n(t, r) && s === t.indexOf ? e : s;
  }, Mf;
}
var Af, FT;
function w$() {
  if (FT) return Af;
  FT = 1;
  var n = /* @__PURE__ */ _$();
  return Af = n, Af;
}
var Nf, $T;
function x$() {
  return $T || ($T = 1, Nf = /* @__PURE__ */ w$()), Nf;
}
var S$ = x$(), Fe = /* @__PURE__ */ Q(S$), jT = {}, YT;
function T$() {
  if (YT) return jT;
  YT = 1;
  var n = /* @__PURE__ */ X(), e = rO().entries;
  return n({ target: "Object", stat: !0 }, {
    entries: function(r) {
      return e(r);
    }
  }), jT;
}
var qf, HT;
function E$() {
  if (HT) return qf;
  HT = 1, T$();
  var n = /* @__PURE__ */ je();
  return qf = n.Object.entries, qf;
}
var Lf, GT;
function D$() {
  if (GT) return Lf;
  GT = 1;
  var n = /* @__PURE__ */ E$();
  return Lf = n, Lf;
}
var Ff, BT;
function O$() {
  return BT || (BT = 1, Ff = /* @__PURE__ */ D$()), Ff;
}
var k$ = O$(), C$ = /* @__PURE__ */ Q(k$), WT = {}, $f, zT;
function R$() {
  if (zT) return $f;
  zT = 1;
  var n = /* @__PURE__ */ kt(), e = /* @__PURE__ */ Ta(), t = /* @__PURE__ */ Zt();
  return $f = function(s) {
    for (var a = n(this), o = t(a), l = arguments.length, c = e(l > 1 ? arguments[1] : void 0, o), d = l > 2 ? arguments[2] : void 0, f = d === void 0 ? o : e(d, o); f > c; ) a[c++] = s;
    return a;
  }, $f;
}
var UT;
function P$() {
  if (UT) return WT;
  UT = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ R$(), t = /* @__PURE__ */ ys();
  return n({ target: "Array", proto: !0 }, {
    fill: e
  }), t("fill"), WT;
}
var jf, VT;
function I$() {
  if (VT) return jf;
  VT = 1, P$();
  var n = /* @__PURE__ */ Ke();
  return jf = n("Array", "fill"), jf;
}
var Yf, KT;
function M$() {
  if (KT) return Yf;
  KT = 1;
  var n = /* @__PURE__ */ He(), e = /* @__PURE__ */ I$(), t = Array.prototype;
  return Yf = function(r) {
    var s = r.fill;
    return r === t || n(t, r) && s === t.fill ? e : s;
  }, Yf;
}
var Hf, XT;
function A$() {
  if (XT) return Hf;
  XT = 1;
  var n = /* @__PURE__ */ M$();
  return Hf = n, Hf;
}
var Gf, ZT;
function N$() {
  return ZT || (ZT = 1, Gf = /* @__PURE__ */ A$()), Gf;
}
var q$ = N$(), nO = /* @__PURE__ */ Q(q$), Bf = { exports: {} }, QT;
function L$() {
  return QT || (QT = 1, (function(n) {
    n.exports = e;
    function e(r) {
      if (r) return t(r);
    }
    function t(r) {
      for (var s in e.prototype)
        r[s] = e.prototype[s];
      return r;
    }
    e.prototype.on = e.prototype.addEventListener = function(r, s) {
      return this._callbacks = this._callbacks || {}, (this._callbacks["$" + r] = this._callbacks["$" + r] || []).push(s), this;
    }, e.prototype.once = function(r, s) {
      function a() {
        this.off(r, a), s.apply(this, arguments);
      }
      return a.fn = s, this.on(r, a), this;
    }, e.prototype.off = e.prototype.removeListener = e.prototype.removeAllListeners = e.prototype.removeEventListener = function(r, s) {
      if (this._callbacks = this._callbacks || {}, arguments.length == 0)
        return this._callbacks = {}, this;
      var a = this._callbacks["$" + r];
      if (!a) return this;
      if (arguments.length == 1)
        return delete this._callbacks["$" + r], this;
      for (var o, l = 0; l < a.length; l++)
        if (o = a[l], o === s || o.fn === s) {
          a.splice(l, 1);
          break;
        }
      return a.length === 0 && delete this._callbacks["$" + r], this;
    }, e.prototype.emit = function(r) {
      this._callbacks = this._callbacks || {};
      for (var s = new Array(arguments.length - 1), a = this._callbacks["$" + r], o = 1; o < arguments.length; o++)
        s[o - 1] = arguments[o];
      if (a) {
        a = a.slice(0);
        for (var o = 0, l = a.length; o < l; ++o)
          a[o].apply(this, s);
      }
      return this;
    }, e.prototype.listeners = function(r) {
      return this._callbacks = this._callbacks || {}, this._callbacks["$" + r] || [];
    }, e.prototype.hasListeners = function(r) {
      return !!this.listeners(r).length;
    };
  })(Bf)), Bf.exports;
}
var F$ = L$(), xm = /* @__PURE__ */ Q(F$);
/*! Hammer.JS - v2.0.17-rc - 2019-12-16
 * http://naver.github.io/egjs
 *
 * Forked By Naver egjs
 * Copyright (c) hammerjs
 * Licensed under the MIT license */
function hr() {
  return hr = Object.assign || function(n) {
    for (var e = 1; e < arguments.length; e++) {
      var t = arguments[e];
      for (var r in t)
        Object.prototype.hasOwnProperty.call(t, r) && (n[r] = t[r]);
    }
    return n;
  }, hr.apply(this, arguments);
}
function or(n, e) {
  n.prototype = Object.create(e.prototype), n.prototype.constructor = n, n.__proto__ = e;
}
function Us(n) {
  if (n === void 0)
    throw new ReferenceError("this hasn't been initialised - super() hasn't been called");
  return n;
}
var Pp;
typeof Object.assign != "function" ? Pp = function(e) {
  if (e == null)
    throw new TypeError("Cannot convert undefined or null to object");
  for (var t = Object(e), r = 1; r < arguments.length; r++) {
    var s = arguments[r];
    if (s != null)
      for (var a in s)
        s.hasOwnProperty(a) && (t[a] = s[a]);
  }
  return t;
} : Pp = Object.assign;
var Si = Pp, JT = ["", "webkit", "Moz", "MS", "ms", "o"], $$ = typeof document > "u" ? {
  style: {}
} : document.createElement("div"), j$ = "function", en = Math.round, Ti = Math.abs, Sm = Date.now;
function qa(n, e) {
  for (var t, r, s = e[0].toUpperCase() + e.slice(1), a = 0; a < JT.length; ) {
    if (t = JT[a], r = t ? t + s : e, r in n)
      return r;
    a++;
  }
}
var Fr;
typeof window > "u" ? Fr = {} : Fr = window;
var sO = qa($$.style, "touchAction"), aO = sO !== void 0;
function Y$() {
  if (!aO)
    return !1;
  var n = {}, e = Fr.CSS && Fr.CSS.supports;
  return ["auto", "manipulation", "pan-y", "pan-x", "pan-x pan-y", "none"].forEach(function(t) {
    return n[t] = e ? Fr.CSS.supports("touch-action", t) : !0;
  }), n;
}
var oO = "compute", lO = "auto", Ip = "manipulation", bi = "none", Vn = "pan-x", Kn = "pan-y", Vs = Y$(), H$ = /mobile|tablet|ip(ad|hone|od)|android/i, uO = "ontouchstart" in Fr, G$ = qa(Fr, "PointerEvent") !== void 0, B$ = uO && H$.test(navigator.userAgent), Ss = "touch", W$ = "pen", Tm = "mouse", z$ = "kinect", U$ = 25, Nt = 1, qi = 2, ot = 4, Ht = 8, ca = 1, Ts = 2, Es = 4, Ds = 8, on = 16, _r = Ts | Es, _i = Ds | on, cO = _r | _i, dO = ["x", "y"], da = ["clientX", "clientY"];
function $r(n, e, t) {
  var r;
  if (n)
    if (n.forEach)
      n.forEach(e, t);
    else if (n.length !== void 0)
      for (r = 0; r < n.length; )
        e.call(t, n[r], r, n), r++;
    else
      for (r in n)
        n.hasOwnProperty(r) && e.call(t, n[r], r, n);
}
function La(n, e) {
  return typeof n === j$ ? n.apply(e && e[0] || void 0, e) : n;
}
function gi(n, e) {
  return n.indexOf(e) > -1;
}
function V$(n) {
  if (gi(n, bi))
    return bi;
  var e = gi(n, Vn), t = gi(n, Kn);
  return e && t ? bi : e || t ? e ? Vn : Kn : gi(n, Ip) ? Ip : lO;
}
var hO = /* @__PURE__ */ (function() {
  function n(t, r) {
    this.manager = t, this.set(r);
  }
  var e = n.prototype;
  return e.set = function(r) {
    r === oO && (r = this.compute()), aO && this.manager.element.style && Vs[r] && (this.manager.element.style[sO] = r), this.actions = r.toLowerCase().trim();
  }, e.update = function() {
    this.set(this.manager.options.touchAction);
  }, e.compute = function() {
    var r = [];
    return $r(this.manager.recognizers, function(s) {
      La(s.options.enable, [s]) && (r = r.concat(s.getTouchAction()));
    }), V$(r.join(" "));
  }, e.preventDefaults = function(r) {
    var s = r.srcEvent, a = r.offsetDirection;
    if (this.manager.session.prevented) {
      s.preventDefault();
      return;
    }
    var o = this.actions, l = gi(o, bi) && !Vs[bi], c = gi(o, Kn) && !Vs[Kn], d = gi(o, Vn) && !Vs[Vn];
    if (l) {
      var f = r.pointers.length === 1, p = r.distance < 2, m = r.deltaTime < 250;
      if (f && p && m)
        return;
    }
    if (!(d && c) && (l || c && a & _r || d && a & _i))
      return this.preventSrc(s);
  }, e.preventSrc = function(r) {
    this.manager.session.prevented = !0, r.preventDefault();
  }, n;
})();
function Em(n, e) {
  for (; n; ) {
    if (n === e)
      return !0;
    n = n.parentNode;
  }
  return !1;
}
function fO(n) {
  var e = n.length;
  if (e === 1)
    return {
      x: en(n[0].clientX),
      y: en(n[0].clientY)
    };
  for (var t = 0, r = 0, s = 0; s < e; )
    t += n[s].clientX, r += n[s].clientY, s++;
  return {
    x: en(t / e),
    y: en(r / e)
  };
}
function eE(n) {
  for (var e = [], t = 0; t < n.pointers.length; )
    e[t] = {
      clientX: en(n.pointers[t].clientX),
      clientY: en(n.pointers[t].clientY)
    }, t++;
  return {
    timeStamp: Sm(),
    pointers: e,
    center: fO(e),
    deltaX: n.deltaX,
    deltaY: n.deltaY
  };
}
function ha(n, e, t) {
  t || (t = dO);
  var r = e[t[0]] - n[t[0]], s = e[t[1]] - n[t[1]];
  return Math.sqrt(r * r + s * s);
}
function Mp(n, e, t) {
  t || (t = dO);
  var r = e[t[0]] - n[t[0]], s = e[t[1]] - n[t[1]];
  return Math.atan2(s, r) * 180 / Math.PI;
}
function pO(n, e) {
  return n === e ? ca : Ti(n) >= Ti(e) ? n < 0 ? Ts : Es : e < 0 ? Ds : on;
}
function K$(n, e) {
  var t = e.center, r = n.offsetDelta || {}, s = n.prevDelta || {}, a = n.prevInput || {};
  (e.eventType === Nt || a.eventType === ot) && (s = n.prevDelta = {
    x: a.deltaX || 0,
    y: a.deltaY || 0
  }, r = n.offsetDelta = {
    x: t.x,
    y: t.y
  }), e.deltaX = s.x + (t.x - r.x), e.deltaY = s.y + (t.y - r.y);
}
function mO(n, e, t) {
  return {
    x: e / n || 0,
    y: t / n || 0
  };
}
function X$(n, e) {
  return ha(e[0], e[1], da) / ha(n[0], n[1], da);
}
function Z$(n, e) {
  return Mp(e[1], e[0], da) + Mp(n[1], n[0], da);
}
function Q$(n, e) {
  var t = n.lastInterval || e, r = e.timeStamp - t.timeStamp, s, a, o, l;
  if (e.eventType !== Ht && (r > U$ || t.velocity === void 0)) {
    var c = e.deltaX - t.deltaX, d = e.deltaY - t.deltaY, f = mO(r, c, d);
    a = f.x, o = f.y, s = Ti(f.x) > Ti(f.y) ? f.x : f.y, l = pO(c, d), n.lastInterval = e;
  } else
    s = t.velocity, a = t.velocityX, o = t.velocityY, l = t.direction;
  e.velocity = s, e.velocityX = a, e.velocityY = o, e.direction = l;
}
function J$(n, e) {
  var t = n.session, r = e.pointers, s = r.length;
  t.firstInput || (t.firstInput = eE(e)), s > 1 && !t.firstMultiple ? t.firstMultiple = eE(e) : s === 1 && (t.firstMultiple = !1);
  var a = t.firstInput, o = t.firstMultiple, l = o ? o.center : a.center, c = e.center = fO(r);
  e.timeStamp = Sm(), e.deltaTime = e.timeStamp - a.timeStamp, e.angle = Mp(l, c), e.distance = ha(l, c), K$(t, e), e.offsetDirection = pO(e.deltaX, e.deltaY);
  var d = mO(e.deltaTime, e.deltaX, e.deltaY);
  e.overallVelocityX = d.x, e.overallVelocityY = d.y, e.overallVelocity = Ti(d.x) > Ti(d.y) ? d.x : d.y, e.scale = o ? X$(o.pointers, r) : 1, e.rotation = o ? Z$(o.pointers, r) : 0, e.maxPointers = t.prevInput ? e.pointers.length > t.prevInput.maxPointers ? e.pointers.length : t.prevInput.maxPointers : e.pointers.length, Q$(t, e);
  var f = n.element, p = e.srcEvent, m;
  p.composedPath ? m = p.composedPath()[0] : p.path ? m = p.path[0] : m = p.target, Em(m, f) && (f = m), e.target = f;
}
function ej(n, e, t) {
  var r = t.pointers.length, s = t.changedPointers.length, a = e & Nt && r - s === 0, o = e & (ot | Ht) && r - s === 0;
  t.isFirst = !!a, t.isFinal = !!o, a && (n.session = {}), t.eventType = e, J$(n, t), n.emit("hammer.input", t), n.recognize(t), n.session.prevInput = t;
}
function Xn(n) {
  return n.trim().split(/\s+/g);
}
function Gn(n, e, t) {
  $r(Xn(e), function(r) {
    n.addEventListener(r, t, !1);
  });
}
function Bn(n, e, t) {
  $r(Xn(e), function(r) {
    n.removeEventListener(r, t, !1);
  });
}
function tE(n) {
  var e = n.ownerDocument || n;
  return e.defaultView || e.parentWindow || window;
}
var En = /* @__PURE__ */ (function() {
  function n(t, r) {
    var s = this;
    this.manager = t, this.callback = r, this.element = t.element, this.target = t.options.inputTarget, this.domHandler = function(a) {
      La(t.options.enable, [t]) && s.handler(a);
    }, this.init();
  }
  var e = n.prototype;
  return e.handler = function() {
  }, e.init = function() {
    this.evEl && Gn(this.element, this.evEl, this.domHandler), this.evTarget && Gn(this.target, this.evTarget, this.domHandler), this.evWin && Gn(tE(this.element), this.evWin, this.domHandler);
  }, e.destroy = function() {
    this.evEl && Bn(this.element, this.evEl, this.domHandler), this.evTarget && Bn(this.target, this.evTarget, this.domHandler), this.evWin && Bn(tE(this.element), this.evWin, this.domHandler);
  }, n;
})();
function Ei(n, e, t) {
  if (n.indexOf && !t)
    return n.indexOf(e);
  for (var r = 0; r < n.length; ) {
    if (t && n[r][t] == e || !t && n[r] === e)
      return r;
    r++;
  }
  return -1;
}
var tj = {
  pointerdown: Nt,
  pointermove: qi,
  pointerup: ot,
  pointercancel: Ht,
  pointerout: Ht
}, rj = {
  2: Ss,
  3: W$,
  4: Tm,
  5: z$
  // see https://twitter.com/jacobrossi/status/480596438489890816
}, vO = "pointerdown", gO = "pointermove pointerup pointercancel";
Fr.MSPointerEvent && !Fr.PointerEvent && (vO = "MSPointerDown", gO = "MSPointerMove MSPointerUp MSPointerCancel");
var yO = /* @__PURE__ */ (function(n) {
  or(e, n);
  function e() {
    var r, s = e.prototype;
    return s.evEl = vO, s.evWin = gO, r = n.apply(this, arguments) || this, r.store = r.manager.session.pointerEvents = [], r;
  }
  var t = e.prototype;
  return t.handler = function(s) {
    var a = this.store, o = !1, l = s.type.toLowerCase().replace("ms", ""), c = tj[l], d = rj[s.pointerType] || s.pointerType, f = d === Ss, p = Ei(a, s.pointerId, "pointerId");
    c & Nt && (s.button === 0 || f) ? p < 0 && (a.push(s), p = a.length - 1) : c & (ot | Ht) && (o = !0), !(p < 0) && (a[p] = s, this.callback(this.manager, c, {
      pointers: a,
      changedPointers: [s],
      pointerType: d,
      srcEvent: s
    }), o && a.splice(p, 1));
  }, e;
})(En);
function Zn(n) {
  return Array.prototype.slice.call(n, 0);
}
function Dm(n, e, t) {
  for (var r = [], s = [], a = 0; a < n.length; ) {
    var o = e ? n[a][e] : n[a];
    Ei(s, o) < 0 && r.push(n[a]), s[a] = o, a++;
  }
  return t && (e ? r = r.sort(function(l, c) {
    return l[e] > c[e];
  }) : r = r.sort()), r;
}
var ij = {
  touchstart: Nt,
  touchmove: qi,
  touchend: ot,
  touchcancel: Ht
}, nj = "touchstart touchmove touchend touchcancel", Om = /* @__PURE__ */ (function(n) {
  or(e, n);
  function e() {
    var r;
    return e.prototype.evTarget = nj, r = n.apply(this, arguments) || this, r.targetIds = {}, r;
  }
  var t = e.prototype;
  return t.handler = function(s) {
    var a = ij[s.type], o = sj.call(this, s, a);
    o && this.callback(this.manager, a, {
      pointers: o[0],
      changedPointers: o[1],
      pointerType: Ss,
      srcEvent: s
    });
  }, e;
})(En);
function sj(n, e) {
  var t = Zn(n.touches), r = this.targetIds;
  if (e & (Nt | qi) && t.length === 1)
    return r[t[0].identifier] = !0, [t, t];
  var s, a, o = Zn(n.changedTouches), l = [], c = this.target;
  if (a = t.filter(function(d) {
    return Em(d.target, c);
  }), e === Nt)
    for (s = 0; s < a.length; )
      r[a[s].identifier] = !0, s++;
  for (s = 0; s < o.length; )
    r[o[s].identifier] && l.push(o[s]), e & (ot | Ht) && delete r[o[s].identifier], s++;
  if (l.length)
    return [
      // merge targetTouches with changedTargetTouches so it contains ALL touches, including 'end' and 'cancel'
      Dm(a.concat(l), "identifier", !0),
      l
    ];
}
var aj = {
  mousedown: Nt,
  mousemove: qi,
  mouseup: ot
}, oj = "mousedown", lj = "mousemove mouseup", km = /* @__PURE__ */ (function(n) {
  or(e, n);
  function e() {
    var r, s = e.prototype;
    return s.evEl = oj, s.evWin = lj, r = n.apply(this, arguments) || this, r.pressed = !1, r;
  }
  var t = e.prototype;
  return t.handler = function(s) {
    var a = aj[s.type];
    a & Nt && s.button === 0 && (this.pressed = !0), a & qi && s.which !== 1 && (a = ot), this.pressed && (a & ot && (this.pressed = !1), this.callback(this.manager, a, {
      pointers: [s],
      changedPointers: [s],
      pointerType: Tm,
      srcEvent: s
    }));
  }, e;
})(En), uj = 2500, rE = 25;
function iE(n) {
  var e = n.changedPointers, t = e[0];
  if (t.identifier === this.primaryTouch) {
    var r = {
      x: t.clientX,
      y: t.clientY
    }, s = this.lastTouches;
    this.lastTouches.push(r);
    var a = function() {
      var l = s.indexOf(r);
      l > -1 && s.splice(l, 1);
    };
    setTimeout(a, uj);
  }
}
function cj(n, e) {
  n & Nt ? (this.primaryTouch = e.changedPointers[0].identifier, iE.call(this, e)) : n & (ot | Ht) && iE.call(this, e);
}
function dj(n) {
  for (var e = n.srcEvent.clientX, t = n.srcEvent.clientY, r = 0; r < this.lastTouches.length; r++) {
    var s = this.lastTouches[r], a = Math.abs(e - s.x), o = Math.abs(t - s.y);
    if (a <= rE && o <= rE)
      return !0;
  }
  return !1;
}
var bO = /* @__PURE__ */ (function() {
  var n = /* @__PURE__ */ (function(e) {
    or(t, e);
    function t(s, a) {
      var o;
      return o = e.call(this, s, a) || this, o.handler = function(l, c, d) {
        var f = d.pointerType === Ss, p = d.pointerType === Tm;
        if (!(p && d.sourceCapabilities && d.sourceCapabilities.firesTouchEvents)) {
          if (f)
            cj.call(Us(Us(o)), c, d);
          else if (p && dj.call(Us(Us(o)), d))
            return;
          o.callback(l, c, d);
        }
      }, o.touch = new Om(o.manager, o.handler), o.mouse = new km(o.manager, o.handler), o.primaryTouch = null, o.lastTouches = [], o;
    }
    var r = t.prototype;
    return r.destroy = function() {
      this.touch.destroy(), this.mouse.destroy();
    }, t;
  })(En);
  return n;
})();
function hj(n) {
  var e, t = n.options.inputClass;
  return t ? e = t : G$ ? e = yO : B$ ? e = Om : uO ? e = bO : e = km, new e(n, ej);
}
function tn(n, e, t) {
  return Array.isArray(n) ? ($r(n, t[e], t), !0) : !1;
}
var na = 1, ir = 2, ln = 4, ei = 8, jr = ei, Qn = 16, yr = 32, fj = 1;
function pj() {
  return fj++;
}
function Ks(n, e) {
  var t = e.manager;
  return t ? t.get(n) : n;
}
function nE(n) {
  return n & Qn ? "cancel" : n & ei ? "end" : n & ln ? "move" : n & ir ? "start" : "";
}
var Os = /* @__PURE__ */ (function() {
  function n(t) {
    t === void 0 && (t = {}), this.options = hr({
      enable: !0
    }, t), this.id = pj(), this.manager = null, this.state = na, this.simultaneous = {}, this.requireFail = [];
  }
  var e = n.prototype;
  return e.set = function(r) {
    return Si(this.options, r), this.manager && this.manager.touchAction.update(), this;
  }, e.recognizeWith = function(r) {
    if (tn(r, "recognizeWith", this))
      return this;
    var s = this.simultaneous;
    return r = Ks(r, this), s[r.id] || (s[r.id] = r, r.recognizeWith(this)), this;
  }, e.dropRecognizeWith = function(r) {
    return tn(r, "dropRecognizeWith", this) ? this : (r = Ks(r, this), delete this.simultaneous[r.id], this);
  }, e.requireFailure = function(r) {
    if (tn(r, "requireFailure", this))
      return this;
    var s = this.requireFail;
    return r = Ks(r, this), Ei(s, r) === -1 && (s.push(r), r.requireFailure(this)), this;
  }, e.dropRequireFailure = function(r) {
    if (tn(r, "dropRequireFailure", this))
      return this;
    r = Ks(r, this);
    var s = Ei(this.requireFail, r);
    return s > -1 && this.requireFail.splice(s, 1), this;
  }, e.hasRequireFailures = function() {
    return this.requireFail.length > 0;
  }, e.canRecognizeWith = function(r) {
    return !!this.simultaneous[r.id];
  }, e.emit = function(r) {
    var s = this, a = this.state;
    function o(l) {
      s.manager.emit(l, r);
    }
    a < ei && o(s.options.event + nE(a)), o(s.options.event), r.additionalEvent && o(r.additionalEvent), a >= ei && o(s.options.event + nE(a));
  }, e.tryEmit = function(r) {
    if (this.canEmit())
      return this.emit(r);
    this.state = yr;
  }, e.canEmit = function() {
    for (var r = 0; r < this.requireFail.length; ) {
      if (!(this.requireFail[r].state & (yr | na)))
        return !1;
      r++;
    }
    return !0;
  }, e.recognize = function(r) {
    var s = Si({}, r);
    if (!La(this.options.enable, [this, s])) {
      this.reset(), this.state = yr;
      return;
    }
    this.state & (jr | Qn | yr) && (this.state = na), this.state = this.process(s), this.state & (ir | ln | ei | Qn) && this.tryEmit(s);
  }, e.process = function(r) {
  }, e.getTouchAction = function() {
  }, e.reset = function() {
  }, n;
})(), Ap = /* @__PURE__ */ (function(n) {
  or(e, n);
  function e(r) {
    var s;
    return r === void 0 && (r = {}), s = n.call(this, hr({
      event: "tap",
      pointers: 1,
      taps: 1,
      interval: 300,
      // max time between the multi-tap taps
      time: 250,
      // max time of the pointer to be down (like finger on the screen)
      threshold: 9,
      // a minimal movement is ok, but keep it low
      posThreshold: 10
    }, r)) || this, s.pTime = !1, s.pCenter = !1, s._timer = null, s._input = null, s.count = 0, s;
  }
  var t = e.prototype;
  return t.getTouchAction = function() {
    return [Ip];
  }, t.process = function(s) {
    var a = this, o = this.options, l = s.pointers.length === o.pointers, c = s.distance < o.threshold, d = s.deltaTime < o.time;
    if (this.reset(), s.eventType & Nt && this.count === 0)
      return this.failTimeout();
    if (c && d && l) {
      if (s.eventType !== ot)
        return this.failTimeout();
      var f = this.pTime ? s.timeStamp - this.pTime < o.interval : !0, p = !this.pCenter || ha(this.pCenter, s.center) < o.posThreshold;
      this.pTime = s.timeStamp, this.pCenter = s.center, !p || !f ? this.count = 1 : this.count += 1, this._input = s;
      var m = this.count % o.taps;
      if (m === 0)
        return this.hasRequireFailures() ? (this._timer = setTimeout(function() {
          a.state = jr, a.tryEmit();
        }, o.interval), ir) : jr;
    }
    return yr;
  }, t.failTimeout = function() {
    var s = this;
    return this._timer = setTimeout(function() {
      s.state = yr;
    }, this.options.interval), yr;
  }, t.reset = function() {
    clearTimeout(this._timer);
  }, t.emit = function() {
    this.state === jr && (this._input.tapCount = this.count, this.manager.emit(this.options.event, this._input));
  }, e;
})(Os), un = /* @__PURE__ */ (function(n) {
  or(e, n);
  function e(r) {
    return r === void 0 && (r = {}), n.call(this, hr({
      pointers: 1
    }, r)) || this;
  }
  var t = e.prototype;
  return t.attrTest = function(s) {
    var a = this.options.pointers;
    return a === 0 || s.pointers.length === a;
  }, t.process = function(s) {
    var a = this.state, o = s.eventType, l = a & (ir | ln), c = this.attrTest(s);
    return l && (o & Ht || !c) ? a | Qn : l || c ? o & ot ? a | ei : a & ir ? a | ln : ir : yr;
  }, e;
})(Os);
function _O(n) {
  return n === on ? "down" : n === Ds ? "up" : n === Ts ? "left" : n === Es ? "right" : "";
}
var Cm = /* @__PURE__ */ (function(n) {
  or(e, n);
  function e(r) {
    var s;
    return r === void 0 && (r = {}), s = n.call(this, hr({
      event: "pan",
      threshold: 10,
      pointers: 1,
      direction: cO
    }, r)) || this, s.pX = null, s.pY = null, s;
  }
  var t = e.prototype;
  return t.getTouchAction = function() {
    var s = this.options.direction, a = [];
    return s & _r && a.push(Kn), s & _i && a.push(Vn), a;
  }, t.directionTest = function(s) {
    var a = this.options, o = !0, l = s.distance, c = s.direction, d = s.deltaX, f = s.deltaY;
    return c & a.direction || (a.direction & _r ? (c = d === 0 ? ca : d < 0 ? Ts : Es, o = d !== this.pX, l = Math.abs(s.deltaX)) : (c = f === 0 ? ca : f < 0 ? Ds : on, o = f !== this.pY, l = Math.abs(s.deltaY))), s.direction = c, o && l > a.threshold && c & a.direction;
  }, t.attrTest = function(s) {
    return un.prototype.attrTest.call(this, s) && // replace with a super call
    (this.state & ir || !(this.state & ir) && this.directionTest(s));
  }, t.emit = function(s) {
    this.pX = s.deltaX, this.pY = s.deltaY;
    var a = _O(s.direction);
    a && (s.additionalEvent = this.options.event + a), n.prototype.emit.call(this, s);
  }, e;
})(un), wO = /* @__PURE__ */ (function(n) {
  or(e, n);
  function e(r) {
    return r === void 0 && (r = {}), n.call(this, hr({
      event: "swipe",
      threshold: 10,
      velocity: 0.3,
      direction: _r | _i,
      pointers: 1
    }, r)) || this;
  }
  var t = e.prototype;
  return t.getTouchAction = function() {
    return Cm.prototype.getTouchAction.call(this);
  }, t.attrTest = function(s) {
    var a = this.options.direction, o;
    return a & (_r | _i) ? o = s.overallVelocity : a & _r ? o = s.overallVelocityX : a & _i && (o = s.overallVelocityY), n.prototype.attrTest.call(this, s) && a & s.offsetDirection && s.distance > this.options.threshold && s.maxPointers === this.options.pointers && Ti(o) > this.options.velocity && s.eventType & ot;
  }, t.emit = function(s) {
    var a = _O(s.offsetDirection);
    a && this.manager.emit(this.options.event + a, s), this.manager.emit(this.options.event, s);
  }, e;
})(un), xO = /* @__PURE__ */ (function(n) {
  or(e, n);
  function e(r) {
    return r === void 0 && (r = {}), n.call(this, hr({
      event: "pinch",
      threshold: 0,
      pointers: 2
    }, r)) || this;
  }
  var t = e.prototype;
  return t.getTouchAction = function() {
    return [bi];
  }, t.attrTest = function(s) {
    return n.prototype.attrTest.call(this, s) && (Math.abs(s.scale - 1) > this.options.threshold || this.state & ir);
  }, t.emit = function(s) {
    if (s.scale !== 1) {
      var a = s.scale < 1 ? "in" : "out";
      s.additionalEvent = this.options.event + a;
    }
    n.prototype.emit.call(this, s);
  }, e;
})(un), SO = /* @__PURE__ */ (function(n) {
  or(e, n);
  function e(r) {
    return r === void 0 && (r = {}), n.call(this, hr({
      event: "rotate",
      threshold: 0,
      pointers: 2
    }, r)) || this;
  }
  var t = e.prototype;
  return t.getTouchAction = function() {
    return [bi];
  }, t.attrTest = function(s) {
    return n.prototype.attrTest.call(this, s) && (Math.abs(s.rotation) > this.options.threshold || this.state & ir);
  }, e;
})(un), TO = /* @__PURE__ */ (function(n) {
  or(e, n);
  function e(r) {
    var s;
    return r === void 0 && (r = {}), s = n.call(this, hr({
      event: "press",
      pointers: 1,
      time: 251,
      // minimal time of the pointer to be pressed
      threshold: 9
    }, r)) || this, s._timer = null, s._input = null, s;
  }
  var t = e.prototype;
  return t.getTouchAction = function() {
    return [lO];
  }, t.process = function(s) {
    var a = this, o = this.options, l = s.pointers.length === o.pointers, c = s.distance < o.threshold, d = s.deltaTime > o.time;
    if (this._input = s, !c || !l || s.eventType & (ot | Ht) && !d)
      this.reset();
    else if (s.eventType & Nt)
      this.reset(), this._timer = setTimeout(function() {
        a.state = jr, a.tryEmit();
      }, o.time);
    else if (s.eventType & ot)
      return jr;
    return yr;
  }, t.reset = function() {
    clearTimeout(this._timer);
  }, t.emit = function(s) {
    this.state === jr && (s && s.eventType & ot ? this.manager.emit(this.options.event + "up", s) : (this._input.timeStamp = Sm(), this.manager.emit(this.options.event, this._input)));
  }, e;
})(Os), EO = {
  /**
   * @private
   * set if DOM events are being triggered.
   * But this is slower and unused by simple implementations, so disabled by default.
   * @type {Boolean}
   * @default false
   */
  domEvents: !1,
  /**
   * @private
   * The value for the touchAction property/fallback.
   * When set to `compute` it will magically set the correct value based on the added recognizers.
   * @type {String}
   * @default compute
   */
  touchAction: oO,
  /**
   * @private
   * @type {Boolean}
   * @default true
   */
  enable: !0,
  /**
   * @private
   * EXPERIMENTAL FEATURE -- can be removed/changed
   * Change the parent input target element.
   * If Null, then it is being set the to main element.
   * @type {Null|EventTarget}
   * @default null
   */
  inputTarget: null,
  /**
   * @private
   * force an input class
   * @type {Null|Function}
   * @default null
   */
  inputClass: null,
  /**
   * @private
   * Some CSS properties can be used to improve the working of Hammer.
   * Add them to this method and they will be set when creating a new Manager.
   * @namespace
   */
  cssProps: {
    /**
     * @private
     * Disables text selection to improve the dragging gesture. Mainly for desktop browsers.
     * @type {String}
     * @default 'none'
     */
    userSelect: "none",
    /**
     * @private
     * Disable the Windows Phone grippers when pressing an element.
     * @type {String}
     * @default 'none'
     */
    touchSelect: "none",
    /**
     * @private
     * Disables the default callout shown when you touch and hold a touch target.
     * On iOS, when you touch and hold a touch target such as a link, Safari displays
     * a callout containing information about the link. This property allows you to disable that callout.
     * @type {String}
     * @default 'none'
     */
    touchCallout: "none",
    /**
     * @private
     * Specifies whether zooming is enabled. Used by IE10>
     * @type {String}
     * @default 'none'
     */
    contentZooming: "none",
    /**
     * @private
     * Specifies that an entire element should be draggable instead of its contents. Mainly for desktop browsers.
     * @type {String}
     * @default 'none'
     */
    userDrag: "none",
    /**
     * @private
     * Overrides the highlight color shown when the user taps a link or a JavaScript
     * clickable element in iOS. This property obeys the alpha value, if specified.
     * @type {String}
     * @default 'rgba(0,0,0,0)'
     */
    tapHighlightColor: "rgba(0,0,0,0)"
  }
}, sE = [[SO, {
  enable: !1
}], [xO, {
  enable: !1
}, ["rotate"]], [wO, {
  direction: _r
}], [Cm, {
  direction: _r
}, ["swipe"]], [Ap], [Ap, {
  event: "doubletap",
  taps: 2
}, ["tap"]], [TO]], mj = 1, aE = 2;
function oE(n, e) {
  var t = n.element;
  if (t.style) {
    var r;
    $r(n.options.cssProps, function(s, a) {
      r = qa(t.style, a), e ? (n.oldCssProps[r] = t.style[r], t.style[r] = s) : t.style[r] = n.oldCssProps[r] || "";
    }), e || (n.oldCssProps = {});
  }
}
function vj(n, e) {
  var t = document.createEvent("Event");
  t.initEvent(n, !0, !0), t.gesture = e, e.target.dispatchEvent(t);
}
var lE = /* @__PURE__ */ (function() {
  function n(t, r) {
    var s = this;
    this.options = Si({}, EO, r || {}), this.options.inputTarget = this.options.inputTarget || t, this.handlers = {}, this.session = {}, this.recognizers = [], this.oldCssProps = {}, this.element = t, this.input = hj(this), this.touchAction = new hO(this, this.options.touchAction), oE(this, !0), $r(this.options.recognizers, function(a) {
      var o = s.add(new a[0](a[1]));
      a[2] && o.recognizeWith(a[2]), a[3] && o.requireFailure(a[3]);
    }, this);
  }
  var e = n.prototype;
  return e.set = function(r) {
    return Si(this.options, r), r.touchAction && this.touchAction.update(), r.inputTarget && (this.input.destroy(), this.input.target = r.inputTarget, this.input.init()), this;
  }, e.stop = function(r) {
    this.session.stopped = r ? aE : mj;
  }, e.recognize = function(r) {
    var s = this.session;
    if (!s.stopped) {
      this.touchAction.preventDefaults(r);
      var a, o = this.recognizers, l = s.curRecognizer;
      (!l || l && l.state & jr) && (s.curRecognizer = null, l = null);
      for (var c = 0; c < o.length; )
        a = o[c], s.stopped !== aE && // 1
        (!l || a === l || // 2
        a.canRecognizeWith(l)) ? a.recognize(r) : a.reset(), !l && a.state & (ir | ln | ei) && (s.curRecognizer = a, l = a), c++;
    }
  }, e.get = function(r) {
    if (r instanceof Os)
      return r;
    for (var s = this.recognizers, a = 0; a < s.length; a++)
      if (s[a].options.event === r)
        return s[a];
    return null;
  }, e.add = function(r) {
    if (tn(r, "add", this))
      return this;
    var s = this.get(r.options.event);
    return s && this.remove(s), this.recognizers.push(r), r.manager = this, this.touchAction.update(), r;
  }, e.remove = function(r) {
    if (tn(r, "remove", this))
      return this;
    var s = this.get(r);
    if (r) {
      var a = this.recognizers, o = Ei(a, s);
      o !== -1 && (a.splice(o, 1), this.touchAction.update());
    }
    return this;
  }, e.on = function(r, s) {
    if (r === void 0 || s === void 0)
      return this;
    var a = this.handlers;
    return $r(Xn(r), function(o) {
      a[o] = a[o] || [], a[o].push(s);
    }), this;
  }, e.off = function(r, s) {
    if (r === void 0)
      return this;
    var a = this.handlers;
    return $r(Xn(r), function(o) {
      s ? a[o] && a[o].splice(Ei(a[o], s), 1) : delete a[o];
    }), this;
  }, e.emit = function(r, s) {
    this.options.domEvents && vj(r, s);
    var a = this.handlers[r] && this.handlers[r].slice();
    if (!(!a || !a.length)) {
      s.type = r, s.preventDefault = function() {
        s.srcEvent.preventDefault();
      };
      for (var o = 0; o < a.length; )
        a[o](s), o++;
    }
  }, e.destroy = function() {
    this.element && oE(this, !1), this.handlers = {}, this.session = {}, this.input.destroy(), this.element = null;
  }, n;
})(), gj = {
  touchstart: Nt,
  touchmove: qi,
  touchend: ot,
  touchcancel: Ht
}, yj = "touchstart", bj = "touchstart touchmove touchend touchcancel", _j = /* @__PURE__ */ (function(n) {
  or(e, n);
  function e() {
    var r, s = e.prototype;
    return s.evTarget = yj, s.evWin = bj, r = n.apply(this, arguments) || this, r.started = !1, r;
  }
  var t = e.prototype;
  return t.handler = function(s) {
    var a = gj[s.type];
    if (a === Nt && (this.started = !0), !!this.started) {
      var o = wj.call(this, s, a);
      a & (ot | Ht) && o[0].length - o[1].length === 0 && (this.started = !1), this.callback(this.manager, a, {
        pointers: o[0],
        changedPointers: o[1],
        pointerType: Ss,
        srcEvent: s
      });
    }
  }, e;
})(En);
function wj(n, e) {
  var t = Zn(n.touches), r = Zn(n.changedTouches);
  return e & (ot | Ht) && (t = Dm(t.concat(r), "identifier", !0)), [t, r];
}
function DO(n, e, t) {
  var r = "DEPRECATED METHOD: " + e + `
` + t + ` AT 
`;
  return function() {
    var s = new Error("get-stack-trace"), a = s && s.stack ? s.stack.replace(/^[^\(]+?[\n$]/gm, "").replace(/^\s+at\s+/gm, "").replace(/^Object.<anonymous>\s*\(/gm, "{anonymous}()@") : "Unknown Stack Trace", o = window.console && (window.console.warn || window.console.log);
    return o && o.call(window.console, r, a), n.apply(this, arguments);
  };
}
var OO = DO(function(n, e, t) {
  for (var r = Object.keys(e), s = 0; s < r.length; )
    (!t || t && n[r[s]] === void 0) && (n[r[s]] = e[r[s]]), s++;
  return n;
}, "extend", "Use `assign`."), xj = DO(function(n, e) {
  return OO(n, e, !0);
}, "merge", "Use `assign`.");
function Sj(n, e, t) {
  var r = e.prototype, s;
  s = n.prototype = Object.create(r), s.constructor = n, s._super = r, t && Si(s, t);
}
function uE(n, e) {
  return function() {
    return n.apply(e, arguments);
  };
}
var Rm = /* @__PURE__ */ (function() {
  var n = (
    /**
      * @private
      * @const {string}
      */
    function(t, r) {
      return r === void 0 && (r = {}), new lE(t, hr({
        recognizers: sE.concat()
      }, r));
    }
  );
  return n.VERSION = "2.0.17-rc", n.DIRECTION_ALL = cO, n.DIRECTION_DOWN = on, n.DIRECTION_LEFT = Ts, n.DIRECTION_RIGHT = Es, n.DIRECTION_UP = Ds, n.DIRECTION_HORIZONTAL = _r, n.DIRECTION_VERTICAL = _i, n.DIRECTION_NONE = ca, n.DIRECTION_DOWN = on, n.INPUT_START = Nt, n.INPUT_MOVE = qi, n.INPUT_END = ot, n.INPUT_CANCEL = Ht, n.STATE_POSSIBLE = na, n.STATE_BEGAN = ir, n.STATE_CHANGED = ln, n.STATE_ENDED = ei, n.STATE_RECOGNIZED = jr, n.STATE_CANCELLED = Qn, n.STATE_FAILED = yr, n.Manager = lE, n.Input = En, n.TouchAction = hO, n.TouchInput = Om, n.MouseInput = km, n.PointerEventInput = yO, n.TouchMouseInput = bO, n.SingleTouchInput = _j, n.Recognizer = Os, n.AttrRecognizer = un, n.Tap = Ap, n.Pan = Cm, n.Swipe = wO, n.Pinch = xO, n.Rotate = SO, n.Press = TO, n.on = Gn, n.off = Bn, n.each = $r, n.merge = xj, n.extend = OO, n.bindFn = uE, n.assign = Si, n.inherit = Sj, n.bindFn = uE, n.prefixed = qa, n.toArray = Zn, n.inArray = Ei, n.uniqueArray = Dm, n.splitStr = Xn, n.boolOrFn = La, n.hasParent = Em, n.addEventListeners = Gn, n.removeEventListeners = Bn, n.defaults = Si({}, EO, {
    preset: sE
  }), n;
})();
Rm.defaults;
const Pm = R2("DELETE");
function kO(n) {
  for (var e = arguments.length, t = new Array(e > 1 ? e - 1 : 0), r = 1; r < e; r++)
    t[r - 1] = arguments[r];
  return Im({}, n, ...t);
}
function Im() {
  const n = fa(...arguments);
  return RO(n), n;
}
function fa() {
  for (var n = arguments.length, e = new Array(n), t = 0; t < n; t++)
    e[t] = arguments[t];
  if (e.length < 2)
    return e[0];
  if (e.length > 2)
    return fa(Im(e[0], e[1]), ...Tn(e).call(e, 2));
  const r = e[0], s = e[1];
  if (r instanceof Date && s instanceof Date)
    return r.setTime(s.getTime()), r;
  for (const a of tO(s))
    Object.prototype.propertyIsEnumerable.call(s, a) && (s[a] === Pm ? delete r[a] : r[a] !== null && s[a] !== null && typeof r[a] == "object" && typeof s[a] == "object" && !ye(r[a]) && !ye(s[a]) ? r[a] = fa(r[a], s[a]) : r[a] = CO(s[a]));
  return r;
}
function CO(n) {
  return ye(n) ? Ae(n).call(n, (e) => CO(e)) : typeof n == "object" && n !== null ? n instanceof Date ? new Date(n.getTime()) : fa({}, n) : n;
}
function RO(n) {
  for (const e of ct(n))
    n[e] === Pm ? delete n[e] : typeof n[e] == "object" && n[e] !== null && RO(n[e]);
}
function Tj() {
  for (var n = arguments.length, e = new Array(n), t = 0; t < n; t++)
    e[t] = arguments[t];
  return Ej(e.length ? e : [Ut()]);
}
function Ej(n) {
  let e = Dj(n), t = St(e, 3), r = t[0], s = t[1], a = t[2], o = 1;
  const l = () => {
    const c = 2091639 * r + o * 23283064365386963e-26;
    return r = s, s = a, a = c - (o = c | 0);
  };
  return l.uint32 = () => l() * 4294967296, l.fract53 = () => l() + (l() * 2097152 | 0) * 11102230246251565e-32, l.algorithm = "Alea", l.seed = n, l.version = "0.9", l;
}
function Dj() {
  const n = Oj();
  let e = n(" "), t = n(" "), r = n(" ");
  for (let s = 0; s < arguments.length; s++)
    e -= n(s < 0 || arguments.length <= s ? void 0 : arguments[s]), e < 0 && (e += 1), t -= n(s < 0 || arguments.length <= s ? void 0 : arguments[s]), t < 0 && (t += 1), r -= n(s < 0 || arguments.length <= s ? void 0 : arguments[s]), r < 0 && (r += 1);
  return [e, t, r];
}
function Oj() {
  let n = 4022871197;
  return function(e) {
    const t = e.toString();
    for (let r = 0; r < t.length; r++) {
      n += t.charCodeAt(r);
      let s = 0.02519603282416938 * n;
      n = s >>> 0, s -= n, s *= n, n = s >>> 0, s -= n, n += s * 4294967296;
    }
    return (n >>> 0) * 23283064365386963e-26;
  };
}
function kj() {
  const n = () => {
  };
  return {
    on: n,
    off: n,
    destroy: n,
    emit: n,
    get() {
      return {
        set: n
      };
    }
  };
}
const Mm = typeof window < "u" ? window.Hammer || Rm : function() {
  return kj();
};
function br(n) {
  var e;
  this._cleanupQueue = [], this.active = !1, this._dom = {
    container: n,
    overlay: document.createElement("div")
  }, this._dom.overlay.classList.add("vis-overlay"), this._dom.container.appendChild(this._dom.overlay), this._cleanupQueue.push(() => {
    this._dom.overlay.parentNode.removeChild(this._dom.overlay);
  });
  const t = Mm(this._dom.overlay);
  t.on("tap", N(e = this._onTapOverlay).call(e, this)), this._cleanupQueue.push(() => {
    t.destroy();
  });
  const r = ["tap", "doubletap", "press", "pinch", "pan", "panstart", "panmove", "panend"];
  B(r).call(r, (s) => {
    t.on(s, (a) => {
      a.srcEvent.stopPropagation();
    });
  }), document && document.body && (this._onClick = (s) => {
    Cj(s.target, n) || this.deactivate();
  }, document.body.addEventListener("click", this._onClick), this._cleanupQueue.push(() => {
    document.body.removeEventListener("click", this._onClick);
  })), this._escListener = (s) => {
    ("key" in s ? s.key === "Escape" : s.keyCode === 27) && this.deactivate();
  };
}
xm(br.prototype);
br.current = null;
br.prototype.destroy = function() {
  this.deactivate();
  for (const t of B2(n = Mt(e = this._cleanupQueue).call(e, 0)).call(n)) {
    var n, e;
    t();
  }
};
br.prototype.activate = function() {
  br.current && br.current.deactivate(), br.current = this, this.active = !0, this._dom.overlay.style.display = "none", this._dom.container.classList.add("vis-active"), this.emit("change"), this.emit("activate"), document.body.addEventListener("keydown", this._escListener);
};
br.prototype.deactivate = function() {
  this.active = !1, this._dom.overlay.style.display = "block", this._dom.container.classList.remove("vis-active"), document.body.removeEventListener("keydown", this._escListener), this.emit("change"), this.emit("deactivate");
};
br.prototype._onTapOverlay = function(n) {
  this.activate(), n.srcEvent.stopPropagation();
};
function Cj(n, e) {
  for (; n; ) {
    if (n === e)
      return !0;
    n = n.parentNode;
  }
  return !1;
}
const Rj = /^\/?Date\((-?\d+)/i, Pj = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i, Ij = /^#?([a-f\d])([a-f\d])([a-f\d])$/i, Mj = /^rgb\( *(1?\d{1,2}|2[0-4]\d|25[0-5]) *, *(1?\d{1,2}|2[0-4]\d|25[0-5]) *, *(1?\d{1,2}|2[0-4]\d|25[0-5]) *\)$/i, Aj = /^rgba\( *(1?\d{1,2}|2[0-4]\d|25[0-5]) *, *(1?\d{1,2}|2[0-4]\d|25[0-5]) *, *(1?\d{1,2}|2[0-4]\d|25[0-5]) *, *([01]|0?\.\d+) *\)$/i;
function Wn(n) {
  return n instanceof Number || typeof n == "number";
}
function PO(n) {
  if (n)
    for (; n.hasChildNodes() === !0; ) {
      const e = n.firstChild;
      e && (PO(e), n.removeChild(e));
    }
}
function Kt(n) {
  return n instanceof String || typeof n == "string";
}
function Np(n) {
  return typeof n == "object" && n !== null;
}
function Nj(n) {
  if (n instanceof Date)
    return !0;
  if (Kt(n)) {
    if (Rj.exec(n))
      return !0;
    if (!isNaN(Date.parse(n)))
      return !0;
  }
  return !1;
}
function Di(n, e, t, r) {
  let s = !1;
  r === !0 && (s = e[t] === null && n[t] !== void 0), s ? delete n[t] : n[t] = e[t];
}
function IO(n, e) {
  let t = arguments.length > 2 && arguments[2] !== void 0 ? arguments[2] : !1;
  for (const r in n)
    if (e[r] !== void 0)
      if (e[r] === null || typeof e[r] != "object")
        Di(n, e, r, t);
      else {
        const s = n[r], a = e[r];
        Np(s) && Np(a) && IO(s, a, t);
      }
}
const qj = Yr;
function Lj(n, e) {
  if (!ye(n))
    throw new Error("Array with property names expected as first argument");
  for (var t = arguments.length, r = new Array(t > 2 ? t - 2 : 0), s = 2; s < t; s++)
    r[s - 2] = arguments[s];
  for (const a of r)
    for (let o = 0; o < n.length; o++) {
      const l = n[o];
      a && Object.prototype.hasOwnProperty.call(a, l) && (e[l] = a[l]);
    }
  return e;
}
function Fj(n, e, t) {
  let r = arguments.length > 3 && arguments[3] !== void 0 ? arguments[3] : !1;
  if (ye(t))
    throw new TypeError("Arrays are not supported by deepExtend");
  for (let s = 0; s < n.length; s++) {
    const a = n[s];
    if (Object.prototype.hasOwnProperty.call(t, a))
      if (t[a] && t[a].constructor === Object)
        e[a] === void 0 && (e[a] = {}), e[a].constructor === Object ? Jn(e[a], t[a], !1, r) : Di(e, t, a, r);
      else {
        if (ye(t[a]))
          throw new TypeError("Arrays are not supported by deepExtend");
        Di(e, t, a, r);
      }
  }
  return e;
}
function $j(n, e, t) {
  let r = arguments.length > 3 && arguments[3] !== void 0 ? arguments[3] : !1;
  if (ye(t))
    throw new TypeError("Arrays are not supported by deepExtend");
  for (const s in t)
    if (Object.prototype.hasOwnProperty.call(t, s) && !an(n).call(n, s))
      if (t[s] && t[s].constructor === Object)
        e[s] === void 0 && (e[s] = {}), e[s].constructor === Object ? Jn(e[s], t[s]) : Di(e, t, s, r);
      else if (ye(t[s])) {
        e[s] = [];
        for (let a = 0; a < t[s].length; a++)
          e[s].push(t[s][a]);
      } else
        Di(e, t, s, r);
  return e;
}
function Jn(n, e) {
  let t = arguments.length > 2 && arguments[2] !== void 0 ? arguments[2] : !1, r = arguments.length > 3 && arguments[3] !== void 0 ? arguments[3] : !1;
  for (const a in e)
    if (Object.prototype.hasOwnProperty.call(e, a) || t === !0)
      if (typeof e[a] == "object" && e[a] !== null && bT(e[a]) === Object.prototype)
        n[a] === void 0 ? n[a] = Jn({}, e[a], t) : typeof n[a] == "object" && n[a] !== null && bT(n[a]) === Object.prototype ? Jn(n[a], e[a], t) : Di(n, e, a, r);
      else if (ye(e[a])) {
        var s;
        n[a] = Tn(s = e[a]).call(s);
      } else
        Di(n, e, a, r);
  return n;
}
function jj(n, e) {
  if (n.length !== e.length)
    return !1;
  for (let t = 0, r = n.length; t < r; t++)
    if (n[t] != e[t])
      return !1;
  return !0;
}
function Hn(n) {
  const e = typeof n;
  return e === "object" ? n === null ? "null" : n instanceof Boolean ? "Boolean" : n instanceof Number ? "Number" : n instanceof String ? "String" : ye(n) ? "Array" : n instanceof Date ? "Date" : "Object" : e === "number" ? "Number" : e === "boolean" ? "Boolean" : e === "string" ? "String" : e === void 0 ? "undefined" : e;
}
function es(n, e) {
  return [...n, e];
}
function MO(n) {
  return Tn(n).call(n);
}
function Yj(n) {
  return n.getBoundingClientRect().left;
}
function Hj(n) {
  return n.getBoundingClientRect().right;
}
function Gj(n) {
  return n.getBoundingClientRect().top;
}
function Bj(n, e) {
  let t = n.className.split(" ");
  const r = e.split(" ");
  t = rt(t).call(t, ie(r).call(r, function(s) {
    return !an(t).call(t, s);
  })), n.className = t.join(" ");
}
function Wj(n, e) {
  let t = n.className.split(" ");
  const r = e.split(" ");
  t = ie(t).call(t, function(s) {
    return !an(r).call(r, s);
  }), n.className = t.join(" ");
}
function zj(n, e) {
  if (ye(n)) {
    const t = n.length;
    for (let r = 0; r < t; r++)
      e(n[r], r, n);
  } else
    for (const t in n)
      Object.prototype.hasOwnProperty.call(n, t) && e(n[t], t, n);
}
const Uj = _m;
function Vj(n, e, t) {
  return n[e] !== t ? (n[e] = t, !0) : !1;
}
function Kj(n) {
  let e = !1;
  return () => {
    e || (e = !0, requestAnimationFrame(() => {
      e = !1, n();
    }));
  };
}
function Xj(n) {
  n || (n = window.event), n && (n.preventDefault ? n.preventDefault() : n.returnValue = !1);
}
function Zj() {
  let n = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : window.event, e = null;
  return n && (n.target ? e = n.target : n.srcElement && (e = n.srcElement)), !(e instanceof Element) || e.nodeType != null && e.nodeType == 3 && (e = e.parentNode, !(e instanceof Element)) ? null : e;
}
function Qj(n, e) {
  let t = n;
  for (; t; ) {
    if (t === e)
      return !0;
    if (t.parentNode)
      t = t.parentNode;
    else
      return !1;
  }
  return !1;
}
const Jj = {
  /**
   * Convert a value into a boolean.
   * @param value - Value to be converted intoboolean, a function will be executed as `(() => unknown)`.
   * @param defaultValue - If the value or the return value of the function == null then this will be returned.
   * @returns Corresponding boolean value, if none then the default value, if none then null.
   */
  asBoolean(n, e) {
    return typeof n == "function" && (n = n()), n != null ? n != !1 : e || null;
  },
  /**
   * Convert a value into a number.
   * @param value - Value to be converted intonumber, a function will be executed as `(() => unknown)`.
   * @param defaultValue - If the value or the return value of the function == null then this will be returned.
   * @returns Corresponding **boxed** number value, if none then the default value, if none then null.
   */
  asNumber(n, e) {
    return typeof n == "function" && (n = n()), n != null ? Number(n) || e || null : e || null;
  },
  /**
   * Convert a value into a string.
   * @param value - Value to be converted intostring, a function will be executed as `(() => unknown)`.
   * @param defaultValue - If the value or the return value of the function == null then this will be returned.
   * @returns Corresponding **boxed** string value, if none then the default value, if none then null.
   */
  asString(n, e) {
    return typeof n == "function" && (n = n()), n != null ? String(n) : e || null;
  },
  /**
   * Convert a value into a size.
   * @param value - Value to be converted intosize, a function will be executed as `(() => unknown)`.
   * @param defaultValue - If the value or the return value of the function == null then this will be returned.
   * @returns Corresponding string value (number + 'px'), if none then the default value, if none then null.
   */
  asSize(n, e) {
    return typeof n == "function" && (n = n()), Kt(n) ? n : Wn(n) ? n + "px" : e || null;
  },
  /**
   * Convert a value into a DOM Element.
   * @param value - Value to be converted into DOM Element, a function will be executed as `(() => unknown)`.
   * @param defaultValue - If the value or the return value of the function == null then this will be returned.
   * @returns The DOM Element, if none then the default value, if none then null.
   */
  asElement(n, e) {
    return typeof n == "function" && (n = n()), n || e || null;
  }
};
function Fa(n) {
  let e;
  switch (n.length) {
    case 3:
    case 4:
      return e = Ij.exec(n), e ? {
        r: Vt(e[1] + e[1], 16),
        g: Vt(e[2] + e[2], 16),
        b: Vt(e[3] + e[3], 16)
      } : null;
    case 6:
    case 7:
      return e = Pj.exec(n), e ? {
        r: Vt(e[1], 16),
        g: Vt(e[2], 16),
        b: Vt(e[3], 16)
      } : null;
    default:
      return null;
  }
}
function eY(n, e) {
  if (an(n).call(n, "rgba"))
    return n;
  if (an(n).call(n, "rgb")) {
    const t = n.substr(Fe(n).call(n, "(") + 1).replace(")", "").split(",");
    return "rgba(" + t[0] + "," + t[1] + "," + t[2] + "," + e + ")";
  } else {
    const t = Fa(n);
    return t == null ? n : "rgba(" + t.r + "," + t.g + "," + t.b + "," + e + ")";
  }
}
function Am(n, e, t) {
  var r;
  return "#" + Tn(r = ((1 << 24) + (n << 16) + (e << 8) + t).toString(16)).call(r, 1);
}
function tY(n, e) {
  if (Kt(n)) {
    let r = n;
    if (qm(r)) {
      var t;
      const s = Ae(t = r.substr(4).substr(0, r.length - 5).split(",")).call(t, function(a) {
        return Vt(a);
      });
      r = Am(s[0], s[1], s[2]);
    }
    if (Nm(r) === !0) {
      const s = NO(r), a = {
        h: s.h,
        s: s.s * 0.8,
        v: Math.min(1, s.v * 1.02)
      }, o = {
        h: s.h,
        s: Math.min(1, s.s * 1.25),
        v: s.v * 0.8
      }, l = qp(o.h, o.s, o.v), c = qp(a.h, a.s, a.v);
      return {
        background: r,
        border: l,
        highlight: {
          background: c,
          border: l
        },
        hover: {
          background: c,
          border: l
        }
      };
    } else
      return {
        background: r,
        border: r,
        highlight: {
          background: r,
          border: r
        },
        hover: {
          background: r,
          border: r
        }
      };
  } else
    return e ? {
      background: n.background || e.background,
      border: n.border || e.border,
      highlight: Kt(n.highlight) ? {
        border: n.highlight,
        background: n.highlight
      } : {
        background: n.highlight && n.highlight.background || e.highlight.background,
        border: n.highlight && n.highlight.border || e.highlight.border
      },
      hover: Kt(n.hover) ? {
        border: n.hover,
        background: n.hover
      } : {
        border: n.hover && n.hover.border || e.hover.border,
        background: n.hover && n.hover.background || e.hover.background
      }
    } : {
      background: n.background || void 0,
      border: n.border || void 0,
      highlight: Kt(n.highlight) ? {
        border: n.highlight,
        background: n.highlight
      } : {
        background: n.highlight && n.highlight.background || void 0,
        border: n.highlight && n.highlight.border || void 0
      },
      hover: Kt(n.hover) ? {
        border: n.hover,
        background: n.hover
      } : {
        border: n.hover && n.hover.border || void 0,
        background: n.hover && n.hover.background || void 0
      }
    };
}
function rn(n, e, t) {
  n = n / 255, e = e / 255, t = t / 255;
  const r = Math.min(n, Math.min(e, t)), s = Math.max(n, Math.max(e, t));
  if (r === s)
    return {
      h: 0,
      s: 0,
      v: r
    };
  const a = n === r ? e - t : t === r ? n - e : t - n, l = 60 * ((n === r ? 3 : t === r ? 1 : 5) - a / (s - r)) / 360, c = (s - r) / s;
  return {
    h: l,
    s: c,
    v: s
  };
}
function AO(n) {
  const e = document.createElement("div"), t = {};
  e.style.cssText = n;
  for (let r = 0; r < e.style.length; ++r)
    t[e.style[r]] = e.style.getPropertyValue(e.style[r]);
  return t;
}
function rY(n, e) {
  const t = AO(e);
  for (const s of C$(t)) {
    var r = St(s, 2);
    const a = r[0], o = r[1];
    n.style.setProperty(a, o);
  }
}
function iY(n, e) {
  const t = AO(e);
  for (const r of ct(t))
    n.style.removeProperty(r);
}
function zn(n, e, t) {
  let r, s, a;
  const o = Math.floor(n * 6), l = n * 6 - o, c = t * (1 - e), d = t * (1 - l * e), f = t * (1 - (1 - l) * e);
  switch (o % 6) {
    case 0:
      r = t, s = f, a = c;
      break;
    case 1:
      r = d, s = t, a = c;
      break;
    case 2:
      r = c, s = t, a = f;
      break;
    case 3:
      r = c, s = d, a = t;
      break;
    case 4:
      r = f, s = c, a = t;
      break;
    case 5:
      r = t, s = c, a = d;
      break;
  }
  return {
    r: Math.floor(r * 255),
    g: Math.floor(s * 255),
    b: Math.floor(a * 255)
  };
}
function qp(n, e, t) {
  const r = zn(n, e, t);
  return Am(r.r, r.g, r.b);
}
function NO(n) {
  const e = Fa(n);
  if (!e)
    throw new TypeError("'".concat(n, "' is not a valid color."));
  return rn(e.r, e.g, e.b);
}
function Nm(n) {
  return /(^#[0-9A-F]{6}$)|(^#[0-9A-F]{3}$)/i.test(n);
}
function qm(n) {
  return Mj.test(n);
}
function qO(n) {
  return Aj.test(n);
}
function nY(n, e) {
  if (e !== null && typeof e == "object") {
    const t = _s(e);
    for (let r = 0; r < n.length; r++)
      Object.prototype.hasOwnProperty.call(e, n[r]) && typeof e[n[r]] == "object" && (t[n[r]] = Lm(e[n[r]]));
    return t;
  } else
    return null;
}
function Lm(n) {
  if (n === null || typeof n != "object")
    return null;
  if (n instanceof Element)
    return n;
  const e = _s(n);
  for (const t in n)
    Object.prototype.hasOwnProperty.call(n, t) && typeof n[t] == "object" && (e[t] = Lm(n[t]));
  return e;
}
function sY(n, e) {
  for (let t = 0; t < n.length; t++) {
    const r = n[t];
    let s;
    for (s = t; s > 0 && e(r, n[s - 1]) < 0; s--)
      n[s] = n[s - 1];
    n[s] = r;
  }
  return n;
}
function aY(n, e, t) {
  let r = arguments.length > 3 && arguments[3] !== void 0 ? arguments[3] : {};
  const s = function(g) {
    return g != null;
  }, a = function(g) {
    return g !== null && typeof g == "object";
  }, o = function(g) {
    for (const b in g)
      if (Object.prototype.hasOwnProperty.call(g, b))
        return !1;
    return !0;
  };
  if (!a(n))
    throw new Error("Parameter mergeTarget must be an object");
  if (!a(e))
    throw new Error("Parameter options must be an object");
  if (!s(t))
    throw new Error("Parameter option must have a value");
  if (!a(r))
    throw new Error("Parameter globalOptions must be an object");
  const l = function(g, b, _) {
    a(g[_]) || (g[_] = {});
    const x = b[_], T = g[_];
    for (const S in x)
      Object.prototype.hasOwnProperty.call(x, S) && (T[S] = x[S]);
  }, c = e[t], f = a(r) && !o(r) ? r[t] : void 0, p = f ? f.enabled : void 0;
  if (c === void 0)
    return;
  if (typeof c == "boolean") {
    a(n[t]) || (n[t] = {}), n[t].enabled = c;
    return;
  }
  if (c === null && !a(n[t]))
    if (s(f))
      n[t] = _s(f);
    else
      return;
  if (!a(c))
    return;
  let m = !0;
  c.enabled !== void 0 ? m = c.enabled : p !== void 0 && (m = f.enabled), l(n, e, t), n[t].enabled = m;
}
function oY(n, e, t, r) {
  let a = 0, o = 0, l = n.length - 1;
  for (; o <= l && a < 1e4; ) {
    const c = Math.floor((o + l) / 2), d = n[c], f = r === void 0 ? d[t] : d[t][r], p = e(f);
    if (p == 0)
      return c;
    p == -1 ? o = c + 1 : l = c - 1, a++;
  }
  return -1;
}
function lY(n, e, t, r, s) {
  let o = 0, l = 0, c = n.length - 1, d, f, p, m;
  for (s = s ?? function(g, b) {
    return g == b ? 0 : g < b ? -1 : 1;
  }; l <= c && o < 1e4; ) {
    if (m = Math.floor(0.5 * (c + l)), d = n[Math.max(0, m - 1)][t], f = n[m][t], p = n[Math.min(n.length - 1, m + 1)][t], s(f, e) == 0)
      return m;
    if (s(d, e) < 0 && s(f, e) > 0)
      return r == "before" ? Math.max(0, m - 1) : m;
    if (s(f, e) < 0 && s(p, e) > 0)
      return r == "before" ? m : Math.min(n.length - 1, m + 1);
    s(f, e) < 0 ? l = m + 1 : c = m - 1, o++;
  }
  return -1;
}
const uY = {
  /**
   * Provides no easing and no acceleration.
   * @param t - Time.
   * @returns Value at time t.
   */
  linear(n) {
    return n;
  },
  /**
   * Accelerate from zero velocity.
   * @param t - Time.
   * @returns Value at time t.
   */
  easeInQuad(n) {
    return n * n;
  },
  /**
   * Decelerate to zero velocity.
   * @param t - Time.
   * @returns Value at time t.
   */
  easeOutQuad(n) {
    return n * (2 - n);
  },
  /**
   * Accelerate until halfway, then decelerate.
   * @param t - Time.
   * @returns Value at time t.
   */
  easeInOutQuad(n) {
    return n < 0.5 ? 2 * n * n : -1 + (4 - 2 * n) * n;
  },
  /**
   * Accelerate from zero velocity.
   * @param t - Time.
   * @returns Value at time t.
   */
  easeInCubic(n) {
    return n * n * n;
  },
  /**
   * Decelerate to zero velocity.
   * @param t - Time.
   * @returns Value at time t.
   */
  easeOutCubic(n) {
    return --n * n * n + 1;
  },
  /**
   * Accelerate until halfway, then decelerate.
   * @param t - Time.
   * @returns Value at time t.
   */
  easeInOutCubic(n) {
    return n < 0.5 ? 4 * n * n * n : (n - 1) * (2 * n - 2) * (2 * n - 2) + 1;
  },
  /**
   * Accelerate from zero velocity.
   * @param t - Time.
   * @returns Value at time t.
   */
  easeInQuart(n) {
    return n * n * n * n;
  },
  /**
   * Decelerate to zero velocity.
   * @param t - Time.
   * @returns Value at time t.
   */
  easeOutQuart(n) {
    return 1 - --n * n * n * n;
  },
  /**
   * Accelerate until halfway, then decelerate.
   * @param t - Time.
   * @returns Value at time t.
   */
  easeInOutQuart(n) {
    return n < 0.5 ? 8 * n * n * n * n : 1 - 8 * --n * n * n * n;
  },
  /**
   * Accelerate from zero velocity.
   * @param t - Time.
   * @returns Value at time t.
   */
  easeInQuint(n) {
    return n * n * n * n * n;
  },
  /**
   * Decelerate to zero velocity.
   * @param t - Time.
   * @returns Value at time t.
   */
  easeOutQuint(n) {
    return 1 + --n * n * n * n * n;
  },
  /**
   * Accelerate until halfway, then decelerate.
   * @param t - Time.
   * @returns Value at time t.
   */
  easeInOutQuint(n) {
    return n < 0.5 ? 16 * n * n * n * n * n : 1 + 16 * --n * n * n * n * n;
  }
};
function cY() {
  const n = document.createElement("p");
  n.style.width = "100%", n.style.height = "200px";
  const e = document.createElement("div");
  e.style.position = "absolute", e.style.top = "0px", e.style.left = "0px", e.style.visibility = "hidden", e.style.width = "200px", e.style.height = "150px", e.style.overflow = "hidden", e.appendChild(n), document.body.appendChild(e);
  const t = n.offsetWidth;
  e.style.overflow = "scroll";
  let r = n.offsetWidth;
  return t == r && (r = e.clientWidth), document.body.removeChild(e), t - r;
}
function dY(n, e) {
  let t;
  ye(e) || (e = [e]);
  for (const r of n)
    if (r) {
      t = r[e[0]];
      for (let s = 1; s < e.length; s++)
        t && (t = t[e[s]]);
      if (typeof t < "u")
        break;
    }
  return t;
}
const hY = {
  black: "#000000",
  navy: "#000080",
  darkblue: "#00008B",
  mediumblue: "#0000CD",
  blue: "#0000FF",
  darkgreen: "#006400",
  green: "#008000",
  teal: "#008080",
  darkcyan: "#008B8B",
  deepskyblue: "#00BFFF",
  darkturquoise: "#00CED1",
  mediumspringgreen: "#00FA9A",
  lime: "#00FF00",
  springgreen: "#00FF7F",
  aqua: "#00FFFF",
  cyan: "#00FFFF",
  midnightblue: "#191970",
  dodgerblue: "#1E90FF",
  lightseagreen: "#20B2AA",
  forestgreen: "#228B22",
  seagreen: "#2E8B57",
  darkslategray: "#2F4F4F",
  limegreen: "#32CD32",
  mediumseagreen: "#3CB371",
  turquoise: "#40E0D0",
  royalblue: "#4169E1",
  steelblue: "#4682B4",
  darkslateblue: "#483D8B",
  mediumturquoise: "#48D1CC",
  indigo: "#4B0082",
  darkolivegreen: "#556B2F",
  cadetblue: "#5F9EA0",
  cornflowerblue: "#6495ED",
  mediumaquamarine: "#66CDAA",
  dimgray: "#696969",
  slateblue: "#6A5ACD",
  olivedrab: "#6B8E23",
  slategray: "#708090",
  lightslategray: "#778899",
  mediumslateblue: "#7B68EE",
  lawngreen: "#7CFC00",
  chartreuse: "#7FFF00",
  aquamarine: "#7FFFD4",
  maroon: "#800000",
  purple: "#800080",
  olive: "#808000",
  gray: "#808080",
  skyblue: "#87CEEB",
  lightskyblue: "#87CEFA",
  blueviolet: "#8A2BE2",
  darkred: "#8B0000",
  darkmagenta: "#8B008B",
  saddlebrown: "#8B4513",
  darkseagreen: "#8FBC8F",
  lightgreen: "#90EE90",
  mediumpurple: "#9370D8",
  darkviolet: "#9400D3",
  palegreen: "#98FB98",
  darkorchid: "#9932CC",
  yellowgreen: "#9ACD32",
  sienna: "#A0522D",
  brown: "#A52A2A",
  darkgray: "#A9A9A9",
  lightblue: "#ADD8E6",
  greenyellow: "#ADFF2F",
  paleturquoise: "#AFEEEE",
  lightsteelblue: "#B0C4DE",
  powderblue: "#B0E0E6",
  firebrick: "#B22222",
  darkgoldenrod: "#B8860B",
  mediumorchid: "#BA55D3",
  rosybrown: "#BC8F8F",
  darkkhaki: "#BDB76B",
  silver: "#C0C0C0",
  mediumvioletred: "#C71585",
  indianred: "#CD5C5C",
  peru: "#CD853F",
  chocolate: "#D2691E",
  tan: "#D2B48C",
  lightgrey: "#D3D3D3",
  palevioletred: "#D87093",
  thistle: "#D8BFD8",
  orchid: "#DA70D6",
  goldenrod: "#DAA520",
  crimson: "#DC143C",
  gainsboro: "#DCDCDC",
  plum: "#DDA0DD",
  burlywood: "#DEB887",
  lightcyan: "#E0FFFF",
  lavender: "#E6E6FA",
  darksalmon: "#E9967A",
  violet: "#EE82EE",
  palegoldenrod: "#EEE8AA",
  lightcoral: "#F08080",
  khaki: "#F0E68C",
  aliceblue: "#F0F8FF",
  honeydew: "#F0FFF0",
  azure: "#F0FFFF",
  sandybrown: "#F4A460",
  wheat: "#F5DEB3",
  beige: "#F5F5DC",
  whitesmoke: "#F5F5F5",
  mintcream: "#F5FFFA",
  ghostwhite: "#F8F8FF",
  salmon: "#FA8072",
  antiquewhite: "#FAEBD7",
  linen: "#FAF0E6",
  lightgoldenrodyellow: "#FAFAD2",
  oldlace: "#FDF5E6",
  red: "#FF0000",
  fuchsia: "#FF00FF",
  magenta: "#FF00FF",
  deeppink: "#FF1493",
  orangered: "#FF4500",
  tomato: "#FF6347",
  hotpink: "#FF69B4",
  coral: "#FF7F50",
  darkorange: "#FF8C00",
  lightsalmon: "#FFA07A",
  orange: "#FFA500",
  lightpink: "#FFB6C1",
  pink: "#FFC0CB",
  gold: "#FFD700",
  peachpuff: "#FFDAB9",
  navajowhite: "#FFDEAD",
  moccasin: "#FFE4B5",
  bisque: "#FFE4C4",
  mistyrose: "#FFE4E1",
  blanchedalmond: "#FFEBCD",
  papayawhip: "#FFEFD5",
  lavenderblush: "#FFF0F5",
  seashell: "#FFF5EE",
  cornsilk: "#FFF8DC",
  lemonchiffon: "#FFFACD",
  floralwhite: "#FFFAF0",
  snow: "#FFFAFA",
  yellow: "#FFFF00",
  lightyellow: "#FFFFE0",
  ivory: "#FFFFF0",
  white: "#FFFFFF"
};
let LO = class {
  /**
   * @param {number} [pixelRatio]
   */
  constructor() {
    let e = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : 1;
    this.pixelRatio = e, this.generated = !1, this.centerCoordinates = {
      x: 289 / 2,
      y: 289 / 2
    }, this.r = 289 * 0.49, this.color = {
      r: 255,
      g: 255,
      b: 255,
      a: 1
    }, this.hueCircle = void 0, this.initialColor = {
      r: 255,
      g: 255,
      b: 255,
      a: 1
    }, this.previousColor = void 0, this.applied = !1, this.updateCallback = () => {
    }, this.closeCallback = () => {
    }, this._create();
  }
  /**
   * this inserts the colorPicker into a div from the DOM
   * @param {Element} container
   */
  insertTo(e) {
    this.hammer !== void 0 && (this.hammer.destroy(), this.hammer = void 0), this.container = e, this.container.appendChild(this.frame), this._bindHammer(), this._setSize();
  }
  /**
   * the callback is executed on apply and save. Bind it to the application
   * @param {Function} callback
   */
  setUpdateCallback(e) {
    if (typeof e == "function")
      this.updateCallback = e;
    else
      throw new Error("Function attempted to set as colorPicker update callback is not a function.");
  }
  /**
   * the callback is executed on apply and save. Bind it to the application
   * @param {Function} callback
   */
  setCloseCallback(e) {
    if (typeof e == "function")
      this.closeCallback = e;
    else
      throw new Error("Function attempted to set as colorPicker closing callback is not a function.");
  }
  /**
   *
   * @param {string} color
   * @returns {string}
   * @private
   */
  _isColorString(e) {
    if (typeof e == "string")
      return hY[e];
  }
  /**
   * Set the color of the colorPicker
   * Supported formats:
   * 'red'                   --> HTML color string
   * '#ffffff'               --> hex string
   * 'rgb(255,255,255)'      --> rgb string
   * 'rgba(255,255,255,1.0)' --> rgba string
   * {r:255,g:255,b:255}     --> rgb object
   * {r:255,g:255,b:255,a:1.0} --> rgba object
   * @param {string | object} color
   * @param {boolean} [setInitial]
   */
  setColor(e) {
    let t = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : !0;
    if (e === "none")
      return;
    let r;
    const s = this._isColorString(e);
    if (s !== void 0 && (e = s), Kt(e) === !0) {
      if (qm(e) === !0) {
        const a = e.substr(4).substr(0, e.length - 5).split(",");
        r = {
          r: a[0],
          g: a[1],
          b: a[2],
          a: 1
        };
      } else if (qO(e) === !0) {
        const a = e.substr(5).substr(0, e.length - 6).split(",");
        r = {
          r: a[0],
          g: a[1],
          b: a[2],
          a: a[3]
        };
      } else if (Nm(e) === !0) {
        const a = Fa(e);
        r = {
          r: a.r,
          g: a.g,
          b: a.b,
          a: 1
        };
      }
    } else if (e instanceof Object && e.r !== void 0 && e.g !== void 0 && e.b !== void 0) {
      const a = e.a !== void 0 ? e.a : "1.0";
      r = {
        r: e.r,
        g: e.g,
        b: e.b,
        a
      };
    }
    if (r === void 0)
      throw new Error("Unknown color passed to the colorPicker. Supported are strings: rgb, hex, rgba. Object: rgb ({r:r,g:g,b:b,[a:a]}). Supplied: " + Yt(e));
    this._setColor(r, t);
  }
  /**
   * this shows the color picker.
   * The hue circle is constructed once and stored.
   */
  show() {
    this.closeCallback !== void 0 && (this.closeCallback(), this.closeCallback = void 0), this.applied = !1, this.frame.style.display = "block", this._generateHueCircle();
  }
  // ------------------------------------------ PRIVATE ----------------------------- //
  /**
   * Hide the picker. Is called by the cancel button.
   * Optional boolean to store the previous color for easy access later on.
   * @param {boolean} [storePrevious]
   * @private
   */
  _hide() {
    (arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : !0) === !0 && (this.previousColor = Yr({}, this.color)), this.applied === !0 && this.updateCallback(this.initialColor), this.frame.style.display = "none", Dt(() => {
      this.closeCallback !== void 0 && (this.closeCallback(), this.closeCallback = void 0);
    }, 0);
  }
  /**
   * bound to the save button. Saves and hides.
   * @private
   */
  _save() {
    this.updateCallback(this.color), this.applied = !1, this._hide();
  }
  /**
   * Bound to apply button. Saves but does not close. Is undone by the cancel button.
   * @private
   */
  _apply() {
    this.applied = !0, this.updateCallback(this.color), this._updatePicker(this.color);
  }
  /**
   * load the color from the previous session.
   * @private
   */
  _loadLast() {
    this.previousColor !== void 0 ? this.setColor(this.previousColor, !1) : alert("There is no last color to load...");
  }
  /**
   * set the color, place the picker
   * @param {object} rgba
   * @param {boolean} [setInitial]
   * @private
   */
  _setColor(e) {
    (arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : !0) === !0 && (this.initialColor = Yr({}, e)), this.color = e;
    const r = rn(e.r, e.g, e.b), s = 2 * Math.PI, a = this.r * r.s, o = this.centerCoordinates.x + a * Math.sin(s * r.h), l = this.centerCoordinates.y + a * Math.cos(s * r.h);
    this.colorPickerSelector.style.left = o - 0.5 * this.colorPickerSelector.clientWidth + "px", this.colorPickerSelector.style.top = l - 0.5 * this.colorPickerSelector.clientHeight + "px", this._updatePicker(e);
  }
  /**
   * bound to opacity control
   * @param {number} value
   * @private
   */
  _setOpacity(e) {
    this.color.a = e / 100, this._updatePicker(this.color);
  }
  /**
   * bound to brightness control
   * @param {number} value
   * @private
   */
  _setBrightness(e) {
    const t = rn(this.color.r, this.color.g, this.color.b);
    t.v = e / 100;
    const r = zn(t.h, t.s, t.v);
    r.a = this.color.a, this.color = r, this._updatePicker();
  }
  /**
   * update the color picker. A black circle overlays the hue circle to mimic the brightness decreasing.
   * @param {object} rgba
   * @private
   */
  _updatePicker() {
    let e = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : this.color;
    const t = rn(e.r, e.g, e.b), r = this.colorPickerCanvas.getContext("2d");
    this.pixelRation === void 0 && (this.pixelRatio = (window.devicePixelRatio || 1) / (r.webkitBackingStorePixelRatio || r.mozBackingStorePixelRatio || r.msBackingStorePixelRatio || r.oBackingStorePixelRatio || r.backingStorePixelRatio || 1)), r.setTransform(this.pixelRatio, 0, 0, this.pixelRatio, 0, 0);
    const s = this.colorPickerCanvas.clientWidth, a = this.colorPickerCanvas.clientHeight;
    r.clearRect(0, 0, s, a), r.putImageData(this.hueCircle, 0, 0), r.fillStyle = "rgba(0,0,0," + (1 - t.v) + ")", r.circle(this.centerCoordinates.x, this.centerCoordinates.y, this.r), nO(r).call(r), this.brightnessRange.value = 100 * t.v, this.opacityRange.value = 100 * e.a, this.initialColorDiv.style.backgroundColor = "rgba(" + this.initialColor.r + "," + this.initialColor.g + "," + this.initialColor.b + "," + this.initialColor.a + ")", this.newColorDiv.style.backgroundColor = "rgba(" + this.color.r + "," + this.color.g + "," + this.color.b + "," + this.color.a + ")";
  }
  /**
   * used by create to set the size of the canvas.
   * @private
   */
  _setSize() {
    this.colorPickerCanvas.style.width = "100%", this.colorPickerCanvas.style.height = "100%", this.colorPickerCanvas.width = 289 * this.pixelRatio, this.colorPickerCanvas.height = 289 * this.pixelRatio;
  }
  /**
   * create all dom elements
   * TODO: cleanup, lots of similar dom elements
   * @private
   */
  _create() {
    var e, t, r, s;
    if (this.frame = document.createElement("div"), this.frame.className = "vis-color-picker", this.colorPickerDiv = document.createElement("div"), this.colorPickerSelector = document.createElement("div"), this.colorPickerSelector.className = "vis-selector", this.colorPickerDiv.appendChild(this.colorPickerSelector), this.colorPickerCanvas = document.createElement("canvas"), this.colorPickerDiv.appendChild(this.colorPickerCanvas), this.colorPickerCanvas.getContext) {
      const o = this.colorPickerCanvas.getContext("2d");
      this.pixelRatio = (window.devicePixelRatio || 1) / (o.webkitBackingStorePixelRatio || o.mozBackingStorePixelRatio || o.msBackingStorePixelRatio || o.oBackingStorePixelRatio || o.backingStorePixelRatio || 1), this.colorPickerCanvas.getContext("2d").setTransform(this.pixelRatio, 0, 0, this.pixelRatio, 0, 0);
    } else {
      const o = document.createElement("DIV");
      o.style.color = "red", o.style.fontWeight = "bold", o.style.padding = "10px", o.innerText = "Error: your browser does not support HTML canvas", this.colorPickerCanvas.appendChild(o);
    }
    this.colorPickerDiv.className = "vis-color", this.opacityDiv = document.createElement("div"), this.opacityDiv.className = "vis-opacity", this.brightnessDiv = document.createElement("div"), this.brightnessDiv.className = "vis-brightness", this.arrowDiv = document.createElement("div"), this.arrowDiv.className = "vis-arrow", this.opacityRange = document.createElement("input");
    try {
      this.opacityRange.type = "range", this.opacityRange.min = "0", this.opacityRange.max = "100";
    } catch {
    }
    this.opacityRange.value = "100", this.opacityRange.className = "vis-range", this.brightnessRange = document.createElement("input");
    try {
      this.brightnessRange.type = "range", this.brightnessRange.min = "0", this.brightnessRange.max = "100";
    } catch {
    }
    this.brightnessRange.value = "100", this.brightnessRange.className = "vis-range", this.opacityDiv.appendChild(this.opacityRange), this.brightnessDiv.appendChild(this.brightnessRange);
    const a = this;
    this.opacityRange.onchange = function() {
      a._setOpacity(this.value);
    }, this.opacityRange.oninput = function() {
      a._setOpacity(this.value);
    }, this.brightnessRange.onchange = function() {
      a._setBrightness(this.value);
    }, this.brightnessRange.oninput = function() {
      a._setBrightness(this.value);
    }, this.brightnessLabel = document.createElement("div"), this.brightnessLabel.className = "vis-label vis-brightness", this.brightnessLabel.innerText = "brightness:", this.opacityLabel = document.createElement("div"), this.opacityLabel.className = "vis-label vis-opacity", this.opacityLabel.innerText = "opacity:", this.newColorDiv = document.createElement("div"), this.newColorDiv.className = "vis-new-color", this.newColorDiv.innerText = "new", this.initialColorDiv = document.createElement("div"), this.initialColorDiv.className = "vis-initial-color", this.initialColorDiv.innerText = "initial", this.cancelButton = document.createElement("div"), this.cancelButton.className = "vis-button vis-cancel", this.cancelButton.innerText = "cancel", this.cancelButton.onclick = N(e = this._hide).call(e, this, !1), this.applyButton = document.createElement("div"), this.applyButton.className = "vis-button vis-apply", this.applyButton.innerText = "apply", this.applyButton.onclick = N(t = this._apply).call(t, this), this.saveButton = document.createElement("div"), this.saveButton.className = "vis-button vis-save", this.saveButton.innerText = "save", this.saveButton.onclick = N(r = this._save).call(r, this), this.loadButton = document.createElement("div"), this.loadButton.className = "vis-button vis-load", this.loadButton.innerText = "load last", this.loadButton.onclick = N(s = this._loadLast).call(s, this), this.frame.appendChild(this.colorPickerDiv), this.frame.appendChild(this.arrowDiv), this.frame.appendChild(this.brightnessLabel), this.frame.appendChild(this.brightnessDiv), this.frame.appendChild(this.opacityLabel), this.frame.appendChild(this.opacityDiv), this.frame.appendChild(this.newColorDiv), this.frame.appendChild(this.initialColorDiv), this.frame.appendChild(this.cancelButton), this.frame.appendChild(this.applyButton), this.frame.appendChild(this.saveButton), this.frame.appendChild(this.loadButton);
  }
  /**
   * bind hammer to the color picker
   * @private
   */
  _bindHammer() {
    this.drag = {}, this.pinch = {}, this.hammer = new Mm(this.colorPickerCanvas), this.hammer.get("pinch").set({
      enable: !0
    }), this.hammer.on("hammer.input", (e) => {
      e.isFirst && this._moveSelector(e);
    }), this.hammer.on("tap", (e) => {
      this._moveSelector(e);
    }), this.hammer.on("panstart", (e) => {
      this._moveSelector(e);
    }), this.hammer.on("panmove", (e) => {
      this._moveSelector(e);
    }), this.hammer.on("panend", (e) => {
      this._moveSelector(e);
    });
  }
  /**
   * generate the hue circle. This is relatively heavy (200ms) and is done only once on the first time it is shown.
   * @private
   */
  _generateHueCircle() {
    if (this.generated === !1) {
      const e = this.colorPickerCanvas.getContext("2d");
      this.pixelRation === void 0 && (this.pixelRatio = (window.devicePixelRatio || 1) / (e.webkitBackingStorePixelRatio || e.mozBackingStorePixelRatio || e.msBackingStorePixelRatio || e.oBackingStorePixelRatio || e.backingStorePixelRatio || 1)), e.setTransform(this.pixelRatio, 0, 0, this.pixelRatio, 0, 0);
      const t = this.colorPickerCanvas.clientWidth, r = this.colorPickerCanvas.clientHeight;
      e.clearRect(0, 0, t, r);
      let s, a, o, l;
      this.centerCoordinates = {
        x: t * 0.5,
        y: r * 0.5
      }, this.r = 0.49 * t;
      const c = 2 * Math.PI / 360, d = 1 / 360, f = 1 / this.r;
      let p;
      for (o = 0; o < 360; o++)
        for (l = 0; l < this.r; l++)
          s = this.centerCoordinates.x + l * Math.sin(c * o), a = this.centerCoordinates.y + l * Math.cos(c * o), p = zn(o * d, l * f, 1), e.fillStyle = "rgb(" + p.r + "," + p.g + "," + p.b + ")", e.fillRect(s - 0.5, a - 0.5, 2, 2);
      e.strokeStyle = "rgba(0,0,0,1)", e.circle(this.centerCoordinates.x, this.centerCoordinates.y, this.r), e.stroke(), this.hueCircle = e.getImageData(0, 0, t, r);
    }
    this.generated = !0;
  }
  /**
   * move the selector. This is called by hammer functions.
   * @param {Event}  event   The event
   * @private
   */
  _moveSelector(e) {
    const t = this.colorPickerDiv.getBoundingClientRect(), r = e.center.x - t.left, s = e.center.y - t.top, a = 0.5 * this.colorPickerDiv.clientHeight, o = 0.5 * this.colorPickerDiv.clientWidth, l = r - o, c = s - a, d = Math.atan2(l, c), f = 0.98 * Math.min(Math.sqrt(l * l + c * c), o), p = Math.cos(d) * f + a, m = Math.sin(d) * f + o;
    this.colorPickerSelector.style.top = p - 0.5 * this.colorPickerSelector.clientHeight + "px", this.colorPickerSelector.style.left = m - 0.5 * this.colorPickerSelector.clientWidth + "px";
    let g = d / (2 * Math.PI);
    g = g < 0 ? g + 1 : g;
    const b = f / this.r, _ = rn(this.color.r, this.color.g, this.color.b);
    _.h = g, _.s = b;
    const x = zn(_.h, _.s, _.v);
    x.a = this.color.a, this.color = x, this.initialColorDiv.style.backgroundColor = "rgba(" + this.initialColor.r + "," + this.initialColor.g + "," + this.initialColor.b + "," + this.initialColor.a + ")", this.newColorDiv.style.backgroundColor = "rgba(" + this.color.r + "," + this.color.g + "," + this.color.b + "," + this.color.a + ")";
  }
};
function Lp() {
  for (var n = arguments.length, e = new Array(n), t = 0; t < n; t++)
    e[t] = arguments[t];
  if (e.length < 1)
    throw new TypeError("Invalid arguments.");
  if (e.length === 1)
    return document.createTextNode(e[0]);
  {
    const r = document.createElement(e[0]);
    return r.appendChild(Lp(...Tn(e).call(e, 1))), r;
  }
}
let fY = class {
  /**
   * @param {object} parentModule        | the location where parentModule.setOptions() can be called
   * @param {object} defaultContainer    | the default container of the module
   * @param {object} configureOptions    | the fully configured and predefined options set found in allOptions.js
   * @param {number} pixelRatio          | canvas pixel ratio
   * @param {Function} hideOption        | custom logic to dynamically hide options
   */
  constructor(e, t, r) {
    let s = arguments.length > 3 && arguments[3] !== void 0 ? arguments[3] : 1, a = arguments.length > 4 && arguments[4] !== void 0 ? arguments[4] : () => !1;
    this.parent = e, this.changedOptions = [], this.container = t, this.allowCreation = !1, this.hideOption = a, this.options = {}, this.initialized = !1, this.popupCounter = 0, this.defaultOptions = {
      enabled: !1,
      filter: !0,
      container: void 0,
      showButton: !0
    }, Yr(this.options, this.defaultOptions), this.configureOptions = r, this.moduleOptions = {}, this.domElements = [], this.popupDiv = {}, this.popupLimit = 5, this.popupHistory = {}, this.colorPicker = new LO(s), this.wrapper = void 0;
  }
  /**
   * refresh all options.
   * Because all modules parse their options by themselves, we just use their options. We copy them here.
   * @param {object} options
   */
  setOptions(e) {
    if (e !== void 0) {
      this.popupHistory = {}, this._removePopup();
      let t = !0;
      if (typeof e == "string")
        this.options.filter = e;
      else if (ye(e))
        this.options.filter = e.join();
      else if (typeof e == "object") {
        if (e == null)
          throw new TypeError("options cannot be null");
        e.container !== void 0 && (this.options.container = e.container), ie(e) !== void 0 && (this.options.filter = ie(e)), e.showButton !== void 0 && (this.options.showButton = e.showButton), e.enabled !== void 0 && (t = e.enabled);
      } else typeof e == "boolean" ? (this.options.filter = !0, t = e) : typeof e == "function" && (this.options.filter = e, t = !0);
      ie(this.options) === !1 && (t = !1), this.options.enabled = t;
    }
    this._clean();
  }
  /**
   *
   * @param {object} moduleOptions
   */
  setModuleOptions(e) {
    this.moduleOptions = e, this.options.enabled === !0 && (this._clean(), this.options.container !== void 0 && (this.container = this.options.container), this._create());
  }
  /**
   * Create all DOM elements
   * @private
   */
  _create() {
    this._clean(), this.changedOptions = [];
    const e = ie(this.options);
    let t = 0, r = !1;
    for (const s in this.configureOptions)
      Object.prototype.hasOwnProperty.call(this.configureOptions, s) && (this.allowCreation = !1, r = !1, typeof e == "function" ? (r = e(s, []), r = r || this._handleObject(this.configureOptions[s], [s], !0)) : (e === !0 || Fe(e).call(e, s) !== -1) && (r = !0), r !== !1 && (this.allowCreation = !0, t > 0 && this._makeItem([]), this._makeHeader(s), this._handleObject(this.configureOptions[s], [s])), t++);
    this._makeButton(), this._push();
  }
  /**
   * draw all DOM elements on the screen
   * @private
   */
  _push() {
    this.wrapper = document.createElement("div"), this.wrapper.className = "vis-configuration-wrapper", this.container.appendChild(this.wrapper);
    for (let e = 0; e < this.domElements.length; e++)
      this.wrapper.appendChild(this.domElements[e]);
    this._showPopupIfNeeded();
  }
  /**
   * delete all DOM elements
   * @private
   */
  _clean() {
    for (let e = 0; e < this.domElements.length; e++)
      this.wrapper.removeChild(this.domElements[e]);
    this.wrapper !== void 0 && (this.container.removeChild(this.wrapper), this.wrapper = void 0), this.domElements = [], this._removePopup();
  }
  /**
   * get the value from the actualOptions if it exists
   * @param {Array} path    | where to look for the actual option
   * @returns {*}
   * @private
   */
  _getValue(e) {
    let t = this.moduleOptions;
    for (let r = 0; r < e.length; r++)
      if (t[e[r]] !== void 0)
        t = t[e[r]];
      else {
        t = void 0;
        break;
      }
    return t;
  }
  /**
   * all option elements are wrapped in an item
   * @param {Array} path    | where to look for the actual option
   * @param {Array.<Element>} domElements
   * @returns {number}
   * @private
   */
  _makeItem(e) {
    if (this.allowCreation === !0) {
      const a = document.createElement("div");
      a.className = "vis-configuration vis-config-item vis-config-s" + e.length;
      for (var t = arguments.length, r = new Array(t > 1 ? t - 1 : 0), s = 1; s < t; s++)
        r[s - 1] = arguments[s];
      return B(r).call(r, (o) => {
        a.appendChild(o);
      }), this.domElements.push(a), this.domElements.length;
    }
    return 0;
  }
  /**
   * header for major subjects
   * @param {string} name
   * @private
   */
  _makeHeader(e) {
    const t = document.createElement("div");
    t.className = "vis-configuration vis-config-header", t.innerText = e, this._makeItem([], t);
  }
  /**
   * make a label, if it is an object label, it gets different styling.
   * @param {string} name
   * @param {Array} path    | where to look for the actual option
   * @param {string} objectLabel
   * @returns {HTMLElement}
   * @private
   */
  _makeLabel(e, t) {
    let r = arguments.length > 2 && arguments[2] !== void 0 ? arguments[2] : !1;
    const s = document.createElement("div");
    if (s.className = "vis-configuration vis-config-label vis-config-s" + t.length, r === !0) {
      for (; s.firstChild; )
        s.removeChild(s.firstChild);
      s.appendChild(Lp("i", "b", e));
    } else
      s.innerText = e + ":";
    return s;
  }
  /**
   * make a dropdown list for multiple possible string optoins
   * @param {Array.<number>} arr
   * @param {number} value
   * @param {Array} path    | where to look for the actual option
   * @private
   */
  _makeDropdown(e, t, r) {
    const s = document.createElement("select");
    s.className = "vis-configuration vis-config-select";
    let a = 0;
    t !== void 0 && Fe(e).call(e, t) !== -1 && (a = Fe(e).call(e, t));
    for (let c = 0; c < e.length; c++) {
      const d = document.createElement("option");
      d.value = e[c], c === a && (d.selected = "selected"), d.innerText = e[c], s.appendChild(d);
    }
    const o = this;
    s.onchange = function() {
      o._update(this.value, r);
    };
    const l = this._makeLabel(r[r.length - 1], r);
    this._makeItem(r, l, s);
  }
  /**
   * make a range object for numeric options
   * @param {Array.<number>} arr
   * @param {number} value
   * @param {Array} path    | where to look for the actual option
   * @private
   */
  _makeRange(e, t, r) {
    const s = e[0], a = e[1], o = e[2], l = e[3], c = document.createElement("input");
    c.className = "vis-configuration vis-config-range";
    try {
      c.type = "range", c.min = a, c.max = o;
    } catch {
    }
    c.step = l;
    let d = "", f = 0;
    t !== void 0 ? (t < 0 && t * 1.2 < a ? (c.min = Math.ceil(t * 1.2), f = c.min, d = "range increased") : t / 1.2 < a && (c.min = Math.ceil(t / 1.2), f = c.min, d = "range increased"), t * 1.2 > o && o !== 1 && (c.max = Math.ceil(t * 1.2), f = c.max, d = "range increased"), c.value = t) : c.value = s;
    const p = document.createElement("input");
    p.className = "vis-configuration vis-config-rangeinput", p.value = c.value;
    const m = this;
    c.onchange = function() {
      p.value = this.value, m._update(Number(this.value), r);
    }, c.oninput = function() {
      p.value = this.value;
    };
    const g = this._makeLabel(r[r.length - 1], r), b = this._makeItem(r, g, c, p);
    d !== "" && this.popupHistory[b] !== f && (this.popupHistory[b] = f, this._setupPopup(d, b));
  }
  /**
   * make a button object
   * @private
   */
  _makeButton() {
    if (this.options.showButton === !0) {
      const e = document.createElement("div");
      e.className = "vis-configuration vis-config-button", e.innerText = "generate options", e.onclick = () => {
        this._printOptions();
      }, e.onmouseover = () => {
        e.className = "vis-configuration vis-config-button hover";
      }, e.onmouseout = () => {
        e.className = "vis-configuration vis-config-button";
      }, this.optionsContainer = document.createElement("div"), this.optionsContainer.className = "vis-configuration vis-config-option-container", this.domElements.push(this.optionsContainer), this.domElements.push(e);
    }
  }
  /**
   * prepare the popup
   * @param {string} string
   * @param {number} index
   * @private
   */
  _setupPopup(e, t) {
    if (this.initialized === !0 && this.allowCreation === !0 && this.popupCounter < this.popupLimit) {
      const r = document.createElement("div");
      r.id = "vis-configuration-popup", r.className = "vis-configuration-popup", r.innerText = e, r.onclick = () => {
        this._removePopup();
      }, this.popupCounter += 1, this.popupDiv = {
        html: r,
        index: t
      };
    }
  }
  /**
   * remove the popup from the dom
   * @private
   */
  _removePopup() {
    this.popupDiv.html !== void 0 && (this.popupDiv.html.parentNode.removeChild(this.popupDiv.html), clearTimeout(this.popupDiv.hideTimeout), clearTimeout(this.popupDiv.deleteTimeout), this.popupDiv = {});
  }
  /**
   * Show the popup if it is needed.
   * @private
   */
  _showPopupIfNeeded() {
    if (this.popupDiv.html !== void 0) {
      const t = this.domElements[this.popupDiv.index].getBoundingClientRect();
      this.popupDiv.html.style.left = t.left + "px", this.popupDiv.html.style.top = t.top - 30 + "px", document.body.appendChild(this.popupDiv.html), this.popupDiv.hideTimeout = Dt(() => {
        this.popupDiv.html.style.opacity = 0;
      }, 1500), this.popupDiv.deleteTimeout = Dt(() => {
        this._removePopup();
      }, 1800);
    }
  }
  /**
   * make a checkbox for boolean options.
   * @param {number} defaultValue
   * @param {number} value
   * @param {Array} path    | where to look for the actual option
   * @private
   */
  _makeCheckbox(e, t, r) {
    const s = document.createElement("input");
    s.type = "checkbox", s.className = "vis-configuration vis-config-checkbox", s.checked = e, t !== void 0 && (s.checked = t, t !== e && (typeof e == "object" ? t !== e.enabled && this.changedOptions.push({
      path: r,
      value: t
    }) : this.changedOptions.push({
      path: r,
      value: t
    })));
    const a = this;
    s.onchange = function() {
      a._update(this.checked, r);
    };
    const o = this._makeLabel(r[r.length - 1], r);
    this._makeItem(r, o, s);
  }
  /**
   * make a text input field for string options.
   * @param {number} defaultValue
   * @param {number} value
   * @param {Array} path    | where to look for the actual option
   * @private
   */
  _makeTextInput(e, t, r) {
    const s = document.createElement("input");
    s.type = "text", s.className = "vis-configuration vis-config-text", s.value = t, t !== e && this.changedOptions.push({
      path: r,
      value: t
    });
    const a = this;
    s.onchange = function() {
      a._update(this.value, r);
    };
    const o = this._makeLabel(r[r.length - 1], r);
    this._makeItem(r, o, s);
  }
  /**
   * make a color field with a color picker for color fields
   * @param {Array.<number>} arr
   * @param {number} value
   * @param {Array} path    | where to look for the actual option
   * @private
   */
  _makeColorField(e, t, r) {
    const s = e[1], a = document.createElement("div");
    t = t === void 0 ? s : t, t !== "none" ? (a.className = "vis-configuration vis-config-colorBlock", a.style.backgroundColor = t) : a.className = "vis-configuration vis-config-colorBlock none", t = t === void 0 ? s : t, a.onclick = () => {
      this._showColorPicker(t, a, r);
    };
    const o = this._makeLabel(r[r.length - 1], r);
    this._makeItem(r, o, a);
  }
  /**
   * used by the color buttons to call the color picker.
   * @param {number} value
   * @param {HTMLElement} div
   * @param {Array} path    | where to look for the actual option
   * @private
   */
  _showColorPicker(e, t, r) {
    t.onclick = function() {
    }, this.colorPicker.insertTo(t), this.colorPicker.show(), this.colorPicker.setColor(e), this.colorPicker.setUpdateCallback((s) => {
      const a = "rgba(" + s.r + "," + s.g + "," + s.b + "," + s.a + ")";
      t.style.backgroundColor = a, this._update(a, r);
    }), this.colorPicker.setCloseCallback(() => {
      t.onclick = () => {
        this._showColorPicker(e, t, r);
      };
    });
  }
  /**
   * parse an object and draw the correct items
   * @param {object} obj
   * @param {Array} [path]    | where to look for the actual option
   * @param {boolean} [checkOnly]
   * @returns {boolean}
   * @private
   */
  _handleObject(e) {
    let t = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : [], r = arguments.length > 2 && arguments[2] !== void 0 ? arguments[2] : !1, s = !1;
    const a = ie(this.options);
    let o = !1;
    for (const l in e)
      if (Object.prototype.hasOwnProperty.call(e, l)) {
        s = !0;
        const c = e[l], d = es(t, l);
        if (typeof a == "function" && (s = a(l, t), s === !1 && !ye(c) && typeof c != "string" && typeof c != "boolean" && c instanceof Object && (this.allowCreation = !1, s = this._handleObject(c, d, !0), this.allowCreation = r === !1)), s !== !1) {
          o = !0;
          const f = this._getValue(d);
          if (ye(c))
            this._handleArray(c, f, d);
          else if (typeof c == "string")
            this._makeTextInput(c, f, d);
          else if (typeof c == "boolean")
            this._makeCheckbox(c, f, d);
          else if (c instanceof Object) {
            if (!this.hideOption(t, l, this.moduleOptions))
              if (c.enabled !== void 0) {
                const p = es(d, "enabled"), m = this._getValue(p);
                if (m === !0) {
                  const g = this._makeLabel(l, d, !0);
                  this._makeItem(d, g), o = this._handleObject(c, d) || o;
                } else
                  this._makeCheckbox(c, m, d);
              } else {
                const p = this._makeLabel(l, d, !0);
                this._makeItem(d, p), o = this._handleObject(c, d) || o;
              }
          } else
            console.error("dont know how to handle", c, l, d);
        }
      }
    return o;
  }
  /**
   * handle the array type of option
   * @param {Array.<number>} arr
   * @param {number} value
   * @param {Array} path    | where to look for the actual option
   * @private
   */
  _handleArray(e, t, r) {
    typeof e[0] == "string" && e[0] === "color" ? (this._makeColorField(e, t, r), e[1] !== t && this.changedOptions.push({
      path: r,
      value: t
    })) : typeof e[0] == "string" ? (this._makeDropdown(e, t, r), e[0] !== t && this.changedOptions.push({
      path: r,
      value: t
    })) : typeof e[0] == "number" && (this._makeRange(e, t, r), e[0] !== t && this.changedOptions.push({
      path: r,
      value: Number(t)
    }));
  }
  /**
   * called to update the network with the new settings.
   * @param {number} value
   * @param {Array} path    | where to look for the actual option
   * @private
   */
  _update(e, t) {
    const r = this._constructOptions(e, t);
    this.parent.body && this.parent.body.emitter && this.parent.body.emitter.emit && this.parent.body.emitter.emit("configChange", r), this.initialized = !0, this.parent.setOptions(r);
  }
  /**
   *
   * @param {string | boolean} value
   * @param {Array.<string>} path
   * @param {{}} optionsObj
   * @returns {{}}
   * @private
   */
  _constructOptions(e, t) {
    let r = arguments.length > 2 && arguments[2] !== void 0 ? arguments[2] : {}, s = r;
    e = e === "true" ? !0 : e, e = e === "false" ? !1 : e;
    for (let a = 0; a < t.length; a++)
      t[a] !== "global" && (s[t[a]] === void 0 && (s[t[a]] = {}), a !== t.length - 1 ? s = s[t[a]] : s[t[a]] = e);
    return r;
  }
  /**
   * @private
   */
  _printOptions() {
    const e = this.getOptions();
    for (; this.optionsContainer.firstChild; )
      this.optionsContainer.removeChild(this.optionsContainer.firstChild);
    this.optionsContainer.appendChild(Lp("pre", "const options = " + Yt(e, null, 2)));
  }
  /**
   *
   * @returns {{}} options
   */
  getOptions() {
    const e = {};
    for (let t = 0; t < this.changedOptions.length; t++)
      this._constructOptions(this.changedOptions[t].value, this.changedOptions[t].path, e);
    return e;
  }
}, pY = class {
  /**
   * @param {Element} container       The container object.
   * @param {string}  overflowMethod  How the popup should act to overflowing ('flip' or 'cap')
   */
  constructor(e, t) {
    this.container = e, this.overflowMethod = t || "cap", this.x = 0, this.y = 0, this.padding = 5, this.hidden = !1, this.frame = document.createElement("div"), this.frame.className = "vis-tooltip", this.container.appendChild(this.frame);
  }
  /**
   * @param {number} x   Horizontal position of the popup window
   * @param {number} y   Vertical position of the popup window
   */
  setPosition(e, t) {
    this.x = Vt(e), this.y = Vt(t);
  }
  /**
   * Set the content for the popup window. This can be HTML code or text.
   * @param {string | Element} content
   */
  setText(e) {
    if (e instanceof Element) {
      for (; this.frame.firstChild; )
        this.frame.removeChild(this.frame.firstChild);
      this.frame.appendChild(e);
    } else
      this.frame.innerText = e;
  }
  /**
   * Show the popup window
   * @param {boolean} [doShow]    Show or hide the window
   */
  show(e) {
    if (e === void 0 && (e = !0), e === !0) {
      const t = this.frame.clientHeight, r = this.frame.clientWidth, s = this.frame.parentNode.clientHeight, a = this.frame.parentNode.clientWidth;
      let o = 0, l = 0;
      if (this.overflowMethod == "flip") {
        let c = !1, d = !0;
        this.y - t < this.padding && (d = !1), this.x + r > a - this.padding && (c = !0), c ? o = this.x - r : o = this.x, d ? l = this.y - t : l = this.y;
      } else
        l = this.y - t, l + t + this.padding > s && (l = s - t - this.padding), l < this.padding && (l = this.padding), o = this.x, o + r + this.padding > a && (o = a - r - this.padding), o < this.padding && (o = this.padding);
      this.frame.style.left = o + "px", this.frame.style.top = l + "px", this.frame.style.visibility = "visible", this.hidden = !1;
    } else
      this.hide();
  }
  /**
   * Hide the popup window
   */
  hide() {
    this.hidden = !0, this.frame.style.left = "0", this.frame.style.top = "0", this.frame.style.visibility = "hidden";
  }
  /**
   * Remove the popup window
   */
  destroy() {
    this.frame.parentNode.removeChild(this.frame);
  }
}, qn = !1, cE;
const Fp = "background: #FFeeee; color: #dd0000";
let mY = class Xe {
  /**
   * Main function to be called
   * @param {object} options
   * @param {object} referenceOptions
   * @param {object} subObject
   * @returns {boolean}
   * @static
   */
  static validate(e, t, r) {
    qn = !1, cE = t;
    let s = t;
    return r !== void 0 && (s = t[r]), Xe.parse(e, s, []), qn;
  }
  /**
   * Will traverse an object recursively and check every value
   * @param {object} options
   * @param {object} referenceOptions
   * @param {Array} path    | where to look for the actual option
   * @static
   */
  static parse(e, t, r) {
    for (const s in e)
      Object.prototype.hasOwnProperty.call(e, s) && Xe.check(s, e, t, r);
  }
  /**
   * Check every value. If the value is an object, call the parse function on that object.
   * @param {string} option
   * @param {object} options
   * @param {object} referenceOptions
   * @param {Array} path    | where to look for the actual option
   * @static
   */
  static check(e, t, r, s) {
    if (r[e] === void 0 && r.__any__ === void 0) {
      Xe.getSuggestion(e, r, s);
      return;
    }
    let a = e, o = !0;
    r[e] === void 0 && r.__any__ !== void 0 && (a = "__any__", o = Xe.getType(t[e]) === "object");
    let l = r[a];
    o && l.__type__ !== void 0 && (l = l.__type__), Xe.checkFields(e, t, r, a, l, s);
  }
  /**
   *
   * @param {string}  option           | the option property
   * @param {object}  options          | The supplied options object
   * @param {object}  referenceOptions | The reference options containing all options and their allowed formats
   * @param {string}  referenceOption  | Usually this is the same as option, except when handling an __any__ tag.
   * @param {string}  refOptionObj     | This is the type object from the reference options
   * @param {Array}   path             | where in the object is the option
   * @static
   */
  static checkFields(e, t, r, s, a, o) {
    const l = function(f) {
      console.error("%c" + f + Xe.printLocation(o, e), Fp);
    }, c = Xe.getType(t[e]), d = a[c];
    d !== void 0 ? Xe.getType(d) === "array" && Fe(d).call(d, t[e]) === -1 ? (l('Invalid option detected in "' + e + '". Allowed values are:' + Xe.print(d) + ' not "' + t[e] + '". '), qn = !0) : c === "object" && s !== "__any__" && (o = es(o, e), Xe.parse(t[e], r[s], o)) : a.any === void 0 && (l('Invalid type received for "' + e + '". Expected: ' + Xe.print(ct(a)) + ". Received [" + c + '] "' + t[e] + '"'), qn = !0);
  }
  /**
   *
   * @param {object | boolean | number | string | Array.<number> | Date | Node | Moment | undefined | null} object
   * @returns {string}
   * @static
   */
  static getType(e) {
    const t = typeof e;
    return t === "object" ? e === null ? "null" : e instanceof Boolean ? "boolean" : e instanceof Number ? "number" : e instanceof String ? "string" : ye(e) ? "array" : e instanceof Date ? "date" : e.nodeType !== void 0 ? "dom" : e._isAMomentObject === !0 ? "moment" : "object" : t === "number" ? "number" : t === "boolean" ? "boolean" : t === "string" ? "string" : t === void 0 ? "undefined" : t;
  }
  /**
   * @param {string} option
   * @param {object} options
   * @param {Array.<string>} path
   * @static
   */
  static getSuggestion(e, t, r) {
    const s = Xe.findInOptions(e, t, r, !1), a = Xe.findInOptions(e, cE, [], !0), o = 8, l = 4;
    let c;
    s.indexMatch !== void 0 ? c = " in " + Xe.printLocation(s.path, e, "") + 'Perhaps it was incomplete? Did you mean: "' + s.indexMatch + `"?

` : a.distance <= l && s.distance > a.distance ? c = " in " + Xe.printLocation(s.path, e, "") + "Perhaps it was misplaced? Matching option found at: " + Xe.printLocation(a.path, a.closestMatch, "") : s.distance <= o ? c = '. Did you mean "' + s.closestMatch + '"?' + Xe.printLocation(s.path, e) : c = ". Did you mean one of these: " + Xe.print(ct(t)) + Xe.printLocation(r, e), console.error('%cUnknown option detected: "' + e + '"' + c, Fp), qn = !0;
  }
  /**
   * traverse the options in search for a match.
   * @param {string} option
   * @param {object} options
   * @param {Array} path    | where to look for the actual option
   * @param {boolean} [recursive]
   * @returns {{closestMatch: string, path: Array, distance: number}}
   * @static
   */
  static findInOptions(e, t, r) {
    let s = arguments.length > 3 && arguments[3] !== void 0 ? arguments[3] : !1, a = 1e9, o = "", l = [];
    const c = e.toLowerCase();
    let d;
    for (const p in t) {
      let m;
      if (t[p].__type__ !== void 0 && s === !0) {
        const g = Xe.findInOptions(e, t[p], es(r, p));
        a > g.distance && (o = g.closestMatch, l = g.path, a = g.distance, d = g.indexMatch);
      } else {
        var f;
        Fe(f = p.toLowerCase()).call(f, c) !== -1 && (d = p), m = Xe.levenshteinDistance(e, p), a > m && (o = p, l = MO(r), a = m);
      }
    }
    return {
      closestMatch: o,
      path: l,
      distance: a,
      indexMatch: d
    };
  }
  /**
   * @param {Array.<string>} path
   * @param {object} option
   * @param {string} prefix
   * @returns {string}
   * @static
   */
  static printLocation(e, t) {
    let s = `

` + (arguments.length > 2 && arguments[2] !== void 0 ? arguments[2] : `Problem value found at: 
`) + `options = {
`;
    for (let a = 0; a < e.length; a++) {
      for (let o = 0; o < a + 1; o++)
        s += "  ";
      s += e[a] + `: {
`;
    }
    for (let a = 0; a < e.length + 1; a++)
      s += "  ";
    s += t + `
`;
    for (let a = 0; a < e.length + 1; a++) {
      for (let o = 0; o < e.length - a; o++)
        s += "  ";
      s += `}
`;
    }
    return s + `

`;
  }
  /**
   * @param {object} options
   * @returns {string}
   * @static
   */
  static print(e) {
    return Yt(e).replace(/(")|(\[)|(\])|(,"__type__")/g, "").replace(/(,)/g, ", ");
  }
  /**
   *  Compute the edit distance between the two given strings
   *  http://en.wikibooks.org/wiki/Algorithm_Implementation/Strings/Levenshtein_distance#JavaScript
   *
   *  Copyright (c) 2011 Andrei Mackenzie
   *
   *  Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
   *
   *  The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
   *
   *  THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
   * @param {string} a
   * @param {string} b
   * @returns {Array.<Array.<number>>}}
   * @static
   */
  static levenshteinDistance(e, t) {
    if (e.length === 0) return t.length;
    if (t.length === 0) return e.length;
    const r = [];
    let s;
    for (s = 0; s <= t.length; s++)
      r[s] = [s];
    let a;
    for (a = 0; a <= e.length; a++)
      r[0][a] = a;
    for (s = 1; s <= t.length; s++)
      for (a = 1; a <= e.length; a++)
        t.charAt(s - 1) == e.charAt(a - 1) ? r[s][a] = r[s - 1][a - 1] : r[s][a] = Math.min(
          r[s - 1][a - 1] + 1,
          // substitution
          Math.min(
            r[s][a - 1] + 1,
            // insertion
            r[s - 1][a] + 1
          )
        );
    return r[t.length][e.length];
  }
};
const vY = br, gY = LO, yY = fY, bY = Mm, _Y = pY, wY = Fp, xY = mY;
var FO = /* @__PURE__ */ Object.freeze({
  __proto__: null,
  Activator: vY,
  Alea: Tj,
  ColorPicker: gY,
  Configurator: yY,
  DELETE: Pm,
  HSVToHex: qp,
  HSVToRGB: zn,
  Hammer: bY,
  Popup: _Y,
  RGBToHSV: rn,
  RGBToHex: Am,
  VALIDATOR_PRINT_STYLE: wY,
  Validator: xY,
  addClassName: Bj,
  addCssText: rY,
  binarySearchCustom: oY,
  binarySearchValue: lY,
  bridgeObject: Lm,
  copyAndExtendArray: es,
  copyArray: MO,
  deepExtend: Jn,
  deepObjectAssign: Im,
  easingFunctions: uY,
  equalArray: jj,
  extend: qj,
  fillIfDefined: IO,
  forEach: zj,
  getAbsoluteLeft: Yj,
  getAbsoluteRight: Hj,
  getAbsoluteTop: Gj,
  getScrollBarWidth: cY,
  getTarget: Zj,
  getType: Hn,
  hasParent: Qj,
  hexToHSV: NO,
  hexToRGB: Fa,
  insertSort: sY,
  isDate: Nj,
  isNumber: Wn,
  isObject: Np,
  isString: Kt,
  isValidHex: Nm,
  isValidRGB: qm,
  isValidRGBA: qO,
  mergeOptions: aY,
  option: Jj,
  overrideOpacity: eY,
  parseColor: tY,
  preventDefault: Xj,
  pureDeepObjectAssign: kO,
  recursiveDOMDelete: PO,
  removeClassName: Wj,
  removeCssText: iY,
  selectiveBridgeObject: nY,
  selectiveDeepExtend: Fj,
  selectiveExtend: Lj,
  selectiveNotDeepExtend: $j,
  throttle: Kj,
  toArray: Uj,
  topMost: dY,
  updateProperty: Vj
});
const mt = [];
for (let n = 0; n < 256; ++n)
  mt.push((n + 256).toString(16).slice(1));
function SY(n, e = 0) {
  return (mt[n[e + 0]] + mt[n[e + 1]] + mt[n[e + 2]] + mt[n[e + 3]] + "-" + mt[n[e + 4]] + mt[n[e + 5]] + "-" + mt[n[e + 6]] + mt[n[e + 7]] + "-" + mt[n[e + 8]] + mt[n[e + 9]] + "-" + mt[n[e + 10]] + mt[n[e + 11]] + mt[n[e + 12]] + mt[n[e + 13]] + mt[n[e + 14]] + mt[n[e + 15]]).toLowerCase();
}
const TY = new Uint8Array(16);
function EY() {
  return crypto.getRandomValues(TY);
}
function wi(n, e, t) {
  return crypto.randomUUID ? crypto.randomUUID() : DY(n);
}
function DY(n, e, t) {
  var s;
  n = n || {};
  const r = n.random ?? ((s = n.rng) == null ? void 0 : s.call(n)) ?? EY();
  if (r.length < 16)
    throw new Error("Random bytes length must be >= 16");
  return r[6] = r[6] & 15 | 64, r[8] = r[8] & 63 | 128, SY(r);
}
function dE(n, e) {
  var t = ct(n);
  if (la) {
    var r = la(n);
    e && (r = ie(r).call(r, function(s) {
      return Ra(n, s).enumerable;
    })), t.push.apply(t, r);
  }
  return t;
}
function hE(n) {
  for (var e = 1; e < arguments.length; e++) {
    var t, r, s = arguments[e] != null ? arguments[e] : {};
    e % 2 ? B(t = dE(Object(s), !0)).call(t, function(a) {
      Le(n, a, s[a]);
    }) : ua ? OD(n, ua(s)) : B(r = dE(Object(s))).call(r, function(a) {
      Na(n, a, Ra(s, a));
    });
  }
  return n;
}
function OY(n) {
  return new CY(n);
}
class kY {
  /**
   * Create a new data pipe.
   * @param source - The data set or data view that will be observed.
   * @param transformers - An array of transforming functions to be used to
   * filter or transform the items in the pipe.
   * @param target - The data set or data view that will receive the items.
   */
  constructor(e, t, r) {
    var s, a, o;
    Le(this, "_source", void 0), Le(this, "_transformers", void 0), Le(this, "_target", void 0), Le(this, "_listeners", {
      add: N(s = this._add).call(s, this),
      remove: N(a = this._remove).call(a, this),
      update: N(o = this._update).call(o, this)
    }), this._source = e, this._transformers = t, this._target = r;
  }
  /** @inheritDoc */
  all() {
    return this._target.update(this._transformItems(this._source.get())), this;
  }
  /** @inheritDoc */
  start() {
    return this._source.on("add", this._listeners.add), this._source.on("remove", this._listeners.remove), this._source.on("update", this._listeners.update), this;
  }
  /** @inheritDoc */
  stop() {
    return this._source.off("add", this._listeners.add), this._source.off("remove", this._listeners.remove), this._source.off("update", this._listeners.update), this;
  }
  /**
   * Apply the transformers to the items.
   * @param items - The items to be transformed.
   * @returns The transformed items.
   */
  _transformItems(e) {
    var t;
    return Aa(t = this._transformers).call(t, (r, s) => s(r), e);
  }
  /**
   * Handle an add event.
   * @param _name - Ignored.
   * @param payload - The payload containing the ids of the added items.
   */
  _add(e, t) {
    t != null && this._target.add(this._transformItems(this._source.get(t.items)));
  }
  /**
   * Handle an update event.
   * @param _name - Ignored.
   * @param payload - The payload containing the ids of the updated items.
   */
  _update(e, t) {
    t != null && this._target.update(this._transformItems(this._source.get(t.items)));
  }
  /**
   * Handle a remove event.
   * @param _name - Ignored.
   * @param payload - The payload containing the data of the removed items.
   */
  _remove(e, t) {
    t != null && this._target.remove(this._transformItems(t.oldData));
  }
}
class CY {
  /**
   * Create a new data pipe factory. This is an internal constructor that
   * should never be called from outside of this file.
   * @param source - The source data set or data view for this pipe.
   */
  constructor(e) {
    Le(this, "_source", void 0), Le(this, "_transformers", []), this._source = e;
  }
  /**
   * Filter the items.
   * @param callback - A filtering function that returns true if given item
   * should be piped and false if not.
   * @returns This factory for further configuration.
   */
  filter(e) {
    return this._transformers.push((t) => ie(t).call(t, e)), this;
  }
  /**
   * Map each source item to a new type.
   * @param callback - A mapping function that takes a source item and returns
   * corresponding mapped item.
   * @typeParam TI - Target item type.
   * @typeParam TP - Target item type's id property name.
   * @returns This factory for further configuration.
   */
  map(e) {
    return this._transformers.push((t) => Ae(t).call(t, e)), this;
  }
  /**
   * Map each source item to zero or more items of a new type.
   * @param callback - A mapping function that takes a source item and returns
   * an array of corresponding mapped items.
   * @typeParam TI - Target item type.
   * @typeParam TP - Target item type's id property name.
   * @returns This factory for further configuration.
   */
  flatMap(e) {
    return this._transformers.push((t) => iL(t).call(t, e)), this;
  }
  /**
   * Connect this pipe to given data set.
   * @param target - The data set that will receive the items from this pipe.
   * @returns The pipe connected between given data sets and performing
   * configured transformation on the processed items.
   */
  to(e) {
    return new kY(this._source, this._transformers, e);
  }
}
function $p(n) {
  return typeof n == "string" || typeof n == "number";
}
class $a {
  /**
   * Construct a new Queue.
   * @param options - Queue configuration.
   */
  constructor(e) {
    Le(this, "delay", void 0), Le(this, "max", void 0), Le(this, "_queue", []), Le(this, "_timeout", null), Le(this, "_extended", null), this.delay = null, this.max = 1 / 0, this.setOptions(e);
  }
  /**
   * Update the configuration of the queue.
   * @param options - Queue configuration.
   */
  setOptions(e) {
    e && typeof e.delay < "u" && (this.delay = e.delay), e && typeof e.max < "u" && (this.max = e.max), this._flushIfNeeded();
  }
  /**
   * Extend an object with queuing functionality.
   * The object will be extended with a function flush, and the methods provided in options.replace will be replaced with queued ones.
   * @param object - The object to be extended.
   * @param options - Additional options.
   * @returns The created queue.
   */
  static extend(e, t) {
    const r = new $a(t);
    if (e.flush !== void 0)
      throw new Error("Target object already has a property flush");
    e.flush = () => {
      r.flush();
    };
    const s = [{
      name: "flush",
      original: void 0
    }];
    if (t && t.replace)
      for (let a = 0; a < t.replace.length; a++) {
        const o = t.replace[a];
        s.push({
          name: o,
          // @TODO: better solution?
          original: e[o]
        }), r.replace(e, o);
      }
    return r._extended = {
      object: e,
      methods: s
    }, r;
  }
  /**
   * Destroy the queue. The queue will first flush all queued actions, and in case it has extended an object, will restore the original object.
   */
  destroy() {
    if (this.flush(), this._extended) {
      const e = this._extended.object, t = this._extended.methods;
      for (let r = 0; r < t.length; r++) {
        const s = t[r];
        s.original ? e[s.name] = s.original : delete e[s.name];
      }
      this._extended = null;
    }
  }
  /**
   * Replace a method on an object with a queued version.
   * @param object - Object having the method.
   * @param method - The method name.
   */
  replace(e, t) {
    const r = this, s = e[t];
    if (!s)
      throw new Error("Method " + t + " undefined");
    e[t] = function() {
      for (var a = arguments.length, o = new Array(a), l = 0; l < a; l++)
        o[l] = arguments[l];
      r.queue({
        args: o,
        fn: s,
        context: this
      });
    };
  }
  /**
   * Queue a call.
   * @param entry - The function or entry to be queued.
   */
  queue(e) {
    typeof e == "function" ? this._queue.push({
      fn: e
    }) : this._queue.push(e), this._flushIfNeeded();
  }
  /**
   * Check whether the queue needs to be flushed.
   */
  _flushIfNeeded() {
    this._queue.length > this.max && this.flush(), this._timeout != null && (clearTimeout(this._timeout), this._timeout = null), this.queue.length > 0 && typeof this.delay == "number" && (this._timeout = Dt(() => {
      this.flush();
    }, this.delay));
  }
  /**
   * Flush all queued calls
   */
  flush() {
    var e, t;
    B(e = Mt(t = this._queue).call(t, 0)).call(e, (r) => {
      r.fn.apply(r.context || r.fn, r.args || []);
    });
  }
}
class ts {
  constructor() {
    Le(this, "_subscribers", {
      "*": [],
      add: [],
      remove: [],
      update: []
    }), Le(this, "subscribe", ts.prototype.on), Le(this, "unsubscribe", ts.prototype.off);
  }
  /**
   * Trigger an event
   * @param event - Event name.
   * @param payload - Event payload.
   * @param senderId - Id of the sender.
   */
  _trigger(e, t, r) {
    var s;
    if (e === "*")
      throw new Error("Cannot trigger event *");
    B(s = [...this._subscribers[e], ...this._subscribers["*"]]).call(s, (a) => {
      a(e, t, r ?? null);
    });
  }
  /**
   * Subscribe to an event, add an event listener.
   * @remarks Non-function callbacks are ignored.
   * @param event - Event name.
   * @param callback - Callback method.
   */
  on(e, t) {
    typeof t == "function" && this._subscribers[e].push(t);
  }
  /**
   * Unsubscribe from an event, remove an event listener.
   * @remarks If the same callback was subscribed more than once **all** occurences will be removed.
   * @param event - Event name.
   * @param callback - Callback method.
   */
  off(e, t) {
    var r;
    this._subscribers[e] = ie(r = this._subscribers[e]).call(r, (s) => s !== t);
  }
}
class yi {
  /**
   * Create a new data stream.
   * @param pairs - The id, item pairs.
   */
  constructor(e) {
    Le(this, "_pairs", void 0), this._pairs = e;
  }
  /**
   * Return an iterable of key, value pairs for every entry in the stream.
   */
  *[vi]() {
    for (const t of this._pairs) {
      var e = St(t, 2);
      const r = e[0], s = e[1];
      yield [r, s];
    }
  }
  /**
   * Return an iterable of key, value pairs for every entry in the stream.
   */
  *entries() {
    for (const t of this._pairs) {
      var e = St(t, 2);
      const r = e[0], s = e[1];
      yield [r, s];
    }
  }
  /**
   * Return an iterable of keys in the stream.
   */
  *keys() {
    for (const t of this._pairs) {
      var e = St(t, 1);
      yield e[0];
    }
  }
  /**
   * Return an iterable of values in the stream.
   */
  *values() {
    for (const t of this._pairs) {
      var e = St(t, 2);
      yield e[1];
    }
  }
  /**
   * Return an array containing all the ids in this stream.
   * @remarks
   * The array may contain duplicities.
   * @returns The array with all ids from this stream.
   */
  toIdArray() {
    var e;
    return Ae(e = [...this._pairs]).call(e, (t) => t[0]);
  }
  /**
   * Return an array containing all the items in this stream.
   * @remarks
   * The array may contain duplicities.
   * @returns The array with all items from this stream.
   */
  toItemArray() {
    var e;
    return Ae(e = [...this._pairs]).call(e, (t) => t[1]);
  }
  /**
   * Return an array containing all the entries in this stream.
   * @remarks
   * The array may contain duplicities.
   * @returns The array with all entries from this stream.
   */
  toEntryArray() {
    return [...this._pairs];
  }
  /**
   * Return an object map containing all the items in this stream accessible by ids.
   * @remarks
   * In case of duplicate ids (coerced to string so `7 == '7'`) the last encoutered appears in the returned object.
   * @returns The object map of all id → item pairs from this stream.
   */
  toObjectMap() {
    const e = _s(null);
    for (const r of this._pairs) {
      var t = St(r, 2);
      const s = t[0], a = t[1];
      e[s] = a;
    }
    return e;
  }
  /**
   * Return a map containing all the items in this stream accessible by ids.
   * @returns The map of all id → item pairs from this stream.
   */
  toMap() {
    return new QD(this._pairs);
  }
  /**
   * Return a set containing all the (unique) ids in this stream.
   * @returns The set of all ids from this stream.
   */
  toIdSet() {
    return new Jr(this.toIdArray());
  }
  /**
   * Return a set containing all the (unique) items in this stream.
   * @returns The set of all items from this stream.
   */
  toItemSet() {
    return new Jr(this.toItemArray());
  }
  /**
   * Cache the items from this stream.
   * @remarks
   * This method allows for items to be fetched immediatelly and used (possibly multiple times) later.
   * It can also be used to optimize performance as {@link DataStream} would otherwise reevaluate everything upon each iteration.
   *
   * ## Example
   * ```javascript
   * const ds = new DataSet([…])
   *
   * const cachedStream = ds.stream()
   *   .filter(…)
   *   .sort(…)
   *   .map(…)
   *   .cached(…) // Data are fetched, processed and cached here.
   *
   * ds.clear()
   * chachedStream // Still has all the items.
   * ```
   * @returns A new {@link DataStream} with cached items (detached from the original {@link DataSet}).
   */
  cache() {
    return new yi([...this._pairs]);
  }
  /**
   * Get the distinct values of given property.
   * @param callback - The function that picks and possibly converts the property.
   * @typeParam T - The type of the distinct value.
   * @returns A set of all distinct properties.
   */
  distinct(e) {
    const t = new Jr();
    for (const s of this._pairs) {
      var r = St(s, 2);
      const a = r[0], o = r[1];
      t.add(e(o, a));
    }
    return t;
  }
  /**
   * Filter the items of the stream.
   * @param callback - The function that decides whether an item will be included.
   * @returns A new data stream with the filtered items.
   */
  filter(e) {
    const t = this._pairs;
    return new yi({
      *[vi]() {
        for (const s of t) {
          var r = St(s, 2);
          const a = r[0], o = r[1];
          e(o, a) && (yield [a, o]);
        }
      }
    });
  }
  /**
   * Execute a callback for each item of the stream.
   * @param callback - The function that will be invoked for each item.
   */
  forEach(e) {
    for (const r of this._pairs) {
      var t = St(r, 2);
      const s = t[0], a = t[1];
      e(a, s);
    }
  }
  /**
   * Map the items into a different type.
   * @param callback - The function that does the conversion.
   * @typeParam Mapped - The type of the item after mapping.
   * @returns A new data stream with the mapped items.
   */
  map(e) {
    const t = this._pairs;
    return new yi({
      *[vi]() {
        for (const s of t) {
          var r = St(s, 2);
          const a = r[0], o = r[1];
          yield [a, e(o, a)];
        }
      }
    });
  }
  /**
   * Get the item with the maximum value of given property.
   * @param callback - The function that picks and possibly converts the property.
   * @returns The item with the maximum if found otherwise null.
   */
  max(e) {
    const t = lh(this._pairs);
    let r = t.next();
    if (r.done)
      return null;
    let s = r.value[1], a = e(r.value[1], r.value[0]);
    for (; !(r = t.next()).done; ) {
      const o = St(r.value, 2), l = o[0], c = o[1], d = e(c, l);
      d > a && (a = d, s = c);
    }
    return s;
  }
  /**
   * Get the item with the minimum value of given property.
   * @param callback - The function that picks and possibly converts the property.
   * @returns The item with the minimum if found otherwise null.
   */
  min(e) {
    const t = lh(this._pairs);
    let r = t.next();
    if (r.done)
      return null;
    let s = r.value[1], a = e(r.value[1], r.value[0]);
    for (; !(r = t.next()).done; ) {
      const o = St(r.value, 2), l = o[0], c = o[1], d = e(c, l);
      d < a && (a = d, s = c);
    }
    return s;
  }
  /**
   * Reduce the items into a single value.
   * @param callback - The function that does the reduction.
   * @param accumulator - The initial value of the accumulator.
   * @typeParam T - The type of the accumulated value.
   * @returns The reduced value.
   */
  reduce(e, t) {
    for (const s of this._pairs) {
      var r = St(s, 2);
      const a = r[0], o = r[1];
      t = e(t, o, a);
    }
    return t;
  }
  /**
   * Sort the items.
   * @param callback - Item comparator.
   * @returns A new stream with sorted items.
   */
  sort(e) {
    return new yi({
      [vi]: () => {
        var t;
        return lh(gt(t = [...this._pairs]).call(t, (r, s) => {
          let a = St(r, 2), o = a[0], l = a[1], c = St(s, 2), d = c[0], f = c[1];
          return e(l, f, o, d);
        }));
      }
    });
  }
}
function RY(n, e) {
  return n[e] == null && (n[e] = wi()), n;
}
class Oi extends ts {
  /** @inheritDoc */
  get idProp() {
    return this._idProp;
  }
  /**
   * Construct a new DataSet.
   * @param data - Initial data or options.
   * @param options - Options (type error if data is also options).
   */
  constructor(e, t) {
    super(), Le(this, "flush", void 0), Le(this, "length", void 0), Le(this, "_options", void 0), Le(this, "_data", void 0), Le(this, "_idProp", void 0), Le(this, "_queue", null), e && !ye(e) && (t = e, e = []), this._options = t || {}, this._data = new QD(), this.length = 0, this._idProp = this._options.fieldId || "id", e && e.length && this.add(e), this.setOptions(t);
  }
  /**
   * Set new options.
   * @param options - The new options.
   */
  setOptions(e) {
    e && e.queue !== void 0 && (e.queue === !1 ? this._queue && (this._queue.destroy(), this._queue = null) : (this._queue || (this._queue = $a.extend(this, {
      replace: ["add", "update", "remove"]
    })), e.queue && typeof e.queue == "object" && this._queue.setOptions(e.queue)));
  }
  /**
   * Add a data item or an array with items.
   *
   * After the items are added to the DataSet, the DataSet will trigger an event `add`. When a `senderId` is provided, this id will be passed with the triggered event to all subscribers.
   *
   * ## Example
   *
   * ```javascript
   * // create a DataSet
   * const data = new vis.DataSet()
   *
   * // add items
   * const ids = data.add([
   * { id: 1, text: 'item 1' },
   * { id: 2, text: 'item 2' },
   * { text: 'item without an id' }
   * ])
   *
   * console.log(ids) // [1, 2, '<UUIDv4>']
   * ```
   * @param data - Items to be added (ids will be generated if missing).
   * @param senderId - Sender id.
   * @returns addedIds - Array with the ids (generated if not present) of the added items.
   * @throws When an item with the same id as any of the added items already exists.
   */
  add(e, t) {
    const r = [];
    let s;
    if (ye(e)) {
      const a = Ae(e).call(e, (o) => o[this._idProp]);
      if (bm(a).call(a, (o) => this._data.has(o)))
        throw new Error("A duplicate id was found in the parameter array.");
      for (let o = 0, l = e.length; o < l; o++)
        s = this._addItem(e[o]), r.push(s);
    } else if (e && typeof e == "object")
      s = this._addItem(e), r.push(s);
    else
      throw new Error("Unknown dataType");
    return r.length && this._trigger("add", {
      items: r
    }, t), r;
  }
  /**
   * Update existing items. When an item does not exist, it will be created.
   * @remarks
   * The provided properties will be merged in the existing item. When an item does not exist, it will be created.
   *
   * After the items are updated, the DataSet will trigger an event `add` for the added items, and an event `update`. When a `senderId` is provided, this id will be passed with the triggered event to all subscribers.
   *
   * ## Example
   *
   * ```javascript
   * // create a DataSet
   * const data = new vis.DataSet([
   *   { id: 1, text: 'item 1' },
   *   { id: 2, text: 'item 2' },
   *   { id: 3, text: 'item 3' }
   * ])
   *
   * // update items
   * const ids = data.update([
   *   { id: 2, text: 'item 2 (updated)' },
   *   { id: 4, text: 'item 4 (new)' }
   * ])
   *
   * console.log(ids) // [2, 4]
   * ```
   *
   * ## Warning for TypeScript users
   * This method may introduce partial items into the data set. Use add or updateOnly instead for better type safety.
   * @param data - Items to be updated (if the id is already present) or added (if the id is missing).
   * @param senderId - Sender id.
   * @returns updatedIds - The ids of the added (these may be newly generated if there was no id in the item from the data) or updated items.
   * @throws When the supplied data is neither an item nor an array of items.
   */
  update(e, t) {
    const r = [], s = [], a = [], o = [], l = this._idProp, c = (d) => {
      const f = d[l];
      if (f != null && this._data.has(f)) {
        const p = d, m = Yr({}, this._data.get(f)), g = this._updateItem(p);
        s.push(g), o.push(p), a.push(m);
      } else {
        const p = this._addItem(d);
        r.push(p);
      }
    };
    if (ye(e))
      for (let d = 0, f = e.length; d < f; d++)
        e[d] && typeof e[d] == "object" ? c(e[d]) : console.warn("Ignoring input item, which is not an object at index " + d);
    else if (e && typeof e == "object")
      c(e);
    else
      throw new Error("Unknown dataType");
    if (r.length && this._trigger("add", {
      items: r
    }, t), s.length) {
      const d = {
        items: s,
        oldData: a,
        data: o
      };
      this._trigger("update", d, t);
    }
    return rt(r).call(r, s);
  }
  /**
   * Update existing items. When an item does not exist, an error will be thrown.
   * @remarks
   * The provided properties will be deeply merged into the existing item.
   * When an item does not exist (id not present in the data set or absent), an error will be thrown and nothing will be changed.
   *
   * After the items are updated, the DataSet will trigger an event `update`.
   * When a `senderId` is provided, this id will be passed with the triggered event to all subscribers.
   *
   * ## Example
   *
   * ```javascript
   * // create a DataSet
   * const data = new vis.DataSet([
   *   { id: 1, text: 'item 1' },
   *   { id: 2, text: 'item 2' },
   *   { id: 3, text: 'item 3' },
   * ])
   *
   * // update items
   * const ids = data.update([
   *   { id: 2, text: 'item 2 (updated)' }, // works
   *   // { id: 4, text: 'item 4 (new)' }, // would throw
   *   // { text: 'item 4 (new)' }, // would also throw
   * ])
   *
   * console.log(ids) // [2]
   * ```
   * @param data - Updates (the id and optionally other props) to the items in this data set.
   * @param senderId - Sender id.
   * @returns updatedIds - The ids of the updated items.
   * @throws When the supplied data is neither an item nor an array of items, when the ids are missing.
   */
  updateOnly(e, t) {
    var r;
    ye(e) || (e = [e]);
    const s = Ae(r = Ae(e).call(e, (a) => {
      const o = this._data.get(a[this._idProp]);
      if (o == null)
        throw new Error("Updating non-existent items is not allowed.");
      return {
        oldData: o,
        update: a
      };
    })).call(r, (a) => {
      let o = a.oldData, l = a.update;
      const c = o[this._idProp], d = kO(o, l);
      return this._data.set(c, d), {
        id: c,
        oldData: o,
        updatedData: d
      };
    });
    if (s.length) {
      const a = {
        items: Ae(s).call(s, (o) => o.id),
        oldData: Ae(s).call(s, (o) => o.oldData),
        data: Ae(s).call(s, (o) => o.updatedData)
      };
      return this._trigger("update", a, t), a.items;
    } else
      return [];
  }
  /** @inheritDoc */
  get(e, t) {
    let r, s, a;
    $p(e) ? (r = e, a = t) : ye(e) ? (s = e, a = t) : a = e;
    const o = a && a.returnType === "Object" ? "Object" : "Array", l = a && ie(a), c = [];
    let d, f, p;
    if (r != null)
      d = this._data.get(r), d && l && !l(d) && (d = void 0);
    else if (s != null)
      for (let g = 0, b = s.length; g < b; g++)
        d = this._data.get(s[g]), d != null && (!l || l(d)) && c.push(d);
    else {
      var m;
      f = [...mi(m = this._data).call(m)];
      for (let g = 0, b = f.length; g < b; g++)
        p = f[g], d = this._data.get(p), d != null && (!l || l(d)) && c.push(d);
    }
    if (a && a.order && r == null && this._sort(c, a.order), a && a.fields) {
      const g = a.fields;
      if (r != null && d != null)
        d = this._filterFields(d, g);
      else
        for (let b = 0, _ = c.length; b < _; b++)
          c[b] = this._filterFields(c[b], g);
    }
    if (o == "Object") {
      const g = {};
      for (let b = 0, _ = c.length; b < _; b++) {
        const x = c[b], T = x[this._idProp];
        g[T] = x;
      }
      return g;
    } else
      return r != null ? d ?? null : c;
  }
  /** @inheritDoc */
  getIds(e) {
    const t = this._data, r = e && ie(e), s = e && e.order, a = [...mi(t).call(t)], o = [];
    if (r)
      if (s) {
        const l = [];
        for (let c = 0, d = a.length; c < d; c++) {
          const f = a[c], p = this._data.get(f);
          p != null && r(p) && l.push(p);
        }
        this._sort(l, s);
        for (let c = 0, d = l.length; c < d; c++)
          o.push(l[c][this._idProp]);
      } else
        for (let l = 0, c = a.length; l < c; l++) {
          const d = a[l], f = this._data.get(d);
          f != null && r(f) && o.push(f[this._idProp]);
        }
    else if (s) {
      const l = [];
      for (let c = 0, d = a.length; c < d; c++) {
        const f = a[c];
        l.push(t.get(f));
      }
      this._sort(l, s);
      for (let c = 0, d = l.length; c < d; c++)
        o.push(l[c][this._idProp]);
    } else
      for (let l = 0, c = a.length; l < c; l++) {
        const d = a[l], f = t.get(d);
        f != null && o.push(f[this._idProp]);
      }
    return o;
  }
  /** @inheritDoc */
  getDataSet() {
    return this;
  }
  /** @inheritDoc */
  forEach(e, t) {
    const r = t && ie(t), s = this._data, a = [...mi(s).call(s)];
    if (t && t.order) {
      const o = this.get(t);
      for (let l = 0, c = o.length; l < c; l++) {
        const d = o[l], f = d[this._idProp];
        e(d, f);
      }
    } else
      for (let o = 0, l = a.length; o < l; o++) {
        const c = a[o], d = this._data.get(c);
        d != null && (!r || r(d)) && e(d, c);
      }
  }
  /** @inheritDoc */
  map(e, t) {
    const r = t && ie(t), s = [], a = this._data, o = [...mi(a).call(a)];
    for (let l = 0, c = o.length; l < c; l++) {
      const d = o[l], f = this._data.get(d);
      f != null && (!r || r(f)) && s.push(e(f, d));
    }
    return t && t.order && this._sort(s, t.order), s;
  }
  /**
   * Filter the fields of an item.
   * @param item - The item whose fields should be filtered.
   * @param fields - The names of the fields that will be kept.
   * @typeParam K - Field name type.
   * @returns The item without any additional fields.
   */
  _filterFields(e, t) {
    var r;
    return e && Aa(r = ye(t) ? (
      // Use the supplied array
      t
    ) : (
      // Use the keys of the supplied object
      ct(t)
    )).call(r, (s, a) => (s[a] = e[a], s), {});
  }
  /**
   * Sort the provided array with items.
   * @param items - Items to be sorted in place.
   * @param order - A field name or custom sort function.
   * @typeParam T - The type of the items in the items array.
   */
  _sort(e, t) {
    if (typeof t == "string") {
      const r = t;
      gt(e).call(e, (s, a) => {
        const o = s[r], l = a[r];
        return o > l ? 1 : o < l ? -1 : 0;
      });
    } else if (typeof t == "function")
      gt(e).call(e, t);
    else
      throw new TypeError("Order must be a function or a string");
  }
  /**
   * Remove an item or multiple items by “reference” (only the id is used) or by id.
   *
   * The method ignores removal of non-existing items, and returns an array containing the ids of the items which are actually removed from the DataSet.
   *
   * After the items are removed, the DataSet will trigger an event `remove` for the removed items. When a `senderId` is provided, this id will be passed with the triggered event to all subscribers.
   *
   * ## Example
   * ```javascript
   * // create a DataSet
   * const data = new vis.DataSet([
   * { id: 1, text: 'item 1' },
   * { id: 2, text: 'item 2' },
   * { id: 3, text: 'item 3' }
   * ])
   *
   * // remove items
   * const ids = data.remove([2, { id: 3 }, 4])
   *
   * console.log(ids) // [2, 3]
   * ```
   * @param id - One or more items or ids of items to be removed.
   * @param senderId - Sender id.
   * @returns The ids of the removed items.
   */
  remove(e, t) {
    const r = [], s = [], a = ye(e) ? e : [e];
    for (let o = 0, l = a.length; o < l; o++) {
      const c = this._remove(a[o]);
      if (c) {
        const d = c[this._idProp];
        d != null && (r.push(d), s.push(c));
      }
    }
    return r.length && this._trigger("remove", {
      items: r,
      oldData: s
    }, t), r;
  }
  /**
   * Remove an item by its id or reference.
   * @param id - Id of an item or the item itself.
   * @returns The removed item if removed, null otherwise.
   */
  _remove(e) {
    let t;
    if ($p(e) ? t = e : e && typeof e == "object" && (t = e[this._idProp]), t != null && this._data.has(t)) {
      const r = this._data.get(t) || null;
      return this._data.delete(t), --this.length, r;
    }
    return null;
  }
  /**
   * Clear the entire data set.
   *
   * After the items are removed, the {@link DataSet} will trigger an event `remove` for all removed items. When a `senderId` is provided, this id will be passed with the triggered event to all subscribers.
   * @param senderId - Sender id.
   * @returns removedIds - The ids of all removed items.
   */
  clear(e) {
    var t;
    const r = [...mi(t = this._data).call(t)], s = [];
    for (let a = 0, o = r.length; a < o; a++)
      s.push(this._data.get(r[a]));
    return this._data.clear(), this.length = 0, this._trigger("remove", {
      items: r,
      oldData: s
    }, e), r;
  }
  /**
   * Find the item with maximum value of a specified field.
   * @param field - Name of the property that should be searched for max value.
   * @returns Item containing max value, or null if no items.
   */
  max(e) {
    let t = null, r = null;
    for (const a of TS(s = this._data).call(s)) {
      var s;
      const o = a[e];
      typeof o == "number" && (r == null || o > r) && (t = a, r = o);
    }
    return t || null;
  }
  /**
   * Find the item with minimum value of a specified field.
   * @param field - Name of the property that should be searched for min value.
   * @returns Item containing min value, or null if no items.
   */
  min(e) {
    let t = null, r = null;
    for (const a of TS(s = this._data).call(s)) {
      var s;
      const o = a[e];
      typeof o == "number" && (r == null || o < r) && (t = a, r = o);
    }
    return t || null;
  }
  /**
   * Find all distinct values of a specified field
   * @param prop - The property name whose distinct values should be returned.
   * @returns Unordered array containing all distinct values. Items without specified property are ignored.
   */
  distinct(e) {
    const t = this._data, r = [...mi(t).call(t)], s = [];
    let a = 0;
    for (let o = 0, l = r.length; o < l; o++) {
      const c = r[o], f = t.get(c)[e];
      let p = !1;
      for (let m = 0; m < a; m++)
        if (s[m] == f) {
          p = !0;
          break;
        }
      !p && f !== void 0 && (s[a] = f, a++);
    }
    return s;
  }
  /**
   * Add a single item. Will fail when an item with the same id already exists.
   * @param item - A new item to be added.
   * @returns Added item's id. An id is generated when it is not present in the item.
   */
  _addItem(e) {
    const t = RY(e, this._idProp), r = t[this._idProp];
    if (this._data.has(r))
      throw new Error("Cannot add item: item with id " + r + " already exists");
    return this._data.set(r, t), ++this.length, r;
  }
  /**
   * Update a single item: merge with existing item.
   * Will fail when the item has no id, or when there does not exist an item with the same id.
   * @param update - The new item
   * @returns The id of the updated item.
   */
  _updateItem(e) {
    const t = e[this._idProp];
    if (t == null)
      throw new Error("Cannot update item: item has no id (item: " + Yt(e) + ")");
    const r = this._data.get(t);
    if (!r)
      throw new Error("Cannot update item: no item with id " + t + " found");
    return this._data.set(t, hE(hE({}, r), e)), t;
  }
  /** @inheritDoc */
  stream(e) {
    if (e) {
      const r = this._data;
      return new yi({
        *[vi]() {
          for (const s of e) {
            const a = r.get(s);
            a != null && (yield [s, a]);
          }
        }
      });
    } else {
      var t;
      return new yi({
        [vi]: N(t = _2(this._data)).call(t, this._data)
      });
    }
  }
}
class ja extends ts {
  /** @inheritDoc */
  get idProp() {
    return this.getDataSet().idProp;
  }
  /**
   * Create a DataView.
   * @param data - The instance containing data (directly or indirectly).
   * @param options - Options to configure this data view.
   */
  constructor(e, t) {
    var r;
    super(), Le(this, "length", 0), Le(this, "_listener", void 0), Le(this, "_data", void 0), Le(this, "_ids", new Jr()), Le(this, "_options", void 0), this._options = t || {}, this._listener = N(r = this._onEvent).call(r, this), this.setData(e);
  }
  // TODO: implement a function .config() to dynamically update things like configured filter
  // and trigger changes accordingly
  /**
   * Set a data source for the view.
   * @param data - The instance containing data (directly or indirectly).
   * @remarks
   * Note that when the data view is bound to a data set it won't be garbage
   * collected unless the data set is too. Use `dataView.setData(null)` or
   * `dataView.dispose()` to enable garbage collection before you lose the last
   * reference.
   */
  setData(e) {
    if (this._data) {
      this._data.off && this._data.off("*", this._listener);
      const t = this._data.getIds({
        filter: ie(this._options)
      }), r = this._data.get(t);
      this._ids.clear(), this.length = 0, this._trigger("remove", {
        items: t,
        oldData: r
      });
    }
    if (e != null) {
      this._data = e;
      const t = this._data.getIds({
        filter: ie(this._options)
      });
      for (let r = 0, s = t.length; r < s; r++) {
        const a = t[r];
        this._ids.add(a);
      }
      this.length = t.length, this._trigger("add", {
        items: t
      });
    } else
      this._data = new Oi();
    this._data.on && this._data.on("*", this._listener);
  }
  /**
   * Refresh the DataView.
   * Useful when the DataView has a filter function containing a variable parameter.
   */
  refresh() {
    const e = this._data.getIds({
      filter: ie(this._options)
    }), t = [...this._ids], r = {}, s = [], a = [], o = [];
    for (let l = 0, c = e.length; l < c; l++) {
      const d = e[l];
      r[d] = !0, this._ids.has(d) || (s.push(d), this._ids.add(d));
    }
    for (let l = 0, c = t.length; l < c; l++) {
      const d = t[l], f = this._data.get(d);
      f == null ? console.error("If you see this, report it please.") : r[d] || (a.push(d), o.push(f), this._ids.delete(d));
    }
    this.length += s.length - a.length, s.length && this._trigger("add", {
      items: s
    }), a.length && this._trigger("remove", {
      items: a,
      oldData: o
    });
  }
  /** @inheritDoc */
  get(e, t) {
    if (this._data == null)
      return null;
    let r = null, s;
    $p(e) || ye(e) ? (r = e, s = t) : s = e;
    const a = Yr({}, this._options, s), o = ie(this._options), l = s && ie(s);
    return o && l && (a.filter = (c) => o(c) && l(c)), r == null ? this._data.get(a) : this._data.get(r, a);
  }
  /** @inheritDoc */
  getIds(e) {
    if (this._data.length) {
      const t = ie(this._options), r = e != null ? ie(e) : null;
      let s;
      return r ? t ? s = (a) => t(a) && r(a) : s = r : s = t, this._data.getIds({
        filter: s,
        order: e && e.order
      });
    } else
      return [];
  }
  /** @inheritDoc */
  forEach(e, t) {
    if (this._data) {
      var r;
      const s = ie(this._options), a = t && ie(t);
      let o;
      a ? s ? o = function(l) {
        return s(l) && a(l);
      } : o = a : o = s, B(r = this._data).call(r, e, {
        filter: o,
        order: t && t.order
      });
    }
  }
  /** @inheritDoc */
  map(e, t) {
    if (this._data) {
      var r;
      const s = ie(this._options), a = t && ie(t);
      let o;
      return a ? s ? o = (l) => s(l) && a(l) : o = a : o = s, Ae(r = this._data).call(r, e, {
        filter: o,
        order: t && t.order
      });
    } else
      return [];
  }
  /** @inheritDoc */
  getDataSet() {
    return this._data.getDataSet();
  }
  /** @inheritDoc */
  stream(e) {
    var t;
    return this._data.stream(e || {
      [vi]: N(t = mi(this._ids)).call(t, this._ids)
    });
  }
  /**
   * Render the instance unusable prior to garbage collection.
   * @remarks
   * The intention of this method is to help discover scenarios where the data
   * view is being used when the programmer thinks it has been garbage collected
   * already. It's stricter version of `dataView.setData(null)`.
   */
  dispose() {
    var e;
    (e = this._data) !== null && e !== void 0 && e.off && this._data.off("*", this._listener);
    const t = "This data view has already been disposed of.", r = {
      get: () => {
        throw new Error(t);
      },
      set: () => {
        throw new Error(t);
      },
      configurable: !1
    };
    for (const s of tO(ja.prototype))
      Na(this, s, r);
  }
  /**
   * Event listener. Will propagate all events from the connected data set to the subscribers of the DataView, but will filter the items and only trigger when there are changes in the filtered data set.
   * @param event - The name of the event.
   * @param params - Parameters of the event.
   * @param senderId - Id supplied by the sender.
   */
  _onEvent(e, t, r) {
    if (!t || !t.items || !this._data)
      return;
    const s = t.items, a = [], o = [], l = [], c = [], d = [], f = [];
    switch (e) {
      case "add":
        for (let p = 0, m = s.length; p < m; p++) {
          const g = s[p];
          this.get(g) && (this._ids.add(g), a.push(g));
        }
        break;
      case "update":
        for (let p = 0, m = s.length; p < m; p++) {
          const g = s[p];
          this.get(g) ? this._ids.has(g) ? (o.push(g), d.push(t.data[p]), c.push(t.oldData[p])) : (this._ids.add(g), a.push(g)) : this._ids.has(g) && (this._ids.delete(g), l.push(g), f.push(t.oldData[p]));
        }
        break;
      case "remove":
        for (let p = 0, m = s.length; p < m; p++) {
          const g = s[p];
          this._ids.has(g) && (this._ids.delete(g), l.push(g), f.push(t.oldData[p]));
        }
        break;
    }
    this.length += a.length - l.length, a.length && this._trigger("add", {
      items: a
    }, r), o.length && this._trigger("update", {
      items: o,
      oldData: c,
      data: d
    }, r), l.length && this._trigger("remove", {
      items: l,
      oldData: f
    }, r);
  }
}
function PY(n, e) {
  return typeof e == "object" && e !== null && n === e.idProp && typeof e.add == "function" && typeof e.clear == "function" && typeof e.distinct == "function" && typeof B(e) == "function" && typeof e.get == "function" && typeof e.getDataSet == "function" && typeof e.getIds == "function" && typeof e.length == "number" && typeof Ae(e) == "function" && typeof e.max == "function" && typeof e.min == "function" && typeof e.off == "function" && typeof e.on == "function" && typeof e.remove == "function" && typeof e.setOptions == "function" && typeof e.stream == "function" && typeof e.update == "function" && typeof e.updateOnly == "function";
}
function IY(n, e) {
  return typeof e == "object" && e !== null && n === e.idProp && typeof B(e) == "function" && typeof e.get == "function" && typeof e.getDataSet == "function" && typeof e.getIds == "function" && typeof e.length == "number" && typeof Ae(e) == "function" && typeof e.off == "function" && typeof e.on == "function" && typeof e.stream == "function" && PY(n, e.getDataSet());
}
function Lr(n) {
  for (var e in n)
    Object.prototype.hasOwnProperty.call(n, e) && (n[e].redundant = n[e].used, n[e].used = []);
}
function zt(n) {
  for (var e in n) {
    if (!Object.prototype.hasOwnProperty.call(n, e)) continue;
    const r = n[e];
    for (var t = 0; t < r.redundant.length; t++)
      r.redundant[t].parentNode.removeChild(r.redundant[t]);
    r.redundant = [];
  }
}
function $O(n) {
  Lr(n), zt(n), Lr(n);
}
function nr(n, e, t) {
  var r;
  return Object.prototype.hasOwnProperty.call(e, n) ? e[n].redundant.length > 0 ? (r = e[n].redundant[0], e[n].redundant.shift()) : (r = document.createElementNS("http://www.w3.org/2000/svg", n), t.appendChild(r)) : (r = document.createElementNS("http://www.w3.org/2000/svg", n), e[n] = {
    used: [],
    redundant: []
  }, t.appendChild(r)), e[n].used.push(r), r;
}
function sa(n, e, t, r) {
  var s;
  return Object.prototype.hasOwnProperty.call(e, n) ? e[n].redundant.length > 0 ? (s = e[n].redundant[0], e[n].redundant.shift()) : (s = document.createElement(n), r !== void 0 ? t.insertBefore(s, r) : t.appendChild(s)) : (s = document.createElement(n), e[n] = {
    used: [],
    redundant: []
  }, r !== void 0 ? t.insertBefore(s, r) : t.appendChild(s)), e[n].used.push(s), s;
}
function ki(n, e, t, r, s, a) {
  var o;
  if (t.style == "circle" ? (o = nr("circle", r, s), o.setAttributeNS(null, "cx", n), o.setAttributeNS(null, "cy", e), o.setAttributeNS(null, "r", 0.5 * t.size)) : (o = nr("rect", r, s), o.setAttributeNS(null, "x", n - 0.5 * t.size), o.setAttributeNS(null, "y", e - 0.5 * t.size), o.setAttributeNS(null, "width", t.size), o.setAttributeNS(null, "height", t.size)), t.styles !== void 0 && o.setAttributeNS(null, "style", t.styles), o.setAttributeNS(null, "class", t.className + " vis-point"), a) {
    var l = nr("text", r, s);
    a.xOffset && (n = n + a.xOffset), a.yOffset && (e = e + a.yOffset), a.content && (l.textContent = a.content), a.className && l.setAttributeNS(null, "class", a.className + " vis-label"), l.setAttributeNS(null, "x", n), l.setAttributeNS(null, "y", e);
  }
  return o;
}
function pa(n, e, t, r, s, a, o, l) {
  if (r != 0) {
    r < 0 && (r *= -1, e -= r);
    var c = nr("rect", a, o);
    c.setAttributeNS(null, "x", n - 0.5 * t), c.setAttributeNS(null, "y", e), c.setAttributeNS(null, "width", t), c.setAttributeNS(null, "height", r), c.setAttributeNS(null, "class", s), l && c.setAttributeNS(null, "style", l);
  }
}
function jO() {
  try {
    return navigator ? navigator.languages && navigator.languages.length ? navigator.languages : navigator.userLanguage || navigator.language || navigator.browserLanguage || "en" : "en";
  } catch {
    return "en";
  }
}
var MY = /* @__PURE__ */ Object.freeze({
  __proto__: null,
  cleanupElements: zt,
  drawBar: pa,
  drawPoint: ki,
  getDOMElement: sa,
  getNavigatorLanguage: jO,
  getSVGElement: nr,
  prepareElements: Lr,
  resetElements: $O
}), Ln = null;
function jp(n, e) {
  var t = e || {
    preventDefault: !1
  };
  if (n.Manager) {
    var r = n, s = function(d, f) {
      var p = Object.create(t);
      return f && r.assign(p, f), jp(new r(d, p), p);
    };
    return r.assign(s, r), s.Manager = function(d, f) {
      var p = Object.create(t);
      return f && r.assign(p, f), jp(new r.Manager(d, p), p);
    }, s;
  }
  var a = Object.create(n), o = n.element;
  o.hammer || (o.hammer = []), o.hammer.push(a), n.on("hammer.input", function(d) {
    (t.preventDefault === !0 || t.preventDefault === d.pointerType) && d.preventDefault(), d.isFirst && (Ln = d.target);
  }), a._handlers = {}, a.on = function(d, f) {
    return l(d).forEach(function(p) {
      var m = a._handlers[p];
      m || (a._handlers[p] = m = [], n.on(p, c)), m.push(f);
    }), a;
  }, a.off = function(d, f) {
    return l(d).forEach(function(p) {
      var m = a._handlers[p];
      m && (m = f ? m.filter(function(g) {
        return g !== f;
      }) : [], m.length > 0 ? a._handlers[p] = m : (n.off(p, c), delete a._handlers[p]));
    }), a;
  }, a.emit = function(d, f) {
    Ln = f.target, n.emit(d, f);
  }, a.destroy = function() {
    var d = n.element.hammer, f = d.indexOf(a);
    f !== -1 && d.splice(f, 1), d.length || delete n.element.hammer, a._handlers = {}, n.destroy();
  };
  function l(d) {
    return d.match(/[^ ]+/g);
  }
  function c(d) {
    if (d.type !== "hammer.input") {
      if (d.srcEvent._handled || (d.srcEvent._handled = {}), d.srcEvent._handled[d.type])
        return;
      d.srcEvent._handled[d.type] = !0;
    }
    var f = !1;
    d.stopPropagation = function() {
      f = !0;
    };
    var p = d.srcEvent.stopPropagation.bind(d.srcEvent);
    typeof p == "function" && (d.srcEvent.stopPropagation = function() {
      p(), d.stopPropagation();
    }), d.firstTarget = Ln;
    for (var m = Ln.isConnected ? Ln : d.target; m && !f; ) {
      var g = m.hammer;
      if (g) {
        for (var b, _ = 0; _ < g.length; _++)
          if (b = g[_]._handlers[d.type], b) for (var x = 0; x < b.length && !f; x++)
            b[x](d);
      }
      m = m.parentNode;
    }
  }
  return a;
}
function AY() {
  const n = () => {
  };
  return {
    on: n,
    off: n,
    destroy: n,
    emit: n,
    get() {
      return {
        set: n
      };
    }
  };
}
const Tt = (function() {
  if (typeof window < "u") {
    const n = window.Hammer || Rm;
    return jp(n, {
      preventDefault: "mouse"
    });
  } else
    return function() {
      return AY();
    };
})();
var NY = pr(), tr = /* @__PURE__ */ Q(NY);
const Et = typeof window < "u" && window.moment || tr;
var Xs = { exports: {} }, We = {}, Zs = { exports: {} }, pi = {}, fE;
function YO() {
  if (fE) return pi;
  fE = 1;
  function n() {
    var a = {};
    return a["align-content"] = !1, a["align-items"] = !1, a["align-self"] = !1, a["alignment-adjust"] = !1, a["alignment-baseline"] = !1, a.all = !1, a["anchor-point"] = !1, a.animation = !1, a["animation-delay"] = !1, a["animation-direction"] = !1, a["animation-duration"] = !1, a["animation-fill-mode"] = !1, a["animation-iteration-count"] = !1, a["animation-name"] = !1, a["animation-play-state"] = !1, a["animation-timing-function"] = !1, a.azimuth = !1, a["backface-visibility"] = !1, a.background = !0, a["background-attachment"] = !0, a["background-clip"] = !0, a["background-color"] = !0, a["background-image"] = !0, a["background-origin"] = !0, a["background-position"] = !0, a["background-repeat"] = !0, a["background-size"] = !0, a["baseline-shift"] = !1, a.binding = !1, a.bleed = !1, a["bookmark-label"] = !1, a["bookmark-level"] = !1, a["bookmark-state"] = !1, a.border = !0, a["border-bottom"] = !0, a["border-bottom-color"] = !0, a["border-bottom-left-radius"] = !0, a["border-bottom-right-radius"] = !0, a["border-bottom-style"] = !0, a["border-bottom-width"] = !0, a["border-collapse"] = !0, a["border-color"] = !0, a["border-image"] = !0, a["border-image-outset"] = !0, a["border-image-repeat"] = !0, a["border-image-slice"] = !0, a["border-image-source"] = !0, a["border-image-width"] = !0, a["border-left"] = !0, a["border-left-color"] = !0, a["border-left-style"] = !0, a["border-left-width"] = !0, a["border-radius"] = !0, a["border-right"] = !0, a["border-right-color"] = !0, a["border-right-style"] = !0, a["border-right-width"] = !0, a["border-spacing"] = !0, a["border-style"] = !0, a["border-top"] = !0, a["border-top-color"] = !0, a["border-top-left-radius"] = !0, a["border-top-right-radius"] = !0, a["border-top-style"] = !0, a["border-top-width"] = !0, a["border-width"] = !0, a.bottom = !1, a["box-decoration-break"] = !0, a["box-shadow"] = !0, a["box-sizing"] = !0, a["box-snap"] = !0, a["box-suppress"] = !0, a["break-after"] = !0, a["break-before"] = !0, a["break-inside"] = !0, a["caption-side"] = !1, a.chains = !1, a.clear = !0, a.clip = !1, a["clip-path"] = !1, a["clip-rule"] = !1, a.color = !0, a["color-interpolation-filters"] = !0, a["column-count"] = !1, a["column-fill"] = !1, a["column-gap"] = !1, a["column-rule"] = !1, a["column-rule-color"] = !1, a["column-rule-style"] = !1, a["column-rule-width"] = !1, a["column-span"] = !1, a["column-width"] = !1, a.columns = !1, a.contain = !1, a.content = !1, a["counter-increment"] = !1, a["counter-reset"] = !1, a["counter-set"] = !1, a.crop = !1, a.cue = !1, a["cue-after"] = !1, a["cue-before"] = !1, a.cursor = !1, a.direction = !1, a.display = !0, a["display-inside"] = !0, a["display-list"] = !0, a["display-outside"] = !0, a["dominant-baseline"] = !1, a.elevation = !1, a["empty-cells"] = !1, a.filter = !1, a.flex = !1, a["flex-basis"] = !1, a["flex-direction"] = !1, a["flex-flow"] = !1, a["flex-grow"] = !1, a["flex-shrink"] = !1, a["flex-wrap"] = !1, a.float = !1, a["float-offset"] = !1, a["flood-color"] = !1, a["flood-opacity"] = !1, a["flow-from"] = !1, a["flow-into"] = !1, a.font = !0, a["font-family"] = !0, a["font-feature-settings"] = !0, a["font-kerning"] = !0, a["font-language-override"] = !0, a["font-size"] = !0, a["font-size-adjust"] = !0, a["font-stretch"] = !0, a["font-style"] = !0, a["font-synthesis"] = !0, a["font-variant"] = !0, a["font-variant-alternates"] = !0, a["font-variant-caps"] = !0, a["font-variant-east-asian"] = !0, a["font-variant-ligatures"] = !0, a["font-variant-numeric"] = !0, a["font-variant-position"] = !0, a["font-weight"] = !0, a.grid = !1, a["grid-area"] = !1, a["grid-auto-columns"] = !1, a["grid-auto-flow"] = !1, a["grid-auto-rows"] = !1, a["grid-column"] = !1, a["grid-column-end"] = !1, a["grid-column-start"] = !1, a["grid-row"] = !1, a["grid-row-end"] = !1, a["grid-row-start"] = !1, a["grid-template"] = !1, a["grid-template-areas"] = !1, a["grid-template-columns"] = !1, a["grid-template-rows"] = !1, a["hanging-punctuation"] = !1, a.height = !0, a.hyphens = !1, a.icon = !1, a["image-orientation"] = !1, a["image-resolution"] = !1, a["ime-mode"] = !1, a["initial-letters"] = !1, a["inline-box-align"] = !1, a["justify-content"] = !1, a["justify-items"] = !1, a["justify-self"] = !1, a.left = !1, a["letter-spacing"] = !0, a["lighting-color"] = !0, a["line-box-contain"] = !1, a["line-break"] = !1, a["line-grid"] = !1, a["line-height"] = !1, a["line-snap"] = !1, a["line-stacking"] = !1, a["line-stacking-ruby"] = !1, a["line-stacking-shift"] = !1, a["line-stacking-strategy"] = !1, a["list-style"] = !0, a["list-style-image"] = !0, a["list-style-position"] = !0, a["list-style-type"] = !0, a.margin = !0, a["margin-bottom"] = !0, a["margin-left"] = !0, a["margin-right"] = !0, a["margin-top"] = !0, a["marker-offset"] = !1, a["marker-side"] = !1, a.marks = !1, a.mask = !1, a["mask-box"] = !1, a["mask-box-outset"] = !1, a["mask-box-repeat"] = !1, a["mask-box-slice"] = !1, a["mask-box-source"] = !1, a["mask-box-width"] = !1, a["mask-clip"] = !1, a["mask-image"] = !1, a["mask-origin"] = !1, a["mask-position"] = !1, a["mask-repeat"] = !1, a["mask-size"] = !1, a["mask-source-type"] = !1, a["mask-type"] = !1, a["max-height"] = !0, a["max-lines"] = !1, a["max-width"] = !0, a["min-height"] = !0, a["min-width"] = !0, a["move-to"] = !1, a["nav-down"] = !1, a["nav-index"] = !1, a["nav-left"] = !1, a["nav-right"] = !1, a["nav-up"] = !1, a["object-fit"] = !1, a["object-position"] = !1, a.opacity = !1, a.order = !1, a.orphans = !1, a.outline = !1, a["outline-color"] = !1, a["outline-offset"] = !1, a["outline-style"] = !1, a["outline-width"] = !1, a.overflow = !1, a["overflow-wrap"] = !1, a["overflow-x"] = !1, a["overflow-y"] = !1, a.padding = !0, a["padding-bottom"] = !0, a["padding-left"] = !0, a["padding-right"] = !0, a["padding-top"] = !0, a.page = !1, a["page-break-after"] = !1, a["page-break-before"] = !1, a["page-break-inside"] = !1, a["page-policy"] = !1, a.pause = !1, a["pause-after"] = !1, a["pause-before"] = !1, a.perspective = !1, a["perspective-origin"] = !1, a.pitch = !1, a["pitch-range"] = !1, a["play-during"] = !1, a.position = !1, a["presentation-level"] = !1, a.quotes = !1, a["region-fragment"] = !1, a.resize = !1, a.rest = !1, a["rest-after"] = !1, a["rest-before"] = !1, a.richness = !1, a.right = !1, a.rotation = !1, a["rotation-point"] = !1, a["ruby-align"] = !1, a["ruby-merge"] = !1, a["ruby-position"] = !1, a["shape-image-threshold"] = !1, a["shape-outside"] = !1, a["shape-margin"] = !1, a.size = !1, a.speak = !1, a["speak-as"] = !1, a["speak-header"] = !1, a["speak-numeral"] = !1, a["speak-punctuation"] = !1, a["speech-rate"] = !1, a.stress = !1, a["string-set"] = !1, a["tab-size"] = !1, a["table-layout"] = !1, a["text-align"] = !0, a["text-align-last"] = !0, a["text-combine-upright"] = !0, a["text-decoration"] = !0, a["text-decoration-color"] = !0, a["text-decoration-line"] = !0, a["text-decoration-skip"] = !0, a["text-decoration-style"] = !0, a["text-emphasis"] = !0, a["text-emphasis-color"] = !0, a["text-emphasis-position"] = !0, a["text-emphasis-style"] = !0, a["text-height"] = !0, a["text-indent"] = !0, a["text-justify"] = !0, a["text-orientation"] = !0, a["text-overflow"] = !0, a["text-shadow"] = !0, a["text-space-collapse"] = !0, a["text-transform"] = !0, a["text-underline-position"] = !0, a["text-wrap"] = !0, a.top = !1, a.transform = !1, a["transform-origin"] = !1, a["transform-style"] = !1, a.transition = !1, a["transition-delay"] = !1, a["transition-duration"] = !1, a["transition-property"] = !1, a["transition-timing-function"] = !1, a["unicode-bidi"] = !1, a["vertical-align"] = !1, a.visibility = !1, a["voice-balance"] = !1, a["voice-duration"] = !1, a["voice-family"] = !1, a["voice-pitch"] = !1, a["voice-range"] = !1, a["voice-rate"] = !1, a["voice-stress"] = !1, a["voice-volume"] = !1, a.volume = !1, a["white-space"] = !1, a.widows = !1, a.width = !0, a["will-change"] = !1, a["word-break"] = !0, a["word-spacing"] = !0, a["word-wrap"] = !0, a["wrap-flow"] = !1, a["wrap-through"] = !1, a["writing-mode"] = !1, a["z-index"] = !1, a;
  }
  function e(a, o, l) {
  }
  function t(a, o, l) {
  }
  var r = /javascript\s*\:/img;
  function s(a, o) {
    return r.test(o) ? "" : o;
  }
  return pi.whiteList = n(), pi.getDefaultWhiteList = n, pi.onAttr = e, pi.onIgnoreAttr = t, pi.safeAttrValue = s, pi;
}
var Wf, pE;
function HO() {
  return pE || (pE = 1, Wf = {
    indexOf: function(n, e) {
      var t, r;
      if (Array.prototype.indexOf)
        return n.indexOf(e);
      for (t = 0, r = n.length; t < r; t++)
        if (n[t] === e)
          return t;
      return -1;
    },
    forEach: function(n, e, t) {
      var r, s;
      if (Array.prototype.forEach)
        return n.forEach(e, t);
      for (r = 0, s = n.length; r < s; r++)
        e.call(t, n[r], r, n);
    },
    trim: function(n) {
      return String.prototype.trim ? n.trim() : n.replace(/(^\s*)|(\s*$)/g, "");
    },
    trimRight: function(n) {
      return String.prototype.trimRight ? n.trimRight() : n.replace(/(\s*$)/g, "");
    }
  }), Wf;
}
var zf, mE;
function qY() {
  if (mE) return zf;
  mE = 1;
  var n = HO();
  function e(t, r) {
    t = n.trimRight(t), t[t.length - 1] !== ";" && (t += ";");
    var s = t.length, a = !1, o = 0, l = 0, c = "";
    function d() {
      if (!a) {
        var m = n.trim(t.slice(o, l)), g = m.indexOf(":");
        if (g !== -1) {
          var b = n.trim(m.slice(0, g)), _ = n.trim(m.slice(g + 1));
          if (b) {
            var x = r(o, c.length, b, _, m);
            x && (c += x + "; ");
          }
        }
      }
      o = l + 1;
    }
    for (; l < s; l++) {
      var f = t[l];
      if (f === "/" && t[l + 1] === "*") {
        var p = t.indexOf("*/", l + 2);
        if (p === -1) break;
        l = p + 1, o = l + 1, a = !1;
      } else f === "(" ? a = !0 : f === ")" ? a = !1 : f === ";" ? a || d() : f === `
` && d();
    }
    return n.trim(c);
  }
  return zf = e, zf;
}
var Uf, vE;
function LY() {
  if (vE) return Uf;
  vE = 1;
  var n = YO(), e = qY();
  HO();
  function t(a) {
    return a == null;
  }
  function r(a) {
    var o = {};
    for (var l in a)
      o[l] = a[l];
    return o;
  }
  function s(a) {
    a = r(a || {}), a.whiteList = a.whiteList || n.whiteList, a.onAttr = a.onAttr || n.onAttr, a.onIgnoreAttr = a.onIgnoreAttr || n.onIgnoreAttr, a.safeAttrValue = a.safeAttrValue || n.safeAttrValue, this.options = a;
  }
  return s.prototype.process = function(a) {
    if (a = a || "", a = a.toString(), !a) return "";
    var o = this, l = o.options, c = l.whiteList, d = l.onAttr, f = l.onIgnoreAttr, p = l.safeAttrValue, m = e(a, function(g, b, _, x, T) {
      var S = c[_], k = !1;
      if (S === !0 ? k = S : typeof S == "function" ? k = S(x) : S instanceof RegExp && (k = S.test(x)), k !== !0 && (k = !1), x = p(_, x), !!x) {
        var D = {
          position: b,
          sourcePosition: g,
          source: T,
          isWhite: k
        };
        if (k) {
          var I = d(_, x, D);
          return t(I) ? _ + ":" + x : I;
        } else {
          var I = f(_, x, D);
          if (!t(I))
            return I;
        }
      }
    });
    return m;
  }, Uf = s, Uf;
}
var gE;
function Yp() {
  return gE || (gE = 1, (function(n, e) {
    var t = YO(), r = LY();
    function s(o, l) {
      var c = new r(l);
      return c.process(o);
    }
    e = n.exports = s, e.FilterCSS = r;
    for (var a in t) e[a] = t[a];
    typeof window < "u" && (window.filterCSS = n.exports);
  })(Zs, Zs.exports)), Zs.exports;
}
var Vf, yE;
function Fm() {
  return yE || (yE = 1, Vf = {
    indexOf: function(n, e) {
      var t, r;
      if (Array.prototype.indexOf)
        return n.indexOf(e);
      for (t = 0, r = n.length; t < r; t++)
        if (n[t] === e)
          return t;
      return -1;
    },
    forEach: function(n, e, t) {
      var r, s;
      if (Array.prototype.forEach)
        return n.forEach(e, t);
      for (r = 0, s = n.length; r < s; r++)
        e.call(t, n[r], r, n);
    },
    trim: function(n) {
      return String.prototype.trim ? n.trim() : n.replace(/(^\s*)|(\s*$)/g, "");
    },
    spaceIndex: function(n) {
      var e = /\s|\n|\t/, t = e.exec(n);
      return t ? t.index : -1;
    }
  }), Vf;
}
var bE;
function GO() {
  if (bE) return We;
  bE = 1;
  var n = Yp().FilterCSS, e = Yp().getDefaultWhiteList, t = Fm();
  function r() {
    return {
      a: ["target", "href", "title"],
      abbr: ["title"],
      address: [],
      area: ["shape", "coords", "href", "alt"],
      article: [],
      aside: [],
      audio: [
        "autoplay",
        "controls",
        "crossorigin",
        "loop",
        "muted",
        "preload",
        "src"
      ],
      b: [],
      bdi: ["dir"],
      bdo: ["dir"],
      big: [],
      blockquote: ["cite"],
      br: [],
      caption: [],
      center: [],
      cite: [],
      code: [],
      col: ["align", "valign", "span", "width"],
      colgroup: ["align", "valign", "span", "width"],
      dd: [],
      del: ["datetime"],
      details: ["open"],
      div: [],
      dl: [],
      dt: [],
      em: [],
      figcaption: [],
      figure: [],
      font: ["color", "size", "face"],
      footer: [],
      h1: [],
      h2: [],
      h3: [],
      h4: [],
      h5: [],
      h6: [],
      header: [],
      hr: [],
      i: [],
      img: ["src", "alt", "title", "width", "height", "loading"],
      ins: ["datetime"],
      kbd: [],
      li: [],
      mark: [],
      nav: [],
      ol: [],
      p: [],
      pre: [],
      s: [],
      section: [],
      small: [],
      span: [],
      sub: [],
      summary: [],
      sup: [],
      strong: [],
      strike: [],
      table: ["width", "border", "align", "valign"],
      tbody: ["align", "valign"],
      td: ["width", "rowspan", "colspan", "align", "valign"],
      tfoot: ["align", "valign"],
      th: ["width", "rowspan", "colspan", "align", "valign"],
      thead: ["align", "valign"],
      tr: ["rowspan", "align", "valign"],
      tt: [],
      u: [],
      ul: [],
      video: [
        "autoplay",
        "controls",
        "crossorigin",
        "loop",
        "muted",
        "playsinline",
        "poster",
        "preload",
        "src",
        "height",
        "width"
      ]
    };
  }
  var s = new n();
  function a(Y, J, z) {
  }
  function o(Y, J, z) {
  }
  function l(Y, J, z) {
  }
  function c(Y, J, z) {
  }
  function d(Y) {
    return Y.replace(p, "&lt;").replace(m, "&gt;");
  }
  function f(Y, J, z, te) {
    if (z = q(z), J === "href" || J === "src") {
      if (z = t.trim(z), z === "#") return "#";
      if (!(z.substr(0, 7) === "http://" || z.substr(0, 8) === "https://" || z.substr(0, 7) === "mailto:" || z.substr(0, 4) === "tel:" || z.substr(0, 11) === "data:image/" || z.substr(0, 6) === "ftp://" || z.substr(0, 2) === "./" || z.substr(0, 3) === "../" || z[0] === "#" || z[0] === "/"))
        return "";
    } else if (J === "background") {
      if (S.lastIndex = 0, S.test(z))
        return "";
    } else if (J === "style") {
      if (k.lastIndex = 0, k.test(z) || (D.lastIndex = 0, D.test(z) && (S.lastIndex = 0, S.test(z))))
        return "";
      te !== !1 && (te = te || s, z = te.process(z));
    }
    return z = F(z), z;
  }
  var p = /</g, m = />/g, g = /"/g, b = /&quot;/g, _ = /&#([a-zA-Z0-9]*);?/gim, x = /&colon;?/gim, T = /&newline;?/gim, S = /((j\s*a\s*v\s*a|v\s*b|l\s*i\s*v\s*e)\s*s\s*c\s*r\s*i\s*p\s*t\s*|m\s*o\s*c\s*h\s*a):/gi, k = /e\s*x\s*p\s*r\s*e\s*s\s*s\s*i\s*o\s*n\s*\(.*/gi, D = /u\s*r\s*l\s*\(.*/gi;
  function I(Y) {
    return Y.replace(g, "&quot;");
  }
  function C(Y) {
    return Y.replace(b, '"');
  }
  function P(Y) {
    return Y.replace(_, function(z, te) {
      return te[0] === "x" || te[0] === "X" ? String.fromCharCode(parseInt(te.substr(1), 16)) : String.fromCharCode(parseInt(te, 10));
    });
  }
  function R(Y) {
    return Y.replace(x, ":").replace(T, " ");
  }
  function M(Y) {
    for (var J = "", z = 0, te = Y.length; z < te; z++)
      J += Y.charCodeAt(z) < 32 ? " " : Y.charAt(z);
    return t.trim(J);
  }
  function q(Y) {
    return Y = C(Y), Y = P(Y), Y = R(Y), Y = M(Y), Y;
  }
  function F(Y) {
    return Y = I(Y), Y = d(Y), Y;
  }
  function H() {
    return "";
  }
  function Z(Y, J) {
    typeof J != "function" && (J = function() {
    });
    var z = !Array.isArray(Y);
    function te(ae) {
      return z ? !0 : t.indexOf(Y, ae) !== -1;
    }
    var K = [], re = !1;
    return {
      onIgnoreTag: function(ae, be, se) {
        if (te(ae))
          if (se.isClosing) {
            var j = "[/removed]", xe = se.position + j.length;
            return K.push([
              re !== !1 ? re : se.position,
              xe
            ]), re = !1, j;
          } else
            return re || (re = se.position), "[removed]";
        else
          return J(ae, be, se);
      },
      remove: function(ae) {
        var be = "", se = 0;
        return t.forEach(K, function(j) {
          be += ae.slice(se, j[0]), se = j[1];
        }), be += ae.slice(se), be;
      }
    };
  }
  function ve(Y) {
    for (var J = "", z = 0; z < Y.length; ) {
      var te = Y.indexOf("<!--", z);
      if (te === -1) {
        J += Y.slice(z);
        break;
      }
      J += Y.slice(z, te);
      var K = Y.indexOf("-->", te);
      if (K === -1)
        break;
      z = K + 3;
    }
    return J;
  }
  function Me(Y) {
    var J = Y.split("");
    return J = J.filter(function(z) {
      var te = z.charCodeAt(0);
      return te === 127 ? !1 : te <= 31 ? te === 10 || te === 13 : !0;
    }), J.join("");
  }
  return We.whiteList = r(), We.getDefaultWhiteList = r, We.onTag = a, We.onIgnoreTag = o, We.onTagAttr = l, We.onIgnoreTagAttr = c, We.safeAttrValue = f, We.escapeHtml = d, We.escapeQuote = I, We.unescapeQuote = C, We.escapeHtmlEntities = P, We.escapeDangerHtml5Entities = R, We.clearNonPrintableCharacter = M, We.friendlyAttrValue = q, We.escapeAttrValue = F, We.onIgnoreTagStripAll = H, We.StripTagBody = Z, We.stripCommentTag = ve, We.stripBlankChar = Me, We.attributeWrapSign = '"', We.cssFilter = s, We.getDefaultCSSWhiteList = e, We;
}
var Qs = {}, _E;
function BO() {
  if (_E) return Qs;
  _E = 1;
  var n = Fm();
  function e(p) {
    var m = n.spaceIndex(p), g;
    return m === -1 ? g = p.slice(1, -1) : g = p.slice(1, m + 1), g = n.trim(g).toLowerCase(), g.slice(0, 1) === "/" && (g = g.slice(1)), g.slice(-1) === "/" && (g = g.slice(0, -1)), g;
  }
  function t(p) {
    return p.slice(0, 2) === "</";
  }
  function r(p, m, g) {
    var b = "", _ = 0, x = !1, T = !1, S = 0, k = p.length, D = "", I = "";
    e: for (S = 0; S < k; S++) {
      var C = p.charAt(S);
      if (x === !1) {
        if (C === "<") {
          x = S;
          continue;
        }
      } else if (T === !1) {
        if (C === "<") {
          b += g(p.slice(_, S)), x = S, _ = S;
          continue;
        }
        if (C === ">" || S === k - 1) {
          b += g(p.slice(_, x)), I = p.slice(x, S + 1), D = e(I), b += m(
            x,
            b.length,
            D,
            I,
            t(I)
          ), _ = S + 1, x = !1;
          continue;
        }
        if (C === '"' || C === "'")
          for (var P = 1, R = p.charAt(S - P); R.trim() === "" || R === "="; ) {
            if (R === "=") {
              T = C;
              continue e;
            }
            R = p.charAt(S - ++P);
          }
      } else if (C === T) {
        T = !1;
        continue;
      }
    }
    return _ < k && (b += g(p.substr(_))), b;
  }
  var s = /[^a-zA-Z0-9\\_:.-]/gim;
  function a(p, m) {
    var g = 0, b = 0, _ = [], x = !1, T = p.length;
    function S(P, R) {
      if (P = n.trim(P), P = P.replace(s, "").toLowerCase(), !(P.length < 1)) {
        var M = m(P, R || "");
        M && _.push(M);
      }
    }
    for (var k = 0; k < T; k++) {
      var D = p.charAt(k), I, C;
      if (x === !1 && D === "=") {
        x = p.slice(g, k), g = k + 1, b = p.charAt(g) === '"' || p.charAt(g) === "'" ? g : l(p, k + 1);
        continue;
      }
      if (x !== !1 && k === b) {
        if (C = p.indexOf(D, k + 1), C === -1)
          break;
        I = n.trim(p.slice(b + 1, C)), S(x, I), x = !1, k = C, g = k + 1;
        continue;
      }
      if (/\s|\n|\t/.test(D))
        if (p = p.replace(/\s|\n|\t/g, " "), x === !1)
          if (C = o(p, k), C === -1) {
            I = n.trim(p.slice(g, k)), S(I), x = !1, g = k + 1;
            continue;
          } else {
            k = C - 1;
            continue;
          }
        else if (C = c(p, k - 1), C === -1) {
          I = n.trim(p.slice(g, k)), I = f(I), S(x, I), x = !1, g = k + 1;
          continue;
        } else
          continue;
    }
    return g < p.length && (x === !1 ? S(p.slice(g)) : S(x, f(n.trim(p.slice(g))))), n.trim(_.join(" "));
  }
  function o(p, m) {
    for (; m < p.length; m++) {
      var g = p[m];
      if (g !== " ")
        return g === "=" ? m : -1;
    }
  }
  function l(p, m) {
    for (; m < p.length; m++) {
      var g = p[m];
      if (g !== " ")
        return g === "'" || g === '"' ? m : -1;
    }
  }
  function c(p, m) {
    for (; m > 0; m--) {
      var g = p[m];
      if (g !== " ")
        return g === "=" ? m : -1;
    }
  }
  function d(p) {
    return p[0] === '"' && p[p.length - 1] === '"' || p[0] === "'" && p[p.length - 1] === "'";
  }
  function f(p) {
    return d(p) ? p.substr(1, p.length - 2) : p;
  }
  return Qs.parseTag = r, Qs.parseAttr = a, Qs;
}
var Kf, wE;
function FY() {
  if (wE) return Kf;
  wE = 1;
  var n = Yp().FilterCSS, e = GO(), t = BO(), r = t.parseTag, s = t.parseAttr, a = Fm();
  function o(p) {
    return p == null;
  }
  function l(p) {
    var m = a.spaceIndex(p);
    if (m === -1)
      return {
        html: "",
        closing: p[p.length - 2] === "/"
      };
    p = a.trim(p.slice(m + 1, -1));
    var g = p[p.length - 1] === "/";
    return g && (p = a.trim(p.slice(0, -1))), {
      html: p,
      closing: g
    };
  }
  function c(p) {
    var m = {};
    for (var g in p)
      m[g] = p[g];
    return m;
  }
  function d(p) {
    var m = {};
    for (var g in p)
      Array.isArray(p[g]) ? m[g.toLowerCase()] = p[g].map(function(b) {
        return b.toLowerCase();
      }) : m[g.toLowerCase()] = p[g];
    return m;
  }
  function f(p) {
    p = c(p || {}), p.stripIgnoreTag && (p.onIgnoreTag && console.error(
      'Notes: cannot use these two options "stripIgnoreTag" and "onIgnoreTag" at the same time'
    ), p.onIgnoreTag = e.onIgnoreTagStripAll), p.whiteList || p.allowList ? p.whiteList = d(p.whiteList || p.allowList) : p.whiteList = e.whiteList, this.attributeWrapSign = p.singleQuotedAttributeValue === !0 ? "'" : e.attributeWrapSign, p.onTag = p.onTag || e.onTag, p.onTagAttr = p.onTagAttr || e.onTagAttr, p.onIgnoreTag = p.onIgnoreTag || e.onIgnoreTag, p.onIgnoreTagAttr = p.onIgnoreTagAttr || e.onIgnoreTagAttr, p.safeAttrValue = p.safeAttrValue || e.safeAttrValue, p.escapeHtml = p.escapeHtml || e.escapeHtml, this.options = p, p.css === !1 ? this.cssFilter = !1 : (p.css = p.css || {}, this.cssFilter = new n(p.css));
  }
  return f.prototype.process = function(p) {
    if (p = p || "", p = p.toString(), !p) return "";
    var m = this, g = m.options, b = g.whiteList, _ = g.onTag, x = g.onIgnoreTag, T = g.onTagAttr, S = g.onIgnoreTagAttr, k = g.safeAttrValue, D = g.escapeHtml, I = m.attributeWrapSign, C = m.cssFilter;
    g.stripBlankChar && (p = e.stripBlankChar(p)), g.allowCommentTag || (p = e.stripCommentTag(p));
    var P = !1;
    g.stripIgnoreTagBody && (P = e.StripTagBody(
      g.stripIgnoreTagBody,
      x
    ), x = P.onIgnoreTag);
    var R = r(
      p,
      function(M, q, F, H, Z) {
        var ve = {
          sourcePosition: M,
          position: q,
          isClosing: Z,
          isWhite: Object.prototype.hasOwnProperty.call(b, F)
        }, Me = _(F, H, ve);
        if (!o(Me)) return Me;
        if (ve.isWhite) {
          if (ve.isClosing)
            return "</" + F + ">";
          var Y = l(H), J = b[F], z = s(Y.html, function(te, K) {
            var re = a.indexOf(J, te) !== -1, ae = T(F, te, K, re);
            return o(ae) ? re ? (K = k(F, te, K, C), K ? te + "=" + I + K + I : te) : (ae = S(F, te, K, re), o(ae) ? void 0 : ae) : ae;
          });
          return H = "<" + F, z && (H += " " + z), Y.closing && (H += " /"), H += ">", H;
        } else
          return Me = x(F, H, ve), o(Me) ? D(H) : Me;
      },
      D
    );
    return P && (R = P.remove(R)), R;
  }, Kf = f, Kf;
}
var xE;
function $Y() {
  return xE || (xE = 1, (function(n, e) {
    var t = GO(), r = BO(), s = FY();
    function a(l, c) {
      var d = new s(c);
      return d.process(l);
    }
    e = n.exports = a, e.filterXSS = a, e.FilterXSS = s, (function() {
      for (var l in t)
        e[l] = t[l];
      for (var c in r)
        e[c] = r[c];
    })(), typeof window < "u" && (window.filterXSS = n.exports);
    function o() {
      return typeof self < "u" && typeof DedicatedWorkerGlobalScope < "u" && self instanceof DedicatedWorkerGlobalScope;
    }
    o() && (self.filterXSS = n.exports);
  })(Xs, Xs.exports)), Xs.exports;
}
var jY = $Y(), YY = /* @__PURE__ */ Q(jY);
function SE(n, e) {
  var t = ct(n);
  if (la) {
    var r = la(n);
    e && (r = ie(r).call(r, function(s) {
      return Ra(n, s).enumerable;
    })), t.push.apply(t, r);
  }
  return t;
}
function TE(n) {
  for (var e = 1; e < arguments.length; e++) {
    var t, r, s = arguments[e] != null ? arguments[e] : {};
    e % 2 ? B(t = SE(Object(s), !0)).call(t, function(a) {
      Le(n, a, s[a]);
    }) : ua ? OD(n, ua(s)) : B(r = SE(Object(s))).call(r, function(a) {
      Na(n, a, Ra(s, a));
    });
  }
  return n;
}
function Hr(n) {
  var e;
  if (!n)
    return !1;
  let t = (e = n.idProp) !== null && e !== void 0 ? e : n._idProp;
  return t ? IY(t, n) : !1;
}
const Xf = /^\/?Date\((-?\d+)/i, HY = /^\d+$/;
function $m(n, e) {
  let t;
  if (n !== void 0) {
    if (n === null)
      return null;
    if (!e)
      return n;
    if (typeof e != "string" && !(e instanceof String))
      throw new Error("Type must be a string");
    switch (e) {
      case "boolean":
      case "Boolean":
        return !!n;
      case "number":
      case "Number":
        return Kt(n) && !isNaN(Date.parse(n)) ? tr(n).valueOf() : Number(n.valueOf());
      case "string":
      case "String":
        return String(n);
      case "Date":
        try {
          return $m(n, "Moment").toDate();
        } catch (r) {
          throw r instanceof TypeError ? new TypeError("Cannot convert object of type " + Hn(n) + " to type " + e, {
            cause: r
          }) : r;
        }
      case "Moment":
        if (Wn(n))
          return tr(n);
        if (n instanceof Date)
          return tr(n.valueOf());
        if (tr.isMoment(n))
          return tr(n);
        if (Kt(n))
          return t = Xf.exec(n), t ? tr(Number(t[1])) : (t = HY.exec(n), tr(t ? Number(n) : n));
        throw new TypeError("Cannot convert object of type " + Hn(n) + " to type " + e);
      case "ISODate":
        if (Wn(n))
          return new Date(n);
        if (n instanceof Date)
          return n.toISOString();
        if (tr.isMoment(n))
          return n.toDate().toISOString();
        if (Kt(n))
          return t = Xf.exec(n), t ? new Date(Number(t[1])).toISOString() : tr(n).format();
        throw new Error("Cannot convert object of type " + Hn(n) + " to type ISODate");
      case "ASPDate":
        if (Wn(n))
          return "/Date(" + n + ")/";
        if (n instanceof Date || tr.isMoment(n))
          return "/Date(" + n.valueOf() + ")/";
        if (Kt(n)) {
          t = Xf.exec(n);
          let r;
          return t ? r = new Date(Number(t[1])).valueOf() : r = new Date(n).valueOf(), "/Date(" + r + ")/";
        } else
          throw new Error("Cannot convert object of type " + Hn(n) + " to type ASPDate");
      default:
        throw new Error("Unknown type ".concat(e));
    }
  }
}
function cn(n) {
  var e, t, r, s, a, o;
  let l = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : {
    start: "Date",
    end: "Date"
  };
  const c = n._idProp, d = new Oi({
    fieldId: c
  }), f = Ae(e = OY(n)).call(e, (p) => {
    var m;
    return Aa(m = ct(p)).call(m, (g, b) => (g[b] = $m(p[b], l[b]), g), {});
  }).to(d);
  return f.all().start(), {
    // Write only.
    add: function() {
      return n.getDataSet().add(...arguments);
    },
    remove: function() {
      return n.getDataSet().remove(...arguments);
    },
    update: function() {
      return n.getDataSet().update(...arguments);
    },
    updateOnly: function() {
      return n.getDataSet().updateOnly(...arguments);
    },
    clear: function() {
      return n.getDataSet().clear(...arguments);
    },
    // Read only.
    forEach: N(t = B(d)).call(t, d),
    get: N(r = d.get).call(r, d),
    getIds: N(s = d.getIds).call(s, d),
    off: N(a = d.off).call(a, d),
    on: N(o = d.on).call(o, d),
    get length() {
      return d.length;
    },
    // Non standard.
    idProp: c,
    type: l,
    rawDS: n,
    coercedDS: d,
    dispose: () => f.stop()
  };
}
const WO = (n) => {
  const e = new YY.FilterXSS(n);
  return (t) => typeof t == "string" ? e.process(t) : t;
}, GY = (n) => n;
let Hp = WO();
const BY = (n) => {
  n && (n.disabled === !0 ? (Hp = GY, console.warn("You disabled XSS protection for vis-Timeline. I sure hope you know what you're doing!")) : n.filterOptions && (Hp = WO(n.filterOptions)));
}, E = TE(TE({}, FO), {}, {
  convert: $m,
  setupXSSProtection: BY
});
Na(E, "xss", {
  get: function() {
    return Hp;
  }
});
const Wt = 1e-3;
function zO(n) {
  gt(n).call(n, (e, t) => e.data.start - t.data.start);
}
function UO(n) {
  gt(n).call(n, (e, t) => {
    const r = "end" in e.data ? e.data.end : e.data.start, s = "end" in t.data ? t.data.end : t.data.start;
    return r - s;
  });
}
function Gp(n, e, t, r) {
  return jm(n, e.item, !1, (a) => a.stack && (t || a.top === null), (a) => a.stack, () => e.axis, r) === null;
}
function VO(n, e, t) {
  const r = jm(n, e.item, !1, (s) => s.stack, () => !0, (s) => s.baseTop);
  t.height = r - t.top + 0.5 * e.item.vertical;
}
function KO(n, e, t, r) {
  for (let s = 0; s < n.length; s++) {
    if (n[s].data.subgroup == null) {
      n[s].top = e.item.vertical;
      continue;
    }
    if (n[s].data.subgroup === void 0 || !r) continue;
    let a = 0;
    for (const o in t)
      !Object.prototype.hasOwnProperty.call(t, o) || t[o].visible !== !0 || t[o].index >= t[n[s].data.subgroup].index || (a += t[o].height, t[n[s].data.subgroup].top = a);
    n[s].top = a + 0.5 * e.item.vertical;
  }
  r || XO(n, e, t);
}
function XO(n, e, t) {
  jm(_m(t).toSorted((r, s) => r.index > s.index ? 1 : r.index < s.index ? -1 : 0), {
    vertical: 0
  }, !0, () => !0, () => !0, () => 0);
  for (let r = 0; r < n.length; r++)
    n[r].data.subgroup !== void 0 && (n[r].top = t[n[r].data.subgroup].top + 0.5 * e.item.vertical);
}
function Bp(n, e, t) {
  let r = !1;
  const s = [];
  for (let a in t)
    Object.prototype.hasOwnProperty.call(t[a], "index") ? s[t[a].index] = a : s.push(a);
  for (let a = 0; a < s.length; a++) {
    let o = s[a];
    if (!Object.prototype.hasOwnProperty.call(t, o)) continue;
    r = r || t[o].stack, t[o].top = 0;
    for (const c in t)
      t[c].visible && t[o].index > t[c].index && (t[o].top += t[c].height);
    const l = n[o];
    for (let c = 0; c < l.length; c++)
      l[c].data.subgroup !== void 0 && (l[c].top = t[l[c].data.subgroup].top + 0.5 * e.item.vertical, t[o].stack && (l[c].baseTop = l[c].top));
    r && t[o].stack && VO(n[o], e, t[o]);
  }
}
function jm(n, e, t, r, s, a, o) {
  let l = (T) => T.start, c = (T) => T.end;
  t || (!!(n[0] && n[0].options.rtl) ? l = (S) => S.right : l = (S) => S.left, c = (S) => l(S) + S.width + e.horizontal);
  const d = [], f = [];
  let p = null, m = 0;
  for (const T of n)
    if (r(T))
      d.push(T);
    else if (s(T)) {
      const S = l(T);
      p !== null && S < p - Wt && (m = 0), p = S, m = Js(f, (k) => l(k) - Wt > S, m), Mt(f).call(f, m, 0, T), m++;
    }
  p = null;
  let g = null;
  m = 0;
  let b = 0, _ = 0, x = 0;
  for (; d.length > 0; ) {
    const T = d.shift();
    T.top = a(T);
    const S = l(T), k = c(T);
    p !== null && S < p - Wt && (b = 0, _ = 0, m = 0, g = null), (p === null || S > p + Wt) && (b = Js(f, (C) => S < c(C) - Wt, b)), p = S, (g === null || g < k - Wt) && (_ = Js(f, (C) => k < l(C) - Wt, Math.max(b, _))), g !== null && g - Wt > k && (_ = zY(f, (C) => k + Wt >= l(C), b, _) + 1), g = k;
    const D = UY(f, (C) => S < c(C) - Wt, b, _).toSorted((C, P) => C.top - P.top);
    for (let C = 0; C < D.length; C++) {
      const P = D[C];
      WY(T, P, e) && (T.top = P.top + P.height + e.vertical);
    }
    s(T) && (m = Js(f, (C) => l(C) - Wt > S, m), Mt(f).call(f, m, 0, T), m < b && b++, m <= _ && _++, m++);
    const I = T.top + T.height;
    if (I > x && (x = I), o && o())
      return null;
  }
  return x;
}
function WY(n, e, t) {
  return n.top - t.vertical + Wt < e.top + e.height && n.top + n.height + t.vertical - Wt > e.top;
}
function Js(n, e, t) {
  t || (t = 0);
  for (let r = t; r < n.length; r++)
    if (e(n[r]))
      return r;
  return n.length;
}
function zY(n, e, t, r) {
  t || (t = 0), r || (r = n.length);
  for (let s = r - 1; s >= t; s--)
    if (e(n[s]))
      return s;
  return t - 1;
}
function UY(n, e, t, r) {
  t || (t = 0), r ? r = Math.min(r, n.length) : r = n.length;
  const s = [];
  for (let a = t; a < r; a++)
    e(n[a]) && s.push(n[a]);
  return s;
}
var VY = /* @__PURE__ */ Object.freeze({
  __proto__: null,
  nostack: KO,
  orderByEnd: UO,
  orderByStart: zO,
  stack: Gp,
  stackSubgroups: XO,
  stackSubgroupsWithInnerStack: Bp,
  substack: VO
});
const KY = "__background__", EE = {
  BACKGROUND: KY
};
class ma {
  /**
   * @param {number | string} groupId
   * @param {Object} data
   * @param {ItemSet} itemSet
   * @constructor Group
   */
  constructor(e, t, r) {
    if (this.groupId = e, this.subgroups = {}, this.subgroupStack = {}, this.subgroupStackAll = !1, this.subgroupVisibility = {}, this.doInnerStack = !1, this.shouldBailStackItems = !1, this.subgroupIndex = 0, this.subgroupOrderer = t && t.subgroupOrder, this.itemSet = r, this.isVisible = null, this.height = 0, this.stackDirty = !0, this._disposeCallbacks = [], t && t.nestedGroups && (this.nestedGroups = t.nestedGroups, t.showNested == !1 ? this.showNested = !1 : this.showNested = !0), t && t.subgroupStack)
      if (typeof t.subgroupStack == "boolean")
        this.doInnerStack = t.subgroupStack, this.subgroupStackAll = t.subgroupStack;
      else
        for (const a in t.subgroupStack)
          Object.prototype.hasOwnProperty.call(t.subgroupStack, a) && (this.subgroupStack[a] = t.subgroupStack[a], this.doInnerStack = this.doInnerStack || t.subgroupStack[a]);
    t && t.heightMode ? this.heightMode = t.heightMode : this.heightMode = r.options.groupHeightMode, this.nestedInGroup = null, this.dom = {}, this.props = {
      label: {
        width: 0,
        height: 0
      }
    }, this.className = null, this.items = {}, this.visibleItems = [], this.itemsInRange = [], this.orderedItems = {
      byStart: [],
      byEnd: []
    }, this.checkRangedItems = !1;
    const s = () => {
      this.checkRangedItems = !0;
    };
    this.itemSet.body.emitter.on("checkRangedItems", s), this._disposeCallbacks.push(() => {
      this.itemSet.body.emitter.off("checkRangedItems", s);
    }), this._create(), this.setData(t);
  }
  /**
   * Create DOM elements for the group
   * @private
   */
  _create() {
    const e = document.createElement("div");
    this.itemSet.options.groupEditable.order ? e.className = "vis-label draggable" : e.className = "vis-label", this.dom.label = e;
    const t = document.createElement("div");
    t.className = "vis-inner", e.appendChild(t), this.dom.inner = t;
    const r = document.createElement("div");
    r.className = "vis-group", r["vis-group"] = this, this.dom.foreground = r, this.dom.background = document.createElement("div"), this.dom.background.className = "vis-group", this.dom.axis = document.createElement("div"), this.dom.axis.className = "vis-group", this.dom.marker = document.createElement("div"), this.dom.marker.style.visibility = "hidden", this.dom.marker.style.position = "absolute", this.dom.marker.innerHTML = "", this.dom.background.appendChild(this.dom.marker);
  }
  /**
   * Set the group data for this group
   * @param {Object} data   Group data, can contain properties content and className
   */
  setData(e) {
    if (this.itemSet.groupTouchParams.isDragging) return;
    let t, r;
    if (e && e.subgroupVisibility)
      for (const o in e.subgroupVisibility)
        Object.prototype.hasOwnProperty.call(e.subgroupVisibility, o) && (this.subgroupVisibility[o] = e.subgroupVisibility[o]);
    if (this.itemSet.options && this.itemSet.options.groupTemplate) {
      var s;
      r = N(s = this.itemSet.options.groupTemplate).call(s, this), t = r(e, this.dom.inner);
    } else
      t = e && e.content;
    if (t instanceof Element) {
      for (; this.dom.inner.firstChild; )
        this.dom.inner.removeChild(this.dom.inner.firstChild);
      this.dom.inner.appendChild(t);
    } else t instanceof Object && t.isReactComponent || (t instanceof Object ? r(e, this.dom.inner) : t != null ? this.dom.inner.innerHTML = E.xss(t) : this.dom.inner.innerHTML = E.xss(this.groupId || ""));
    this.dom.label.title = e && e.title || "", this.dom.inner.firstChild ? E.removeClassName(this.dom.inner, "vis-hidden") : E.addClassName(this.dom.inner, "vis-hidden"), e && e.nestedGroups ? ((!this.nestedGroups || this.nestedGroups != e.nestedGroups) && (this.nestedGroups = e.nestedGroups), (e.showNested !== void 0 || this.showNested === void 0) && (e.showNested == !1 ? this.showNested = !1 : this.showNested = !0), E.addClassName(this.dom.label, "vis-nesting-group"), this.showNested ? (E.removeClassName(this.dom.label, "collapsed"), E.addClassName(this.dom.label, "expanded")) : (E.removeClassName(this.dom.label, "expanded"), E.addClassName(this.dom.label, "collapsed"))) : this.nestedGroups && (this.nestedGroups = null, E.removeClassName(this.dom.label, "collapsed"), E.removeClassName(this.dom.label, "expanded"), E.removeClassName(this.dom.label, "vis-nesting-group")), e && (e.treeLevel || e.nestedInGroup) ? (E.addClassName(this.dom.label, "vis-nested-group"), e.treeLevel ? E.addClassName(this.dom.label, "vis-group-level-" + e.treeLevel) : E.addClassName(this.dom.label, "vis-group-level-unknown-but-gte1")) : E.addClassName(this.dom.label, "vis-group-level-0");
    const a = e && e.className || null;
    a != this.className && (this.className && (E.removeClassName(this.dom.label, this.className), E.removeClassName(this.dom.foreground, this.className), E.removeClassName(this.dom.background, this.className), E.removeClassName(this.dom.axis, this.className)), E.addClassName(this.dom.label, a), E.addClassName(this.dom.foreground, a), E.addClassName(this.dom.background, a), E.addClassName(this.dom.axis, a), this.className = a), this.style && (E.removeCssText(this.dom.label, this.style), this.style = null), e && e.style && (E.addCssText(this.dom.label, e.style), this.style = e.style);
  }
  /**
   * Get the width of the group label
   * @return {number} width
   */
  getLabelWidth() {
    return this.props.label.width;
  }
  /**
   * check if group has had an initial height hange
   * @returns {boolean}
   */
  _didMarkerHeightChange() {
    const e = this.dom.marker.clientHeight;
    if (e != this.lastMarkerHeight) {
      this.lastMarkerHeight = e;
      const t = {};
      let r = 0;
      if (B(E).call(E, this.items, (a, o) => {
        a.dirty = !0, a.displayed && (t[o] = a.redraw(!0), r = t[o].length);
      }), r > 0)
        for (let a = 0; a < r; a++)
          B(E).call(E, t, (o) => {
            o[a]();
          });
      return !0;
    } else
      return !1;
  }
  /**
   * calculate group dimentions and position
   * @param {number} pixels
   */
  _calculateGroupSizeAndPosition() {
    const e = this.dom.foreground, t = e.offsetTop, r = e.offsetLeft, s = e.offsetWidth;
    this.top = t, this.right = r, this.width = s;
  }
  /**
   * checks if should bail redraw of items
   * @returns {boolean} should bail
   */
  _shouldBailItemsRedraw() {
    const e = this, t = this.itemSet.options.onTimeout, r = {
      relativeBailingTime: this.itemSet.itemsSettingTime,
      bailTimeMs: t && t.timeoutMs,
      userBailFunction: t && t.callback,
      shouldBailStackItems: this.shouldBailStackItems
    };
    let s = null;
    if (!this.itemSet.initialDrawDone) {
      if (r.shouldBailStackItems)
        return !0;
      Math.abs(Ut() - new Date(r.relativeBailingTime)) > r.bailTimeMs && (r.userBailFunction && this.itemSet.userContinueNotBail == null ? r.userBailFunction((a) => {
        e.itemSet.userContinueNotBail = a, s = !a;
      }) : e.itemSet.userContinueNotBail == !1 ? s = !0 : s = !1);
    }
    return s;
  }
  /**
   * redraws items
   * @param {boolean} forceRestack
   * @param {boolean} lastIsVisible
   * @param {number} margin
   * @param {object} range
   * @private
   */
  _redrawItems(e, t, r, s) {
    if (e || this.stackDirty || this.isVisible && !t) {
      var o, l, c, d, f, p;
      const x = {
        byEnd: ie(o = this.orderedItems.byEnd).call(o, (D) => !D.isCluster),
        byStart: ie(l = this.orderedItems.byStart).call(l, (D) => !D.isCluster)
      }, T = {
        byEnd: [...new Jr(ie(c = Ae(d = this.orderedItems.byEnd).call(d, (D) => D.cluster)).call(c, (D) => !!D))],
        byStart: [...new Jr(ie(f = Ae(p = this.orderedItems.byStart).call(p, (D) => D.cluster)).call(f, (D) => !!D))]
      }, S = () => {
        var D, I;
        const C = this._updateItemsInRange(x, ie(D = this.visibleItems).call(D, (R) => !R.isCluster), s), P = this._updateClustersInRange(T, ie(I = this.visibleItems).call(I, (R) => R.isCluster), s);
        return [...C, ...P];
      }, k = (D) => {
        let I = {};
        for (const P in this.subgroups) {
          var C;
          if (!Object.prototype.hasOwnProperty.call(this.subgroups, P)) continue;
          const R = ie(C = this.visibleItems).call(C, (M) => M.data.subgroup === P);
          I[P] = D ? R.toSorted((M, q) => D(M.data, q.data)) : R;
        }
        return I;
      };
      if (typeof this.itemSet.options.order == "function") {
        const D = this;
        if (this.doInnerStack && this.itemSet.options.stackSubgroups) {
          const I = k(this.itemSet.options.order);
          Bp(I, r, this.subgroups), this.visibleItems = S(), this._updateSubGroupHeights(r);
        } else {
          var m, g, b;
          this.visibleItems = S(), this._updateSubGroupHeights(r);
          const I = ie(m = Tn(g = this.visibleItems).call(g)).call(m, (C) => C.isCluster || !C.isCluster && !C.cluster).toSorted((C, P) => D.itemSet.options.order(C.data, P.data));
          this.shouldBailStackItems = Gp(I, r, !0, N(b = this._shouldBailItemsRedraw).call(b, this));
        }
      } else if (this.visibleItems = S(), this._updateSubGroupHeights(r), this.itemSet.options.stack)
        if (this.doInnerStack && this.itemSet.options.stackSubgroups) {
          const D = k();
          Bp(D, r, this.subgroups);
        } else {
          var _;
          this.shouldBailStackItems = Gp(this.visibleItems, r, !0, N(_ = this._shouldBailItemsRedraw).call(_, this));
        }
      else
        KO(this.visibleItems, r, this.subgroups, this.itemSet.options.stackSubgroups);
      for (let D = 0; D < this.visibleItems.length; D++)
        this.visibleItems[D].repositionX(), this.subgroupVisibility[this.visibleItems[D].data.subgroup] !== void 0 && (this.subgroupVisibility[this.visibleItems[D].data.subgroup] || this.visibleItems[D].hide());
      this.itemSet.options.cluster && B(E).call(E, this.items, (D) => {
        D.cluster && D.displayed && D.hide();
      }), this.shouldBailStackItems && this.itemSet.body.emitter.emit("destroyTimeline"), this.stackDirty = !1;
    }
  }
  /**
   * check if group resized
   * @param {boolean} resized
   * @param {number} height
   * @return {boolean} did resize
   */
  _didResize(e, t) {
    e = E.updateProperty(this, "height", t) || e;
    const r = this.dom.inner.clientWidth, s = this.dom.inner.clientHeight;
    return e = E.updateProperty(this.props.label, "width", r) || e, e = E.updateProperty(this.props.label, "height", s) || e, e;
  }
  /**
   * apply group height
   * @param {number} height
   */
  _applyGroupHeight(e) {
    this.dom.background.style.height = "".concat(e, "px"), this.dom.foreground.style.height = "".concat(e, "px"), this.dom.label.style.height = "".concat(e, "px");
  }
  /**
   * update vertical position of items after they are re-stacked and the height of the group is calculated
   * @param {number} margin
   */
  _updateItemsVerticalPosition(e) {
    for (let t = 0, r = this.visibleItems.length; t < r; t++) {
      const s = this.visibleItems[t];
      s.repositionY(e), !this.isVisible && this.groupId != EE.BACKGROUND && s.displayed && s.hide();
    }
  }
  /**
   * Repaint this group
   * @param {{start: number, end: number}} range
   * @param {{item: {horizontal: number, vertical: number}, axis: number}} margin
   * @param {boolean} [forceRestack=false]  Force restacking of all items
   * @param {boolean} [returnQueue=false]  return the queue or if the group resized
   * @return {boolean} Returns true if the group is resized or the redraw queue if returnQueue=true
   */
  redraw(e, t, r, s) {
    var a, o, l, c;
    let d = !1;
    const f = this.isVisible;
    let p;
    const m = [
      () => {
        r = this._didMarkerHeightChange.call(this) || r;
      },
      // recalculate the height of the subgroups
      N(a = this._updateSubGroupHeights).call(a, this, t),
      // calculate actual size and position
      N(o = this._calculateGroupSizeAndPosition).call(o, this),
      () => {
        var g;
        this.isVisible = N(g = this._isGroupVisible).call(g, this)(e, t);
      },
      () => {
        var g;
        N(g = this._redrawItems).call(g, this)(r, f, t, e);
      },
      // update subgroups
      N(l = this._updateSubgroupsSizes).call(l, this),
      () => {
        var g;
        p = this.height = N(g = this._calculateHeight).call(g, this)(t);
      },
      // calculate actual size and position again
      N(c = this._calculateGroupSizeAndPosition).call(c, this),
      () => {
        var g;
        d = N(g = this._didResize).call(g, this)(d, p);
      },
      () => {
        var g;
        N(g = this._applyGroupHeight).call(g, this)(p);
      },
      () => {
        var g;
        N(g = this._updateItemsVerticalPosition).call(g, this)(t);
      },
      () => (!this.isVisible && this.height && (d = !1), d)
    ];
    if (s)
      return m;
    {
      let g;
      return B(m).call(m, (b) => {
        g = b();
      }), g;
    }
  }
  /**
   * recalculate the height of the subgroups
   *
   * @param {{item: timeline.Item}} margin
   * @private
   */
  _updateSubGroupHeights(e) {
    if (ct(this.subgroups).length > 0) {
      const t = this;
      this._resetSubgroups(), B(E).call(E, this.visibleItems, (r) => {
        r.data.subgroup !== void 0 && (t.subgroups[r.data.subgroup].height = Math.max(t.subgroups[r.data.subgroup].height, r.height + e.item.vertical), t.subgroups[r.data.subgroup].visible = typeof this.subgroupVisibility[r.data.subgroup] > "u" ? !0 : !!this.subgroupVisibility[r.data.subgroup]);
      });
    }
  }
  /**
   * check if group is visible
   *
   * @param {timeline.Range} range
   * @param {{axis: timeline.DataAxis}} margin
   * @returns {boolean} is visible
   * @private
   */
  _isGroupVisible(e, t) {
    return this.top <= e.body.domProps.centerContainer.height - e.body.domProps.scrollTop + t.axis && this.top + this.height + t.axis >= -e.body.domProps.scrollTop;
  }
  /**
   * recalculate the height of the group
   * @param {{item: {horizontal: number, vertical: number}, axis: number}} margin
   * @returns {number} Returns the height
   * @private
   */
  _calculateHeight(e) {
    let t, r;
    if (this.heightMode === "fixed" ? r = E.toArray(this.items) : r = this.visibleItems, !this.isVisible && this.height)
      t = Math.max(this.height, this.props.label.height);
    else if (r.length > 0) {
      let s = r[0].top, a = r[0].top + r[0].height;
      if (B(E).call(E, r, (o) => {
        s = Math.min(s, o.top), a = Math.max(a, o.top + o.height);
      }), s > e.axis) {
        const o = s - e.axis;
        a -= o, B(E).call(E, r, (l) => {
          l.top -= o;
        });
      }
      t = Math.ceil(a + e.item.vertical / 2), this.heightMode !== "fitItems" && (t = Math.max(t, this.props.label.height));
    } else
      t = this.props.label.height;
    return t;
  }
  /**
   * Show this group: attach to the DOM
   */
  show() {
    this.dom.label.parentNode || this.itemSet.dom.labelSet.appendChild(this.dom.label), this.dom.foreground.parentNode || this.itemSet.dom.foreground.appendChild(this.dom.foreground), this.dom.background.parentNode || this.itemSet.dom.background.appendChild(this.dom.background), this.dom.axis.parentNode || this.itemSet.dom.axis.appendChild(this.dom.axis);
  }
  /**
   * Hide this group: remove from the DOM
   */
  hide() {
    const e = this.dom.label;
    e.parentNode && e.parentNode.removeChild(e);
    const t = this.dom.foreground;
    t.parentNode && t.parentNode.removeChild(t);
    const r = this.dom.background;
    r.parentNode && r.parentNode.removeChild(r);
    const s = this.dom.axis;
    s.parentNode && s.parentNode.removeChild(s);
  }
  /**
   * Add an item to the group
   * @param {Item} item
   */
  add(e) {
    var t;
    if (this.items[e.id] = e, e.setParent(this), this.stackDirty = !0, e.data.subgroup !== void 0 && (this._addToSubgroup(e), this.orderSubgroups()), !an(t = this.visibleItems).call(t, e)) {
      const r = this.itemSet.body.range;
      this._checkIfVisible(e, this.visibleItems, r);
    }
  }
  /**
   * add item to subgroup
   * @param {object} item
   * @param {string} subgroupId
   */
  _addToSubgroup(e) {
    let t = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : e.data.subgroup;
    t != null && this.subgroups[t] === void 0 && (this.subgroups[t] = {
      height: 0,
      top: 0,
      start: e.data.start,
      end: e.data.end || e.data.start,
      visible: !1,
      index: this.subgroupIndex,
      items: [],
      stack: this.subgroupStackAll || this.subgroupStack[t] || !1
    }, this.subgroupIndex++), new Date(e.data.start) < new Date(this.subgroups[t].start) && (this.subgroups[t].start = e.data.start);
    const r = e.data.end || e.data.start;
    new Date(r) > new Date(this.subgroups[t].end) && (this.subgroups[t].end = r), this.subgroups[t].items.push(e);
  }
  /**
   * update subgroup sizes
   */
  _updateSubgroupsSizes() {
    const e = this;
    if (e.subgroups)
      for (const r in e.subgroups) {
        var t;
        if (!Object.prototype.hasOwnProperty.call(e.subgroups, r)) continue;
        const s = e.subgroups[r].items[0].data.end || e.subgroups[r].items[0].data.start;
        let a = e.subgroups[r].items[0].data.start, o = s - 1;
        B(t = e.subgroups[r].items).call(t, (l) => {
          new Date(l.data.start) < new Date(a) && (a = l.data.start);
          const c = l.data.end || l.data.start;
          new Date(c) > new Date(o) && (o = c);
        }), e.subgroups[r].start = a, e.subgroups[r].end = new Date(o - 1);
      }
  }
  /**
   * order subgroups
   */
  orderSubgroups() {
    if (this.subgroupOrderer !== void 0) {
      const e = [];
      if (typeof this.subgroupOrderer == "string") {
        for (const t in this.subgroups)
          Object.prototype.hasOwnProperty.call(this.subgroups, t) && e.push({
            subgroup: t,
            sortField: this.subgroups[t].items[0].data[this.subgroupOrderer]
          });
        gt(e).call(e, (t, r) => t.sortField - r.sortField);
      } else if (typeof this.subgroupOrderer == "function") {
        for (const t in this.subgroups)
          Object.prototype.hasOwnProperty.call(this.subgroups, t) && e.push(this.subgroups[t].items[0].data);
        gt(e).call(e, this.subgroupOrderer);
      }
      if (e.length > 0)
        for (let t = 0; t < e.length; t++)
          this.subgroups[e[t].subgroup].index = t;
    }
  }
  /**
   * add item to subgroup
   */
  _resetSubgroups() {
    for (const e in this.subgroups)
      Object.prototype.hasOwnProperty.call(this.subgroups, e) && (this.subgroups[e].visible = !1, this.subgroups[e].height = 0);
  }
  /**
   * Remove an item from the group
   * @param {Item} item
   */
  remove(e) {
    var t, r;
    delete this.items[e.id], e.setParent(null), this.stackDirty = !0;
    const s = Fe(t = this.visibleItems).call(t, e);
    s != -1 && Mt(r = this.visibleItems).call(r, s, 1), e.data.subgroup !== void 0 && (this._removeFromSubgroup(e), this.orderSubgroups());
  }
  /**
   * remove item from subgroup
   * @param {object} item
   * @param {string} subgroupId
   */
  _removeFromSubgroup(e) {
    let t = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : e.data.subgroup;
    if (t != null) {
      const a = this.subgroups[t];
      if (a) {
        var r;
        const o = Fe(r = a.items).call(r, e);
        if (o >= 0) {
          var s;
          Mt(s = a.items).call(s, o, 1), a.items.length ? this._updateSubgroupsSizes() : delete this.subgroups[t];
        }
      }
    }
  }
  /**
   * Remove an item from the corresponding DataSet
   * @param {Item} item
   */
  removeFromDataSet(e) {
    this.itemSet.removeItem(e.id);
  }
  /**
   * Reorder the items
   */
  order() {
    const e = E.toArray(this.items), t = [], r = [];
    for (let s = 0; s < e.length; s++)
      e[s].data.end !== void 0 && r.push(e[s]), t.push(e[s]);
    this.orderedItems = {
      byStart: t,
      byEnd: r
    }, zO(this.orderedItems.byStart), UO(this.orderedItems.byEnd);
  }
  /**
   * Update the visible items
   * @param {{byStart: Item[], byEnd: Item[]}} orderedItems   All items ordered by start date and by end date
   * @param {Item[]} oldVisibleItems                          The previously visible items.
   * @param {{start: number, end: number}} range              Visible range
   * @return {Item[]} visibleItems                            The new visible items.
   * @private
   */
  _updateItemsInRange(e, t, r) {
    const s = [], a = {};
    if (!this.isVisible && this.height !== void 0 && this.groupId != EE.BACKGROUND) {
      for (let _ = 0; _ < t.length; _++) {
        var o = t[_];
        o.displayed && o.hide();
      }
      return s;
    }
    const l = (r.end - r.start) / 4, c = r.start - l, d = r.end + l, f = (_) => _ < c ? -1 : _ <= d ? 0 : 1;
    if (t.length > 0)
      for (let _ = 0; _ < t.length; _++)
        this._checkIfVisibleWithReference(t[_], s, a, r);
    const p = E.binarySearchCustom(e.byStart, f, "data", "start");
    this._traceVisible(p, e.byStart, s, a, (_) => _.data.start < c || _.data.start > d);
    for (let _ = 0; _ < e.byEnd.length; _++)
      this._checkIfVisibleWithReference(e.byEnd[_], s, a, r);
    this._sortVisibleItems(e.byStart, s, a);
    const m = {};
    let g = 0;
    for (let _ = 0; _ < s.length; _++) {
      const x = s[_];
      x.displayed || (m[_] = x.redraw(!0), g = m[_].length);
    }
    if (g > 0)
      for (let _ = 0; _ < g; _++)
        B(E).call(E, m, (x) => {
          x[_]();
        });
    for (let _ = 0; _ < s.length; _++)
      s[_].repositionX();
    return s;
  }
  /**
   * trace visible items in group
   * @param {number} initialPos
   * @param {array} items
   * @param {aray} visibleItems
   * @param {object} visibleItemsLookup
   * @param {function} breakCondition
   */
  _traceVisible(e, t, r, s, a) {
    if (e != -1) {
      for (let o = e; o >= 0; o--) {
        let l = t[o];
        if (a(l))
          break;
        !(l.isCluster && !l.hasItems()) && !l.cluster && s[l.id] === void 0 && (s[l.id] = !0, r.unshift(l));
      }
      for (let o = e + 1; o < t.length; o++) {
        let l = t[o];
        if (a(l))
          break;
        !(l.isCluster && !l.hasItems()) && !l.cluster && s[l.id] === void 0 && (s[l.id] = !0, r.push(l));
      }
    }
  }
  /**
   * by-ref reordering of visibleItems array to match
   * the specified item superset order
   * @param {array} orderedItems
   * @param {aray} visibleItems
   * @param {object} visibleItemsLookup
   */
  _sortVisibleItems(e, t, r) {
    t.length = 0;
    for (let s = 0; s < e.length; s++) {
      let a = e[s];
      r[a.id] && t.push(a);
    }
  }
  /**
   * this function is very similar to the _checkIfInvisible() but it does not
   * return booleans, hides the item if it should not be seen and always adds to
   * the visibleItems.
   * this one is for brute forcing and hiding.
   *
   * @param {Item} item
   * @param {Array} visibleItems
   * @param {{start:number, end:number}} range
   * @private
   */
  _checkIfVisible(e, t, r) {
    e.isVisible(r) ? (e.displayed || e.show(), e.repositionX(), t.push(e)) : e.displayed && e.hide();
  }
  /**
   * this function is very similar to the _checkIfInvisible() but it does not
   * return booleans, hides the item if it should not be seen and always adds to
   * the visibleItems.
   * this one is for brute forcing and hiding.
   *
   * @param {Item} item
   * @param {Array.<timeline.Item>} visibleItems
   * @param {Object<number, boolean>} visibleItemsLookup
   * @param {{start:number, end:number}} range
   * @private
   */
  _checkIfVisibleWithReference(e, t, r, s) {
    e.isVisible(s) ? r[e.id] === void 0 && (r[e.id] = !0, t.push(e)) : e.displayed && e.hide();
  }
  /**
   * Update the visible items
   * @param {array} orderedClusters
   * @param {array} oldVisibleClusters
   * @param {{start: number, end: number}} range
   * @return {Item[]} visibleItems
   * @private
   */
  _updateClustersInRange(e, t, r) {
    const s = [], a = {};
    if (t.length > 0)
      for (let f = 0; f < t.length; f++)
        this._checkIfVisibleWithReference(t[f], s, a, r);
    for (let f = 0; f < e.byStart.length; f++)
      this._checkIfVisibleWithReference(e.byStart[f], s, a, r);
    for (let f = 0; f < e.byEnd.length; f++)
      this._checkIfVisibleWithReference(e.byEnd[f], s, a, r);
    const o = {};
    let l = 0;
    for (let f = 0; f < s.length; f++) {
      const p = s[f];
      p.displayed || (o[f] = p.redraw(!0), l = o[f].length);
    }
    if (l > 0)
      for (var d = 0; d < l; d++)
        B(E).call(E, o, function(f) {
          f[d]();
        });
    for (let f = 0; f < s.length; f++)
      s[f].repositionX();
    return s;
  }
  /**
   * change item subgroup
   * @param {object} item
   * @param {string} oldSubgroup
   * @param {string} newSubgroup
   */
  changeSubgroup(e, t, r) {
    this._removeFromSubgroup(e, t), this._addToSubgroup(e, r), this.orderSubgroups();
  }
  /**
   * Call this method before you lose the last reference to an instance of this.
   * It will remove listeners etc.
   */
  dispose() {
    this.hide();
    let e;
    for (; e = this._disposeCallbacks.pop(); )
      e();
  }
}
class Ym extends ma {
  /**
   * @param {number | string} groupId
   * @param {Object} data
   * @param {ItemSet} itemSet
   */
  constructor(e, t, r) {
    super(e, t, r), this.width = 0, this.height = 0, this.top = 0, this.left = 0;
  }
  /**
   * Repaint this group
   * @param {{start: number, end: number}} range
   * @param {{item: {horizontal: number, vertical: number}, axis: number}} margin
   * @return {boolean} Returns true if the group is resized
   */
  redraw(e, t) {
    this.visibleItems = this._updateItemsInRange(this.orderedItems, this.visibleItems, e), this.width = this.dom.background.offsetWidth, this.dom.background.style.height = "0";
    for (let s = 0, a = this.visibleItems.length; s < a; s++)
      this.visibleItems[s].repositionY(t);
    return !1;
  }
  /**
   * Show this group: attach to the DOM
   */
  show() {
    this.dom.background.parentNode || this.itemSet.dom.background.appendChild(this.dom.background);
  }
}
class zr {
  /**
   */
  constructor() {
    this.options = null, this.props = null;
  }
  /**
   * Set options for the component. The new options will be merged into the
   * current options.
   * @param {Object} options
   */
  setOptions(e) {
    e && E.extend(this.options, e);
  }
  /**
   * Repaint the component
   * @return {boolean} Returns true if the component is resized
   */
  redraw() {
    return !1;
  }
  /**
   * Destroy the component. Cleanup DOM and event listeners
   */
  destroy() {
  }
  /**
   * Test whether the component is resized since the last time _isResized() was
   * called.
   * @return {Boolean} Returns true if the component is resized
   * @protected
   */
  _isResized() {
    const e = this.props._previousWidth !== this.props.width || this.props._previousHeight !== this.props.height;
    return this.props._previousWidth = this.props.width, this.props._previousHeight = this.props.height, e;
  }
}
const Hm = {
  current: "current",
  time: "time",
  deleteSelected: "Delete selected"
}, XY = Hm, ZY = Hm, Gm = {
  current: "attuale",
  time: "tempo",
  deleteSelected: "Cancella la selezione"
}, QY = Gm, JY = Gm, Bm = {
  current: "huidige",
  time: "tijd",
  deleteSelected: "Selectie verwijderen"
}, eH = Bm, tH = Bm, Wm = {
  current: "Aktuelle",
  time: "Zeit",
  deleteSelected: "Lösche Auswahl"
}, rH = Wm, iH = Wm, ks = {
  current: "actuel",
  time: "heure",
  deleteSelected: "Effacer la selection"
}, nH = ks, sH = ks, aH = ks, oH = ks, ZO = {
  current: "actual",
  time: "hora",
  deleteSelected: "Eliminar selección"
}, lH = ZO, QO = {
  current: "поточний",
  time: "час",
  deleteSelected: "Видалити обране"
}, uH = QO, JO = {
  current: "текущее",
  time: "время",
  deleteSelected: "Удалить выбранное"
}, cH = JO, ek = {
  current: "aktualny",
  time: "czas",
  deleteSelected: "Usuń wybrane"
}, dH = ek, zm = {
  current: "atual",
  time: "data",
  deleteSelected: "Apagar selecionado"
}, hH = zm, fH = zm, tk = {
  current: "güncel",
  time: "zaman",
  deleteSelected: "Seçileni sil"
}, pH = tk, rk = {
  current: "現在",
  time: "時刻",
  deleteSelected: "選択されたものを削除"
}, mH = rk, ik = {
  current: "nuvarande",
  time: "tid",
  deleteSelected: "Radera valda"
}, vH = ik, Ya = {
  current: "nåværende",
  time: "tid",
  deleteSelected: "Slett valgte"
}, gH = Ya, yH = Ya, bH = Ya, nk = {
  current: "einamas",
  time: "laikas",
  deleteSelected: "Pašalinti pasirinktą"
}, _H = nk, dn = {
  en: Hm,
  en_EN: XY,
  en_US: ZY,
  it: Gm,
  it_IT: QY,
  it_CH: JY,
  nl: Bm,
  nl_NL: eH,
  nl_BE: tH,
  de: Wm,
  de_DE: rH,
  de_CH: iH,
  fr: ks,
  fr_FR: nH,
  fr_CA: sH,
  fr_BE: aH,
  fr_CH: oH,
  es: ZO,
  es_ES: lH,
  uk: QO,
  uk_UA: uH,
  ru: JO,
  ru_RU: cH,
  pl: ek,
  pl_PL: dH,
  pt: zm,
  pt_BR: hH,
  pt_PT: fH,
  tr: tk,
  tr_TR: pH,
  ja: rk,
  ja_JP: mH,
  lt: nk,
  lt_LT: _H,
  sv: ik,
  sv_SE: vH,
  nb: Ya,
  nn: yH,
  nb_NO: gH,
  nn_NO: bH
};
class Um extends zr {
  /**
   * @param {{range: Range, dom: Object, domProps: Object}} body
   * @param {Object} [options]        Available parameters:
   *                                  {Boolean} [showCurrentTime]
   *                                  {String}  [alignCurrentTime]
   * @constructor CurrentTime
   * @extends Component
   */
  constructor(e, t) {
    var r;
    super(), this.body = e, this.defaultOptions = {
      rtl: !1,
      showCurrentTime: !0,
      alignCurrentTime: void 0,
      moment: Et,
      locales: dn,
      locale: "en"
    }, this.options = E.extend({}, this.defaultOptions), this.setOptions(t), this.options.locales = E.extend({}, dn, this.options.locales);
    const s = this.defaultOptions.locales[this.defaultOptions.locale];
    B(r = ct(this.options.locales)).call(r, (a) => {
      this.options.locales[a] = E.extend({}, s, this.options.locales[a]);
    }), this.offset = 0, this._create();
  }
  /**
   * Create the HTML DOM for the current time bar
   * @private
   */
  _create() {
    const e = document.createElement("div");
    e.className = "vis-current-time", e.style.position = "absolute", e.style.top = "0px", e.style.height = "100%", this.bar = e;
  }
  /**
   * Destroy the CurrentTime bar
   */
  destroy() {
    this.options.showCurrentTime = !1, this.redraw(), this.body = null;
  }
  /**
   * Set options for the component. Options will be merged in current options.
   * @param {Object} options  Available parameters:
   *                          {boolean} [showCurrentTime]
   *                          {String}  [alignCurrentTime]
   */
  setOptions(e) {
    e && E.selectiveExtend(["rtl", "showCurrentTime", "alignCurrentTime", "moment", "locale", "locales"], this.options, e);
  }
  /**
   * Repaint the component
   * @return {boolean} Returns true if the component is resized
   */
  redraw() {
    if (this.options.showCurrentTime) {
      var e, t;
      const r = this.body.dom.backgroundVertical;
      this.bar.parentNode != r && (this.bar.parentNode && this.bar.parentNode.removeChild(this.bar), r.appendChild(this.bar), this.start());
      let s = this.options.moment(Ut() + this.offset);
      this.options.alignCurrentTime && (s = s.startOf(this.options.alignCurrentTime));
      const a = this.body.util.toScreen(s);
      let o = this.options.locales[this.options.locale];
      o || (this.warned || (console.warn("WARNING: options.locales['".concat(this.options.locale, "'] not found. See https://visjs.github.io/vis-timeline/docs/timeline/#Localization")), this.warned = !0), o = this.options.locales.en);
      let l = rt(e = rt(t = "".concat(o.current, " ")).call(t, o.time, ": ")).call(e, s.format("dddd, MMMM Do YYYY, H:mm:ss"));
      l = l.charAt(0).toUpperCase() + l.substring(1), this.options.rtl ? this.bar.style.transform = "translateX(".concat(a * -1, "px)") : this.bar.style.transform = "translateX(".concat(a, "px)"), this.bar.title = l;
    } else
      this.bar.parentNode && this.bar.parentNode.removeChild(this.bar), this.stop();
    return !1;
  }
  /**
   * Start auto refreshing the current time bar
   */
  start() {
    const e = this;
    function t() {
      e.stop();
      let s = 1 / e.body.range.conversion(e.body.domProps.center.width).scale / 10;
      s < 30 && (s = 30), s > 1e3 && (s = 1e3), e.redraw(), e.body.emitter.emit("currentTimeTick"), e.currentTimeTimer = Dt(t, s);
    }
    t();
  }
  /**
   * Stop auto refreshing the current time bar
   */
  stop() {
    this.currentTimeTimer !== void 0 && (clearTimeout(this.currentTimeTimer), delete this.currentTimeTimer);
  }
  /**
   * Set a current time. This can be used for example to ensure that a client's
   * time is synchronized with a shared server time.
   * @param {Date | string | number} time     A Date, unix timestamp, or
   *                                          ISO date string.
   */
  setCurrentTime(e) {
    const t = E.convert(e, "Date").valueOf(), r = Ut();
    this.offset = t - r, this.redraw();
  }
  /**
   * Get the current time.
   * @return {Date} Returns the current time.
   */
  getCurrentTime() {
    return new Date(Ut() + this.offset);
  }
}
class Ha extends zr {
  /**
   * @param {{range: Range, dom: Object}} body
   * @param {Object} [options]        Available parameters:
   *                                  {number | string} id
   *                                  {string} locales
   *                                  {string} locale
   * @constructor CustomTime
   * @extends Component
   */
  constructor(e, t) {
    var r;
    super(), this.body = e, this.defaultOptions = {
      moment: Et,
      locales: dn,
      locale: "en",
      id: void 0,
      title: void 0
    }, this.options = E.extend({}, this.defaultOptions), this.setOptions(t), this.options.locales = E.extend({}, dn, this.options.locales);
    const s = this.defaultOptions.locales[this.defaultOptions.locale];
    B(r = ct(this.options.locales)).call(r, (a) => {
      this.options.locales[a] = E.extend({}, s, this.options.locales[a]);
    }), t && t.time != null ? this.customTime = t.time : this.customTime = /* @__PURE__ */ new Date(), this.eventParams = {}, this._create();
  }
  /**
   * Set options for the component. Options will be merged in current options.
   * @param {Object} options  Available parameters:
   *                                  {number | string} id
   *                                  {string} locales
   *                                  {string} locale
   */
  setOptions(e) {
    e && E.selectiveExtend(["moment", "locale", "locales", "id", "title", "rtl", "snap"], this.options, e);
  }
  /**
   * Create the DOM for the custom time
   * @private
   */
  _create() {
    var e, t, r;
    const s = document.createElement("div");
    s["custom-time"] = this, s.className = "vis-custom-time ".concat(this.options.id || ""), s.style.position = "absolute", s.style.top = "0px", s.style.height = "100%", this.bar = s;
    const a = document.createElement("div");
    a.style.position = "relative", a.style.top = "0px", this.options.rtl ? a.style.right = "-10px" : a.style.left = "-10px", a.style.height = "100%", a.style.width = "20px";
    function o(l) {
      this.body.range._onMouseWheel(l);
    }
    a.addEventListener ? (a.addEventListener("mousewheel", N(o).call(o, this), !1), a.addEventListener("DOMMouseScroll", N(o).call(o, this), !1)) : a.attachEvent("onmousewheel", N(o).call(o, this)), s.appendChild(a), this.hammer = new Tt(a), this.hammer.on("panstart", N(e = this._onDragStart).call(e, this)), this.hammer.on("panmove", N(t = this._onDrag).call(t, this)), this.hammer.on("panend", N(r = this._onDragEnd).call(r, this)), this.hammer.get("pan").set({
      threshold: 5,
      direction: Tt.DIRECTION_ALL
    }), this.hammer.get("press").set({
      time: 1e4
    });
  }
  /**
   * Destroy the CustomTime bar
   */
  destroy() {
    this.hide(), this.hammer.destroy(), this.hammer = null, this.body = null;
  }
  /**
   * Repaint the component
   * @return {boolean} Returns true if the component is resized
   */
  redraw() {
    const e = this.body.dom.backgroundVertical;
    this.bar.parentNode != e && (this.bar.parentNode && this.bar.parentNode.removeChild(this.bar), e.appendChild(this.bar));
    const t = this.body.util.toScreen(this.customTime);
    let r = this.options.locales[this.options.locale];
    r || (this.warned || (console.warn("WARNING: options.locales['".concat(this.options.locale, "'] not found. See https://visjs.github.io/vis-timeline/docs/timeline/#Localization")), this.warned = !0), r = this.options.locales.en);
    let s = this.options.title;
    if (s === void 0) {
      var a;
      s = rt(a = "".concat(r.time, ": ")).call(a, this.options.moment(this.customTime).format("dddd, MMMM Do YYYY, H:mm:ss")), s = s.charAt(0).toUpperCase() + s.substring(1);
    } else typeof s == "function" && (s = s.call(this, this.customTime));
    return this.options.rtl ? this.bar.style.right = "".concat(t, "px") : this.bar.style.left = "".concat(t, "px"), this.bar.title = s, !1;
  }
  /**
   * Remove the CustomTime from the DOM
   */
  hide() {
    this.bar.parentNode && this.bar.parentNode.removeChild(this.bar);
  }
  /**
   * Set custom time.
   * @param {Date | number | string} time
   */
  setCustomTime(e) {
    this.customTime = E.convert(e, "Date"), this.redraw();
  }
  /**
   * Retrieve the current custom time.
   * @return {Date} customTime
   */
  getCustomTime() {
    return new Date(this.customTime.valueOf());
  }
  /**
   * Set custom marker.
   * @param {string} [title] Title of the custom marker
   * @param {boolean} [editable] Make the custom marker editable.
   */
  setCustomMarker(e, t) {
    if (this.marker && this.bar.removeChild(this.marker), this.marker = document.createElement("div"), this.marker.className = "vis-custom-time-marker", this.marker.innerHTML = E.xss(e), this.marker.style.position = "absolute", t) {
      var r;
      this.marker.setAttribute("contenteditable", "true"), this.marker.addEventListener("pointerdown", () => {
        this.marker.focus();
      }), this.marker.addEventListener("input", N(r = this._onMarkerChange).call(r, this)), this.marker.title = e, this.marker.addEventListener("blur", (s) => {
        this.title != s.target.innerHTML && (this._onMarkerChanged(s), this.title = s.target.innerHTML);
      });
    }
    this.bar.appendChild(this.marker);
  }
  /**
   * Set custom title.
   * @param {Date | number | string} title
   */
  setCustomTitle(e) {
    this.options.title = e;
  }
  /**
   * Start moving horizontally
   * @param {Event} event
   * @private
   */
  _onDragStart(e) {
    this.eventParams.dragging = !0, this.eventParams.customTime = this.customTime, e.stopPropagation();
  }
  /**
   * Perform moving operating.
   * @param {Event} event
   * @private
   */
  _onDrag(e) {
    if (!this.eventParams.dragging) return;
    let t = this.options.rtl ? -1 * e.deltaX : e.deltaX;
    const r = this.body.util.toScreen(this.eventParams.customTime) + t, s = this.body.util.toTime(r), a = this.body.util.getScale(), o = this.body.util.getStep(), l = this.options.snap, c = l ? l(s, a, o) : s;
    this.setCustomTime(c), this.body.emitter.emit("timechange", {
      id: this.options.id,
      time: new Date(this.customTime.valueOf()),
      event: e
    }), e.stopPropagation();
  }
  /**
   * Stop moving operating.
   * @param {Event} event
   * @private
   */
  _onDragEnd(e) {
    this.eventParams.dragging && (this.body.emitter.emit("timechanged", {
      id: this.options.id,
      time: new Date(this.customTime.valueOf()),
      event: e
    }), e.stopPropagation());
  }
  /**
   * Perform input operating.
   * @param {Event} event
   * @private
   */
  _onMarkerChange(e) {
    this.body.emitter.emit("markerchange", {
      id: this.options.id,
      title: e.target.innerHTML,
      event: e
    }), e.stopPropagation();
  }
  /**
   * Perform change operating.
   * @param {Event} event
   * @private
   */
  _onMarkerChanged(e) {
    this.body.emitter.emit("markerchanged", {
      id: this.options.id,
      title: e.target.innerHTML,
      event: e
    }), e.stopPropagation();
  }
  /**
   * Find a custom time from an event target:
   * searches for the attribute 'custom-time' in the event target's element tree
   * @param {Event} event
   * @return {CustomTime | null} customTime
   */
  static customTimeFromTarget(e) {
    let t = e.target;
    for (; t; ) {
      if (Object.prototype.hasOwnProperty.call(t, "custom-time"))
        return t["custom-time"];
      t = t.parentNode;
    }
    return null;
  }
}
var DE = {}, Zf, OE;
function wH() {
  if (OE) return Zf;
  OE = 1;
  var n = /* @__PURE__ */ Ze(), e = /* @__PURE__ */ we(), t = /* @__PURE__ */ Pe(), r = /* @__PURE__ */ ar(), s = iO().trim, a = /* @__PURE__ */ wm(), o = t("".charAt), l = n.parseFloat, c = n.Symbol, d = c && c.iterator, f = 1 / l(a + "-0") !== -1 / 0 || d && !e(function() {
    l(Object(d));
  });
  return Zf = f ? function(m) {
    var g = s(r(m)), b = l(g);
    return b === 0 && o(g, 0) === "-" ? -0 : b;
  } : l, Zf;
}
var kE;
function xH() {
  if (kE) return DE;
  kE = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ wH();
  return n({ global: !0, forced: parseFloat !== e }, {
    parseFloat: e
  }), DE;
}
var Qf, CE;
function SH() {
  if (CE) return Qf;
  CE = 1, xH();
  var n = /* @__PURE__ */ je();
  return Qf = n.parseFloat, Qf;
}
var Jf, RE;
function TH() {
  if (RE) return Jf;
  RE = 1;
  var n = /* @__PURE__ */ SH();
  return Jf = n, Jf;
}
var ep, PE;
function EH() {
  return PE || (PE = 1, ep = /* @__PURE__ */ TH()), ep;
}
var DH = EH(), va = /* @__PURE__ */ Q(DH);
class sk {
  /**
   *
   * @param {number} start
   * @param {number} end
   * @param {boolean} autoScaleStart
   * @param {boolean} autoScaleEnd
   * @param {number} containerHeight
   * @param {number} majorCharHeight
   * @param {boolean} zeroAlign
   * @param {function} formattingFunction
   * @constructor DataScale
   */
  constructor(e, t, r, s, a, o) {
    let l = arguments.length > 6 && arguments[6] !== void 0 ? arguments[6] : !1, c = arguments.length > 7 && arguments[7] !== void 0 ? arguments[7] : !1;
    if (this.majorSteps = [1, 2, 5, 10], this.minorSteps = [0.25, 0.5, 1, 2], this.customLines = null, this.containerHeight = a, this.majorCharHeight = o, this._start = e, this._end = t, this.scale = 1, this.minorStepIdx = -1, this.magnitudefactor = 1, this.determineScale(), this.zeroAlign = l, this.autoScaleStart = r, this.autoScaleEnd = s, this.formattingFunction = c, r || s) {
      const d = this, f = (p) => {
        const m = p - p % (d.magnitudefactor * d.minorSteps[d.minorStepIdx]);
        return p % (d.magnitudefactor * d.minorSteps[d.minorStepIdx]) > 0.5 * (d.magnitudefactor * d.minorSteps[d.minorStepIdx]) ? m + d.magnitudefactor * d.minorSteps[d.minorStepIdx] : m;
      };
      r && (this._start -= this.magnitudefactor * 2 * this.minorSteps[this.minorStepIdx], this._start = f(this._start)), s && (this._end += this.magnitudefactor * this.minorSteps[this.minorStepIdx], this._end = f(this._end)), this.determineScale();
    }
  }
  /**
   * set chart height
   * @param {number} majorCharHeight
   */
  setCharHeight(e) {
    this.majorCharHeight = e;
  }
  /**
   * set height
   * @param {number} containerHeight
   */
  setHeight(e) {
    this.containerHeight = e;
  }
  /**
   * determine scale
   */
  determineScale() {
    const e = this._end - this._start;
    this.scale = this.containerHeight / e;
    const t = this.majorCharHeight / this.scale, r = e > 0 ? Math.round(Math.log(e) / Math.LN10) : 0;
    this.minorStepIdx = -1, this.magnitudefactor = Math.pow(10, r);
    let s = 0;
    r < 0 && (s = r);
    let a = !1;
    for (let o = s; Math.abs(o) <= Math.abs(r); o++) {
      this.magnitudefactor = Math.pow(10, o);
      for (let l = 0; l < this.minorSteps.length; l++)
        if (this.magnitudefactor * this.minorSteps[l] >= t) {
          a = !0, this.minorStepIdx = l;
          break;
        }
      if (a === !0)
        break;
    }
  }
  /**
   * returns if value is major
   * @param {number} value
   * @returns {boolean}
   */
  is_major(e) {
    return e % (this.magnitudefactor * this.majorSteps[this.minorStepIdx]) === 0;
  }
  /**
   * returns step size
   * @returns {number}
   */
  getStep() {
    return this.magnitudefactor * this.minorSteps[this.minorStepIdx];
  }
  /**
   * returns first major
   * @returns {number}
   */
  getFirstMajor() {
    const e = this.magnitudefactor * this.majorSteps[this.minorStepIdx];
    return this.convertValue(this._start + (e - this._start % e) % e);
  }
  /**
   * returns first major
   * @param {date} current
   * @returns {date} formatted date
   */
  formatValue(e) {
    let t = e.toPrecision(5);
    return typeof this.formattingFunction == "function" && (t = this.formattingFunction(e)), typeof t == "number" ? "".concat(t) : typeof t == "string" ? t : e.toPrecision(5);
  }
  /**
   * returns lines
   * @returns {object} lines
   */
  getLines() {
    const e = [], t = this.getStep(), r = (t - this._start % t) % t;
    for (let s = this._start + r; this._end - s > 1e-5; s += t)
      s != this._start && e.push({
        major: this.is_major(s),
        y: this.convertValue(s),
        val: this.formatValue(s)
      });
    return e;
  }
  /**
   * follow scale
   * @param {object} other
   */
  followScale(e) {
    const t = this.minorStepIdx, r = this._start, s = this._end, a = this, o = () => {
      a.magnitudefactor *= 2;
    }, l = () => {
      a.magnitudefactor /= 2;
    };
    e.minorStepIdx <= 1 && this.minorStepIdx <= 1 || e.minorStepIdx > 1 && this.minorStepIdx > 1 || (e.minorStepIdx < this.minorStepIdx ? (this.minorStepIdx = 1, t == 2 || o(), o()) : (this.minorStepIdx = 2, t == 1 || l(), l()));
    const c = e.convertValue(0), d = e.getStep() * e.scale;
    let f = !1, p = 0;
    for (; !f && p++ < 5; ) {
      this.scale = d / (this.minorSteps[this.minorStepIdx] * this.magnitudefactor);
      const m = this.containerHeight / this.scale;
      this._start = r, this._end = this._start + m;
      const g = this._end * this.scale, b = this.magnitudefactor * this.majorSteps[this.minorStepIdx], _ = this.getFirstMajor() - e.getFirstMajor();
      if (this.zeroAlign) {
        const x = c - g;
        this._end += x / this.scale, this._start = this._end - m;
      } else
        this.autoScaleStart ? (this._start -= _ / this.scale, this._end = this._start + m) : (this._start += b - _ / this.scale, this._end = this._start + m);
      if (!this.autoScaleEnd && this._end > s + 1e-5) {
        l(), f = !1;
        continue;
      }
      if (!this.autoScaleStart && this._start < r - 1e-5)
        if (this.zeroAlign && r >= 0)
          console.warn("Can't adhere to given 'min' range, due to zeroalign");
        else {
          l(), f = !1;
          continue;
        }
      if (this.autoScaleStart && this.autoScaleEnd && m < s - r) {
        o(), f = !1;
        continue;
      }
      f = !0;
    }
  }
  /**
   * convert value
   * @param {number} value
   * @returns {number}
   */
  convertValue(e) {
    return this.containerHeight - (e - this._start) * this.scale;
  }
  /**
   * returns screen to value
   * @param {number} pixels
   * @returns {number}
   */
  screenToValue(e) {
    return (this.containerHeight - e) / this.scale + this._start;
  }
}
class Wp extends zr {
  /**
   * @param {Object} body
   * @param {Object} [options]        See DataAxis.setOptions for the available
   *                                  options.
   * @param {SVGElement} svg
   * @param {timeline.LineGraph.options} linegraphOptions
   * @constructor DataAxis
   * @extends Component
   */
  constructor(e, t, r, s) {
    super(), this.id = wi(), this.body = e, this.defaultOptions = {
      orientation: "left",
      // supported: 'left', 'right'
      showMinorLabels: !0,
      showMajorLabels: !0,
      showWeekScale: !1,
      icons: !1,
      majorLinesOffset: 7,
      minorLinesOffset: 4,
      labelOffsetX: 10,
      labelOffsetY: 2,
      iconWidth: 20,
      width: "40px",
      visible: !0,
      alignZeros: !0,
      left: {
        range: {
          min: void 0,
          max: void 0
        },
        format(o) {
          return "".concat(va(o.toPrecision(3)));
        },
        title: {
          text: void 0,
          style: void 0
        }
      },
      right: {
        range: {
          min: void 0,
          max: void 0
        },
        format(o) {
          return "".concat(va(o.toPrecision(3)));
        },
        title: {
          text: void 0,
          style: void 0
        }
      }
    }, this.linegraphOptions = s, this.linegraphSVG = r, this.props = {}, this.DOMelements = {
      // dynamic elements
      lines: {},
      labels: {},
      title: {}
    }, this.dom = {}, this.scale = void 0, this.range = {
      start: 0,
      end: 0
    }, this.options = E.extend({}, this.defaultOptions), this.conversionFactor = 1, this.setOptions(t), this.width = Number("".concat(this.options.width).replace("px", "")), this.minWidth = this.width, this.height = this.linegraphSVG.getBoundingClientRect().height, this.hidden = !1, this.stepPixels = 25, this.zeroCrossing = -1, this.amountOfSteps = -1, this.lineOffset = 0, this.master = !0, this.masterAxis = null, this.svgElements = {}, this.iconsRemoved = !1, this.groups = {}, this.amountOfGroups = 0, this._create(), this.scale == null && this._redrawLabels(), this.framework = {
      svg: this.svg,
      svgElements: this.svgElements,
      options: this.options,
      groups: this.groups
    };
    const a = this;
    this.body.emitter.on("verticalDrag", () => {
      a.dom.lineContainer.style.top = "".concat(a.body.domProps.scrollTop, "px");
    });
  }
  /**
   * Adds group to data axis
   * @param {string} label
   * @param {object} graphOptions
   */
  addGroup(e, t) {
    Object.prototype.hasOwnProperty.call(this.groups, e) || (this.groups[e] = t), this.amountOfGroups += 1;
  }
  /**
   * updates group of data axis
   * @param {string} label
   * @param {object} graphOptions
   */
  updateGroup(e, t) {
    Object.prototype.hasOwnProperty.call(this.groups, e) || (this.amountOfGroups += 1), this.groups[e] = t;
  }
  /**
   * removes group of data axis
   * @param {string} label
   */
  removeGroup(e) {
    Object.prototype.hasOwnProperty.call(this.groups, e) && (delete this.groups[e], this.amountOfGroups -= 1);
  }
  /**
   * sets options
   * @param {object} options
   */
  setOptions(e) {
    if (e) {
      let t = !1;
      this.options.orientation != e.orientation && e.orientation !== void 0 && (t = !0);
      const r = ["orientation", "showMinorLabels", "showMajorLabels", "icons", "majorLinesOffset", "minorLinesOffset", "labelOffsetX", "labelOffsetY", "iconWidth", "width", "visible", "left", "right", "alignZeros"];
      E.selectiveDeepExtend(r, this.options, e), this.minWidth = Number("".concat(this.options.width).replace("px", "")), t === !0 && this.dom.frame && (this.hide(), this.show());
    }
  }
  /**
   * Create the HTML DOM for the DataAxis
   */
  _create() {
    this.dom.frame = document.createElement("div"), this.dom.frame.style.width = this.options.width, this.dom.frame.style.height = this.height, this.dom.lineContainer = document.createElement("div"), this.dom.lineContainer.style.width = "100%", this.dom.lineContainer.style.height = this.height, this.dom.lineContainer.style.position = "relative", this.dom.lineContainer.style.visibility = "visible", this.dom.lineContainer.style.display = "block", this.svg = document.createElementNS("http://www.w3.org/2000/svg", "svg"), this.svg.style.position = "absolute", this.svg.style.top = "0px", this.svg.style.height = "100%", this.svg.style.width = "100%", this.svg.style.display = "block", this.dom.frame.appendChild(this.svg);
  }
  /**
   * redraws groups icons
   */
  _redrawGroupIcons() {
    Lr(this.svgElements);
    let e;
    const t = this.options.iconWidth, r = 15, s = 4;
    let a = s + 0.5 * r;
    this.options.orientation === "left" ? e = s : e = this.width - t - s;
    const o = ct(this.groups);
    gt(o).call(o, (l, c) => l < c ? -1 : 1);
    for (const l of o)
      this.groups[l].visible === !0 && (this.linegraphOptions.visibility[l] === void 0 || this.linegraphOptions.visibility[l] === !0) && (this.groups[l].getLegend(t, r, this.framework, e, a), a += r + s);
    zt(this.svgElements), this.iconsRemoved = !1;
  }
  /**
   * Cleans up icons
   */
  _cleanupIcons() {
    this.iconsRemoved === !1 && (Lr(this.svgElements), zt(this.svgElements), this.iconsRemoved = !0);
  }
  /**
   * Create the HTML DOM for the DataAxis
   */
  show() {
    this.hidden = !1, this.dom.frame.parentNode || (this.options.orientation === "left" ? this.body.dom.left.appendChild(this.dom.frame) : this.body.dom.right.appendChild(this.dom.frame)), this.dom.lineContainer.parentNode || this.body.dom.backgroundHorizontal.appendChild(this.dom.lineContainer), this.dom.lineContainer.style.display = "block";
  }
  /**
   * Create the HTML DOM for the DataAxis
   */
  hide() {
    this.hidden = !0, this.dom.frame.parentNode && this.dom.frame.parentNode.removeChild(this.dom.frame), this.dom.lineContainer.style.display = "none";
  }
  /**
   * Set a range (start and end)
   * @param {number} start
   * @param {number} end
   */
  setRange(e, t) {
    this.range.start = e, this.range.end = t;
  }
  /**
   * Repaint the component
   * @return {boolean} Returns true if the component is resized
   */
  redraw() {
    let e = !1, t = 0;
    this.dom.lineContainer.style.top = "".concat(this.body.domProps.scrollTop, "px");
    for (const r in this.groups)
      Object.prototype.hasOwnProperty.call(this.groups, r) && this.groups[r].visible === !0 && (this.linegraphOptions.visibility[r] === void 0 || this.linegraphOptions.visibility[r] === !0) && t++;
    if (this.amountOfGroups === 0 || t === 0)
      this.hide();
    else {
      this.show(), this.height = Number(this.linegraphSVG.style.height.replace("px", "")), this.dom.lineContainer.style.height = "".concat(this.height, "px"), this.width = this.options.visible === !0 ? Number("".concat(this.options.width).replace("px", "")) : 0;
      const r = this.props, s = this.dom.frame;
      s.className = "vis-data-axis", this._calculateCharSize();
      const a = this.options.orientation, o = this.options.showMinorLabels, l = this.options.showMajorLabels, c = this.body.dom.backgroundHorizontal.offsetWidth;
      r.minorLabelHeight = o ? r.minorCharHeight : 0, r.majorLabelHeight = l ? r.majorCharHeight : 0, r.minorLineWidth = c - this.lineOffset - this.width + 2 * this.options.minorLinesOffset, r.minorLineHeight = 1, r.majorLineWidth = c - this.lineOffset - this.width + 2 * this.options.majorLinesOffset, r.majorLineHeight = 1, a === "left" ? (s.style.top = "0", s.style.left = "0", s.style.bottom = "", s.style.width = "".concat(this.width, "px"), s.style.height = "".concat(this.height, "px"), this.props.width = this.body.domProps.left.width, this.props.height = this.body.domProps.left.height) : (s.style.top = "", s.style.bottom = "0", s.style.left = "0", s.style.width = "".concat(this.width, "px"), s.style.height = "".concat(this.height, "px"), this.props.width = this.body.domProps.right.width, this.props.height = this.body.domProps.right.height), e = this._redrawLabels(), e = this._isResized() || e, this.options.icons === !0 ? this._redrawGroupIcons() : this._cleanupIcons(), this._redrawTitle(a);
    }
    return e;
  }
  /**
   * Repaint major and minor text labels and vertical grid lines
   *
   * @returns {boolean}
   * @private
   */
  _redrawLabels() {
    let e = !1;
    Lr(this.DOMelements.lines), Lr(this.DOMelements.labels);
    const t = this.options.orientation, r = this.options[t].range != null ? this.options[t].range : {};
    let s = !0;
    r.max != null && (this.range.end = r.max, s = !1);
    let a = !0;
    r.min != null && (this.range.start = r.min, a = !1), this.scale = new sk(this.range.start, this.range.end, a, s, this.dom.frame.offsetHeight, this.props.majorCharHeight, this.options.alignZeros, this.options[t].format), this.master === !1 && this.masterAxis != null ? (this.scale.followScale(this.masterAxis.scale), this.dom.lineContainer.style.display = "none") : this.dom.lineContainer.style.display = "block", this.maxLabelSize = 0;
    const o = this.scale.getLines();
    B(o).call(o, (d) => {
      const f = d.y, p = d.major;
      this.options.showMinorLabels && p === !1 && this._redrawLabel(f - 2, d.val, t, "vis-y-axis vis-minor", this.props.minorCharHeight), p && f >= 0 && this._redrawLabel(f - 2, d.val, t, "vis-y-axis vis-major", this.props.majorCharHeight), this.master === !0 && (p ? this._redrawLine(f, t, "vis-grid vis-horizontal vis-major", this.options.majorLinesOffset, this.props.majorLineWidth) : this._redrawLine(f, t, "vis-grid vis-horizontal vis-minor", this.options.minorLinesOffset, this.props.minorLineWidth));
    });
    let l = 0;
    this.options[t].title !== void 0 && this.options[t].title.text !== void 0 && (l = this.props.titleCharHeight);
    const c = this.options.icons === !0 ? Math.max(this.options.iconWidth, l) + this.options.labelOffsetX + 15 : l + this.options.labelOffsetX + 15;
    return this.maxLabelSize > this.width - c && this.options.visible === !0 ? (this.width = this.maxLabelSize + c, this.options.width = "".concat(this.width, "px"), zt(this.DOMelements.lines), zt(this.DOMelements.labels), this.redraw(), e = !0) : this.maxLabelSize < this.width - c && this.options.visible === !0 && this.width > this.minWidth ? (this.width = Math.max(this.minWidth, this.maxLabelSize + c), this.options.width = "".concat(this.width, "px"), zt(this.DOMelements.lines), zt(this.DOMelements.labels), this.redraw(), e = !0) : (zt(this.DOMelements.lines), zt(this.DOMelements.labels), e = !1), e;
  }
  /**
   * converts value
   * @param {number} value
   * @returns {number} converted number
   */
  convertValue(e) {
    return this.scale.convertValue(e);
  }
  /**
   * converts value
   * @param {number} x
   * @returns {number} screen value
   */
  screenToValue(e) {
    return this.scale.screenToValue(e);
  }
  /**
   * Create a label for the axis at position x
   *
   * @param {number} y
   * @param {string} text
   * @param {'top'|'right'|'bottom'|'left'} orientation
   * @param {string} className
   * @param {number} characterHeight
   * @private
   */
  _redrawLabel(e, t, r, s, a) {
    const o = sa("div", this.DOMelements.labels, this.dom.frame);
    o.className = s, o.innerHTML = E.xss(t), r === "left" ? (o.style.left = "-".concat(this.options.labelOffsetX, "px"), o.style.textAlign = "right") : (o.style.right = "-".concat(this.options.labelOffsetX, "px"), o.style.textAlign = "left"), o.style.top = "".concat(e - 0.5 * a + this.options.labelOffsetY, "px"), t += "";
    const l = Math.max(this.props.majorCharWidth, this.props.minorCharWidth);
    this.maxLabelSize < t.length * l && (this.maxLabelSize = t.length * l);
  }
  /**
   * Create a minor line for the axis at position y
   * @param {number} y
   * @param {'top'|'right'|'bottom'|'left'} orientation
   * @param {string} className
   * @param {number} offset
   * @param {number} width
   */
  _redrawLine(e, t, r, s, a) {
    if (this.master === !0) {
      const o = sa("div", this.DOMelements.lines, this.dom.lineContainer);
      o.className = r, o.innerHTML = "", t === "left" ? o.style.left = "".concat(this.width - s, "px") : o.style.right = "".concat(this.width - s, "px"), o.style.width = "".concat(a, "px"), o.style.top = "".concat(e, "px");
    }
  }
  /**
   * Create a title for the axis
   * @private
   * @param {'top'|'right'|'bottom'|'left'} orientation
   */
  _redrawTitle(e) {
    if (Lr(this.DOMelements.title), this.options[e].title !== void 0 && this.options[e].title.text !== void 0) {
      const t = sa("div", this.DOMelements.title, this.dom.frame);
      t.className = "vis-y-axis vis-title vis-".concat(e), t.innerHTML = E.xss(this.options[e].title.text), this.options[e].title.style !== void 0 && E.addCssText(t, this.options[e].title.style), e === "left" ? t.style.left = "".concat(this.props.titleCharHeight, "px") : t.style.right = "".concat(this.props.titleCharHeight, "px"), t.style.width = "".concat(this.height, "px");
    }
    zt(this.DOMelements.title);
  }
  /**
   * Determine the size of text on the axis (both major and minor axis).
   * The size is calculated only once and then cached in this.props.
   * @private
   */
  _calculateCharSize() {
    if (!("minorCharHeight" in this.props)) {
      const e = document.createTextNode("0"), t = document.createElement("div");
      t.className = "vis-y-axis vis-minor vis-measure", t.appendChild(e), this.dom.frame.appendChild(t), this.props.minorCharHeight = t.clientHeight, this.props.minorCharWidth = t.clientWidth, this.dom.frame.removeChild(t);
    }
    if (!("majorCharHeight" in this.props)) {
      const e = document.createTextNode("0"), t = document.createElement("div");
      t.className = "vis-y-axis vis-major vis-measure", t.appendChild(e), this.dom.frame.appendChild(t), this.props.majorCharHeight = t.clientHeight, this.props.majorCharWidth = t.clientWidth, this.dom.frame.removeChild(t);
    }
    if (!("titleCharHeight" in this.props)) {
      const e = document.createTextNode("0"), t = document.createElement("div");
      t.className = "vis-y-axis vis-title vis-measure", t.appendChild(e), this.dom.frame.appendChild(t), this.props.titleCharHeight = t.clientHeight, this.props.titleCharWidth = t.clientWidth, this.dom.frame.removeChild(t);
    }
  }
}
function Cs() {
}
Cs.draw = function(n, e, t, r) {
  r = r || 0;
  for (var s = OH(t, e), a = 0; a < n.length; a++)
    if (!s)
      ki(n[a].screen_x + r, n[a].screen_y, zp(e), t.svgElements, t.svg, n[a].label);
    else {
      var o = s(n[a], e);
      (o === !0 || typeof o == "object") && ki(n[a].screen_x + r, n[a].screen_y, zp(e, o), t.svgElements, t.svg, n[a].label);
    }
};
Cs.drawIcon = function(n, e, t, r, s, a) {
  var o = s * 0.5, l = nr("rect", a.svgElements, a.svg);
  l.setAttributeNS(null, "x", e), l.setAttributeNS(null, "y", t - o), l.setAttributeNS(null, "width", r), l.setAttributeNS(null, "height", 2 * o), l.setAttributeNS(null, "class", "vis-outline"), ki(e + 0.5 * r, t, zp(n), a.svgElements, a.svg);
};
function zp(n, e) {
  return e = typeof e > "u" ? {} : e, {
    style: e.style || n.options.drawPoints.style,
    styles: e.styles || n.options.drawPoints.styles,
    size: e.size || n.options.drawPoints.size,
    className: e.className || n.className
  };
}
function OH(n, e) {
  var t = void 0;
  return n.options && n.options.drawPoints && n.options.drawPoints.onRender && typeof n.options.drawPoints.onRender == "function" && (t = n.options.drawPoints.onRender), e.group.options && e.group.options.drawPoints && e.group.options.drawPoints.onRender && typeof e.group.options.drawPoints.onRender == "function" && (t = e.group.options.drawPoints.onRender), t;
}
function At() {
}
At.drawIcon = function(n, e, t, r, s, a) {
  var o = s * 0.5, l = nr("rect", a.svgElements, a.svg);
  l.setAttributeNS(null, "x", e), l.setAttributeNS(null, "y", t - o), l.setAttributeNS(null, "width", r), l.setAttributeNS(null, "height", 2 * o), l.setAttributeNS(null, "class", "vis-outline");
  var c = Math.round(0.3 * r), d = n.options.barChart.width, f = d / c, p = Math.round(0.4 * s), m = Math.round(0.75 * s), g = Math.round((r - 2 * c) / 3);
  if (pa(e + 0.5 * c + g, t + o - p - 1, c, p, n.className + " vis-bar", a.svgElements, a.svg, n.style), pa(e + 1.5 * c + g + 2, t + o - m - 1, c, m, n.className + " vis-bar", a.svgElements, a.svg, n.style), n.options.drawPoints.enabled == !0) {
    var b = {
      style: n.options.drawPoints.style,
      styles: n.options.drawPoints.styles,
      size: n.options.drawPoints.size / f,
      className: n.className
    };
    ki(e + 0.5 * c + g, t + o - p - 1, b, a.svgElements, a.svg), ki(e + 1.5 * c + g + 2, t + o - m - 1, b, a.svgElements, a.svg);
  }
};
At.draw = function(n, e, t) {
  var r = [], s = {}, a, o, l, c, d, f, p = 0;
  for (d = 0; d < n.length; d++)
    if (c = t.groups[n[d]], c.options.style === "bar" && c.visible === !0 && (t.options.groups.visibility[n[d]] === void 0 || t.options.groups.visibility[n[d]] === !0))
      for (f = 0; f < e[n[d]].length; f++)
        r.push({
          screen_x: e[n[d]][f].screen_x,
          screen_end: e[n[d]][f].screen_end,
          screen_y: e[n[d]][f].screen_y,
          x: e[n[d]][f].x,
          end: e[n[d]][f].end,
          y: e[n[d]][f].y,
          groupId: n[d],
          label: e[n[d]][f].label
        }), p += 1;
  if (p !== 0)
    for (gt(r).call(r, function(_, x) {
      return _.screen_x === x.screen_x ? _.groupId < x.groupId ? -1 : 1 : _.screen_x - x.screen_x;
    }), At._getDataIntersections(s, r), d = 0; d < r.length; d++) {
      c = t.groups[r[d].groupId];
      var m = c.options.barChart.minWidth != null ? c.options.barChart.minWidth : 0.1 * c.options.barChart.width;
      o = r[d].screen_x;
      var g = 0;
      if (s[o] === void 0)
        d + 1 < r.length && (a = Math.abs(r[d + 1].screen_x - o)), l = At._getSafeDrawData(a, c, m);
      else {
        var b = d + (s[o].amount - s[o].resolved);
        b < r.length && (a = Math.abs(r[b].screen_x - o)), l = At._getSafeDrawData(a, c, m), s[o].resolved += 1, c.options.stack === !0 && c.options.excludeFromStacking !== !0 ? r[d].screen_y < c.zeroPosition ? (g = s[o].accumulatedNegative, s[o].accumulatedNegative += c.zeroPosition - r[d].screen_y) : (g = s[o].accumulatedPositive, s[o].accumulatedPositive += c.zeroPosition - r[d].screen_y) : c.options.barChart.sideBySide === !0 && (l.width = l.width / s[o].amount, l.offset += s[o].resolved * l.width - 0.5 * l.width * (s[o].amount + 1));
      }
      let _ = l.width, x = r[d].screen_x;
      if (r[d].screen_end != null ? (_ = r[d].screen_end - r[d].screen_x, x += _ * 0.5) : x += l.offset, pa(x, r[d].screen_y - g, _, c.zeroPosition - r[d].screen_y, c.className + " vis-bar", t.svgElements, t.svg, c.style), c.options.drawPoints.enabled === !0) {
        let T = {
          screen_x: r[d].screen_x,
          screen_y: r[d].screen_y - g,
          x: r[d].x,
          y: r[d].y,
          groupId: r[d].groupId,
          label: r[d].label
        };
        Cs.draw([T], c, t, l.offset);
      }
    }
};
At._getDataIntersections = function(n, e) {
  for (var t, r = 0; r < e.length; r++)
    r + 1 < e.length && (t = Math.abs(e[r + 1].screen_x - e[r].screen_x)), r > 0 && (t = Math.min(t, Math.abs(e[r - 1].screen_x - e[r].screen_x))), t === 0 && (n[e[r].screen_x] === void 0 && (n[e[r].screen_x] = {
      amount: 0,
      resolved: 0,
      accumulatedPositive: 0,
      accumulatedNegative: 0
    }), n[e[r].screen_x].amount += 1);
};
At._getSafeDrawData = function(n, e, t) {
  var r, s;
  return n < e.options.barChart.width && n > 0 ? (r = n < t ? t : n, s = 0, e.options.barChart.align === "left" ? s -= 0.5 * n : e.options.barChart.align === "right" && (s += 0.5 * n)) : (r = e.options.barChart.width, s = 0, e.options.barChart.align === "left" ? s -= 0.5 * e.options.barChart.width : e.options.barChart.align === "right" && (s += 0.5 * e.options.barChart.width)), {
    width: r,
    offset: s
  };
};
At.getStackedYRange = function(n, e, t, r, s) {
  if (n.length > 0) {
    gt(n).call(n, function(o, l) {
      return o.screen_x === l.screen_x ? o.groupId < l.groupId ? -1 : 1 : o.screen_x - l.screen_x;
    });
    var a = {};
    At._getDataIntersections(a, n), e[r] = At._getStackedYRange(a, n), e[r].yAxisOrientation = s, t.push(r);
  }
};
At._getStackedYRange = function(n, e) {
  for (var t, r = e[0].screen_y, s = e[0].screen_y, a = 0; a < e.length; a++)
    t = e[a].screen_x, n[t] === void 0 ? (r = r > e[a].screen_y ? e[a].screen_y : r, s = s < e[a].screen_y ? e[a].screen_y : s) : e[a].screen_y < 0 ? n[t].accumulatedNegative += e[a].screen_y : n[t].accumulatedPositive += e[a].screen_y;
  for (var o in n)
    Object.prototype.hasOwnProperty.call(n, o) && (r = r > n[o].accumulatedNegative ? n[o].accumulatedNegative : r, r = r > n[o].accumulatedPositive ? n[o].accumulatedPositive : r, s = s < n[o].accumulatedNegative ? n[o].accumulatedNegative : s, s = s < n[o].accumulatedPositive ? n[o].accumulatedPositive : s);
  return {
    min: r,
    max: s
  };
};
function vt() {
}
vt.calcPath = function(n, e) {
  if (n != null && n.length > 0) {
    var t = [];
    return e.options.interpolation.enabled == !0 ? t = vt._catmullRom(n, e) : t = vt._linear(n), t;
  }
};
vt.drawIcon = function(n, e, t, r, s, a) {
  var o = s * 0.5, l, c, d = nr("rect", a.svgElements, a.svg);
  if (d.setAttributeNS(null, "x", e), d.setAttributeNS(null, "y", t - o), d.setAttributeNS(null, "width", r), d.setAttributeNS(null, "height", 2 * o), d.setAttributeNS(null, "class", "vis-outline"), l = nr("path", a.svgElements, a.svg), l.setAttributeNS(null, "class", n.className), n.style !== void 0 && l.setAttributeNS(null, "style", n.style), l.setAttributeNS(null, "d", "M" + e + "," + t + " L" + (e + r) + "," + t), n.options.shaded.enabled == !0 && (c = nr("path", a.svgElements, a.svg), n.options.shaded.orientation == "top" ? c.setAttributeNS(null, "d", "M" + e + ", " + (t - o) + "L" + e + "," + t + " L" + (e + r) + "," + t + " L" + (e + r) + "," + (t - o)) : c.setAttributeNS(null, "d", "M" + e + "," + t + " L" + e + "," + (t + o) + " L" + (e + r) + "," + (t + o) + "L" + (e + r) + "," + t), c.setAttributeNS(null, "class", n.className + " vis-icon-fill"), n.options.shaded.style !== void 0 && n.options.shaded.style !== "" && c.setAttributeNS(null, "style", n.options.shaded.style)), n.options.drawPoints.enabled == !0) {
    var f = {
      style: n.options.drawPoints.style,
      styles: n.options.drawPoints.styles,
      size: n.options.drawPoints.size,
      className: n.className
    };
    ki(e + 0.5 * r, t, f, a.svgElements, a.svg);
  }
};
vt.drawShading = function(n, e, t, r) {
  if (e.options.shaded.enabled == !0) {
    var s = Number(r.svg.style.height.replace("px", "")), a = nr("path", r.svgElements, r.svg), o = "L";
    e.options.interpolation.enabled == !0 && (o = "C");
    var l, c = 0;
    e.options.shaded.orientation == "top" ? c = 0 : e.options.shaded.orientation == "bottom" ? c = s : c = Math.min(Math.max(0, e.zeroPosition), s), e.options.shaded.orientation == "group" && t != null && t != null ? l = "M" + n[0][0] + "," + n[0][1] + " " + this.serializePath(n, o, !1) + " L" + t[t.length - 1][0] + "," + t[t.length - 1][1] + " " + this.serializePath(t, o, !0) + t[0][0] + "," + t[0][1] + " Z" : l = "M" + n[0][0] + "," + n[0][1] + " " + this.serializePath(n, o, !1) + " V" + c + " H" + n[0][0] + " Z", a.setAttributeNS(null, "class", e.className + " vis-fill"), e.options.shaded.style !== void 0 && a.setAttributeNS(null, "style", e.options.shaded.style), a.setAttributeNS(null, "d", l);
  }
};
vt.draw = function(n, e, t) {
  if (n != null && n != null) {
    var r = nr("path", t.svgElements, t.svg);
    r.setAttributeNS(null, "class", e.className), e.style !== void 0 && r.setAttributeNS(null, "style", e.style);
    var s = "L";
    e.options.interpolation.enabled == !0 && (s = "C"), r.setAttributeNS(null, "d", "M" + n[0][0] + "," + n[0][1] + " " + this.serializePath(n, s, !1));
  }
};
vt.serializePath = function(n, e, t) {
  if (n.length < 2)
    return "";
  var r = e, s;
  if (t)
    for (s = n.length - 2; s > 0; s--)
      r += n[s][0] + "," + n[s][1] + " ";
  else
    for (s = 1; s < n.length; s++)
      r += n[s][0] + "," + n[s][1] + " ";
  return r;
};
vt._catmullRomUniform = function(n) {
  var e, t, r, s, a, o, l = [];
  l.push([Math.round(n[0].screen_x), Math.round(n[0].screen_y)]);
  for (var c = 1 / 6, d = n.length, f = 0; f < d - 1; f++)
    e = f == 0 ? n[0] : n[f - 1], t = n[f], r = n[f + 1], s = f + 2 < d ? n[f + 2] : r, a = {
      screen_x: (-e.screen_x + 6 * t.screen_x + r.screen_x) * c,
      screen_y: (-e.screen_y + 6 * t.screen_y + r.screen_y) * c
    }, o = {
      screen_x: (t.screen_x + 6 * r.screen_x - s.screen_x) * c,
      screen_y: (t.screen_y + 6 * r.screen_y - s.screen_y) * c
    }, l.push([a.screen_x, a.screen_y]), l.push([o.screen_x, o.screen_y]), l.push([r.screen_x, r.screen_y]);
  return l;
};
vt._catmullRom = function(n, e) {
  var t = e.options.interpolation.alpha;
  if (t == 0 || t === void 0)
    return this._catmullRomUniform(n);
  var r, s, a, o, l, c, d, f, p, m, g, b, _, x, T, S, k, D, I, C = [];
  C.push([Math.round(n[0].screen_x), Math.round(n[0].screen_y)]);
  for (var P = n.length, R = 0; R < P - 1; R++)
    r = R == 0 ? n[0] : n[R - 1], s = n[R], a = n[R + 1], o = R + 2 < P ? n[R + 2] : a, d = Math.sqrt(Math.pow(r.screen_x - s.screen_x, 2) + Math.pow(r.screen_y - s.screen_y, 2)), f = Math.sqrt(Math.pow(s.screen_x - a.screen_x, 2) + Math.pow(s.screen_y - a.screen_y, 2)), p = Math.sqrt(Math.pow(a.screen_x - o.screen_x, 2) + Math.pow(a.screen_y - o.screen_y, 2)), x = Math.pow(p, t), S = Math.pow(p, 2 * t), T = Math.pow(f, t), k = Math.pow(f, 2 * t), I = Math.pow(d, t), D = Math.pow(d, 2 * t), m = 2 * D + 3 * I * T + k, g = 2 * S + 3 * x * T + k, b = 3 * I * (I + T), b > 0 && (b = 1 / b), _ = 3 * x * (x + T), _ > 0 && (_ = 1 / _), l = {
      screen_x: (-k * r.screen_x + m * s.screen_x + D * a.screen_x) * b,
      screen_y: (-k * r.screen_y + m * s.screen_y + D * a.screen_y) * b
    }, c = {
      screen_x: (S * s.screen_x + g * a.screen_x - k * o.screen_x) * _,
      screen_y: (S * s.screen_y + g * a.screen_y - k * o.screen_y) * _
    }, l.screen_x == 0 && l.screen_y == 0 && (l = s), c.screen_x == 0 && c.screen_y == 0 && (c = a), C.push([l.screen_x, l.screen_y]), C.push([c.screen_x, c.screen_y]), C.push([a.screen_x, a.screen_y]);
  return C;
};
vt._linear = function(n) {
  for (var e = [], t = 0; t < n.length; t++)
    e.push([n[t].screen_x, n[t].screen_y]);
  return e;
};
function Ur(n, e, t, r) {
  this.id = e;
  var s = ["sampling", "style", "sort", "yAxisOrientation", "barChart", "drawPoints", "shaded", "interpolation", "zIndex", "excludeFromStacking", "excludeFromLegend"];
  this.options = E.selectiveBridgeObject(s, t), this.usingDefaultStyle = n.className === void 0, this.groupsUsingDefaultStyles = r, this.zeroPosition = 0, this.update(n), this.usingDefaultStyle == !0 && (this.groupsUsingDefaultStyles[0] += 1), this.itemsData = [], this.visible = n.visible === void 0 ? !0 : n.visible;
}
Ur.prototype.setItems = function(n) {
  n != null ? (this.itemsData = n, gt(this.options) == !0 && E.insertSort(this.itemsData, function(e, t) {
    return e.x > t.x ? 1 : -1;
  })) : this.itemsData = [];
};
Ur.prototype.getItems = function() {
  return this.itemsData;
};
Ur.prototype.setZeroPosition = function(n) {
  this.zeroPosition = n;
};
Ur.prototype.setOptions = function(n) {
  if (n !== void 0) {
    var e = ["sampling", "style", "sort", "yAxisOrientation", "barChart", "zIndex", "excludeFromStacking", "excludeFromLegend"];
    E.selectiveDeepExtend(e, this.options, n), typeof n.drawPoints == "function" && (n.drawPoints = {
      onRender: n.drawPoints
    }), E.mergeOptions(this.options, n, "interpolation"), E.mergeOptions(this.options, n, "drawPoints"), E.mergeOptions(this.options, n, "shaded"), n.interpolation && typeof n.interpolation == "object" && n.interpolation.parametrization && (n.interpolation.parametrization == "uniform" ? this.options.interpolation.alpha = 0 : n.interpolation.parametrization == "chordal" ? this.options.interpolation.alpha = 1 : (this.options.interpolation.parametrization = "centripetal", this.options.interpolation.alpha = 0.5));
  }
};
Ur.prototype.update = function(n) {
  this.group = n, this.content = n.content || "graph", this.className = n.className || this.className || "vis-graph-group" + this.groupsUsingDefaultStyles[0] % 10, this.visible = n.visible === void 0 ? !0 : n.visible, this.style = n.style, this.setOptions(n.options);
};
Ur.prototype.getLegend = function(n, e, t, r, s) {
  if (t == null || t == null) {
    var a = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    t = {
      svg: a,
      svgElements: {},
      options: this.options,
      groups: [this]
    };
  }
  switch ((r == null || r == null) && (r = 0), (s == null || s == null) && (s = 0.5 * e), this.options.style) {
    case "line":
      vt.drawIcon(this, r, s, n, e, t);
      break;
    case "points":
    //explicit no break
    case "point":
      Cs.drawIcon(this, r, s, n, e, t);
      break;
    case "bar":
      At.drawIcon(this, r, s, n, e, t);
      break;
  }
  return {
    icon: t.svg,
    label: this.content,
    orientation: this.options.yAxisOrientation
  };
};
Ur.prototype.getYRange = function(n) {
  for (var e = n[0].y, t = n[0].y, r = 0; r < n.length; r++)
    e = e > n[r].y ? n[r].y : e, t = t < n[r].y ? n[r].y : t;
  return {
    min: e,
    max: t,
    yAxisOrientation: this.options.yAxisOrientation
  };
};
class Li {
  /**
   * @constructor Item
   * @param {Object} data             Object containing (optional) parameters type,
   *                                  start, end, content, group, className.
   * @param {{toScreen: function, toTime: function}} conversion
   *                                  Conversion functions from time to screen and vice versa
   * @param {Object} options          Configuration options
   *                                  // TODO: describe available options
   */
  constructor(e, t, r) {
    var s;
    this.id = null, this.parent = null, this.data = e, this.dom = null, this.conversion = t || {}, this.defaultOptions = {
      locales: dn,
      locale: "en"
    }, this.options = E.extend({}, this.defaultOptions, r), this.options.locales = E.extend({}, dn, this.options.locales);
    const a = this.defaultOptions.locales[this.defaultOptions.locale];
    B(s = ct(this.options.locales)).call(s, (o) => {
      this.options.locales[o] = E.extend({}, a, this.options.locales[o]);
    }), this.selected = !1, this.displayed = !1, this.groupShowing = !0, this.selectable = r && r.selectable || !1, this.dirty = !0, this.top = null, this.right = null, this.left = null, this.width = null, this.height = null, this.setSelectability(e), this.editable = null, this._updateEditStatus();
  }
  /**
   * Select current item
   */
  select() {
    this.selectable && (this.selected = !0, this.dirty = !0, this.displayed && this.redraw());
  }
  /**
   * Unselect current item
   */
  unselect() {
    this.selected = !1, this.dirty = !0, this.displayed && this.redraw();
  }
  /**
   * Set data for the item. Existing data will be updated. The id should not
   * be changed. When the item is displayed, it will be redrawn immediately.
   * @param {Object} data
   */
  setData(e) {
    e.group != null && this.data.group != e.group && this.parent != null && this.parent.itemSet._moveToGroup(this, e.group), this.setSelectability(e), this.parent && (this.parent.stackDirty = !0), e.subgroup != null && this.data.subgroup != e.subgroup && this.parent != null && this.parent.changeSubgroup(this, this.data.subgroup, e.subgroup), this.data = e, this._updateEditStatus(), this.dirty = !0, this.displayed && this.redraw();
  }
  /**
   * Set whether the item can be selected.
   * Can only be set/unset if the timeline's `selectable` configuration option is `true`.
   * @param {Object} data `data` from `constructor` and `setData`
   */
  setSelectability(e) {
    e && (this.selectable = typeof e.selectable > "u" ? !0 : !!e.selectable);
  }
  /**
   * Set a parent for the item
   * @param {Group} parent
   */
  setParent(e) {
    this.displayed ? (this.hide(), this.parent = e, this.parent && this.show()) : this.parent = e;
  }
  /**
   * Check whether this item is visible inside given range
   * @returns {boolean} True if visible
   */
  isVisible() {
    return !1;
  }
  /**
   * Show the Item in the DOM (when not already visible)
   * @return {Boolean} changed
   */
  show() {
    return !1;
  }
  /**
   * Hide the Item from the DOM (when visible)
   * @return {Boolean} changed
   */
  hide() {
    return !1;
  }
  /**
   * Repaint the item
   */
  redraw() {
  }
  /**
   * Reposition the Item horizontally
   */
  repositionX() {
  }
  /**
   * Reposition the Item vertically
   */
  repositionY() {
  }
  /**
   * Repaint a drag area on the center of the item when the item is selected
   * @protected
   */
  _repaintDragCenter() {
    if (this.selected && this.editable.updateTime && !this.dom.dragCenter) {
      var e, t;
      const r = this, s = document.createElement("div");
      s.className = "vis-drag-center", s.dragCenterItem = this, this.hammerDragCenter = new Tt(s), this.hammerDragCenter.on("tap", (a) => {
        r.parent.itemSet.body.emitter.emit("click", {
          event: a,
          item: r.id
        });
      }), this.hammerDragCenter.on("doubletap", (a) => {
        a.stopPropagation(), r.parent.itemSet._onUpdateItem(r), r.parent.itemSet.body.emitter.emit("doubleClick", {
          event: a,
          item: r.id
        });
      }), this.hammerDragCenter.on("panstart", (a) => {
        a.stopPropagation(), r.parent.itemSet._onDragStart(a);
      }), this.hammerDragCenter.on("panmove", N(e = r.parent.itemSet._onDrag).call(e, r.parent.itemSet)), this.hammerDragCenter.on("panend", N(t = r.parent.itemSet._onDragEnd).call(t, r.parent.itemSet)), this.hammerDragCenter.get("press").set({
        time: 1e4
      }), this.dom.box ? this.dom.dragLeft ? this.dom.box.insertBefore(s, this.dom.dragLeft) : this.dom.box.appendChild(s) : this.dom.point && this.dom.point.appendChild(s), this.dom.dragCenter = s;
    } else !this.selected && this.dom.dragCenter && (this.dom.dragCenter.parentNode && this.dom.dragCenter.parentNode.removeChild(this.dom.dragCenter), this.dom.dragCenter = null, this.hammerDragCenter && (this.hammerDragCenter.destroy(), this.hammerDragCenter = null));
  }
  /**
   * Repaint a delete button on the top right of the item when the item is selected
   * @param {HTMLElement} anchor
   * @protected
   */
  _repaintDeleteButton(e) {
    const t = (this.options.editable.overrideItems || this.editable == null) && this.options.editable.remove || !this.options.editable.overrideItems && this.editable != null && this.editable.remove;
    if (this.selected && t && !this.dom.deleteButton) {
      const r = this, s = document.createElement("div");
      this.options.rtl ? s.className = "vis-delete-rtl" : s.className = "vis-delete";
      let a = this.options.locales[this.options.locale];
      a || (this.warned || (console.warn("WARNING: options.locales['".concat(this.options.locale, "'] not found. See https://visjs.github.io/vis-timeline/docs/timeline/#Localization")), this.warned = !0), a = this.options.locales.en), s.title = a.deleteSelected, this.hammerDeleteButton = new Tt(s).on("tap", (o) => {
        o.stopPropagation(), r.parent.removeFromDataSet(r);
      }), e.appendChild(s), this.dom.deleteButton = s;
    } else (!this.selected || !t) && this.dom.deleteButton && (this.dom.deleteButton.parentNode && this.dom.deleteButton.parentNode.removeChild(this.dom.deleteButton), this.dom.deleteButton = null, this.hammerDeleteButton && (this.hammerDeleteButton.destroy(), this.hammerDeleteButton = null));
  }
  /**
   * Repaint a onChange tooltip on the top right of the item when the item is selected
   * @param {HTMLElement} anchor
   * @protected
   */
  _repaintOnItemUpdateTimeTooltip(e) {
    if (!this.options.tooltipOnItemUpdateTime) return;
    const t = (this.options.editable.updateTime || this.data.editable === !0) && this.data.editable !== !1;
    if (this.selected && t && !this.dom.onItemUpdateTimeTooltip) {
      const s = document.createElement("div");
      s.className = "vis-onUpdateTime-tooltip", e.appendChild(s), this.dom.onItemUpdateTimeTooltip = s;
    } else !this.selected && this.dom.onItemUpdateTimeTooltip && (this.dom.onItemUpdateTimeTooltip.parentNode && this.dom.onItemUpdateTimeTooltip.parentNode.removeChild(this.dom.onItemUpdateTimeTooltip), this.dom.onItemUpdateTimeTooltip = null);
    if (this.dom.onItemUpdateTimeTooltip) {
      this.dom.onItemUpdateTimeTooltip.style.visibility = this.parent.itemSet.touchParams.itemIsDragging ? "visible" : "hidden", this.dom.onItemUpdateTimeTooltip.style.transform = "translateX(-50%)", this.dom.onItemUpdateTimeTooltip.style.left = "50%";
      const s = 50, a = this.parent.itemSet.body.domProps.scrollTop;
      let o;
      this.options.orientation.item == "top" ? o = this.top : o = this.parent.height - this.top - this.height, o + this.parent.top - s < -a ? (this.dom.onItemUpdateTimeTooltip.style.bottom = "", this.dom.onItemUpdateTimeTooltip.style.top = "".concat(this.height + 2, "px")) : (this.dom.onItemUpdateTimeTooltip.style.top = "", this.dom.onItemUpdateTimeTooltip.style.bottom = "".concat(this.height + 2, "px"));
      let c, d;
      if (this.options.tooltipOnItemUpdateTime && this.options.tooltipOnItemUpdateTime.template) {
        var r;
        d = N(r = this.options.tooltipOnItemUpdateTime.template).call(r, this), c = d(this.data);
      } else
        c = "start: ".concat(Et(this.data.start).format("MM/DD/YYYY hh:mm")), this.data.end && (c += "<br> end: ".concat(Et(this.data.end).format("MM/DD/YYYY hh:mm")));
      this.dom.onItemUpdateTimeTooltip.innerHTML = E.xss(c);
    }
  }
  /**
   * get item data
   * @return {object}
   * @private
   */
  _getItemData() {
    return this.parent.itemSet.itemsData.get(this.id);
  }
  /**
   * Set HTML contents for the item
   * @param {Element} element   HTML element to fill with the contents
   * @private
   */
  _updateContents(e) {
    let t, r, s, a, o;
    const l = this._getItemData(), d = (this.dom.box || this.dom.point).getElementsByClassName("vis-item-visible-frame")[0];
    if (this.options.visibleFrameTemplate) {
      var f;
      o = N(f = this.options.visibleFrameTemplate).call(f, this), a = E.xss(o(l, d));
    } else
      a = "";
    if (d) {
      if (a instanceof Object && !(a instanceof Element))
        o(l, d);
      else if (r = this._contentToString(this.itemVisibleFrameContent) !== this._contentToString(a), r) {
        if (a instanceof Element)
          d.innerHTML = "", d.appendChild(a);
        else if (a != null)
          d.innerHTML = E.xss(a);
        else if (!(this.data.type == "background" && this.data.content === void 0))
          throw new Error('Property "content" missing in item '.concat(this.id));
        this.itemVisibleFrameContent = a;
      }
    }
    if (this.options.template) {
      var p;
      s = N(p = this.options.template).call(p, this), t = s(l, e, this.data);
    } else
      t = this.data.content;
    if (t instanceof Object && !(t instanceof Element))
      s(l, e);
    else if (r = this._contentToString(this.content) !== this._contentToString(t), r) {
      if (t instanceof Element)
        e.innerHTML = "", e.appendChild(t);
      else if (t != null)
        e.innerHTML = E.xss(t);
      else if (!(this.data.type == "background" && this.data.content === void 0))
        throw new Error('Property "content" missing in item '.concat(this.id));
      this.content = t;
    }
  }
  /**
   * Process dataAttributes timeline option and set as data- attributes on dom.content
   * @param {Element} element   HTML element to which the attributes will be attached
   * @private
   */
  _updateDataAttributes(e) {
    if (this.options.dataAttributes && this.options.dataAttributes.length > 0) {
      let t = [];
      if (ye(this.options.dataAttributes))
        t = this.options.dataAttributes;
      else if (this.options.dataAttributes == "all")
        t = ct(this.data);
      else
        return;
      for (const r of t) {
        const s = this.data[r];
        s != null ? e.setAttribute("data-".concat(r), s) : e.removeAttribute("data-".concat(r));
      }
    }
  }
  /**
   * Update custom styles of the element
   * @param {Element} element
   * @private
   */
  _updateStyle(e) {
    this.style && (E.removeCssText(e, this.style), this.style = null), this.data.style && (E.addCssText(e, this.data.style), this.style = this.data.style);
  }
  /**
   * Stringify the items contents
   * @param {string | Element | undefined} content
   * @returns {string | undefined}
   * @private
   */
  _contentToString(e) {
    return typeof e == "string" ? e : e && "outerHTML" in e ? e.outerHTML : e;
  }
  /**
   * Update the editability of this item.
   */
  _updateEditStatus() {
    this.options && (typeof this.options.editable == "boolean" ? this.editable = {
      updateTime: this.options.editable,
      updateGroup: this.options.editable,
      remove: this.options.editable
    } : typeof this.options.editable == "object" && (this.editable = {}, E.selectiveExtend(["updateTime", "updateGroup", "remove"], this.editable, this.options.editable))), (!this.options || !this.options.editable || this.options.editable.overrideItems !== !0) && this.data && (typeof this.data.editable == "boolean" ? this.editable = {
      updateTime: this.data.editable,
      updateGroup: this.data.editable,
      remove: this.data.editable
    } : typeof this.data.editable == "object" && (this.editable = {}, E.selectiveExtend(["updateTime", "updateGroup", "remove"], this.editable, this.data.editable)));
  }
  /**
   * Return the width of the item left from its start date
   * @return {number}
   */
  getWidthLeft() {
    return 0;
  }
  /**
   * Return the width of the item right from the max of its start and end date
   * @return {number}
   */
  getWidthRight() {
    return 0;
  }
  /**
   * Return the title of the item
   * @return {string | undefined}
   */
  getTitle() {
    if (this.options.tooltip && this.options.tooltip.template) {
      var e;
      return N(e = this.options.tooltip.template).call(e, this)(this._getItemData(), this.data);
    }
    return this.data.title;
  }
}
Li.prototype.stack = !0;
class ti extends Li {
  /**
   * @param {Object} data             Object containing parameters start, end
   *                                  content, className.
   * @param {{toScreen: function, toTime: function}} conversion
   *                                  Conversion functions from time to screen and vice versa
   * @param {Object} [options]        Configuration options
   *                                  // TODO: describe options
   */
  constructor(e, t, r) {
    if (super(e, t, r), this.props = {
      content: {
        width: 0
      }
    }, this.overflow = !1, e) {
      if (e.start == null)
        throw new Error('Property "start" missing in item '.concat(e.id));
      if (e.end == null)
        throw new Error('Property "end" missing in item '.concat(e.id));
    }
  }
  /**
   * Check whether this item is visible inside given range
   *
   * @param {timeline.Range} range with a timestamp for start and end
   * @returns {boolean} True if visible
   */
  isVisible(e) {
    return this.cluster ? !1 : this.data.start < e.end && this.data.end > e.start;
  }
  /**
   * create DOM elements
   * @private
   */
  _createDomElement() {
    this.dom || (this.dom = {}, this.dom.box = document.createElement("div"), this.dom.frame = document.createElement("div"), this.dom.frame.className = "vis-item-overflow", this.dom.box.appendChild(this.dom.frame), this.dom.visibleFrame = document.createElement("div"), this.dom.visibleFrame.className = "vis-item-visible-frame", this.dom.box.appendChild(this.dom.visibleFrame), this.dom.content = document.createElement("div"), this.dom.content.className = "vis-item-content", this.dom.frame.appendChild(this.dom.content), this.dom.box["vis-item"] = this, this.dirty = !0);
  }
  /**
   * append element to DOM
   * @private
   */
  _appendDomElement() {
    if (!this.parent)
      throw new Error("Cannot redraw item: no parent attached");
    if (!this.dom.box.parentNode) {
      const e = this.parent.dom.foreground;
      if (!e)
        throw new Error("Cannot redraw item: parent has no foreground container element");
      e.appendChild(this.dom.box);
    }
    this.displayed = !0;
  }
  /**
   * update dirty DOM components
   * @private
   */
  _updateDirtyDomComponents() {
    if (this.dirty) {
      this._updateContents(this.dom.content), this._updateDataAttributes(this.dom.box), this._updateStyle(this.dom.box);
      const e = this.editable.updateTime || this.editable.updateGroup, t = (this.data.className ? " " + this.data.className : "") + (this.selected ? " vis-selected" : "") + (e ? " vis-editable" : " vis-readonly");
      this.dom.box.className = this.baseClassName + t, this.dom.content.style.maxWidth = "none";
    }
  }
  /**
   * get DOM component sizes
   * @return {object}
   * @private
   */
  _getDomComponentsSizes() {
    return this.overflow = window.getComputedStyle(this.dom.frame).overflow !== "hidden", this.whiteSpace = window.getComputedStyle(this.dom.content).whiteSpace !== "nowrap", {
      content: {
        width: this.dom.content.offsetWidth
      },
      box: {
        height: this.dom.box.offsetHeight
      }
    };
  }
  /**
   * update DOM component sizes
   * @param {array} sizes
   * @private
   */
  _updateDomComponentsSizes(e) {
    this.props.content.width = e.content.width, this.height = e.box.height, this.dom.content.style.maxWidth = "", this.dirty = !1;
  }
  /**
   * repaint DOM additional components
   * @private
   */
  _repaintDomAdditionals() {
    this._repaintOnItemUpdateTimeTooltip(this.dom.box), this._repaintDeleteButton(this.dom.box), this._repaintDragCenter(), this._repaintDragLeft(), this._repaintDragRight();
  }
  /**
   * Repaint the item
   * @param {boolean} [returnQueue=false]  return the queue
   * @return {boolean} the redraw queue if returnQueue=true
   */
  redraw(e) {
    var t, r, s, a;
    let o;
    const l = [
      // create item DOM
      N(t = this._createDomElement).call(t, this),
      // append DOM to parent DOM
      N(r = this._appendDomElement).call(r, this),
      // update dirty DOM
      N(s = this._updateDirtyDomComponents).call(s, this),
      () => {
        if (this.dirty) {
          var c;
          o = N(c = this._getDomComponentsSizes).call(c, this)();
        }
      },
      () => {
        if (this.dirty) {
          var c;
          N(c = this._updateDomComponentsSizes).call(c, this)(o);
        }
      },
      // repaint DOM additionals
      N(a = this._repaintDomAdditionals).call(a, this)
    ];
    if (e)
      return l;
    {
      let c;
      return B(l).call(l, (d) => {
        c = d();
      }), c;
    }
  }
  /**
   * Show the item in the DOM (when not already visible). The items DOM will
   * be created when needed.
   * @param {boolean} [returnQueue=false]  whether to return a queue of functions to execute instead of just executing them
   * @return {boolean} the redraw queue if returnQueue=true
   */
  show(e) {
    if (!this.displayed)
      return this.redraw(e);
  }
  /**
   * Hide the item from the DOM (when visible)
   */
  hide() {
    if (this.displayed) {
      const e = this.dom.box;
      e.parentNode && e.parentNode.removeChild(e), this.displayed = !1;
    }
  }
  /**
   * Reposition the item horizontally
   * @param {boolean} [limitSize=true] If true (default), the width of the range
   *                                   item will be limited, as the browser cannot
   *                                   display very wide divs. This means though
   *                                   that the applied left and width may
   *                                   not correspond to the ranges start and end
   * @Override
   */
  repositionX(e) {
    const t = this.parent.width;
    let r = this.conversion.toScreen(this.data.start), s = this.conversion.toScreen(this.data.end);
    const a = this.data.align === void 0 ? this.options.align : this.data.align;
    let o, l;
    this.data.limitSize !== !1 && (e === void 0 || e === !0) && (r < -t && (r = -t), s > 2 * t && (s = 2 * t));
    const c = Math.max(Math.round((s - r) * 1e3) / 1e3, 1);
    switch (this.overflow ? (this.options.rtl ? this.right = r : this.left = r, this.width = c + this.props.content.width, l = this.props.content.width) : (this.options.rtl ? this.right = r : this.left = r, this.width = c, l = Math.min(s - r, this.props.content.width)), this.options.rtl ? this.dom.box.style.transform = "translateX(".concat(this.right * -1, "px)") : this.dom.box.style.transform = "translateX(".concat(this.left, "px)"), this.dom.box.style.width = "".concat(c, "px"), this.whiteSpace && (this.height = this.dom.box.offsetHeight), a) {
      case "left":
        this.dom.content.style.transform = "translateX(0)";
        break;
      case "right":
        if (this.options.rtl) {
          const d = Math.max(c - l, 0) * -1;
          this.dom.content.style.transform = "translateX(".concat(d, "px)");
        } else
          this.dom.content.style.transform = "translateX(".concat(Math.max(c - l, 0), "px)");
        break;
      case "center":
        if (this.options.rtl) {
          const d = Math.max((c - l) / 2, 0) * -1;
          this.dom.content.style.transform = "translateX(".concat(d, "px)");
        } else
          this.dom.content.style.transform = "translateX(".concat(Math.max((c - l) / 2, 0), "px)");
        break;
      default:
        if (this.overflow ? s > 0 ? o = Math.max(-r, 0) : o = -l : r < 0 ? o = -r : o = 0, this.options.rtl) {
          const d = o * -1;
          this.dom.content.style.transform = "translateX(".concat(d, "px)");
        } else
          this.dom.content.style.transform = "translateX(".concat(o, "px)");
    }
  }
  /**
   * Reposition the item vertically
   * @Override
   */
  repositionY() {
    const e = this.options.orientation.item, t = this.dom.box;
    e == "top" ? t.style.top = "".concat(this.top, "px") : t.style.top = "".concat(this.parent.height - this.top - this.height, "px");
  }
  /**
   * Repaint a drag area on the left side of the range when the range is selected
   * @protected
   */
  _repaintDragLeft() {
    if ((this.selected || this.options.itemsAlwaysDraggable.range) && this.editable.updateTime && !this.dom.dragLeft) {
      const e = document.createElement("div");
      e.className = "vis-drag-left", e.dragLeftItem = this, this.dom.box.appendChild(e), this.dom.dragLeft = e;
    } else !this.selected && !this.options.itemsAlwaysDraggable.range && this.dom.dragLeft && (this.dom.dragLeft.parentNode && this.dom.dragLeft.parentNode.removeChild(this.dom.dragLeft), this.dom.dragLeft = null);
  }
  /**
   * Repaint a drag area on the right side of the range when the range is selected
   * @protected
   */
  _repaintDragRight() {
    if ((this.selected || this.options.itemsAlwaysDraggable.range) && this.editable.updateTime && !this.dom.dragRight) {
      const e = document.createElement("div");
      e.className = "vis-drag-right", e.dragRightItem = this, this.dom.box.appendChild(e), this.dom.dragRight = e;
    } else !this.selected && !this.options.itemsAlwaysDraggable.range && this.dom.dragRight && (this.dom.dragRight.parentNode && this.dom.dragRight.parentNode.removeChild(this.dom.dragRight), this.dom.dragRight = null);
  }
}
ti.prototype.baseClassName = "vis-item vis-range";
class li extends Li {
  /**
   * @constructor BackgroundItem
   * @param {Object} data             Object containing parameters start, end
   *                                  content, className.
   * @param {{toScreen: function, toTime: function}} conversion
   *                                  Conversion functions from time to screen and vice versa
   * @param {Object} [options]        Configuration options
   *                                  // TODO: describe options
   * // TODO: implement support for the BackgroundItem just having a start, then being displayed as a sort of an annotation
   */
  constructor(e, t, r) {
    if (super(e, t, r), this.props = {
      content: {
        width: 0
      }
    }, this.overflow = !1, e) {
      if (e.start == null)
        throw new Error('Property "start" missing in item '.concat(e.id));
      if (e.end == null)
        throw new Error('Property "end" missing in item '.concat(e.id));
    }
  }
  /**
   * Check whether this item is visible inside given range
   * @param {timeline.Range} range with a timestamp for start and end
   * @returns {boolean} True if visible
   */
  isVisible(e) {
    return this.data.start < e.end && this.data.end > e.start;
  }
  /**
   * create DOM element
   * @private
   */
  _createDomElement() {
    this.dom || (this.dom = {}, this.dom.box = document.createElement("div"), this.dom.frame = document.createElement("div"), this.dom.frame.className = "vis-item-overflow", this.dom.box.appendChild(this.dom.frame), this.dom.content = document.createElement("div"), this.dom.content.className = "vis-item-content", this.dom.frame.appendChild(this.dom.content), this.dirty = !0);
  }
  /**
   * append DOM element
   * @private
   */
  _appendDomElement() {
    if (!this.parent)
      throw new Error("Cannot redraw item: no parent attached");
    if (!this.dom.box.parentNode) {
      const e = this.parent.dom.background;
      if (!e)
        throw new Error("Cannot redraw item: parent has no background container element");
      e.appendChild(this.dom.box);
    }
    this.displayed = !0;
  }
  /**
   * update DOM Dirty components
   * @private
   */
  _updateDirtyDomComponents() {
    if (this.dirty) {
      this._updateContents(this.dom.content), this._updateDataAttributes(this.dom.content), this._updateStyle(this.dom.box);
      const e = (this.data.className ? " " + this.data.className : "") + (this.selected ? " vis-selected" : "");
      this.dom.box.className = this.baseClassName + e;
    }
  }
  /**
   * get DOM components sizes
   * @return {object}
   * @private
   */
  _getDomComponentsSizes() {
    return this.overflow = window.getComputedStyle(this.dom.content).overflow !== "hidden", {
      content: {
        width: this.dom.content.offsetWidth
      }
    };
  }
  /**
   * update DOM components sizes
   * @param {object} sizes
   * @private
   */
  _updateDomComponentsSizes(e) {
    this.props.content.width = e.content.width, this.height = 0, this.dirty = !1;
  }
  /**
   * repaint DOM additionals
   * @private
   */
  _repaintDomAdditionals() {
  }
  /**
   * Repaint the item
   * @param {boolean} [returnQueue=false]  return the queue
   * @return {boolean} the redraw result or the redraw queue if returnQueue=true
   */
  redraw(e) {
    var t, r, s, a;
    let o;
    const l = [
      // create item DOM
      N(t = this._createDomElement).call(t, this),
      // append DOM to parent DOM
      N(r = this._appendDomElement).call(r, this),
      N(s = this._updateDirtyDomComponents).call(s, this),
      () => {
        if (this.dirty) {
          var c;
          o = N(c = this._getDomComponentsSizes).call(c, this)();
        }
      },
      () => {
        if (this.dirty) {
          var c;
          N(c = this._updateDomComponentsSizes).call(c, this)(o);
        }
      },
      // repaint DOM additionals
      N(a = this._repaintDomAdditionals).call(a, this)
    ];
    if (e)
      return l;
    {
      let c;
      return B(l).call(l, (d) => {
        c = d();
      }), c;
    }
  }
  /**
   * Reposition the item vertically
   * @Override
   */
  repositionY() {
    let e;
    const t = this.options.orientation.item;
    if (this.data.subgroup !== void 0) {
      const r = this.data.subgroup;
      this.dom.box.style.height = "".concat(this.parent.subgroups[r].height, "px"), t == "top" ? this.dom.box.style.top = "".concat(this.parent.top + this.parent.subgroups[r].top, "px") : this.dom.box.style.top = "".concat(this.parent.top + this.parent.height - this.parent.subgroups[r].top - this.parent.subgroups[r].height, "px"), this.dom.box.style.bottom = "";
    } else
      this.parent instanceof Ym ? (e = Math.max(this.parent.height, this.parent.itemSet.body.domProps.center.height, this.parent.itemSet.body.domProps.centerContainer.height), this.dom.box.style.bottom = t == "bottom" ? "0" : "", this.dom.box.style.top = t == "top" ? "0" : "") : (e = this.parent.height, this.dom.box.style.top = "".concat(this.parent.top, "px"), this.dom.box.style.bottom = "");
    this.dom.box.style.height = "".concat(e, "px");
  }
}
li.prototype.baseClassName = "vis-item vis-background";
li.prototype.stack = !1;
li.prototype.show = ti.prototype.show;
li.prototype.hide = ti.prototype.hide;
li.prototype.repositionX = ti.prototype.repositionX;
class ak extends Li {
  /**
   * @param {Object} data             Object containing parameters start
   *                                  content, className.
   * @param {{toScreen: function, toTime: function}} conversion
   *                                  Conversion functions from time to screen and vice versa
   * @param {Object} [options]        Configuration options
   *                                  // TODO: describe available options
   */
  constructor(e, t, r) {
    if (super(e, t, r), this.props = {
      dot: {
        width: 0,
        height: 0
      },
      line: {
        width: 0,
        height: 0
      }
    }, e && e.start == null)
      throw new Error('Property "start" missing in item '.concat(e));
  }
  /**
   * Check whether this item is visible inside given range
   * @param {{start: number, end: number}} range with a timestamp for start and end
   * @returns {boolean} True if visible
   */
  isVisible(e) {
    if (this.cluster)
      return !1;
    let t;
    const r = this.data.align || this.options.align, s = this.width * e.getMillisecondsPerPixel();
    return r == "right" ? t = this.data.start.getTime() > e.start && this.data.start.getTime() - s < e.end : r == "left" ? t = this.data.start.getTime() + s > e.start && this.data.start.getTime() < e.end : t = this.data.start.getTime() + s / 2 > e.start && this.data.start.getTime() - s / 2 < e.end, t;
  }
  /**
   * create DOM element
   * @private
   */
  _createDomElement() {
    this.dom || (this.dom = {}, this.dom.box = document.createElement("DIV"), this.dom.content = document.createElement("DIV"), this.dom.content.className = "vis-item-content", this.dom.box.appendChild(this.dom.content), this.dom.line = document.createElement("DIV"), this.dom.line.className = "vis-line", this.dom.dot = document.createElement("DIV"), this.dom.dot.className = "vis-dot", this.dom.box["vis-item"] = this, this.dirty = !0);
  }
  /**
   * append DOM element
   * @private
   */
  _appendDomElement() {
    if (!this.parent)
      throw new Error("Cannot redraw item: no parent attached");
    if (!this.dom.box.parentNode) {
      const t = this.parent.dom.foreground;
      if (!t) throw new Error("Cannot redraw item: parent has no foreground container element");
      t.appendChild(this.dom.box);
    }
    if (!this.dom.line.parentNode) {
      var e = this.parent.dom.background;
      if (!e) throw new Error("Cannot redraw item: parent has no background container element");
      e.appendChild(this.dom.line);
    }
    if (!this.dom.dot.parentNode) {
      const t = this.parent.dom.axis;
      if (!e) throw new Error("Cannot redraw item: parent has no axis container element");
      t.appendChild(this.dom.dot);
    }
    this.displayed = !0;
  }
  /**
   * update dirty DOM element
   * @private
   */
  _updateDirtyDomComponents() {
    if (this.dirty) {
      this._updateContents(this.dom.content), this._updateDataAttributes(this.dom.box), this._updateStyle(this.dom.box);
      const e = this.editable.updateTime || this.editable.updateGroup, t = (this.data.className ? " " + this.data.className : "") + (this.selected ? " vis-selected" : "") + (e ? " vis-editable" : " vis-readonly");
      this.dom.box.className = "vis-item vis-box".concat(t), this.dom.line.className = "vis-item vis-line".concat(t), this.dom.dot.className = "vis-item vis-dot".concat(t);
    }
  }
  /**
   * get DOM components sizes
   * @return {object}
   * @private
   */
  _getDomComponentsSizes() {
    return {
      previous: {
        right: this.dom.box.style.right,
        left: this.dom.box.style.left
      },
      dot: {
        height: this.dom.dot.offsetHeight,
        width: this.dom.dot.offsetWidth
      },
      line: {
        width: this.dom.line.offsetWidth
      },
      box: {
        width: this.dom.box.offsetWidth,
        height: this.dom.box.offsetHeight
      }
    };
  }
  /**
   * update DOM components sizes
   * @param {object} sizes
   * @private
   */
  _updateDomComponentsSizes(e) {
    this.options.rtl ? this.dom.box.style.right = "0px" : this.dom.box.style.left = "0px", this.props.dot.height = e.dot.height, this.props.dot.width = e.dot.width, this.props.line.width = e.line.width, this.width = e.box.width, this.height = e.box.height, this.options.rtl ? this.dom.box.style.right = e.previous.right : this.dom.box.style.left = e.previous.left, this.dirty = !1;
  }
  /**
   * repaint DOM additionals
   * @private
   */
  _repaintDomAdditionals() {
    this._repaintOnItemUpdateTimeTooltip(this.dom.box), this._repaintDragCenter(), this._repaintDeleteButton(this.dom.box);
  }
  /**
   * Repaint the item
   * @param {boolean} [returnQueue=false]  return the queue
   * @return {boolean} the redraw queue if returnQueue=true
   */
  redraw(e) {
    var t, r, s, a;
    let o;
    const l = [
      // create item DOM
      N(t = this._createDomElement).call(t, this),
      // append DOM to parent DOM
      N(r = this._appendDomElement).call(r, this),
      // update dirty DOM
      N(s = this._updateDirtyDomComponents).call(s, this),
      () => {
        this.dirty && (o = this._getDomComponentsSizes());
      },
      () => {
        if (this.dirty) {
          var c;
          N(c = this._updateDomComponentsSizes).call(c, this)(o);
        }
      },
      // repaint DOM additionals
      N(a = this._repaintDomAdditionals).call(a, this)
    ];
    if (e)
      return l;
    {
      let c;
      return B(l).call(l, (d) => {
        c = d();
      }), c;
    }
  }
  /**
   * Show the item in the DOM (when not already visible). The items DOM will
   * be created when needed.
   * @param {boolean} [returnQueue=false]  whether to return a queue of functions to execute instead of just executing them
   * @return {boolean} the redraw queue if returnQueue=true
   */
  show(e) {
    if (!this.displayed)
      return this.redraw(e);
  }
  /**
   * Hide the item from the DOM (when visible)
   */
  hide() {
    if (this.displayed) {
      const e = this.dom;
      e.box.remove ? e.box.remove() : e.box.parentNode && e.box.parentNode.removeChild(e.box), e.line.remove ? e.line.remove() : e.line.parentNode && e.line.parentNode.removeChild(e.line), e.dot.remove ? e.dot.remove() : e.dot.parentNode && e.dot.parentNode.removeChild(e.dot), this.displayed = !1;
    }
  }
  /**
   * Reposition the item XY
   */
  repositionXY() {
    const e = this.options.rtl, t = function(r, s, a) {
      var o;
      let l = arguments.length > 3 && arguments[3] !== void 0 ? arguments[3] : !1;
      if (s === void 0 && a === void 0) return;
      const c = l ? s * -1 : s;
      if (a === void 0) {
        r.style.transform = "translateX(".concat(c, "px)");
        return;
      }
      if (s === void 0) {
        r.style.transform = "translateY(".concat(a, "px)");
        return;
      }
      r.style.transform = rt(o = "translate(".concat(c, "px, ")).call(o, a, "px)");
    };
    t(this.dom.box, this.boxX, this.boxY, e), t(this.dom.dot, this.dotX, this.dotY, e), t(this.dom.line, this.lineX, this.lineY, e);
  }
  /**
   * Reposition the item horizontally
   * @Override
   */
  repositionX() {
    const e = this.conversion.toScreen(this.data.start), t = this.data.align === void 0 ? this.options.align : this.data.align, r = this.props.line.width, s = this.props.dot.width;
    t == "right" ? (this.boxX = e - this.width, this.lineX = e - r, this.dotX = e - r / 2 - s / 2) : t == "left" ? (this.boxX = e, this.lineX = e, this.dotX = e + r / 2 - s / 2) : (this.boxX = e - this.width / 2, this.lineX = this.options.rtl ? e - r : e - r / 2, this.dotX = e - s / 2), this.options.rtl ? this.right = this.boxX : this.left = this.boxX, this.repositionXY();
  }
  /**
   * Reposition the item vertically
   * @Override
   */
  repositionY() {
    const e = this.options.orientation.item, t = this.dom.line.style;
    if (e == "top") {
      const r = this.parent.top + this.top + 1;
      this.boxY = this.top || 0, t.height = "".concat(r, "px"), t.bottom = "", t.top = "0";
    } else {
      const s = this.parent.itemSet.props.height - this.parent.top - this.parent.height + this.top;
      this.boxY = this.parent.height - this.top - (this.height || 0), t.height = "".concat(s, "px"), t.top = "", t.bottom = "0";
    }
    this.dotY = -this.props.dot.height / 2, this.repositionXY();
  }
  /**
   * Return the width of the item left from its start date
   * @return {number}
   */
  getWidthLeft() {
    return this.width / 2;
  }
  /**
   * Return the width of the item right from its start date
   * @return {number}
   */
  getWidthRight() {
    return this.width / 2;
  }
}
class Rs extends Li {
  /**
   * @constructor Item
   * @param {Object} data             Object containing (optional) parameters type,
   *                                  start, end, content, group, className.
   * @param {{toScreen: function, toTime: function}} conversion
   *                                  Conversion functions from time to screen and vice versa
   * @param {Object} options          Configuration options
   *                                  // TODO: describe available options
   */
  constructor(e, t, r) {
    const s = Yr({}, {
      fitOnDoubleClick: !0
    }, r, {
      editable: !1
    });
    if (super(e, t, s), this.props = {
      content: {
        width: 0,
        height: 0
      }
    }, !e || e.uiItems == null)
      throw new Error('Property "uiItems" missing in item ' + e.id);
    this.id = wi(), this.group = e.group, this._setupRange(), this.emitter = this.data.eventEmitter, this.range = this.data.range, this.attached = !1, this.isCluster = !0, this.data.isCluster = !0;
  }
  /**
   * check if there are items
   * @return {boolean}
   */
  hasItems() {
    return this.data.uiItems && this.data.uiItems.length && this.attached;
  }
  /**
   * set UI items
   * @param {array} items
   */
  setUiItems(e) {
    this.detach(), this.data.uiItems = e, this._setupRange(), this.attach();
  }
  /**
   * check is visible
   * @param {object} range
   * @return {boolean}
   */
  isVisible(e) {
    const t = this.data.end ? this.data.end - this.data.start : 0, r = this.width * e.getMillisecondsPerPixel(), s = Math.max(this.data.start.getTime() + t, this.data.start.getTime() + r);
    return this.data.start < e.end && s > e.start && this.hasItems();
  }
  /**
   * get cluster data
   * @return {object}
   */
  getData() {
    return {
      isCluster: !0,
      id: this.id,
      items: this.data.items || [],
      data: this.data
    };
  }
  /**
   * redraw cluster item
   * @param {boolean} returnQueue
   * @return {boolean}
   */
  redraw(e) {
    var t, r, s, a, o, l, c, d = [
      // create item DOM
      N(t = this._createDomElement).call(t, this),
      // append DOM to parent DOM
      N(r = this._appendDomElement).call(r, this),
      // update dirty DOM
      N(s = this._updateDirtyDomComponents).call(s, this),
      N(a = function() {
        this.dirty && (c = this._getDomComponentsSizes());
      }).call(a, this),
      N(o = function() {
        if (this.dirty) {
          var p;
          N(p = this._updateDomComponentsSizes).call(p, this)(c);
        }
      }).call(o, this),
      // repaint DOM additionals
      N(l = this._repaintDomAdditionals).call(l, this)
    ];
    if (e)
      return d;
    var f;
    return B(d).call(d, function(p) {
      f = p();
    }), f;
  }
  /**
   * show cluster item
   */
  show() {
    this.displayed || this.redraw();
  }
  /**
   * Hide the item from the DOM (when visible)
   */
  hide() {
    if (this.displayed) {
      var e = this.dom;
      e.box.parentNode && e.box.parentNode.removeChild(e.box), this.options.showStipes && (e.line.parentNode && e.line.parentNode.removeChild(e.line), e.dot.parentNode && e.dot.parentNode.removeChild(e.dot)), this.displayed = !1;
    }
  }
  /**
   * reposition item x axis
   */
  repositionX() {
    let e = this.conversion.toScreen(this.data.start), t = this.data.end ? this.conversion.toScreen(this.data.end) : 0;
    if (t)
      this.repositionXWithRanges(e, t);
    else {
      let r = this.data.align === void 0 ? this.options.align : this.data.align;
      this.repositionXWithoutRanges(e, r);
    }
    this.options.showStipes && (this.dom.line.style.display = this._isStipeVisible() ? "block" : "none", this.dom.dot.style.display = this._isStipeVisible() ? "block" : "none", this._isStipeVisible() && this.repositionStype(e, t));
  }
  /**
   * reposition item stype
   * @param {date} start
   * @param {date} end
   */
  repositionStype(e, t) {
    this.dom.line.style.display = "block", this.dom.dot.style.display = "block";
    const r = this.dom.line.offsetWidth, s = this.dom.dot.offsetWidth;
    if (t) {
      const a = r + e + (t - e) / 2, o = a - s / 2, l = this.options.rtl ? a * -1 : a, c = this.options.rtl ? o * -1 : o;
      this.dom.line.style.transform = "translateX(".concat(l, "px)"), this.dom.dot.style.transform = "translateX(".concat(c, "px)");
    } else {
      const a = this.options.rtl ? e * -1 : e, o = this.options.rtl ? (e - s / 2) * -1 : e - s / 2;
      this.dom.line.style.transform = "translateX(".concat(a, "px)"), this.dom.dot.style.transform = "translateX(".concat(o, "px)");
    }
  }
  /**
   * reposition x without ranges
   * @param {date} start
   * @param {string} align
   */
  repositionXWithoutRanges(e, t) {
    t == "right" ? this.options.rtl ? (this.right = e - this.width, this.dom.box.style.right = this.right + "px") : (this.left = e - this.width, this.dom.box.style.left = this.left + "px") : t == "left" ? this.options.rtl ? (this.right = e, this.dom.box.style.right = this.right + "px") : (this.left = e, this.dom.box.style.left = this.left + "px") : this.options.rtl ? (this.right = e - this.width / 2, this.dom.box.style.right = this.right + "px") : (this.left = e - this.width / 2, this.dom.box.style.left = this.left + "px");
  }
  /**
   * reposition x with ranges
   * @param {date} start
   * @param {date} end
   */
  repositionXWithRanges(e, t) {
    let r = Math.round(Math.max(t - e + 0.5, 1));
    this.options.rtl ? this.right = e : this.left = e, this.width = Math.max(r, this.minWidth || 0), this.options.rtl ? this.dom.box.style.right = this.right + "px" : this.dom.box.style.left = this.left + "px", this.dom.box.style.width = r + "px";
  }
  /**
   * reposition item y axis
   */
  repositionY() {
    var e = this.options.orientation.item, t = this.dom.box;
    if (e == "top" ? t.style.top = (this.top || 0) + "px" : t.style.top = (this.parent.height - this.top - this.height || 0) + "px", this.options.showStipes) {
      if (e == "top")
        this.dom.line.style.top = "0", this.dom.line.style.height = this.parent.top + this.top + 1 + "px", this.dom.line.style.bottom = "";
      else {
        var r = this.parent.itemSet.props.height, s = r - this.parent.top - this.parent.height + this.top;
        this.dom.line.style.top = r - s + "px", this.dom.line.style.bottom = "0";
      }
      this.dom.dot.style.top = -this.dom.dot.offsetHeight / 2 + "px";
    }
  }
  /**
   * get width left
   * @return {number}
   */
  getWidthLeft() {
    return this.width / 2;
  }
  /**
   * get width right
   * @return {number}
   */
  getWidthRight() {
    return this.width / 2;
  }
  /**
   * move cluster item
   */
  move() {
    this.repositionX(), this.repositionY();
  }
  /**
   * attach
   */
  attach() {
    var e;
    for (let t of this.data.uiItems)
      t.cluster = this;
    this.data.items = Ae(e = this.data.uiItems).call(e, (t) => t.data), this.attached = !0, this.dirty = !0;
  }
  /**
   * detach
   * @param {boolean} detachFromParent
   * @return {void}
   */
  detach() {
    let e = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : !1;
    if (this.hasItems()) {
      for (let t of this.data.uiItems)
        delete t.cluster;
      this.attached = !1, e && this.group && (this.group.remove(this), this.group = null), this.data.items = [], this.dirty = !0;
    }
  }
  /**
   * handle on double click
   */
  _onDoubleClick() {
    this._fit();
  }
  /**
   * set range
   */
  _setupRange() {
    var e, t, r;
    const s = Ae(e = this.data.uiItems).call(e, (l) => ({
      start: l.data.start.valueOf(),
      end: l.data.end ? l.data.end.valueOf() : l.data.start.valueOf()
    }));
    this.data.min = Math.min(...Ae(s).call(s, (l) => Math.min(l.start, l.end || l.start))), this.data.max = Math.max(...Ae(s).call(s, (l) => Math.max(l.start, l.end || l.start)));
    const a = Ae(t = this.data.uiItems).call(t, (l) => l.center), o = Aa(a).call(a, (l, c) => l + c, 0) / this.data.uiItems.length;
    bm(r = this.data.uiItems).call(r, (l) => l.data.end) ? (this.data.start = new Date(this.data.min), this.data.end = new Date(this.data.max)) : (this.data.start = new Date(o), this.data.end = null);
  }
  /**
   * get UI items
   * @return {array}
   */
  _getUiItems() {
    if (this.data.uiItems && this.data.uiItems.length) {
      var e;
      return ie(e = this.data.uiItems).call(e, (t) => t.cluster === this);
    }
    return [];
  }
  /**
   * create DOM element
   */
  _createDomElement() {
    if (!this.dom) {
      if (this.dom = {}, this.dom.box = document.createElement("DIV"), this.dom.content = document.createElement("DIV"), this.dom.content.className = "vis-item-content", this.dom.box.appendChild(this.dom.content), this.options.showStipes && (this.dom.line = document.createElement("DIV"), this.dom.line.className = "vis-cluster-line", this.dom.line.style.display = "none", this.dom.dot = document.createElement("DIV"), this.dom.dot.className = "vis-cluster-dot", this.dom.dot.style.display = "none"), this.options.fitOnDoubleClick) {
        var e;
        this.dom.box.ondblclick = N(e = Rs.prototype._onDoubleClick).call(e, this);
      }
      this.dom.box["vis-item"] = this, this.dirty = !0;
    }
  }
  /**
   * append element to DOM
   */
  _appendDomElement() {
    if (!this.parent)
      throw new Error("Cannot redraw item: no parent attached");
    if (!this.dom.box.parentNode) {
      const r = this.parent.dom.foreground;
      if (!r)
        throw new Error("Cannot redraw item: parent has no foreground container element");
      r.appendChild(this.dom.box);
    }
    const e = this.parent.dom.background;
    if (this.options.showStipes) {
      if (!this.dom.line.parentNode) {
        if (!e) throw new Error("Cannot redraw item: parent has no background container element");
        e.appendChild(this.dom.line);
      }
      if (!this.dom.dot.parentNode) {
        var t = this.parent.dom.axis;
        if (!e) throw new Error("Cannot redraw item: parent has no axis container element");
        t.appendChild(this.dom.dot);
      }
    }
    this.displayed = !0;
  }
  /**
   * update dirty DOM components
   */
  _updateDirtyDomComponents() {
    if (this.dirty) {
      this._updateContents(this.dom.content), this._updateDataAttributes(this.dom.box), this._updateStyle(this.dom.box);
      const e = this.baseClassName + " " + (this.data.className ? " " + this.data.className : "") + (this.selected ? " vis-selected" : "") + " vis-readonly";
      this.dom.box.className = "vis-item " + e, this.options.showStipes && (this.dom.line.className = "vis-item vis-cluster-line " + (this.selected ? " vis-selected" : ""), this.dom.dot.className = "vis-item vis-cluster-dot " + (this.selected ? " vis-selected" : "")), this.data.end && (this.dom.content.style.maxWidth = "none");
    }
  }
  /**
   * get DOM components sizes
   * @return {object}
   */
  _getDomComponentsSizes() {
    const e = {
      previous: {
        right: this.dom.box.style.right,
        left: this.dom.box.style.left
      },
      box: {
        width: this.dom.box.offsetWidth,
        height: this.dom.box.offsetHeight
      }
    };
    return this.options.showStipes && (e.dot = {
      height: this.dom.dot.offsetHeight,
      width: this.dom.dot.offsetWidth
    }, e.line = {
      width: this.dom.line.offsetWidth
    }), e;
  }
  /**
   * update DOM components sizes
   * @param {object} sizes
   */
  _updateDomComponentsSizes(e) {
    this.options.rtl ? this.dom.box.style.right = "0px" : this.dom.box.style.left = "0px", this.data.end ? this.minWidth = e.box.width : this.width = e.box.width, this.height = e.box.height, this.options.rtl ? this.dom.box.style.right = e.previous.right : this.dom.box.style.left = e.previous.left, this.dirty = !1;
  }
  /**
   * repaint DOM additional components
   */
  _repaintDomAdditionals() {
    this._repaintOnItemUpdateTimeTooltip(this.dom.box);
  }
  /**
   * check is stripe visible
   * @return {number}
   * @private
   */
  _isStipeVisible() {
    return this.minWidth >= this.width || !this.data.end;
  }
  /**
   * get fit range
   * @return {object}
   * @private
   */
  _getFitRange() {
    const e = 0.05 * (this.data.max - this.data.min) / 2;
    return {
      fitStart: this.data.min - e,
      fitEnd: this.data.max + e
    };
  }
  /**
   * fit
   * @private
   */
  _fit() {
    if (this.emitter) {
      const e = this._getFitRange(), t = e.fitStart, r = e.fitEnd, s = {
        start: new Date(t),
        end: new Date(r),
        animation: !0
      };
      this.emitter.emit("fit", s);
    }
  }
  /**
   * get item data
   * @return {object}
   * @private
   */
  _getItemData() {
    return this.data;
  }
}
Rs.prototype.baseClassName = "vis-item vis-range vis-cluster";
class ok extends Li {
  /**
   * @param {Object} data             Object containing parameters start
   *                                  content, className.
   * @param {{toScreen: function, toTime: function}} conversion
   *                                  Conversion functions from time to screen and vice versa
   * @param {Object} [options]        Configuration options
   *                                  // TODO: describe available options
   */
  constructor(e, t, r) {
    if (super(e, t, r), this.props = {
      dot: {
        top: 0,
        width: 0,
        height: 0
      },
      content: {
        height: 0,
        marginLeft: 0,
        marginRight: 0
      }
    }, e && e.start == null)
      throw new Error('Property "start" missing in item '.concat(e));
  }
  /**
   * Check whether this item is visible inside given range
   * @param {{start: number, end: number}} range with a timestamp for start and end
   * @returns {boolean} True if visible
   */
  isVisible(e) {
    if (this.cluster)
      return !1;
    const t = this.width * e.getMillisecondsPerPixel();
    return this.data.start.getTime() + t > e.start && this.data.start < e.end;
  }
  /**
   * create DOM element
   * @private
   */
  _createDomElement() {
    this.dom || (this.dom = {}, this.dom.point = document.createElement("div"), this.dom.content = document.createElement("div"), this.dom.content.className = "vis-item-content", this.dom.point.appendChild(this.dom.content), this.dom.dot = document.createElement("div"), this.dom.point.appendChild(this.dom.dot), this.dom.point["vis-item"] = this, this.dirty = !0);
  }
  /**
   * append DOM element
   * @private
   */
  _appendDomElement() {
    if (!this.parent)
      throw new Error("Cannot redraw item: no parent attached");
    if (!this.dom.point.parentNode) {
      const e = this.parent.dom.foreground;
      if (!e)
        throw new Error("Cannot redraw item: parent has no foreground container element");
      e.appendChild(this.dom.point);
    }
    this.displayed = !0;
  }
  /**
   * update dirty DOM components
   * @private
   */
  _updateDirtyDomComponents() {
    if (this.dirty) {
      this._updateContents(this.dom.content), this._updateDataAttributes(this.dom.point), this._updateStyle(this.dom.point);
      const e = this.editable.updateTime || this.editable.updateGroup, t = (this.data.className ? " " + this.data.className : "") + (this.selected ? " vis-selected" : "") + (e ? " vis-editable" : " vis-readonly");
      this.dom.point.className = "vis-item vis-point".concat(t), this.dom.dot.className = "vis-item vis-dot".concat(t);
    }
  }
  /**
   * get DOM component sizes
   * @return {object}
   * @private
   */
  _getDomComponentsSizes() {
    return {
      dot: {
        width: this.dom.dot.offsetWidth,
        height: this.dom.dot.offsetHeight
      },
      content: {
        width: this.dom.content.offsetWidth,
        height: this.dom.content.offsetHeight
      },
      point: {
        width: this.dom.point.offsetWidth,
        height: this.dom.point.offsetHeight
      }
    };
  }
  /**
   * update DOM components sizes
   * @param {array} sizes
   * @private
   */
  _updateDomComponentsSizes(e) {
    this.props.dot.width = e.dot.width, this.props.dot.height = e.dot.height, this.props.content.height = e.content.height, this.options.rtl ? this.dom.content.style.marginRight = "".concat(this.props.dot.width / 2, "px") : this.dom.content.style.marginLeft = "".concat(this.props.dot.width / 2, "px"), this.width = e.point.width, this.height = e.point.height, this.dom.dot.style.top = "".concat((this.height - this.props.dot.height) / 2, "px");
    const t = this.props.dot.width, r = this.options.rtl ? t / 2 : t / 2 * -1;
    this.dom.dot.style.transform = "translateX(".concat(r, "px"), this.dirty = !1;
  }
  /**
   * Repain DOM additionals
   * @private
   */
  _repaintDomAdditionals() {
    this._repaintOnItemUpdateTimeTooltip(this.dom.point), this._repaintDragCenter(), this._repaintDeleteButton(this.dom.point);
  }
  /**
   * Repaint the item
   * @param {boolean} [returnQueue=false]  return the queue
   * @return {boolean} the redraw queue if returnQueue=true
   */
  redraw(e) {
    var t, r, s, a;
    let o;
    const l = [
      // create item DOM
      N(t = this._createDomElement).call(t, this),
      // append DOM to parent DOM
      N(r = this._appendDomElement).call(r, this),
      // update dirty DOM
      N(s = this._updateDirtyDomComponents).call(s, this),
      () => {
        this.dirty && (o = this._getDomComponentsSizes());
      },
      () => {
        if (this.dirty) {
          var c;
          N(c = this._updateDomComponentsSizes).call(c, this)(o);
        }
      },
      // repaint DOM additionals
      N(a = this._repaintDomAdditionals).call(a, this)
    ];
    if (e)
      return l;
    {
      let c;
      return B(l).call(l, (d) => {
        c = d();
      }), c;
    }
  }
  /**
   * Reposition XY
   */
  repositionXY() {
    const e = this.options.rtl;
    (function(r, s, a) {
      var o;
      let l = arguments.length > 3 && arguments[3] !== void 0 ? arguments[3] : !1;
      if (s === void 0 && a === void 0) return;
      const c = l ? s * -1 : s;
      if (a === void 0) {
        r.style.transform = "translateX(".concat(c, "px)");
        return;
      }
      if (s === void 0) {
        r.style.transform = "translateY(".concat(a, "px)");
        return;
      }
      r.style.transform = rt(o = "translate(".concat(c, "px, ")).call(o, a, "px)");
    })(this.dom.point, this.pointX, this.pointY, e);
  }
  /**
   * Show the item in the DOM (when not already visible). The items DOM will
   * be created when needed.
   * @param {boolean} [returnQueue=false]  whether to return a queue of functions to execute instead of just executing them
   * @return {boolean} the redraw queue if returnQueue=true
   */
  show(e) {
    if (!this.displayed)
      return this.redraw(e);
  }
  /**
   * Hide the item from the DOM (when visible)
   */
  hide() {
    this.displayed && (this.dom.point.parentNode && this.dom.point.parentNode.removeChild(this.dom.point), this.displayed = !1);
  }
  /**
   * Reposition the item horizontally
   * @Override
   */
  repositionX() {
    const e = this.conversion.toScreen(this.data.start);
    this.pointX = e, this.options.rtl ? this.right = e - this.props.dot.width : this.left = e - this.props.dot.width, this.repositionXY();
  }
  /**
   * Reposition the item vertically
   * @Override
   */
  repositionY() {
    this.options.orientation.item == "top" ? this.pointY = this.top : this.pointY = this.parent.height - this.top - this.height, this.repositionXY();
  }
  /**
   * Return the width of the item left from its start date
   * @return {number}
   */
  getWidthLeft() {
    return this.props.dot.width;
  }
  /**
   * Return the width of the item right from  its start date
   * @return {number}
   */
  getWidthRight() {
    return this.props.dot.width;
  }
}
var IE = {}, ME;
function kH() {
  if (ME) return IE;
  ME = 1;
  var n = /* @__PURE__ */ X(), e = oi().find, t = /* @__PURE__ */ ys(), r = "find", s = !0;
  return r in [] && Array(1)[r](function() {
    s = !1;
  }), n({ target: "Array", proto: !0, forced: s }, {
    find: function(o) {
      return e(this, o, arguments.length > 1 ? arguments[1] : void 0);
    }
  }), t(r), IE;
}
var tp, AE;
function CH() {
  if (AE) return tp;
  AE = 1, kH();
  var n = /* @__PURE__ */ Ke();
  return tp = n("Array", "find"), tp;
}
var rp, NE;
function RH() {
  if (NE) return rp;
  NE = 1;
  var n = /* @__PURE__ */ He(), e = /* @__PURE__ */ CH(), t = Array.prototype;
  return rp = function(r) {
    var s = r.find;
    return r === t || n(t, r) && s === t.find ? e : s;
  }, rp;
}
var ip, qE;
function PH() {
  if (qE) return ip;
  qE = 1;
  var n = /* @__PURE__ */ RH();
  return ip = n, ip;
}
var np, LE;
function IH() {
  return LE || (LE = 1, np = /* @__PURE__ */ PH()), np;
}
var MH = IH(), AH = /* @__PURE__ */ Q(MH);
class NH {
  /**
   * @param {Element} container       The container object.
   * @param {string}  overflowMethod  How the popup should act to overflowing ('flip', 'cap' or 'none')
   */
  constructor(e, t) {
    this.container = e, this.overflowMethod = t || "cap", this.x = 0, this.y = 0, this.padding = 5, this.hidden = !1, this.frame = document.createElement("div"), this.frame.className = "vis-tooltip", this.container.appendChild(this.frame);
  }
  /**
   * @param {number} x   Horizontal position of the popup window
   * @param {number} y   Vertical position of the popup window
   */
  setPosition(e, t) {
    this.x = Vt(e), this.y = Vt(t);
  }
  /**
   * Set the content for the popup window. This can be HTML code or text.
   * @param {string | Element} content
   */
  setText(e) {
    e instanceof Element ? (this.frame.innerHTML = "", this.frame.appendChild(e)) : this.frame.innerHTML = E.xss(e);
  }
  /**
   * Show the popup window
   * @param {boolean} [doShow]    Show or hide the window
   */
  show(e) {
    if (e === void 0 && (e = !0), e === !0) {
      var t = this.frame.clientHeight, r = this.frame.clientWidth, s = this.frame.parentNode.clientHeight, a = this.frame.parentNode.clientWidth, o = 0, l = 0;
      if (this.overflowMethod == "flip" || this.overflowMethod == "none") {
        let c = !1, d = !0;
        this.overflowMethod == "flip" && (this.y - t < this.padding && (d = !1), this.x + r > a - this.padding && (c = !0)), c ? o = this.x - r : o = this.x, d ? l = this.y - t : l = this.y;
      } else
        l = this.y - t, l + t + this.padding > s && (l = s - t - this.padding), l < this.padding && (l = this.padding), o = this.x, o + r + this.padding > a && (o = a - r - this.padding), o < this.padding && (o = this.padding);
      this.frame.style.left = o + "px", this.frame.style.top = l + "px", this.frame.style.visibility = "visible", this.hidden = !1;
    } else
      this.hide();
  }
  /**
   * Hide the popup window
   */
  hide() {
    this.hidden = !0, this.frame.style.left = "0", this.frame.style.top = "0", this.frame.style.visibility = "hidden";
  }
  /**
   * Remove the popup window
   */
  destroy() {
    this.frame.parentNode.removeChild(this.frame);
  }
}
var FE = {}, $E;
function qH() {
  if ($E) return FE;
  $E = 1;
  var n = /* @__PURE__ */ X(), e = /* @__PURE__ */ eO();
  return n({ target: "String", proto: !0 }, {
    repeat: e
  }), FE;
}
var sp, jE;
function LH() {
  if (jE) return sp;
  jE = 1, qH();
  var n = /* @__PURE__ */ Ke();
  return sp = n("String", "repeat"), sp;
}
var ap, YE;
function FH() {
  if (YE) return ap;
  YE = 1;
  var n = /* @__PURE__ */ He(), e = /* @__PURE__ */ LH(), t = String.prototype;
  return ap = function(r) {
    var s = r.repeat;
    return typeof r == "string" || r === t || n(t, r) && s === t.repeat ? e : s;
  }, ap;
}
var op, HE;
function $H() {
  if (HE) return op;
  HE = 1;
  var n = /* @__PURE__ */ FH();
  return op = n, op;
}
var lp, GE;
function jH() {
  return GE || (GE = 1, lp = /* @__PURE__ */ $H()), lp;
}
var YH = jH(), Qi = /* @__PURE__ */ Q(YH);
function Ga(n, e, t) {
  if (t && !ye(t))
    return Ga(n, e, [t]);
  if (e.hiddenDates = [], t && ye(t) == !0) {
    var r;
    for (let s = 0; s < t.length; s++)
      if (Qi(t[s]) === void 0) {
        const a = {};
        a.start = n(t[s].start).toDate().valueOf(), a.end = n(t[s].end).toDate().valueOf(), e.hiddenDates.push(a);
      }
    gt(r = e.hiddenDates).call(r, (s, a) => s.start - a.start);
  }
}
function rs(n, e, t) {
  if (t && !ye(t))
    return rs(n, e, [t]);
  if (t && e.domProps.centerContainer.width !== void 0) {
    Ga(n, e, t);
    const r = n(e.range.start), s = n(e.range.end), o = (e.range.end - e.range.start) / e.domProps.centerContainer.width;
    for (let p = 0; p < t.length; p++)
      if (Qi(t[p]) !== void 0) {
        let m = n(t[p].start), g = n(t[p].end);
        if (m._d == "Invalid Date")
          throw new Error("Supplied start date is not valid: ".concat(t[p].start));
        if (g._d == "Invalid Date")
          throw new Error("Supplied end date is not valid: ".concat(t[p].end));
        if (g - m >= 4 * o) {
          let _ = 0, x = s.clone();
          switch (Qi(t[p])) {
            case "daily":
              m.day() != g.day() && (_ = 1), m = m.dayOfYear(r.dayOfYear()).year(r.year()).subtract(7, "days"), g = g.dayOfYear(r.dayOfYear()).year(r.year()).subtract(7 - _, "days"), x.add(1, "weeks");
              break;
            case "weekly": {
              const T = g.diff(m, "days"), S = m.day();
              m = m.date(r.date()).month(r.month()).year(r.year()), g = m.clone(), m = m.day(S).subtract(1, "weeks"), g = g.day(S).add(T, "days").subtract(1, "weeks"), x.add(1, "weeks");
              break;
            }
            case "monthly":
              m.month() != g.month() && (_ = 1), m = m.month(r.month()).year(r.year()).subtract(1, "months"), g = g.month(r.month()).year(r.year()).subtract(1, "months").add(_, "months"), x.add(1, "months");
              break;
            case "yearly":
              m.year() != g.year() && (_ = 1), m = m.year(r.year()).subtract(1, "years"), g = g.year(r.year()).subtract(1, "years").add(_, "years"), x.add(1, "years");
              break;
            default:
              console.log("Wrong repeat format, allowed are: daily, weekly, monthly, yearly. Given:", Qi(t[p]));
              return;
          }
          for (; m < x; )
            switch (e.hiddenDates.push({
              start: m.valueOf(),
              end: g.valueOf()
            }), Qi(t[p])) {
              case "daily":
                m = m.add(1, "days"), g = g.add(1, "days");
                break;
              case "weekly":
                m = m.add(1, "weeks"), g = g.add(1, "weeks");
                break;
              case "monthly":
                m = m.add(1, "months"), g = g.add(1, "months");
                break;
              case "yearly":
                m = m.add(1, "y"), g = g.add(1, "y");
                break;
              default:
                console.log("Wrong repeat format, allowed are: daily, weekly, monthly, yearly. Given:", Qi(t[p]));
                return;
            }
          e.hiddenDates.push({
            start: m.valueOf(),
            end: g.valueOf()
          });
        }
      }
    lk(e);
    const l = ns(e.range.start, e.hiddenDates), c = ns(e.range.end, e.hiddenDates);
    let d = e.range.start, f = e.range.end;
    l.hidden == !0 && (d = e.range.startToFront == !0 ? l.startDate - 1 : l.endDate + 1), c.hidden == !0 && (f = e.range.endToFront == !0 ? c.startDate - 1 : c.endDate + 1), (l.hidden == !0 || c.hidden == !0) && e.range._applyRange(d, f);
  }
}
function lk(n) {
  var e;
  const t = n.hiddenDates, r = [];
  for (var s = 0; s < t.length; s++)
    for (let a = 0; a < t.length; a++)
      s != a && t[a].remove != !0 && t[s].remove != !0 && (t[a].start >= t[s].start && t[a].end <= t[s].end ? t[a].remove = !0 : t[a].start >= t[s].start && t[a].start <= t[s].end ? (t[s].end = t[a].end, t[a].remove = !0) : t[a].end >= t[s].start && t[a].end <= t[s].end && (t[s].start = t[a].start, t[a].remove = !0));
  for (s = 0; s < t.length; s++)
    t[s].remove !== !0 && r.push(t[s]);
  n.hiddenDates = r, gt(e = n.hiddenDates).call(e, (a, o) => a.start - o.start);
}
function HH(n) {
  for (let e = 0; e < n.length; e++)
    console.log(e, new Date(n[e].start), new Date(n[e].end), n[e].start, n[e].end, n[e].remove);
}
function uk(n, e, t) {
  let r = !1;
  const s = e.current.valueOf();
  for (let o = 0; o < e.hiddenDates.length; o++) {
    const l = e.hiddenDates[o].start;
    var a = e.hiddenDates[o].end;
    if (s >= l && s < a) {
      r = !0;
      break;
    }
  }
  if (r == !0 && s < e._end.valueOf() && s != t) {
    const o = n(t), l = n(a);
    o.year() != l.year() ? e.switchedYear = !0 : o.month() != l.month() ? e.switchedMonth = !0 : o.dayOfYear() != l.dayOfYear() && (e.switchedDay = !0), e.current = l;
  }
}
function Up(n, e, t) {
  let r;
  if (n.body.hiddenDates.length == 0)
    return r = n.range.conversion(t), (e.valueOf() - r.offset) * r.scale;
  {
    const s = ns(e, n.body.hiddenDates);
    s.hidden == !0 && (e = s.startDate);
    const a = sn(n.body.hiddenDates, n.range.start, n.range.end);
    if (e < n.range.start) {
      r = n.range.conversion(t, a);
      const o = ck(n.body.hiddenDates, e, r.offset);
      return e = n.options.moment(e).toDate().valueOf(), e = e + o, -(r.offset - e.valueOf()) * r.scale;
    } else if (e > n.range.end) {
      const o = {
        start: n.range.start,
        end: e
      };
      return e = Kp(n.options.moment, n.body.hiddenDates, o, e), r = n.range.conversion(t, a), (e.valueOf() - r.offset) * r.scale;
    } else
      return e = Kp(n.options.moment, n.body.hiddenDates, n.range, e), r = n.range.conversion(t, a), (e.valueOf() - r.offset) * r.scale;
  }
}
function Vp(n, e, t) {
  if (n.body.hiddenDates.length == 0) {
    const r = n.range.conversion(t);
    return new Date(e / r.scale + r.offset);
  } else {
    const r = sn(n.body.hiddenDates, n.range.start, n.range.end), a = (n.range.end - n.range.start - r) * e / t, o = dk(n.body.hiddenDates, n.range, a);
    return new Date(o + a + n.range.start);
  }
}
function sn(n, e, t) {
  let r = 0;
  for (let s = 0; s < n.length; s++) {
    const a = n[s].start, o = n[s].end;
    a >= e && o < t && (r += o - a);
  }
  return r;
}
function ck(n, e, t) {
  let r = 0;
  for (let s = 0; s < n.length; s++) {
    const a = n[s].start, o = n[s].end;
    a >= e && o <= t && (r += o - a);
  }
  return r;
}
function Kp(n, e, t, r) {
  return r = n(r).toDate().valueOf(), r -= is(n, e, t, r), r;
}
function is(n, e, t, r) {
  let s = 0;
  r = n(r).toDate().valueOf();
  for (let a = 0; a < e.length; a++) {
    const o = e[a].start, l = e[a].end;
    o >= t.start && l < t.end && r >= l && (s += l - o);
  }
  return s;
}
function dk(n, e, t) {
  let r = 0, s = 0, a = e.start;
  for (let o = 0; o < n.length; o++) {
    const l = n[o].start, c = n[o].end;
    if (l >= e.start && c < e.end) {
      if (s += l - a, a = c, s >= t)
        break;
      r += c - l;
    }
  }
  return r;
}
function Qr(n, e, t, r) {
  const s = ns(e, n);
  return s.hidden == !0 ? t < 0 ? r == !0 ? s.startDate - (s.endDate - e) - 1 : s.startDate - 1 : r == !0 ? s.endDate + (e - s.startDate) + 1 : s.endDate + 1 : e;
}
function ns(n, e) {
  for (let s = 0; s < e.length; s++) {
    var t = e[s].start, r = e[s].end;
    if (n >= t && n < r)
      return {
        hidden: !0,
        startDate: t,
        endDate: r
      };
  }
  return {
    hidden: !1,
    startDate: t,
    endDate: r
  };
}
var GH = /* @__PURE__ */ Object.freeze({
  __proto__: null,
  convertHiddenOptions: Ga,
  correctTimeForHidden: Kp,
  getAccumulatedHiddenDuration: dk,
  getHiddenDurationBefore: is,
  getHiddenDurationBeforeStart: ck,
  getHiddenDurationBetween: sn,
  getIsHidden: ns,
  printDates: HH,
  removeDuplicates: lk,
  snapAwayFromHidden: Qr,
  stepOverHiddenDates: uk,
  toScreen: Up,
  toTime: Vp,
  updateHiddenDates: rs
});
class ri {
  /**
   * @param {Date} [start]         The start date, for example new Date(2010, 9, 21)
   *                               or new Date(2010, 9, 21, 23, 45, 00)
   * @param {Date} [end]           The end date
   * @param {number} [minimumStep] Optional. Minimum step size in milliseconds
   * @param {Date|Array.<Date>} [hiddenDates] Optional.
   * @param {{showMajorLabels: boolean, showWeekScale: boolean}} [options] Optional.
   * @constructor  TimeStep
   */
  constructor(e, t, r, s, a) {
    this.moment = a && a.moment || Et, this.options = a || {}, this.current = this.moment(), this._start = this.moment(), this._end = this.moment(), this.autoScale = !0, this.scale = "day", this.step = 1, this.setRange(e, t, r), this.switchedDay = !1, this.switchedMonth = !1, this.switchedYear = !1, ye(s) ? this.hiddenDates = s : s != null ? this.hiddenDates = [s] : this.hiddenDates = [], this.format = ri.FORMAT;
  }
  /**
   * Set custom constructor function for moment. Can be used to set dates
   * to UTC or to set a utcOffset.
   * @param {function} moment
   */
  setMoment(e) {
    this.moment = e, this.current = this.moment(this.current.valueOf()), this._start = this.moment(this._start.valueOf()), this._end = this.moment(this._end.valueOf());
  }
  /**
   * Set custom formatting for the minor an major labels of the TimeStep.
   * Both `minorLabels` and `majorLabels` are an Object with properties:
   * 'millisecond', 'second', 'minute', 'hour', 'weekday', 'day', 'week', 'month', 'year'.
   * @param {{minorLabels: Object, majorLabels: Object}} format
   */
  setFormat(e) {
    const t = E.deepExtend({}, ri.FORMAT);
    this.format = E.deepExtend(t, e);
  }
  /**
   * Set a new range
   * If minimumStep is provided, the step size is chosen as close as possible
   * to the minimumStep but larger than minimumStep. If minimumStep is not
   * provided, the scale is set to 1 DAY.
   * The minimumStep should correspond with the onscreen size of about 6 characters
   * @param {Date} [start]      The start date and time.
   * @param {Date} [end]        The end date and time.
   * @param {int} [minimumStep] Optional. Minimum step size in milliseconds
   */
  setRange(e, t, r) {
    if (!(e instanceof Date) || !(t instanceof Date))
      throw "No legal start or end date in method setRange";
    this._start = e != null ? this.moment(e.valueOf()) : Ut(), this._end = t != null ? this.moment(t.valueOf()) : Ut(), this.autoScale && this.setMinimumStep(r);
  }
  /**
   * Set the range iterator to the start date.
   */
  start() {
    this.current = this._start.clone(), this.roundToMinor();
  }
  /**
   * Round the current date to the first minor date value
   * This must be executed once when the current date is set to start Date
   */
  roundToMinor() {
    switch (this.scale == "week" && this.current.weekday(0), this.scale) {
      case "year":
        this.current = this.current.year(this.step * Math.floor(this.current.year() / this.step)).month(0);
      // eslint-disable-next-line no-fallthrough
      case "month":
        this.current = this.current.date(1);
      // eslint-disable-next-line no-fallthrough
      case "week":
      case "day":
      case "weekday":
        this.current = this.current.hours(0);
      // eslint-disable-next-line no-fallthrough
      case "hour":
        this.current = this.current.minutes(0);
      // eslint-disable-next-line no-fallthrough
      case "minute":
        this.current = this.current.seconds(0);
      // eslint-disable-next-line no-fallthrough
      case "second":
        this.current = this.current.milliseconds(0);
    }
    if (this.step != 1) {
      let e = this.current.clone();
      switch (this.scale) {
        case "millisecond":
          this.current = this.current.subtract(this.current.milliseconds() % this.step, "milliseconds");
          break;
        case "second":
          this.current = this.current.subtract(this.current.seconds() % this.step, "seconds");
          break;
        case "minute":
          this.current = this.current.subtract(this.current.minutes() % this.step, "minutes");
          break;
        case "hour":
          this.current = this.current.subtract(this.current.hours() % this.step, "hours");
          break;
        case "weekday":
        // intentional fall through
        case "day":
          this.current = this.current.subtract((this.current.date() - 1) % this.step, "day");
          break;
        case "week":
          this.current = this.current.subtract(this.current.week() % this.step, "week");
          break;
        case "month":
          this.current = this.current.subtract(this.current.month() % this.step, "month");
          break;
        case "year":
          this.current = this.current.subtract(this.current.year() % this.step, "year");
          break;
      }
      e.isSame(this.current) || (this.current = this.moment(Qr(this.hiddenDates, this.current.valueOf(), -1, !0)));
    }
  }
  /**
   * Check if the there is a next step
   * @return {boolean}  true if the current date has not passed the end date
   */
  hasNext() {
    return this.current.valueOf() <= this._end.valueOf();
  }
  /**
   * Do the next step
   */
  next() {
    const e = this.current.valueOf();
    switch (this.scale) {
      case "millisecond":
        this.current = this.current.add(this.step, "millisecond");
        break;
      case "second":
        this.current = this.current.add(this.step, "second");
        break;
      case "minute":
        this.current = this.current.add(this.step, "minute");
        break;
      case "hour":
        this.current = this.current.add(this.step, "hour"), this.current.month() < 6 ? this.current = this.current.subtract(this.current.hours() % this.step, "hour") : this.current.hours() % this.step !== 0 && (this.current = this.current.add(this.step - this.current.hours() % this.step, "hour"));
        break;
      case "weekday":
      // intentional fall through
      case "day":
        this.current = this.current.add(this.step, "day");
        break;
      case "week":
        if (this.current.weekday() !== 0)
          this.current = this.current.weekday(0).add(this.step, "week");
        else if (this.options.showMajorLabels === !1)
          this.current = this.current.add(this.step, "week");
        else {
          const t = this.current.clone();
          t.add(1, "week"), t.isSame(this.current, "month") ? this.current = this.current.add(this.step, "week") : this.current = this.current.add(this.step, "week").date(1);
        }
        break;
      case "month":
        this.current = this.current.add(this.step, "month");
        break;
      case "year":
        this.current = this.current.add(this.step, "year");
        break;
    }
    if (this.step != 1)
      switch (this.scale) {
        case "millisecond":
          this.current.milliseconds() > 0 && this.current.milliseconds() < this.step && (this.current = this.current.milliseconds(0));
          break;
        case "second":
          this.current.seconds() > 0 && this.current.seconds() < this.step && (this.current = this.current.seconds(0));
          break;
        case "minute":
          this.current.minutes() > 0 && this.current.minutes() < this.step && (this.current = this.current.minutes(0));
          break;
        case "hour":
          this.current.hours() > 0 && this.current.hours() < this.step && (this.current = this.current.hours(0));
          break;
        case "weekday":
        // intentional fall through
        case "day":
          this.current.date() < this.step + 1 && (this.current = this.current.date(1));
          break;
        case "week":
          this.current.week() < this.step && (this.current = this.current.week(1));
          break;
        case "month":
          this.current.month() < this.step && (this.current = this.current.month(0));
          break;
      }
    this.current.valueOf() == e && (this.current = this._end.clone()), this.switchedDay = !1, this.switchedMonth = !1, this.switchedYear = !1, uk(this.moment, this, e);
  }
  /**
   * Get the current datetime
   * @return {Moment}  current The current date
   */
  getCurrent() {
    return this.current.clone();
  }
  /**
   * Set a custom scale. Autoscaling will be disabled.
   * For example setScale('minute', 5) will result
   * in minor steps of 5 minutes, and major steps of an hour.
   *
   * @param {{scale: string, step: number}} params
   *                               An object containing two properties:
   *                               - A string 'scale'. Choose from 'millisecond', 'second',
   *                                 'minute', 'hour', 'weekday', 'day', 'week', 'month', 'year'.
   *                               - A number 'step'. A step size, by default 1.
   *                                 Choose for example 1, 2, 5, or 10.
   */
  setScale(e) {
    e && typeof e.scale == "string" && (this.scale = e.scale, this.step = e.step > 0 ? e.step : 1, this.autoScale = !1);
  }
  /**
   * Enable or disable autoscaling
   * @param {boolean} enable  If true, autoascaling is set true
   */
  setAutoScale(e) {
    this.autoScale = e;
  }
  /**
   * Automatically determine the scale that bests fits the provided minimum step
   * @param {number} [minimumStep]  The minimum step size in milliseconds
   */
  setMinimumStep(e) {
    if (e == null)
      return;
    const t = 1e3 * 60 * 60 * 24 * 30 * 12, r = 1e3 * 60 * 60 * 24 * 30, s = 1e3 * 60 * 60 * 24, a = 1e3 * 60 * 60, o = 1e3 * 60, l = 1e3, c = 1;
    t * 1e3 > e && (this.scale = "year", this.step = 1e3), t * 500 > e && (this.scale = "year", this.step = 500), t * 100 > e && (this.scale = "year", this.step = 100), t * 50 > e && (this.scale = "year", this.step = 50), t * 10 > e && (this.scale = "year", this.step = 10), t * 5 > e && (this.scale = "year", this.step = 5), t > e && (this.scale = "year", this.step = 1), r * 3 > e && (this.scale = "month", this.step = 3), r > e && (this.scale = "month", this.step = 1), s * 7 > e && this.options.showWeekScale && (this.scale = "week", this.step = 1), s * 2 > e && (this.scale = "day", this.step = 2), s > e && (this.scale = "day", this.step = 1), s / 2 > e && (this.scale = "weekday", this.step = 1), a * 4 > e && (this.scale = "hour", this.step = 4), a > e && (this.scale = "hour", this.step = 1), o * 15 > e && (this.scale = "minute", this.step = 15), o * 10 > e && (this.scale = "minute", this.step = 10), o * 5 > e && (this.scale = "minute", this.step = 5), o > e && (this.scale = "minute", this.step = 1), l * 15 > e && (this.scale = "second", this.step = 15), l * 10 > e && (this.scale = "second", this.step = 10), l * 5 > e && (this.scale = "second", this.step = 5), l > e && (this.scale = "second", this.step = 1), c * 200 > e && (this.scale = "millisecond", this.step = 200), c * 100 > e && (this.scale = "millisecond", this.step = 100), c * 50 > e && (this.scale = "millisecond", this.step = 50), c * 10 > e && (this.scale = "millisecond", this.step = 10), c * 5 > e && (this.scale = "millisecond", this.step = 5), c > e && (this.scale = "millisecond", this.step = 1);
  }
  /**
   * Snap a date to a rounded value.
   * The snap intervals are dependent on the current scale and step.
   * Static function
   * @param {Date} date    the date to be snapped.
   * @param {string} scale Current scale, can be 'millisecond', 'second',
   *                       'minute', 'hour', 'weekday, 'day', 'week', 'month', 'year'.
   * @param {number} step  Current step (1, 2, 4, 5, ...
   * @return {Date} snappedDate
   */
  static snap(e, t, r) {
    let s = Et(e);
    if (t == "year") {
      const a = s.year() + Math.round(s.month() / 12);
      s = s.year(Math.round(a / r) * r).month(0).date(0).hours(0).minutes(0).seconds(0).milliseconds(0);
    } else if (t == "month")
      s.date() > 15 ? s = s.date(1).add(1, "month") : s = s.date(1), s = s.hours(0).minutes(0).seconds(0).milliseconds(0);
    else if (t == "week")
      s.weekday() > 2 ? s = s.weekday(0).add(1, "week") : s = s.weekday(0), s = s.hours(0).minutes(0).seconds(0).milliseconds(0);
    else if (t == "day") {
      switch (r) {
        case 5:
        case 2:
          s = s.hours(Math.round(s.hours() / 24) * 24);
          break;
        default:
          s = s.hours(Math.round(s.hours() / 12) * 12);
          break;
      }
      s = s.minutes(0).seconds(0).milliseconds(0);
    } else if (t == "weekday") {
      switch (r) {
        case 5:
        case 2:
          s = s.hours(Math.round(s.hours() / 12) * 12);
          break;
        default:
          s = s.hours(Math.round(s.hours() / 6) * 6);
          break;
      }
      s = s.minutes(0).seconds(0).milliseconds(0);
    } else if (t == "hour") {
      switch (r) {
        case 4:
          s = s.minutes(Math.round(s.minutes() / 60) * 60);
          break;
        default:
          s = s.minutes(Math.round(s.minutes() / 30) * 30);
          break;
      }
      s = s.seconds(0).milliseconds(0);
    } else if (t == "minute") {
      switch (r) {
        case 15:
        case 10:
          s = s.minutes(Math.round(s.minutes() / 5) * 5).seconds(0);
          break;
        case 5:
          s = s.seconds(Math.round(s.seconds() / 60) * 60);
          break;
        default:
          s = s.seconds(Math.round(s.seconds() / 30) * 30);
          break;
      }
      s = s.milliseconds(0);
    } else if (t == "second")
      switch (r) {
        case 15:
        case 10:
          s = s.seconds(Math.round(s.seconds() / 5) * 5).milliseconds(0);
          break;
        case 5:
          s = s.milliseconds(Math.round(s.milliseconds() / 1e3) * 1e3);
          break;
        default:
          s = s.milliseconds(Math.round(s.milliseconds() / 500) * 500);
          break;
      }
    else if (t == "millisecond") {
      const a = r > 5 ? r / 2 : 1;
      s = s.milliseconds(Math.round(s.milliseconds() / a) * a);
    }
    return s;
  }
  /**
   * Check if the current value is a major value (for example when the step
   * is DAY, a major value is each first day of the MONTH)
   * @return {boolean} true if current date is major, else false.
   */
  isMajor() {
    if (this.switchedYear == !0)
      switch (this.scale) {
        case "year":
        case "month":
        case "week":
        case "weekday":
        case "day":
        case "hour":
        case "minute":
        case "second":
        case "millisecond":
          return !0;
        default:
          return !1;
      }
    else if (this.switchedMonth == !0)
      switch (this.scale) {
        case "week":
        case "weekday":
        case "day":
        case "hour":
        case "minute":
        case "second":
        case "millisecond":
          return !0;
        default:
          return !1;
      }
    else if (this.switchedDay == !0)
      switch (this.scale) {
        case "millisecond":
        case "second":
        case "minute":
        case "hour":
          return !0;
        default:
          return !1;
      }
    const e = this.moment(this.current);
    switch (this.scale) {
      case "millisecond":
        return e.milliseconds() == 0;
      case "second":
        return e.seconds() == 0;
      case "minute":
        return e.hours() == 0 && e.minutes() == 0;
      case "hour":
        return e.hours() == 0;
      case "weekday":
      // intentional fall through
      case "day":
        return this.options.showWeekScale ? e.isoWeekday() == 1 : e.date() == 1;
      case "week":
        return e.date() == 1;
      case "month":
        return e.month() == 0;
      case "year":
        return !1;
      default:
        return !1;
    }
  }
  /**
   * Returns formatted text for the minor axislabel, depending on the current
   * date and the scale. For example when scale is MINUTE, the current time is
   * formatted as "hh:mm".
   * @param {Date} [date=this.current] custom date. if not provided, current date is taken
   * @returns {String}
   */
  getLabelMinor(e) {
    if (e == null && (e = this.current), e instanceof Date && (e = this.moment(e)), typeof this.format.minorLabels == "function")
      return this.format.minorLabels(e, this.scale, this.step);
    const t = this.format.minorLabels[this.scale];
    switch (this.scale) {
      case "week":
        if (e.date() === 1 && e.weekday() !== 0)
          return "";
      // eslint-disable-next-line no-fallthrough
      default:
        return t && t.length > 0 ? this.moment(e).format(t) : "";
    }
  }
  /**
   * Returns formatted text for the major axis label, depending on the current
   * date and the scale. For example when scale is MINUTE, the major scale is
   * hours, and the hour will be formatted as "hh".
   * @param {Date} [date=this.current] custom date. if not provided, current date is taken
   * @returns {String}
   */
  getLabelMajor(e) {
    if (e == null && (e = this.current), e instanceof Date && (e = this.moment(e)), typeof this.format.majorLabels == "function")
      return this.format.majorLabels(e, this.scale, this.step);
    const t = this.format.majorLabels[this.scale];
    return t && t.length > 0 ? this.moment(e).format(t) : "";
  }
  /**
   * get class name
   * @return {string} class name
   */
  getClassName() {
    var e;
    const t = this.moment, r = this.moment(this.current), s = r.locale ? r.locale("en") : r.lang("en"), a = this.step, o = [];
    function l(m) {
      return m / a % 2 == 0 ? " vis-even" : " vis-odd";
    }
    function c(m) {
      return m.isSame(Ut(), "day") ? " vis-today" : m.isSame(t().add(1, "day"), "day") ? " vis-tomorrow" : m.isSame(t().add(-1, "day"), "day") ? " vis-yesterday" : "";
    }
    function d(m) {
      return m.isSame(Ut(), "week") ? " vis-current-week" : "";
    }
    function f(m) {
      return m.isSame(Ut(), "month") ? " vis-current-month" : "";
    }
    function p(m) {
      return m.isSame(Ut(), "year") ? " vis-current-year" : "";
    }
    switch (this.scale) {
      case "millisecond":
        o.push(c(s)), o.push(l(s.milliseconds()));
        break;
      case "second":
        o.push(c(s)), o.push(l(s.seconds()));
        break;
      case "minute":
        o.push(c(s)), o.push(l(s.minutes()));
        break;
      case "hour":
        o.push(rt(e = "vis-h".concat(s.hours())).call(e, this.step == 4 ? "-h" + (s.hours() + 4) : "")), o.push(c(s)), o.push(l(s.hours()));
        break;
      case "weekday":
        o.push("vis-".concat(s.format("dddd").toLowerCase())), o.push(c(s)), o.push(d(s)), o.push(l(s.date()));
        break;
      case "day":
        o.push("vis-day".concat(s.date())), o.push("vis-".concat(s.format("MMMM").toLowerCase())), o.push(c(s)), o.push(f(s)), o.push(this.step <= 2 ? c(s) : ""), o.push(this.step === 1 ? "vis-".concat(s.format("dddd").toLowerCase()) : ""), o.push(l(s.date() - 1));
        break;
      case "week":
        o.push("vis-week".concat(s.format("w"))), o.push(d(s)), o.push(l(s.week()));
        break;
      case "month":
        o.push("vis-".concat(s.format("MMMM").toLowerCase())), o.push(f(s)), o.push(l(s.month()));
        break;
      case "year":
        o.push("vis-year".concat(s.year())), o.push(p(s)), o.push(l(s.year()));
        break;
    }
    return ie(o).call(o, String).join(" ");
  }
}
ri.FORMAT = {
  minorLabels: {
    millisecond: "SSS",
    second: "s",
    minute: "HH:mm",
    hour: "HH:mm",
    weekday: "ddd D",
    day: "D",
    week: "w",
    month: "MMM",
    year: "YYYY"
  },
  majorLabels: {
    millisecond: "HH:mm:ss",
    second: "D MMMM HH:mm",
    minute: "ddd D MMMM",
    hour: "ddd D MMMM",
    weekday: "MMMM YYYY",
    day: "MMMM YYYY",
    week: "MMMM YYYY",
    month: "YYYY",
    year: ""
  }
};
var BE = {}, WE;
function BH() {
  if (WE) return BE;
  WE = 1;
  var n = /* @__PURE__ */ X(), e = oi().every, t = /* @__PURE__ */ Sn(), r = t("every");
  return n({ target: "Array", proto: !0, forced: !r }, {
    every: function(a) {
      return e(this, a, arguments.length > 1 ? arguments[1] : void 0);
    }
  }), BE;
}
var up, zE;
function WH() {
  if (zE) return up;
  zE = 1, BH();
  var n = /* @__PURE__ */ Ke();
  return up = n("Array", "every"), up;
}
var cp, UE;
function zH() {
  if (UE) return cp;
  UE = 1;
  var n = /* @__PURE__ */ He(), e = /* @__PURE__ */ WH(), t = Array.prototype;
  return cp = function(r) {
    var s = r.every;
    return r === t || n(t, r) && s === t.every ? e : s;
  }, cp;
}
var dp, VE;
function UH() {
  if (VE) return dp;
  VE = 1;
  var n = /* @__PURE__ */ zH();
  return dp = n, dp;
}
var hp, KE;
function VH() {
  return KE || (KE = 1, hp = /* @__PURE__ */ UH()), hp;
}
var KH = VH(), XH = /* @__PURE__ */ Q(KH);
const ZH = "__ungrouped__", QH = {
  UNGROUPED: ZH
};
class JH {
  /**
   * @param {ItemSet} itemSet itemsSet instance
   * @constructor ClusterGenerator
   */
  constructor(e) {
    this.itemSet = e, this.groups = {}, this.cache = {}, this.cache[-1] = [];
  }
  /**
   * @param {Object} itemData             Object containing parameters start content, className.
   * @param {{toScreen: function, toTime: function}} conversion
   *                                  Conversion functions from time to screen and vice versa
   * @param {Object} [options]        Configuration options
   * @return {Object} newItem
   */
  createClusterItem(e, t, r) {
    return new Rs(e, t, r);
  }
  /**
   * Set the items to be clustered.
   * This will clear cached clusters.
   * @param {Item[]} items
   * @param {Object} [options]  Available options:
   *                            {boolean} applyOnChangedLevel
   *                                If true (default), the changed data is applied
   *                                as soon the cluster level changes. If false,
   *                                The changed data is applied immediately
   */
  setItems(e, t) {
    this.items = e || [], this.dataChanged = !0, this.applyOnChangedLevel = !1, t && t.applyOnChangedLevel && (this.applyOnChangedLevel = t.applyOnChangedLevel);
  }
  /**
   * Update the current data set: clear cache, and recalculate the clustering for
   * the current level
   */
  updateData() {
    this.dataChanged = !0, this.applyOnChangedLevel = !1;
  }
  /**
   * Cluster the items which are too close together
   * @param {array} oldClusters
   * @param {number} scale      The scale of the current window : (windowWidth / (endDate - startDate))
   * @param {{maxItems: number, clusterCriteria: function, titleTemplate: string}} options
   * @return {array} clusters
   */
  getClusters(e, t, r) {
    let s = typeof r == "boolean" ? {} : r, a = s.maxItems, o = s.clusterCriteria;
    o || (o = () => !0), a = a || 1;
    let l = -1, c = 2, d = 0;
    if (t > 0) {
      if (t >= 1)
        return [];
      l = Math.abs(Math.round(Math.log(100 / t) / Math.log(c))), d = Math.abs(Math.pow(c, l));
    }
    if (this.dataChanged) {
      const p = l != this.cacheLevel;
      (this.applyOnChangedLevel ? p : !0) && (this._dropLevelsCache(), this._filterData());
    }
    this.cacheLevel = l;
    let f = this.cache[l];
    if (!f) {
      f = [];
      for (let p in this.groups) {
        if (!Object.prototype.hasOwnProperty.call(this.groups, p)) continue;
        const m = this.groups[p], g = m.length;
        let b = 0;
        for (; b < g; ) {
          let _ = m[b], x = 1, T = b - 1;
          for (; T >= 0 && _.center - m[T].center < d / 2; )
            !m[T].cluster && o(_.data, m[T].data) && x++, T--;
          let S = b + 1;
          for (; S < m.length && m[S].center - _.center < d / 2; )
            o(_.data, m[S].data) && x++, S++;
          let k = f.length - 1;
          for (; k >= 0 && _.center - f[k].center < d; )
            _.group == f[k].group && o(_.data, f[k].data) && x++, k--;
          if (x > a) {
            const D = x - a + 1, I = [];
            let C = b;
            for (; I.length < D && C < m.length; )
              o(m[b].data, m[C].data) && I.push(m[C]), C++;
            const P = this.itemSet.getGroupId(_.data), R = this.itemSet.groups[P] || this.itemSet.groups[QH.UNGROUPED];
            let M = this._getClusterForItems(I, R, e, r);
            f.push(M), b += D;
          } else
            delete _.cluster, b += 1;
        }
      }
      this.cache[l] = f;
    }
    return f;
  }
  /**
   * Filter the items per group.
   * @private
   */
  _filterData() {
    const e = {};
    this.groups = e;
    for (const r of _m(this.items)) {
      const s = r.parent ? r.parent.groupId : "";
      let a = e[s];
      a || (a = [], e[s] = a), a.push(r), r.data.start && (r.data.end ? r.center = (r.data.start.valueOf() + r.data.end.valueOf()) / 2 : r.center = r.data.start.valueOf());
    }
    for (let r in e) {
      var t;
      Object.prototype.hasOwnProperty.call(e, r) && gt(t = e[r]).call(t, (s, a) => s.center - a.center);
    }
    this.dataChanged = !1;
  }
  /**
   * Create new cluster or return existing
   * @private
   * @param {array} clusterItems
   * @param {object} group
   * @param {array} oldClusters
   * @param {object} options
   * @returns {object} cluster
   */
  _getClusterForItems(e, t, r, s) {
    var a;
    const o = Ae(a = r || []).call(a, (b) => {
      var _;
      return {
        cluster: b,
        itemsIds: new Jr(Ae(_ = b.data.uiItems).call(_, (x) => x.id))
      };
    });
    let l;
    if (o.length) {
      for (let b of o)
        if (b.itemsIds.size === e.length && XH(e).call(e, (_) => b.itemsIds.has(_.id))) {
          l = b.cluster;
          break;
        }
    }
    if (l)
      return l.setUiItems(e), l.group !== t && (l.group && l.group.remove(l), t && (t.add(l), l.group = t)), l;
    let c = s.titleTemplate || "";
    const d = {
      toScreen: this.itemSet.body.util.toScreen,
      toTime: this.itemSet.body.util.toTime
    }, f = c.replace(/{count}/, e.length), p = '<div title="' + f + '">' + e.length + "</div>", m = Yr({}, s, this.itemSet.options), g = {
      content: p,
      title: f,
      group: t,
      uiItems: e,
      eventEmitter: this.itemSet.body.emitter,
      range: this.itemSet.body.range
    };
    return l = this.createClusterItem(g, d, m), t && (t.add(l), l.group = t), l.attach(), l;
  }
  /**
   * Drop cache
   * @private
   */
  _dropLevelsCache() {
    this.cache = {}, this.cacheLevel = -1, this.cache[this.cacheLevel] = [];
  }
}
const Wi = "__ungrouped__", Zr = "__background__";
class ii extends zr {
  /**
   * @param {{dom: Object, domProps: Object, emitter: Emitter, range: Range}} body
   * @param {Object} [options]      See ItemSet.setOptions for the available options.
   * @constructor ItemSet
   * @extends Component
   */
  constructor(e, t) {
    super(), this.body = e, this.defaultOptions = {
      type: null,
      // 'box', 'point', 'range', 'background'
      orientation: {
        item: "bottom"
        // item orientation: 'top' or 'bottom'
      },
      align: "auto",
      // alignment of box items
      stack: !0,
      stackSubgroups: !0,
      groupOrderSwap(s, a) {
        const o = a.order;
        a.order = s.order, s.order = o;
      },
      groupOrder: "order",
      selectable: !0,
      multiselect: !1,
      longSelectPressTime: 251,
      itemsAlwaysDraggable: {
        item: !1,
        range: !1
      },
      editable: {
        updateTime: !1,
        updateGroup: !1,
        add: !1,
        remove: !1,
        overrideItems: !1
      },
      groupEditable: {
        order: !1,
        add: !1,
        remove: !1
      },
      snap: ri.snap,
      // Only called when `objectData.target === 'item'.
      onDropObjectOnItem(s, a, o) {
        o(a);
      },
      onAdd(s, a) {
        a(s);
      },
      onUpdate(s, a) {
        a(s);
      },
      onMove(s, a) {
        a(s);
      },
      onRemove(s, a) {
        a(s);
      },
      onMoving(s, a) {
        a(s);
      },
      onAddGroup(s, a) {
        a(s);
      },
      onMoveGroup(s, a) {
        a(s);
      },
      onRemoveGroup(s, a) {
        a(s);
      },
      margin: {
        item: {
          horizontal: 10,
          vertical: 10
        },
        axis: 20
      },
      showTooltips: !0,
      tooltip: {
        followMouse: !1,
        overflowMethod: "flip",
        delay: 500
      },
      tooltipOnItemUpdateTime: !1
    }, this.options = E.extend({}, this.defaultOptions), this.options.rtl = t.rtl, this.options.onTimeout = t.onTimeout, this.conversion = {
      toScreen: e.util.toScreen,
      toTime: e.util.toTime
    }, this.dom = {}, this.props = {}, this.hammer = null;
    const r = this;
    this.itemsData = null, this.groupsData = null, this.itemsSettingTime = null, this.initialItemSetDrawn = !1, this.userContinueNotBail = null, this.sequentialSelection = !1, this.itemListeners = {
      add(s, a) {
        r._onAdd(a.items), r.options.cluster && r.clusterGenerator.setItems(r.items, {
          applyOnChangedLevel: !1
        }), r.redraw();
      },
      update(s, a) {
        r._onUpdate(a.items), r.options.cluster && r.clusterGenerator.setItems(r.items, {
          applyOnChangedLevel: !1
        }), r.redraw();
      },
      remove(s, a) {
        r._onRemove(a.items), r.options.cluster && r.clusterGenerator.setItems(r.items, {
          applyOnChangedLevel: !1
        }), r.redraw();
      }
    }, this.groupListeners = {
      add(s, a, o) {
        if (r._onAddGroups(a.items), r.groupsData && r.groupsData.length > 0) {
          var l;
          const c = r.groupsData.getDataSet();
          B(l = c.get()).call(l, (d) => {
            if (d.nestedGroups) {
              var f;
              d.showNested != !1 && (d.showNested = !0);
              let p = [];
              B(f = d.nestedGroups).call(f, (m) => {
                const g = c.get(m);
                g && (g.nestedInGroup = d.id, d.showNested == !1 && (g.visible = !1), p = rt(p).call(p, g));
              }), c.update(p, o);
            }
          });
        }
      },
      update(s, a) {
        r._onUpdateGroups(a.items);
      },
      remove(s, a) {
        r._onRemoveGroups(a.items);
      }
    }, this.items = {}, this.groups = {}, this.groupIds = [], this.selection = [], this.popup = null, this.popupTimer = null, this.touchParams = {}, this.groupTouchParams = {
      group: null,
      isDragging: !1
    }, this._create(), this.setOptions(t), this.clusters = [];
  }
  /**
   * Create the HTML DOM for the ItemSet
   */
  _create() {
    var e, t, r, s, a, o, l, c, d, f, p, m, g, b, _;
    const x = document.createElement("div");
    x.className = "vis-itemset", x["vis-itemset"] = this, this.dom.frame = x;
    const T = document.createElement("div");
    T.className = "vis-background", x.appendChild(T), this.dom.background = T;
    const S = document.createElement("div");
    S.className = "vis-foreground", x.appendChild(S), this.dom.foreground = S;
    const k = document.createElement("div");
    k.className = "vis-axis", this.dom.axis = k;
    const D = document.createElement("div");
    D.className = "vis-labelset", this.dom.labelSet = D, this._updateUngrouped();
    const I = new Ym(Zr, null, this);
    I.show(), this.groups[Zr] = I, this.hammer = new Tt(this.body.dom.centerContainer), this.hammer.on("hammer.input", (C) => {
      C.isFirst && this._onTouch(C);
    }), this.hammer.on("panstart", N(e = this._onDragStart).call(e, this)), this.hammer.on("panmove", N(t = this._onDrag).call(t, this)), this.hammer.on("panend", N(r = this._onDragEnd).call(r, this)), this.hammer.get("pan").set({
      threshold: 5,
      direction: Tt.ALL
    }), this.hammer.get("press").set({
      time: 1e4
    }), this.hammer.on("tap", N(s = this._onSelectItem).call(s, this)), this.hammer.on("press", N(a = this._onMultiSelectItem).call(a, this)), this.hammer.get("press").set({
      time: 1e4
    }), this.hammer.on("doubletap", N(o = this._onAddItem).call(o, this)), this.options.rtl ? this.groupHammer = new Tt(this.body.dom.rightContainer) : this.groupHammer = new Tt(this.body.dom.leftContainer), this.groupHammer.on("tap", N(l = this._onGroupClick).call(l, this)), this.groupHammer.on("panstart", N(c = this._onGroupDragStart).call(c, this)), this.groupHammer.on("panmove", N(d = this._onGroupDrag).call(d, this)), this.groupHammer.on("panend", N(f = this._onGroupDragEnd).call(f, this)), this.groupHammer.get("pan").set({
      threshold: 5,
      direction: Tt.DIRECTION_VERTICAL
    }), this.body.dom.centerContainer.addEventListener("mouseover", N(p = this._onMouseOver).call(p, this)), this.body.dom.centerContainer.addEventListener("mouseout", N(m = this._onMouseOut).call(m, this)), this.body.dom.centerContainer.addEventListener("mousemove", N(g = this._onMouseMove).call(g, this)), this.body.dom.centerContainer.addEventListener("contextmenu", N(b = this._onDragEnd).call(b, this)), this.body.dom.centerContainer.addEventListener("mousewheel", N(_ = this._onMouseWheel).call(_, this)), this.show();
  }
  /**
   * Set options for the ItemSet. Existing options will be extended/overwritten.
   * @param {Object} [options] The following options are available:
   *                           {string} type
   *                              Default type for the items. Choose from 'box'
   *                              (default), 'point', 'range', or 'background'.
   *                              The default style can be overwritten by
   *                              individual items.
   *                           {string} align
   *                              Alignment for the items, only applicable for
   *                              BoxItem. Choose 'center' (default), 'left', or
   *                              'right'.
   *                           {string} orientation.item
   *                              Orientation of the item set. Choose 'top' or
   *                              'bottom' (default).
   *                           {Function} groupOrder
   *                              A sorting function for ordering groups
   *                           {boolean} stack
   *                              If true (default), items will be stacked on
   *                              top of each other.
   *                           {number} margin.axis
   *                              Margin between the axis and the items in pixels.
   *                              Default is 20.
   *                           {number} margin.item.horizontal
   *                              Horizontal margin between items in pixels.
   *                              Default is 10.
   *                           {number} margin.item.vertical
   *                              Vertical Margin between items in pixels.
   *                              Default is 10.
   *                           {number} margin.item
   *                              Margin between items in pixels in both horizontal
   *                              and vertical direction. Default is 10.
   *                           {number} margin
   *                              Set margin for both axis and items in pixels.
   *                           {boolean} selectable
   *                              If true (default), items can be selected.
   *                           {boolean} multiselect
   *                              If true, multiple items can be selected.
   *                              False by default.
   *                           {boolean} editable
   *                              Set all editable options to true or false
   *                           {boolean} editable.updateTime
   *                              Allow dragging an item to an other moment in time
   *                           {boolean} editable.updateGroup
   *                              Allow dragging an item to an other group
   *                           {boolean} editable.add
   *                              Allow creating new items on double tap
   *                           {boolean} editable.remove
   *                              Allow removing items by clicking the delete button
   *                              top right of a selected item.
   *                           {Function(item: Item, callback: Function)} onAdd
   *                              Callback function triggered when an item is about to be added:
   *                              when the user double taps an empty space in the Timeline.
   *                           {Function(item: Item, callback: Function)} onUpdate
   *                              Callback function fired when an item is about to be updated.
   *                              This function typically has to show a dialog where the user
   *                              change the item. If not implemented, nothing happens.
   *                           {Function(item: Item, callback: Function)} onMove
   *                              Fired when an item has been moved. If not implemented,
   *                              the move action will be accepted.
   *                           {Function(item: Item, callback: Function)} onRemove
   *                              Fired when an item is about to be deleted.
   *                              If not implemented, the item will be always removed.
   */
  setOptions(e) {
    if (e) {
      var t, r;
      const s = ["type", "rtl", "align", "order", "stack", "stackSubgroups", "selectable", "multiselect", "sequentialSelection", "multiselectPerGroup", "longSelectPressTime", "groupOrder", "dataAttributes", "template", "groupTemplate", "visibleFrameTemplate", "hide", "snap", "groupOrderSwap", "showTooltips", "tooltip", "tooltipOnItemUpdateTime", "groupHeightMode", "onTimeout"];
      E.selectiveExtend(s, this.options, e), "itemsAlwaysDraggable" in e && (typeof e.itemsAlwaysDraggable == "boolean" ? (this.options.itemsAlwaysDraggable.item = e.itemsAlwaysDraggable, this.options.itemsAlwaysDraggable.range = !1) : typeof e.itemsAlwaysDraggable == "object" && (E.selectiveExtend(["item", "range"], this.options.itemsAlwaysDraggable, e.itemsAlwaysDraggable), this.options.itemsAlwaysDraggable.item || (this.options.itemsAlwaysDraggable.range = !1))), "sequentialSelection" in e && typeof e.sequentialSelection == "boolean" && (this.options.sequentialSelection = e.sequentialSelection), "orientation" in e && (typeof e.orientation == "string" ? this.options.orientation.item = e.orientation === "top" ? "top" : "bottom" : typeof e.orientation == "object" && "item" in e.orientation && (this.options.orientation.item = e.orientation.item)), "margin" in e && (typeof e.margin == "number" ? (this.options.margin.axis = e.margin, this.options.margin.item.horizontal = e.margin, this.options.margin.item.vertical = e.margin) : typeof e.margin == "object" && (E.selectiveExtend(["axis"], this.options.margin, e.margin), "item" in e.margin && (typeof e.margin.item == "number" ? (this.options.margin.item.horizontal = e.margin.item, this.options.margin.item.vertical = e.margin.item) : typeof e.margin.item == "object" && E.selectiveExtend(["horizontal", "vertical"], this.options.margin.item, e.margin.item)))), B(t = ["locale", "locales"]).call(t, (o) => {
        o in e && (this.options[o] = e[o]);
      }), "editable" in e && (typeof e.editable == "boolean" ? (this.options.editable.updateTime = e.editable, this.options.editable.updateGroup = e.editable, this.options.editable.add = e.editable, this.options.editable.remove = e.editable, this.options.editable.overrideItems = !1) : typeof e.editable == "object" && E.selectiveExtend(["updateTime", "updateGroup", "add", "remove", "overrideItems"], this.options.editable, e.editable)), "groupEditable" in e && (typeof e.groupEditable == "boolean" ? (this.options.groupEditable.order = e.groupEditable, this.options.groupEditable.add = e.groupEditable, this.options.groupEditable.remove = e.groupEditable) : typeof e.groupEditable == "object" && E.selectiveExtend(["order", "add", "remove"], this.options.groupEditable, e.groupEditable));
      const a = (o) => {
        const l = e[o];
        if (l) {
          if (typeof l != "function") {
            var c;
            throw new Error(rt(c = "option ".concat(o, " must be a function ")).call(c, o, "(item, callback)"));
          }
          this.options[o] = l;
        }
      };
      B(r = ["onDropObjectOnItem", "onAdd", "onUpdate", "onRemove", "onMove", "onMoving", "onAddGroup", "onMoveGroup", "onRemoveGroup"]).call(r, a), e.cluster ? (Yr(this.options, {
        cluster: e.cluster
      }), this.clusterGenerator || (this.clusterGenerator = new JH(this)), this.clusterGenerator.setItems(this.items, {
        applyOnChangedLevel: !1
      }), this.markDirty({
        refreshItems: !0,
        restackGroups: !0
      }), this.redraw()) : this.clusterGenerator ? (this._detachAllClusters(), this.clusters = [], this.clusterGenerator = null, this.options.cluster = void 0, this.markDirty({
        refreshItems: !0,
        restackGroups: !0
      }), this.redraw()) : this.markDirty();
    }
  }
  /**
   * Mark the ItemSet dirty so it will refresh everything with next redraw.
   * Optionally, all items can be marked as dirty and be refreshed.
   * @param {{refreshItems: boolean}} [options]
   */
  markDirty(e) {
    this.groupIds = [], e && (e.refreshItems && B(E).call(E, this.items, (t) => {
      t.dirty = !0, t.displayed && t.redraw();
    }), e.restackGroups && B(E).call(E, this.groups, (t, r) => {
      r !== Zr && (t.stackDirty = !0);
    }));
  }
  /**
   * Destroy the ItemSet
   */
  destroy() {
    this.clearPopupTimer(), this.hide(), this.setItems(null), this.setGroups(null), this.hammer && this.hammer.destroy(), this.groupHammer && this.groupHammer.destroy(), this.hammer = null, this.body = null, this.conversion = null;
  }
  /**
   * Hide the component from the DOM
   */
  hide() {
    this.dom.frame.parentNode && this.dom.frame.parentNode.removeChild(this.dom.frame), this.dom.axis.parentNode && this.dom.axis.parentNode.removeChild(this.dom.axis), this.dom.labelSet.parentNode && this.dom.labelSet.parentNode.removeChild(this.dom.labelSet);
  }
  /**
   * Show the component in the DOM (when not already visible).
   */
  show() {
    this.dom.frame.parentNode || this.body.dom.center.appendChild(this.dom.frame), this.dom.axis.parentNode || this.body.dom.backgroundVertical.appendChild(this.dom.axis), this.dom.labelSet.parentNode || (this.options.rtl ? this.body.dom.right.appendChild(this.dom.labelSet) : this.body.dom.left.appendChild(this.dom.labelSet));
  }
  /**
   * Activates the popup timer to show the given popup after a fixed time.
   * @param {Popup} popup
   */
  setPopupTimer(e) {
    if (this.clearPopupTimer(), e) {
      const t = this.options.tooltip.delay || typeof this.options.tooltip.delay == "number" ? this.options.tooltip.delay : 500;
      this.popupTimer = Dt(function() {
        e.show();
      }, t);
    }
  }
  /**
   * Clears the popup timer for the tooltip.
   */
  clearPopupTimer() {
    this.popupTimer != null && (clearTimeout(this.popupTimer), this.popupTimer = null);
  }
  /**
   * Set selected items by their id. Replaces the current selection
   * Unknown id's are silently ignored.
   * @param {string[] | string} [ids] An array with zero or more id's of the items to be
   *                                  selected, or a single item id. If ids is undefined
   *                                  or an empty array, all items will be unselected.
   */
  setSelection(e) {
    var t;
    e == null && (e = []), ye(e) || (e = [e]);
    const r = ie(t = this.selection).call(t, (s) => Fe(e).call(e, s) === -1);
    for (let s of r) {
      const a = this.getItemById(s);
      a && a.unselect();
    }
    this.selection = [...e];
    for (let s of e) {
      const a = this.getItemById(s);
      a && a.select();
    }
  }
  /**
   * Get the selected items by their id
   * @return {Array} ids  The ids of the selected items
   */
  getSelection() {
    var e;
    return rt(e = this.selection).call(e, []);
  }
  /**
   * Get the id's of the currently visible items.
   * @returns {Array} The ids of the visible items
   */
  getVisibleItems() {
    const e = this.body.range.getRange();
    let t, r;
    this.options.rtl ? (t = this.body.util.toScreen(e.start), r = this.body.util.toScreen(e.end)) : (r = this.body.util.toScreen(e.start), t = this.body.util.toScreen(e.end));
    const s = [];
    for (const a in this.groups) {
      if (!Object.prototype.hasOwnProperty.call(this.groups, a)) continue;
      const o = this.groups[a], l = o.isVisible ? o.visibleItems : [];
      for (const c of l)
        this.options.rtl ? c.right < r && c.right + c.width > t && s.push(c.id) : c.left < t && c.left + c.width > r && s.push(c.id);
    }
    return s;
  }
  /**
   * Get the id's of the items at specific time, where a click takes place on the timeline.
   * @param {Date} timeOfEvent The point in time to query items.
   * @returns {Array} The ids of all items in existence at the time of click event on the timeline.
   */
  getItemsAtCurrentTime(e) {
    let t, r;
    this.options.rtl ? (t = this.body.util.toScreen(e), r = this.body.util.toScreen(e)) : (r = this.body.util.toScreen(e), t = this.body.util.toScreen(e));
    const s = [];
    for (const a in this.groups) {
      if (!Object.prototype.hasOwnProperty.call(this.groups, a)) continue;
      const o = this.groups[a], l = o.isVisible ? o.visibleItems : [];
      for (const c of l)
        this.options.rtl ? c.right < r && c.right + c.width > t && s.push(c.id) : c.left < t && c.left + c.width > r && s.push(c.id);
    }
    return s;
  }
  /**
   * Get the id's of the currently visible groups.
   * @returns {Array} The ids of the visible groups
   */
  getVisibleGroups() {
    const e = [];
    for (const t in this.groups) {
      if (!Object.prototype.hasOwnProperty.call(this.groups, t)) continue;
      this.groups[t].isVisible && e.push(t);
    }
    return e;
  }
  /**
   * get item by id
   * @param {string} id
   * @return {object} item
   */
  getItemById(e) {
    var t;
    return this.items[e] || AH(t = this.clusters).call(t, (r) => r.id === e);
  }
  /**
   * Deselect a selected item
   * @param {string | number} id
   * @private
   */
  _deselect(e) {
    const t = this.selection;
    for (let r = 0, s = t.length; r < s; r++)
      if (t[r] == e) {
        Mt(t).call(t, r, 1);
        break;
      }
  }
  /**
   * Repaint the component
   * @return {boolean} Returns true if the component is resized
   */
  redraw() {
    const e = this.options.margin, t = this.body.range, r = E.option.asSize, s = this.options, a = s.orientation.item;
    let o = !1;
    const l = this.dom.frame;
    this.props.top = this.body.domProps.top.height + this.body.domProps.border.top, this.options.rtl ? this.props.right = this.body.domProps.right.width + this.body.domProps.border.right : this.props.left = this.body.domProps.left.width + this.body.domProps.border.left, l.className = "vis-itemset", this.options.cluster && this._clusterItems(), o = this._orderGroups() || o;
    const c = t.end - t.start, d = c != this.lastVisibleInterval || this.props.width != this.props.lastWidth, f = t.start != this.lastRangeStart, p = s.stack != this.lastStack, m = s.stackSubgroups != this.lastStackSubgroups, g = d || f || p || m;
    this.lastVisibleInterval = c, this.lastRangeStart = t.start, this.lastStack = s.stack, this.lastStackSubgroups = s.stackSubgroups, this.props.lastWidth = this.props.width;
    const b = this._firstGroup(), _ = {
      item: e.item,
      axis: e.axis
    }, x = {
      item: e.item,
      axis: e.item.vertical / 2
    };
    let T = 0;
    const S = e.axis + e.item.vertical;
    this.groups[Zr].redraw(t, x, g);
    const k = {};
    let D = 0;
    if (B(E).call(E, this.groups, (C, P) => {
      if (P === Zr) return;
      const R = C == b ? _ : x, M = !0;
      k[P] = C.redraw(t, R, g, M), D = k[P].length;
    }), D > 0) {
      const C = {};
      for (let P = 0; P < D; P++)
        B(E).call(E, k, (R, M) => {
          C[M] = R[P]();
        });
      B(E).call(E, this.groups, (P, R) => {
        if (R === Zr) return;
        o = C[R] || o, T += P.height;
      }), T = Math.max(T, S);
    }
    return T = Math.max(T, S), l.style.height = r(T), this.props.width = l.offsetWidth, this.props.height = T, this.dom.axis.style.top = r(a == "top" ? this.body.domProps.top.height + this.body.domProps.border.top : this.body.domProps.top.height + this.body.domProps.centerContainer.height), this.options.rtl ? this.dom.axis.style.right = "0" : this.dom.axis.style.left = "0", this.hammer.get("press").set({
      time: this.options.longSelectPressTime
    }), this.initialItemSetDrawn = !0, o = this._isResized() || o, o;
  }
  /**
   * Get the first group, aligned with the axis
   * @return {Group | null} firstGroup
   * @private
   */
  _firstGroup() {
    const e = this.options.orientation.item == "top" ? 0 : this.groupIds.length - 1, t = this.groupIds[e];
    return this.groups[t] || this.groups[Wi] || null;
  }
  /**
   * Create or delete the group holding all ungrouped items. This group is used when
   * there are no groups specified.
   * @protected
   */
  _updateUngrouped() {
    let e = this.groups[Wi], t, r;
    if (this.groupsData) {
      if (e) {
        e.dispose(), delete this.groups[Wi];
        for (r in this.items) {
          if (!Object.prototype.hasOwnProperty.call(this.items, r)) continue;
          t = this.items[r], t.parent && t.parent.remove(t);
          const s = this.getGroupId(t.data), a = this.groups[s];
          a && a.add(t) || t.hide();
        }
      }
    } else if (!e) {
      e = new ma(null, null, this), this.groups[Wi] = e;
      for (r in this.items)
        Object.prototype.hasOwnProperty.call(this.items, r) && (t = this.items[r], e.add(t));
      e.show();
    }
  }
  /**
   * Get the element for the labelset
   * @return {HTMLElement} labelSet
   */
  getLabelSet() {
    return this.dom.labelSet;
  }
  /**
   * Set items
   * @param {vis.DataSet | null} items
   */
  setItems(e) {
    this.itemsSettingTime = /* @__PURE__ */ new Date();
    const t = this;
    let r;
    const s = this.itemsData;
    if (!e)
      this.itemsData = null;
    else if (Hr(e))
      this.itemsData = cn(e);
    else
      throw new TypeError("Data must implement the interface of DataSet or DataView");
    if (s && (B(E).call(E, this.itemListeners, (a, o) => {
      s.off(o, a);
    }), s.dispose(), r = s.getIds(), this._onRemove(r)), this.itemsData) {
      const a = this.id;
      B(E).call(E, this.itemListeners, (o, l) => {
        t.itemsData.on(l, o, a);
      }), r = this.itemsData.getIds(), this._onAdd(r), this._updateUngrouped();
    }
    this.body.emitter.emit("_change", {
      queue: !0
    });
  }
  /**
   * Get the current items
   * @returns {vis.DataSet | null}
   */
  getItems() {
    return this.itemsData != null ? this.itemsData.rawDS : null;
  }
  /**
   * Set groups
   * @param {vis.DataSet} groups
   */
  setGroups(e) {
    const t = this;
    let r;
    if (this.groupsData && (B(E).call(E, this.groupListeners, (a, o) => {
      t.groupsData.off(o, a);
    }), r = this.groupsData.getIds(), this.groupsData = null, this._onRemoveGroups(r)), !e)
      this.groupsData = null;
    else if (Hr(e))
      this.groupsData = e;
    else
      throw new TypeError("Data must implement the interface of DataSet or DataView");
    if (this.groupsData) {
      var s;
      const a = this.groupsData.getDataSet();
      B(s = a.get()).call(s, (l) => {
        if (l.nestedGroups) {
          var c;
          B(c = l.nestedGroups).call(c, (d) => {
            const f = a.get(d);
            f.nestedInGroup = l.id, l.showNested == !1 && (f.visible = !1), a.update(f);
          });
        }
      });
      const o = this.id;
      B(E).call(E, this.groupListeners, (l, c) => {
        t.groupsData.on(c, l, o);
      }), r = this.groupsData.getIds(), this._onAddGroups(r);
    }
    this._updateUngrouped(), this._order(), this.options.cluster && (this.clusterGenerator.updateData(), this._clusterItems(), this.markDirty({
      refreshItems: !0,
      restackGroups: !0
    })), this.body.emitter.emit("_change", {
      queue: !0
    });
  }
  /**
   * Get the current groups
   * @returns {vis.DataSet | null} groups
   */
  getGroups() {
    return this.groupsData;
  }
  /**
   * Remove an item by its id
   * @param {string | number} id
   */
  removeItem(e) {
    const t = this.itemsData.get(e);
    t && this.options.onRemove(t, (r) => {
      r && this.itemsData.remove(e);
    });
  }
  /**
   * Get the time of an item based on it's data and options.type
   * @param {Object} itemData
   * @returns {string} Returns the type
   * @private
   */
  _getType(e) {
    return e.type || this.options.type || (e.end ? "range" : "box");
  }
  /**
   * Get the group id for an item
   * @param {Object} itemData
   * @returns {string} Returns the groupId
   * @private
   */
  getGroupId(e) {
    return this._getType(e) == "background" && e.group == null ? Zr : this.groupsData ? e.group : Wi;
  }
  /**
   * Handle updated items
   * @param {number[]} ids
   * @protected
   */
  _onUpdate(e) {
    const t = this;
    B(e).call(e, (r) => {
      const s = t.itemsData.get(r);
      let a = t.items[r];
      const o = s ? t._getType(s) : null, l = ii.types[o];
      let c;
      if (a && (!l || !(a instanceof l) ? (c = a.selected, t._removeItem(a), a = null) : t._updateItem(a, s)), !a && s)
        if (l)
          a = new l(s, t.conversion, t.options), a.id = r, t._addItem(a), c && (this.selection.push(r), a.select());
        else
          throw new TypeError('Unknown item type "'.concat(o, '"'));
    }), this._order(), this.options.cluster && (this.clusterGenerator.setItems(this.items, {
      applyOnChangedLevel: !1
    }), this._clusterItems()), this.body.emitter.emit("_change", {
      queue: !0
    });
  }
  /**
   * Handle removed items
   * @param {number[]} ids
   * @protected
   */
  _onRemove(e) {
    let t = 0;
    const r = this;
    B(e).call(e, (s) => {
      const a = r.items[s];
      a && (t++, r._removeItem(a));
    }), t && (this._order(), this.body.emitter.emit("_change", {
      queue: !0
    }));
  }
  /**
   * Update the order of item in all groups
   * @private
   */
  _order() {
    B(E).call(E, this.groups, (e) => {
      e.order();
    });
  }
  /**
   * Handle updated groups
   * @param {number[]} ids
   * @private
   */
  _onUpdateGroups(e) {
    this._onAddGroups(e);
  }
  /**
   * Handle changed groups (added or updated)
   * @param {number[]} ids
   * @private
   */
  _onAddGroups(e) {
    const t = this;
    B(e).call(e, (r) => {
      const s = t.groupsData.get(r);
      let a = t.groups[r];
      if (a)
        a.setData(s);
      else {
        if (r == Wi || r == Zr)
          throw new Error("Illegal group id. ".concat(r, " is a reserved id."));
        const o = _s(t.options);
        E.extend(o, {
          height: null
        }), a = new ma(r, s, t), t.groups[r] = a;
        for (const l in t.items) {
          if (!Object.prototype.hasOwnProperty.call(t.items, l)) continue;
          const c = t.items[l];
          c.data.group == r && a.add(c);
        }
        a.order(), a.show();
      }
    }), this.body.emitter.emit("_change", {
      queue: !0
    });
  }
  /**
   * Handle removed groups
   * @param {number[]} ids
   * @private
   */
  _onRemoveGroups(e) {
    B(e).call(e, (t) => {
      const r = this.groups[t];
      r && (r.dispose(), delete this.groups[t]);
    }), this.options.cluster && (this.clusterGenerator.updateData(), this._clusterItems()), this.markDirty({
      restackGroups: !!this.options.cluster
    }), this.body.emitter.emit("_change", {
      queue: !0
    });
  }
  /**
   * Reorder the groups if needed
   * @return {boolean} changed
   * @private
   */
  _orderGroups() {
    if (this.groupsData) {
      let e = this.groupsData.getIds({
        order: this.options.groupOrder
      });
      e = this._orderNestedGroups(e);
      const t = !E.equalArray(e, this.groupIds);
      if (t) {
        const r = this.groups;
        B(e).call(e, (s) => {
          r[s].hide();
        }), B(e).call(e, (s) => {
          r[s].show();
        }), this.groupIds = e;
      }
      return t;
    } else
      return !1;
  }
  /**
   * Reorder the nested groups
   *
   * @param {Array.<number>} groupIds
   * @returns {Array.<number>}
   * @private
   */
  _orderNestedGroups(e) {
    function t(s, a) {
      let o = [];
      return B(a).call(a, (l) => {
        if (o.push(l), s.groupsData.get(l).nestedGroups) {
          var d;
          const f = Ae(d = s.groupsData.get({
            filter(p) {
              return p.nestedInGroup == l;
            },
            order: s.options.groupOrder
          })).call(d, (p) => p.id);
          o = rt(o).call(o, t(s, f));
        }
      }), o;
    }
    const r = ie(e).call(e, (s) => !this.groupsData.get(s).nestedInGroup);
    return t(this, r);
  }
  /**
   * Add a new item
   * @param {Item} item
   * @private
   */
  _addItem(e) {
    this.items[e.id] = e;
    const t = this.getGroupId(e.data), r = this.groups[t];
    r ? r && r.data && r.data.showNested && (e.groupShowing = !0) : e.groupShowing = !1, r && r.add(e);
  }
  /**
   * Update an existing item
   * @param {Item} item
   * @param {Object} itemData
   * @private
   */
  _updateItem(e, t) {
    e.setData(t);
    const r = this.getGroupId(e.data), s = this.groups[r];
    s ? s && s.data && s.data.showNested && (e.groupShowing = !0) : e.groupShowing = !1;
  }
  /**
   * Delete an item from the ItemSet: remove it from the DOM, from the map
   * with items, and from the map with visible items, and from the selection
   * @param {Item} item
   * @private
   */
  _removeItem(e) {
    var t, r;
    e.hide(), delete this.items[e.id];
    const s = Fe(t = this.selection).call(t, e.id);
    s != -1 && Mt(r = this.selection).call(r, s, 1), e.parent && e.parent.remove(e), this.popup != null && this.popup.hide();
  }
  /**
   * Create an array containing all items being a range (having an end date)
   * @param {Array.<Object>} array
   * @returns {Array}
   * @private
   */
  _constructByEndArray(e) {
    const t = [];
    for (let r = 0; r < e.length; r++)
      e[r] instanceof ti && t.push(e[r]);
    return t;
  }
  /**
   * Register the clicked item on touch, before dragStart is initiated.
   *
   * dragStart is initiated from a mousemove event, AFTER the mouse/touch is
   * already moving. Therefore, the mouse/touch can sometimes be above an other
   * DOM element than the item itself.
   *
   * @param {Event} event
   * @private
   */
  _onTouch(e) {
    this.touchParams.item = this.itemFromTarget(e), this.touchParams.dragLeftItem = e.target.dragLeftItem || !1, this.touchParams.dragRightItem = e.target.dragRightItem || !1, this.touchParams.itemProps = null;
  }
  /**
   * Given an group id, returns the index it has.
   *
   * @param {number} groupId
   * @returns {number} index / groupId
   * @private
   */
  _getGroupIndex(e) {
    for (let t = 0; t < this.groupIds.length; t++)
      if (e == this.groupIds[t]) return t;
  }
  /**
   * Start dragging the selected events
   * @param {Event} event
   * @private
   */
  _onDragStart(e) {
    if (this.touchParams.itemIsDragging)
      return;
    const t = this.touchParams.item || null, r = this;
    let s;
    if (t && (t.selected || this.options.itemsAlwaysDraggable.item)) {
      if (this.options.editable.overrideItems && !this.options.editable.updateTime && !this.options.editable.updateGroup || t.editable != null && !t.editable.updateTime && !t.editable.updateGroup && !this.options.editable.overrideItems)
        return;
      const a = this.touchParams.dragLeftItem, o = this.touchParams.dragRightItem;
      if (this.touchParams.itemIsDragging = !0, this.touchParams.selectedItem = t, a)
        s = {
          item: a,
          initialX: e.center.x,
          dragLeft: !0,
          data: this._cloneItemData(t.data)
        }, this.touchParams.itemProps = [s];
      else if (o)
        s = {
          item: o,
          initialX: e.center.x,
          dragRight: !0,
          data: this._cloneItemData(t.data)
        }, this.touchParams.itemProps = [s];
      else if (this.options.editable.add && (e.srcEvent.ctrlKey || e.srcEvent.metaKey))
        this._onDragStartAddItem(e);
      else {
        this.groupIds.length < 1 && this.redraw();
        const l = this._getGroupIndex(t.data.group), c = this.options.itemsAlwaysDraggable.item && !t.selected ? [t.id] : this.getSelection();
        this.touchParams.itemProps = Ae(c).call(c, (d) => {
          const f = r.items[d], p = r._getGroupIndex(f.data.group);
          return {
            item: f,
            initialX: e.center.x,
            groupOffset: l - p,
            data: this._cloneItemData(f.data)
          };
        });
      }
      e.stopPropagation();
    } else this.options.editable.add && (e.srcEvent.ctrlKey || e.srcEvent.metaKey) && this._onDragStartAddItem(e);
  }
  /**
   * Start creating a new range item by dragging.
   * @param {Event} event
   * @private
   */
  _onDragStartAddItem(e) {
    const t = this.options.snap || null, r = this.dom.frame.getBoundingClientRect(), s = this.options.rtl ? r.right - e.center.x + 10 : e.center.x - r.left - 10, a = this.body.util.toTime(s), o = this.body.util.getScale(), l = this.body.util.getStep(), c = t ? t(a, o, l) : a, f = {
      type: "range",
      start: c,
      end: c,
      content: "new item"
    }, p = wi();
    f[this.itemsData.idProp] = p;
    const m = this.groupFromTarget(e);
    m && (f.group = m.groupId);
    const g = new ti(f, this.conversion, this.options);
    g.id = p, g.data = this._cloneItemData(f), this._addItem(g), this.touchParams.selectedItem = g;
    const b = {
      item: g,
      initialX: e.center.x,
      data: g.data
    };
    this.options.rtl ? b.dragLeft = !0 : b.dragRight = !0, this.touchParams.itemProps = [b], e.stopPropagation();
  }
  /**
   * Drag selected items
   * @param {Event} event
   * @private
   */
  _onDrag(e) {
    if (this.popup != null && this.options.showTooltips && !this.popup.hidden) {
      const r = this.body.dom.centerContainer, s = r.getBoundingClientRect();
      this.popup.setPosition(e.center.x - s.left + r.offsetLeft, e.center.y - s.top + r.offsetTop), this.popup.show();
    }
    if (this.touchParams.itemProps) {
      var t;
      e.stopPropagation();
      const r = this, s = this.options.snap || null, a = this.body.dom.root.offsetLeft, o = this.options.rtl ? a + this.body.domProps.right.width : a + this.body.domProps.left.width, l = this.body.util.getScale(), c = this.body.util.getStep(), d = this.touchParams.selectedItem, f = (this.options.editable.overrideItems || d.editable == null) && this.options.editable.updateGroup || !this.options.editable.overrideItems && d.editable != null && d.editable.updateGroup;
      let p = null;
      if (f && d && d.data.group != null) {
        const m = r.groupFromTarget(e);
        m && (p = this._getGroupIndex(m.groupId));
      }
      B(t = this.touchParams.itemProps).call(t, (m) => {
        const g = r.body.util.toTime(e.center.x - o), b = r.body.util.toTime(m.initialX - o);
        let _, x, T, S, k;
        this.options.rtl ? _ = -(g - b) : _ = g - b;
        let D = this._cloneItemData(m.item.data);
        if (m.item.editable != null && !m.item.editable.updateTime && !m.item.editable.updateGroup && !r.options.editable.overrideItems)
          return;
        if ((this.options.editable.overrideItems || d.editable == null) && this.options.editable.updateTime || !this.options.editable.overrideItems && d.editable != null && d.editable.updateTime) {
          if (m.dragLeft)
            this.options.rtl ? D.end != null && (T = E.convert(m.data.end, "Date"), k = new Date(T.valueOf() + _), D.end = s ? s(k, l, c) : k) : D.start != null && (x = E.convert(m.data.start, "Date"), S = new Date(x.valueOf() + _), D.start = s ? s(S, l, c) : S);
          else if (m.dragRight)
            this.options.rtl ? D.start != null && (x = E.convert(m.data.start, "Date"), S = new Date(x.valueOf() + _), D.start = s ? s(S, l, c) : S) : D.end != null && (T = E.convert(m.data.end, "Date"), k = new Date(T.valueOf() + _), D.end = s ? s(k, l, c) : k);
          else if (D.start != null)
            if (x = E.convert(m.data.start, "Date").valueOf(), S = new Date(x + _), D.end != null) {
              T = E.convert(m.data.end, "Date");
              const C = T.valueOf() - x.valueOf();
              D.start = s ? s(S, l, c) : S, D.end = new Date(D.start.valueOf() + C);
            } else
              D.start = s ? s(S, l, c) : S;
        }
        if (f && !m.dragLeft && !m.dragRight && p != null && D.group != null) {
          let C = p - m.groupOffset;
          C = Math.max(0, C), C = Math.min(r.groupIds.length - 1, C), D.group = r.groupIds[C];
        }
        D = this._cloneItemData(D), r.options.onMoving(D, (C) => {
          C && m.item.setData(this._cloneItemData(C, "Date"));
        });
      }), this.body.emitter.emit("_change");
    }
  }
  /**
   * Move an item to another group
   * @param {Item} item
   * @param {string | number} groupId
   * @private
   */
  _moveToGroup(e, t) {
    const r = this.groups[t];
    if (r && r.groupId != e.data.group) {
      const s = e.parent;
      s.remove(e), s.order(), e.data.group = r.groupId, r.add(e), r.order();
    }
  }
  /**
   * End of dragging selected items
   * @param {Event} event
   * @private
   */
  _onDragEnd(e) {
    if (this.touchParams.itemIsDragging = !1, this.touchParams.itemProps) {
      e.stopPropagation();
      const t = this, r = this.touchParams.itemProps;
      this.touchParams.itemProps = null, B(r).call(r, (s) => {
        const a = s.item.id;
        if (!(t.itemsData.get(a) != null))
          t.options.onAdd(s.item.data, (l) => {
            t._removeItem(s.item), l && t.itemsData.add(l), t.body.emitter.emit("_change");
          });
        else {
          const l = this._cloneItemData(s.item.data);
          t.options.onMove(l, (c) => {
            c ? (c[this.itemsData.idProp] = a, this.itemsData.update(c)) : (s.item.setData(s.data), t.body.emitter.emit("_change"));
          });
        }
      });
    }
  }
  /**
   * On group click
   * @param {Event} event
   * @private
   */
  _onGroupClick(e) {
    const t = this.groupFromTarget(e);
    Dt(() => {
      this.toggleGroupShowNested(t);
    }, 1);
  }
  /**
   * Toggle show nested
   * @param {object} group
   * @param {boolean} force
   */
  toggleGroupShowNested(e) {
    let t = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : void 0;
    if (!e || !e.nestedGroups) return;
    const r = this.groupsData.getDataSet();
    t != null ? e.showNested = !!t : e.showNested = !e.showNested;
    let s = r.get(e.groupId);
    s.showNested = e.showNested;
    let a = e.nestedGroups, o = a;
    for (; o.length > 0; ) {
      let f = o;
      o = [];
      for (let p = 0; p < f.length; p++) {
        let m = r.get(f[p]);
        m.nestedGroups && (o = rt(o).call(o, m.nestedGroups));
      }
      o.length > 0 && (a = rt(a).call(a, o));
    }
    var l;
    if (s.showNested) {
      var c = r.get(s.nestedGroups);
      for (let f = 0; f < c.length; f++) {
        let p = c[f];
        p.nestedGroups && p.nestedGroups.length > 0 && (p.showNested == null || p.showNested == !0) && c.push(...r.get(p.nestedGroups));
      }
      l = Ae(c).call(c, function(f) {
        return f.visible == null && (f.visible = !0), f.visible = !!s.showNested, f;
      });
    } else {
      var d;
      l = Ae(d = r.get(a)).call(d, function(f) {
        return f.visible == null && (f.visible = !0), f.visible = !!s.showNested, f;
      });
    }
    r.update(rt(l).call(l, s)), s.showNested ? (E.removeClassName(e.dom.label, "collapsed"), E.addClassName(e.dom.label, "expanded")) : (E.removeClassName(e.dom.label, "expanded"), E.addClassName(e.dom.label, "collapsed"));
  }
  /**
   * Toggle group drag classname
   * @param {object} group
   */
  toggleGroupDragClassName(e) {
    e.dom.label.classList.toggle("vis-group-is-dragging"), e.dom.foreground.classList.toggle("vis-group-is-dragging");
  }
  /**
   * on drag start
   * @param {Event} event
   * @return {void}
   * @private
   */
  _onGroupDragStart(e) {
    this.groupTouchParams.isDragging || this.options.groupEditable.order && (this.groupTouchParams.group = this.groupFromTarget(e), this.groupTouchParams.group && (e.stopPropagation(), this.groupTouchParams.isDragging = !0, this.toggleGroupDragClassName(this.groupTouchParams.group), this.groupTouchParams.originalOrder = this.groupsData.getIds({
      order: this.options.groupOrder
    })));
  }
  /**
   * on drag
   * @param {Event} event
   * @return {void}
   * @private
   */
  _onGroupDrag(e) {
    if (this.options.groupEditable.order && this.groupTouchParams.group) {
      e.stopPropagation();
      const t = this.groupsData.getDataSet(), r = this.groupFromTarget(e);
      if (r && r.height != this.groupTouchParams.group.height) {
        const s = r.top < this.groupTouchParams.group.top, a = e.center ? e.center.y : e.clientY, o = r.dom.foreground.getBoundingClientRect(), l = this.groupTouchParams.group.height;
        if (s) {
          if (o.top + l < a)
            return;
        } else {
          const c = r.height;
          if (o.top + c - l > a)
            return;
        }
      }
      if (r && r != this.groupTouchParams.group) {
        const s = t.get(r.groupId), a = t.get(this.groupTouchParams.group.groupId);
        a && s && (this.options.groupOrderSwap(a, s, t), t.update(a), t.update(s));
        const o = t.getIds({
          order: this.options.groupOrder
        });
        if (!E.equalArray(o, this.groupTouchParams.originalOrder)) {
          const l = this.groupTouchParams.originalOrder, c = this.groupTouchParams.group.groupId, d = Math.min(l.length, o.length);
          let f = 0, p = 0, m = 0;
          for (; f < d; ) {
            for (; f + p < d && f + m < d && o[f + p] == l[f + m]; )
              f++;
            if (f + p >= d)
              break;
            if (o[f + p] == c)
              p = 1;
            else if (l[f + m] == c)
              m = 1;
            else {
              const g = Fe(o).call(o, l[f + m]), b = t.get(o[f + p]), _ = t.get(l[f + m]);
              this.options.groupOrderSwap(b, _, t), t.update(b), t.update(_);
              const x = o[f + p];
              o[f + p] = l[f + m], o[g] = x, f++;
            }
          }
        }
      }
    }
  }
  /**
   * on drag end
   * @param {Event} event
   * @return {void}
   * @private
   */
  _onGroupDragEnd(e) {
    if (this.groupTouchParams.isDragging = !1, this.options.groupEditable.order && this.groupTouchParams.group) {
      e.stopPropagation();
      const t = this, r = t.groupTouchParams.group.groupId, s = t.groupsData.getDataSet(), a = E.extend({}, s.get(r));
      t.options.onMoveGroup(a, (o) => {
        if (o)
          o[s._idProp] = r, s.update(o);
        else {
          const l = s.getIds({
            order: t.options.groupOrder
          });
          if (!E.equalArray(l, t.groupTouchParams.originalOrder)) {
            const c = t.groupTouchParams.originalOrder, d = Math.min(c.length, l.length);
            let f = 0;
            for (; f < d; ) {
              for (; f < d && l[f] == c[f]; )
                f++;
              if (f >= d)
                break;
              const p = Fe(l).call(l, c[f]), m = s.get(l[f]), g = s.get(c[f]);
              t.options.groupOrderSwap(m, g, s), s.update(m), s.update(g);
              const b = l[f];
              l[f] = c[f], l[p] = b, f++;
            }
          }
        }
      }), t.body.emitter.emit("groupDragged", {
        groupId: r
      }), this.toggleGroupDragClassName(this.groupTouchParams.group), this.groupTouchParams.group = null;
    }
  }
  /**
   * Handle selecting/deselecting an item when tapping it
   * @param {Event} event
   * @private
   */
  _onSelectItem(e) {
    if (!this.options.selectable) return;
    const t = e.srcEvent && (e.srcEvent.ctrlKey || e.srcEvent.metaKey), r = e.srcEvent && e.srcEvent.shiftKey;
    if (t || r) {
      this._onMultiSelectItem(e);
      return;
    }
    const s = this.getSelection(), a = this.itemFromTarget(e), o = a && a.selectable ? [a.id] : [];
    this.setSelection(o);
    const l = this.getSelection();
    (l.length > 0 || s.length > 0) && this.body.emitter.emit("select", {
      items: l,
      event: e
    });
  }
  /**
   * Handle hovering an item
   * @param {Event} event
   * @private
   */
  _onMouseOver(e) {
    const t = this.itemFromTarget(e);
    if (!t) return;
    const r = this.itemFromRelatedTarget(e);
    if (t === r)
      return;
    const s = t.getTitle();
    if (this.options.showTooltips && s) {
      this.popup == null && (this.popup = new NH(this.body.dom.root, this.options.tooltip.overflowMethod || "flip")), this.popup.setText(s);
      const a = this.body.dom.centerContainer, o = a.getBoundingClientRect();
      this.popup.setPosition(e.clientX - o.left + a.offsetLeft, e.clientY - o.top + a.offsetTop), this.setPopupTimer(this.popup);
    } else
      this.clearPopupTimer(), this.popup != null && this.popup.hide();
    this.body.emitter.emit("itemover", {
      item: t.id,
      event: e
    });
  }
  /**
   * on mouse start
   * @param {Event} event
   * @return {void}
   * @private
   */
  _onMouseOut(e) {
    const t = this.itemFromTarget(e);
    if (!t) return;
    const r = this.itemFromRelatedTarget(e);
    t !== r && (this.clearPopupTimer(), this.popup != null && this.popup.hide(), this.body.emitter.emit("itemout", {
      item: t.id,
      event: e
    }));
  }
  /**
   * on mouse move
   * @param {Event} event
   * @return {void}
   * @private
   */
  _onMouseMove(e) {
    if (this.itemFromTarget(e) && (this.popupTimer != null && this.setPopupTimer(this.popup), this.options.showTooltips && this.options.tooltip.followMouse && this.popup && !this.popup.hidden)) {
      const r = this.body.dom.centerContainer, s = r.getBoundingClientRect();
      this.popup.setPosition(e.clientX - s.left + r.offsetLeft, e.clientY - s.top + r.offsetTop), this.popup.show();
    }
  }
  /**
   * Handle mousewheel
   * @param {Event}  event   The event
   * @private
   */
  _onMouseWheel(e) {
    this.touchParams.itemIsDragging && this._onDragEnd(e);
  }
  /**
   * Handle updates of an item on double tap
   * @param {timeline.Item}  item   The item
   * @private
   */
  _onUpdateItem(e) {
    if (!this.options.selectable || !this.options.editable.updateTime && !this.options.editable.updateGroup) return;
    const t = this;
    if (e) {
      const r = t.itemsData.get(e.id);
      this.options.onUpdate(r, (s) => {
        s && t.itemsData.update(s);
      });
    }
  }
  /**
   * Handle drop event of data on item
   * Only called when `objectData.target === 'item'.
   * @param {Event} event The event
   * @private
   */
  _onDropObjectOnItem(e) {
    const t = this.itemFromTarget(e), r = JSON.parse(e.dataTransfer.getData("text"));
    this.options.onDropObjectOnItem(r, t);
  }
  /**
   * Handle creation of an item on double tap or drop of a drag event
   * @param {Event} event   The event
   * @private
   */
  _onAddItem(e) {
    if (!this.options.selectable || !this.options.editable.add) return;
    const t = this, r = this.options.snap || null, s = this.dom.frame.getBoundingClientRect(), a = this.options.rtl ? s.right - e.center.x : e.center.x - s.left, o = this.body.util.toTime(a), l = this.body.util.getScale(), c = this.body.util.getStep();
    let d, f;
    e.type == "drop" ? (f = JSON.parse(e.dataTransfer.getData("text")), f.content = f.content ? f.content : "new item", f.start = f.start ? f.start : r ? r(o, l, c) : o, f.type = f.type || "box", f[this.itemsData.idProp] = f.id || wi(), f.type == "range" && !f.end && (d = this.body.util.toTime(a + this.props.width / 5), f.end = r ? r(d, l, c) : d)) : (f = {
      start: r ? r(o, l, c) : o,
      content: "new item"
    }, f[this.itemsData.idProp] = wi(), this.options.type === "range" && (d = this.body.util.toTime(a + this.props.width / 5), f.end = r ? r(d, l, c) : d));
    const p = this.groupFromTarget(e);
    p && (f.group = p.groupId), f = this._cloneItemData(f), this.options.onAdd(f, (m) => {
      m && (t.itemsData.add(m), e.type == "drop" && t.setSelection([m.id]));
    });
  }
  /**
   * Handle selecting/deselecting multiple items when holding an item
   * @param {Event} event
   * @private
   */
  _onMultiSelectItem(e) {
    if (!this.options.selectable) return;
    const t = this.itemFromTarget(e);
    if (t) {
      let r = this.options.multiselect ? this.getSelection() : [];
      if ((e.srcEvent && e.srcEvent.shiftKey || !1 || this.options.sequentialSelection) && this.options.multiselect) {
        const o = this.itemsData.get(t.id).group;
        let l;
        this.options.multiselectPerGroup && r.length > 0 && (l = this.itemsData.get(r[0]).group), (!this.options.multiselectPerGroup || l == null || l == o) && r.push(t.id);
        const c = ii._getItemRange(this.itemsData.get(r));
        if (!this.options.multiselectPerGroup || l == o) {
          r = [];
          for (const d in this.items) {
            if (!Object.prototype.hasOwnProperty.call(this.items, d)) continue;
            const f = this.items[d], p = f.data.start, m = f.data.end !== void 0 ? f.data.end : p;
            p >= c.min && m <= c.max && (!this.options.multiselectPerGroup || l == this.itemsData.get(f.id).group) && !(f instanceof li) && r.push(f.id);
          }
        }
      } else {
        const o = Fe(r).call(r, t.id);
        o == -1 ? r.push(t.id) : Mt(r).call(r, o, 1);
      }
      const a = ie(r).call(r, (o) => this.getItemById(o).selectable);
      this.setSelection(a), this.body.emitter.emit("select", {
        items: this.getSelection(),
        event: e
      });
    }
  }
  /**
   * Calculate the time range of a list of items
   * @param {Array.<Object>} itemsData
   * @return {{min: Date, max: Date}} Returns the range of the provided items
   * @private
   */
  static _getItemRange(e) {
    let t = null, r = null;
    return B(e).call(e, (s) => {
      (r == null || s.start < r) && (r = s.start), s.end != null ? (t == null || s.end > t) && (t = s.end) : (t == null || s.start > t) && (t = s.start);
    }), {
      min: r,
      max: t
    };
  }
  /**
   * Find an item from an element:
   * searches for the attribute 'vis-item' in the element's tree
   * @param {HTMLElement} element
   * @return {Item | null} item
   */
  itemFromElement(e) {
    let t = e;
    for (; t; ) {
      if (Object.prototype.hasOwnProperty.call(t, "vis-item"))
        return t["vis-item"];
      t = t.parentNode;
    }
    return null;
  }
  /**
   * Find an item from an event target:
   * searches for the attribute 'vis-item' in the event target's element tree
   * @param {Event} event
   * @return {Item | null} item
   */
  itemFromTarget(e) {
    return this.itemFromElement(e.target);
  }
  /**
   * Find an item from an event's related target:
   * searches for the attribute 'vis-item' in the related target's element tree
   * @param {Event} event
   * @return {Item | null} item
   */
  itemFromRelatedTarget(e) {
    return this.itemFromElement(e.relatedTarget);
  }
  /**
   * Find the Group from an event target:
   * searches for the attribute 'vis-group' in the event target's element tree
   * @param {Event} event
   * @return {Group | null} group
   */
  groupFromTarget(e) {
    const t = e.center ? e.center.y : e.clientY;
    let r = this.groupIds;
    r.length <= 0 && this.groupsData && (r = this.groupsData.getIds({
      order: this.options.groupOrder
    }));
    for (let s = 0; s < r.length; s++) {
      const a = r[s], o = this.groups[a], l = o.dom.foreground, c = l.getBoundingClientRect();
      if (t >= c.top && t < c.top + l.offsetHeight)
        return o;
      if (this.options.orientation.item === "top") {
        if (s === this.groupIds.length - 1 && t > c.top)
          return o;
      } else if (s === 0 && t < c.top + l.offset)
        return o;
    }
    return null;
  }
  /**
   * Find the ItemSet from an event target:
   * searches for the attribute 'vis-itemset' in the event target's element tree
   * @param {Event} event
   * @return {ItemSet | null} item
   */
  static itemSetFromTarget(e) {
    let t = e.target;
    for (; t; ) {
      if (Object.prototype.hasOwnProperty.call(t, "vis-itemset"))
        return t["vis-itemset"];
      t = t.parentNode;
    }
    return null;
  }
  /**
   * Clone the data of an item, and "normalize" it: convert the start and end date
   * to the type (Date, Moment, ...) configured in the DataSet. If not configured,
   * start and end are converted to Date.
   * @param {Object} itemData, typically `item.data`
   * @param {string} [type]  Optional Date type. If not provided, the type from the DataSet is taken
   * @return {Object} The cloned object
   * @private
   */
  _cloneItemData(e, t) {
    const r = E.extend({}, e);
    return t || (t = this.itemsData.type), r.start != null && (r.start = E.convert(r.start, t && t.start || "Date")), r.end != null && (r.end = E.convert(r.end, t && t.end || "Date")), r;
  }
  /**
   * cluster items
   * @return {void}
   * @private
   */
  _clusterItems() {
    if (!this.options.cluster)
      return;
    const e = this.body.range.conversion(this.body.domProps.center.width), t = e.scale, r = this.clusterGenerator.getClusters(this.clusters, t, this.options.cluster);
    if (this.clusters != r) {
      if (this._detachAllClusters(), r) {
        for (let s of r)
          s.attach();
        this.clusters = r;
      }
      this._updateClusters(r);
    }
  }
  /**
   * detach all cluster items
   * @private
   */
  _detachAllClusters() {
    if (this.options.cluster && this.clusters && this.clusters.length)
      for (let e of this.clusters)
        e.detach();
  }
  /**
   * update clusters
   * @param {array} clusters
   * @private
   */
  _updateClusters(e) {
    if (this.clusters && this.clusters.length) {
      var t;
      const a = new Jr(Ae(e).call(e, (c) => c.id)), o = ie(t = this.clusters).call(t, (c) => !a.has(c.id));
      let l = !1;
      for (let c of o) {
        var r;
        const d = Fe(r = this.selection).call(r, c.id);
        if (d !== -1) {
          var s;
          c.unselect(), Mt(s = this.selection).call(s, d, 1), l = !0;
        }
      }
      if (l) {
        const c = this.getSelection();
        this.body.emitter.emit("select", {
          items: c,
          event
        });
      }
    }
    this.clusters = e || [];
  }
}
ii.types = {
  background: li,
  box: ak,
  range: ti,
  point: ok
};
ii.prototype._onAdd = ii.prototype._onUpdate;
function Gt(n, e, t, r) {
  this.body = n, this.defaultOptions = {
    enabled: !1,
    icons: !0,
    iconSize: 20,
    iconSpacing: 6,
    left: {
      visible: !0,
      position: "top-left"
      // top/bottom - left,center,right
    },
    right: {
      visible: !0,
      position: "top-right"
      // top/bottom - left,center,right
    }
  }, this.side = t, this.options = E.extend({}, this.defaultOptions), this.linegraphOptions = r, this.svgElements = {}, this.dom = {}, this.groups = {}, this.amountOfGroups = 0, this._create(), this.framework = {
    svg: this.svg,
    svgElements: this.svgElements,
    options: this.options,
    groups: this.groups
  }, this.setOptions(e);
}
Gt.prototype = new zr();
Gt.prototype.clear = function() {
  this.groups = {}, this.amountOfGroups = 0;
};
Gt.prototype.addGroup = function(n, e) {
  e.options.excludeFromLegend != !0 && (Object.prototype.hasOwnProperty.call(this.groups, n) || (this.groups[n] = e), this.amountOfGroups += 1);
};
Gt.prototype.updateGroup = function(n, e) {
  this.groups[n] = e;
};
Gt.prototype.removeGroup = function(n) {
  Object.prototype.hasOwnProperty.call(this.groups, n) && (delete this.groups[n], this.amountOfGroups -= 1);
};
Gt.prototype._create = function() {
  this.dom.frame = document.createElement("div"), this.dom.frame.className = "vis-legend", this.dom.frame.style.position = "absolute", this.dom.frame.style.top = "10px", this.dom.frame.style.display = "block", this.dom.textArea = document.createElement("div"), this.dom.textArea.className = "vis-legend-text", this.dom.textArea.style.position = "relative", this.dom.textArea.style.top = "0px", this.svg = document.createElementNS("http://www.w3.org/2000/svg", "svg"), this.svg.style.position = "absolute", this.svg.style.top = "0px", this.svg.style.width = this.options.iconSize + 5 + "px", this.svg.style.height = "100%", this.dom.frame.appendChild(this.svg), this.dom.frame.appendChild(this.dom.textArea);
};
Gt.prototype.hide = function() {
  this.dom.frame.parentNode && this.dom.frame.parentNode.removeChild(this.dom.frame);
};
Gt.prototype.show = function() {
  this.dom.frame.parentNode || this.body.dom.center.appendChild(this.dom.frame);
};
Gt.prototype.setOptions = function(n) {
  var e = ["enabled", "orientation", "icons", "left", "right"];
  E.selectiveDeepExtend(e, this.options, n);
};
Gt.prototype.redraw = function() {
  var n = 0, e = ct(this.groups);
  gt(e).call(e, function(o, l) {
    return o < l ? -1 : 1;
  });
  for (var t = 0; t < e.length; t++) {
    var r = e[t];
    this.groups[r].visible == !0 && (this.linegraphOptions.visibility[r] === void 0 || this.linegraphOptions.visibility[r] == !0) && n++;
  }
  if (this.options[this.side].visible == !1 || this.amountOfGroups == 0 || this.options.enabled == !1 || n == 0)
    this.hide();
  else {
    if (this.show(), this.options[this.side].position == "top-left" || this.options[this.side].position == "bottom-left" ? (this.dom.frame.style.left = "4px", this.dom.frame.style.textAlign = "left", this.dom.textArea.style.textAlign = "left", this.dom.textArea.style.left = this.options.iconSize + 15 + "px", this.dom.textArea.style.right = "", this.svg.style.left = "0px", this.svg.style.right = "") : (this.dom.frame.style.right = "4px", this.dom.frame.style.textAlign = "right", this.dom.textArea.style.textAlign = "right", this.dom.textArea.style.right = this.options.iconSize + 15 + "px", this.dom.textArea.style.left = "", this.svg.style.right = "0px", this.svg.style.left = ""), this.options[this.side].position == "top-left" || this.options[this.side].position == "top-right")
      this.dom.frame.style.top = 4 - Number(this.body.dom.center.style.top.replace("px", "")) + "px", this.dom.frame.style.bottom = "";
    else {
      var s = this.body.domProps.center.height - this.body.domProps.centerContainer.height;
      this.dom.frame.style.bottom = 4 + s + Number(this.body.dom.center.style.top.replace("px", "")) + "px", this.dom.frame.style.top = "";
    }
    this.options.icons == !1 ? (this.dom.frame.style.width = this.dom.textArea.offsetWidth + 10 + "px", this.dom.textArea.style.right = "", this.dom.textArea.style.left = "", this.svg.style.width = "0px") : (this.dom.frame.style.width = this.options.iconSize + 15 + this.dom.textArea.offsetWidth + 10 + "px", this.drawLegendIcons());
    var a = "";
    for (t = 0; t < e.length; t++)
      r = e[t], this.groups[r].visible == !0 && (this.linegraphOptions.visibility[r] === void 0 || this.linegraphOptions.visibility[r] == !0) && (a += this.groups[r].content + "<br />");
    this.dom.textArea.innerHTML = E.xss(a), this.dom.textArea.style.lineHeight = 0.75 * this.options.iconSize + this.options.iconSpacing + "px";
  }
};
Gt.prototype.drawLegendIcons = function() {
  if (this.dom.frame.parentNode) {
    var n = ct(this.groups);
    gt(n).call(n, function(d, f) {
      return d < f ? -1 : 1;
    }), $O(this.svgElements);
    var e = window.getComputedStyle(this.dom.frame).paddingTop, t = Number(e.replace("px", "")), r = t, s = this.options.iconSize, a = 0.75 * this.options.iconSize, o = t + 0.5 * a + 3;
    this.svg.style.width = s + 5 + t + "px";
    for (var l = 0; l < n.length; l++) {
      var c = n[l];
      this.groups[c].visible == !0 && (this.linegraphOptions.visibility[c] === void 0 || this.linegraphOptions.visibility[c] == !0) && (this.groups[c].getLegend(s, a, this.framework, r, o), o += a + this.options.iconSpacing);
    }
  }
};
var ga = "__ungrouped__";
function Ne(n, e) {
  this.id = wi(), this.body = n, this.defaultOptions = {
    yAxisOrientation: "left",
    defaultGroup: "default",
    sort: !0,
    sampling: !0,
    stack: !1,
    graphHeight: "400px",
    shaded: {
      enabled: !1,
      orientation: "bottom"
      // top, bottom, zero
    },
    style: "line",
    // line, bar
    barChart: {
      width: 50,
      sideBySide: !1,
      align: "center"
      // left, center, right
    },
    interpolation: {
      enabled: !0,
      parametrization: "centripetal",
      // uniform (alpha = 0.0), chordal (alpha = 1.0), centripetal (alpha = 0.5)
      alpha: 0.5
    },
    drawPoints: {
      enabled: !0,
      size: 6,
      style: "square"
      // square, circle
    },
    dataAxis: {},
    //Defaults are done on DataAxis level
    legend: {},
    //Defaults are done on Legend level
    groups: {
      visibility: {}
    }
  }, this.options = E.extend({}, this.defaultOptions), this.dom = {}, this.props = {}, this.hammer = null, this.groups = {}, this.abortedGraphUpdate = !1, this.updateSVGheight = !1, this.updateSVGheightOnResize = !1, this.forceGraphUpdate = !0;
  var t = this;
  this.itemsData = null, this.groupsData = null, this.itemListeners = {
    add: function(r, s) {
      t._onAdd(s.items);
    },
    update: function(r, s) {
      t._onUpdate(s.items);
    },
    remove: function(r, s) {
      t._onRemove(s.items);
    }
  }, this.groupListeners = {
    add: function(r, s) {
      t._onAddGroups(s.items);
    },
    update: function(r, s) {
      t._onUpdateGroups(s.items);
    },
    remove: function(r, s) {
      t._onRemoveGroups(s.items);
    }
  }, this.items = {}, this.selection = [], this.lastStart = this.body.range.start, this.touchParams = {}, this.svgElements = {}, this.setOptions(e), this.groupsUsingDefaultStyles = [0], this.body.emitter.on("rangechanged", function() {
    t.svg.style.left = E.option.asSize(-t.props.width), t.forceGraphUpdate = !0, t.redraw.call(t);
  }), this._create(), this.framework = {
    svg: this.svg,
    svgElements: this.svgElements,
    options: this.options,
    groups: this.groups
  };
}
Ne.prototype = new zr();
Ne.prototype._create = function() {
  var n = document.createElement("div");
  n.className = "vis-line-graph", this.dom.frame = n, this.svg = document.createElementNS("http://www.w3.org/2000/svg", "svg"), this.svg.style.position = "relative", this.svg.style.height = ("" + this.options.graphHeight).replace("px", "") + "px", this.svg.style.display = "block", n.appendChild(this.svg), this.options.dataAxis.orientation = "left", this.yAxisLeft = new Wp(this.body, this.options.dataAxis, this.svg, this.options.groups), this.options.dataAxis.orientation = "right", this.yAxisRight = new Wp(this.body, this.options.dataAxis, this.svg, this.options.groups), delete this.options.dataAxis.orientation, this.legendLeft = new Gt(this.body, this.options.legend, "left", this.options.groups), this.legendRight = new Gt(this.body, this.options.legend, "right", this.options.groups), this.show();
};
Ne.prototype.setOptions = function(n) {
  if (n) {
    var e = ["sampling", "defaultGroup", "stack", "height", "graphHeight", "yAxisOrientation", "style", "barChart", "dataAxis", "sort", "groups"];
    n.graphHeight === void 0 && n.height !== void 0 ? (this.updateSVGheight = !0, this.updateSVGheightOnResize = !0) : this.body.domProps.centerContainer.height !== void 0 && n.graphHeight !== void 0 && Vt((n.graphHeight + "").replace("px", "")) < this.body.domProps.centerContainer.height && (this.updateSVGheight = !0), E.selectiveDeepExtend(e, this.options, n), E.mergeOptions(this.options, n, "interpolation"), E.mergeOptions(this.options, n, "drawPoints"), E.mergeOptions(this.options, n, "shaded"), E.mergeOptions(this.options, n, "legend"), n.interpolation && typeof n.interpolation == "object" && n.interpolation.parametrization && (n.interpolation.parametrization == "uniform" ? this.options.interpolation.alpha = 0 : n.interpolation.parametrization == "chordal" ? this.options.interpolation.alpha = 1 : (this.options.interpolation.parametrization = "centripetal", this.options.interpolation.alpha = 0.5)), this.yAxisLeft && n.dataAxis !== void 0 && (this.yAxisLeft.setOptions(this.options.dataAxis), this.yAxisRight.setOptions(this.options.dataAxis)), this.legendLeft && n.legend !== void 0 && (this.legendLeft.setOptions(this.options.legend), this.legendRight.setOptions(this.options.legend)), Object.prototype.hasOwnProperty.call(this.groups, ga) && this.groups[ga].setOptions(n);
  }
  this.dom.frame && (this.forceGraphUpdate = !0, this.body.emitter.emit("_change", {
    queue: !0
  }));
};
Ne.prototype.hide = function() {
  this.dom.frame.parentNode && this.dom.frame.parentNode.removeChild(this.dom.frame);
};
Ne.prototype.show = function() {
  this.dom.frame.parentNode || this.body.dom.center.appendChild(this.dom.frame);
};
Ne.prototype.setItems = function(n) {
  var e = this, t, r = this.itemsData;
  if (!n)
    this.itemsData = null;
  else if (Hr(n))
    this.itemsData = cn(n);
  else
    throw new TypeError("Data must implement the interface of DataSet or DataView");
  if (r && (B(E).call(E, this.itemListeners, function(a, o) {
    r.off(o, a);
  }), r.dispose(), t = r.getIds(), this._onRemove(t)), this.itemsData) {
    var s = this.id;
    B(E).call(E, this.itemListeners, function(a, o) {
      e.itemsData.on(o, a, s);
    }), t = this.itemsData.getIds(), this._onAdd(t);
  }
};
Ne.prototype.setGroups = function(n) {
  var e = this, t;
  if (this.groupsData) {
    B(E).call(E, this.groupListeners, function(a, o) {
      e.groupsData.off(o, a);
    }), t = this.groupsData.getIds(), this.groupsData = null;
    for (var r = 0; r < t.length; r++)
      this._removeGroup(t[r]);
  }
  if (!n)
    this.groupsData = null;
  else if (Hr(n))
    this.groupsData = n;
  else
    throw new TypeError("Data must implement the interface of DataSet or DataView");
  if (this.groupsData) {
    var s = this.id;
    B(E).call(E, this.groupListeners, function(a, o) {
      e.groupsData.on(o, a, s);
    }), t = this.groupsData.getIds(), this._onAddGroups(t);
  }
};
Ne.prototype._onUpdate = function(n) {
  this._updateAllGroupData(n);
};
Ne.prototype._onAdd = function(n) {
  this._onUpdate(n);
};
Ne.prototype._onRemove = function(n) {
  this._onUpdate(n);
};
Ne.prototype._onUpdateGroups = function(n) {
  this._updateAllGroupData(null, n);
};
Ne.prototype._onAddGroups = function(n) {
  this._onUpdateGroups(n);
};
Ne.prototype._onRemoveGroups = function(n) {
  for (var e = 0; e < n.length; e++)
    this._removeGroup(n[e]);
  this.forceGraphUpdate = !0, this.body.emitter.emit("_change", {
    queue: !0
  });
};
Ne.prototype._removeGroup = function(n) {
  Object.prototype.hasOwnProperty.call(this.groups, n) && (this.groups[n].options.yAxisOrientation == "right" ? (this.yAxisRight.removeGroup(n), this.legendRight.removeGroup(n), this.legendRight.redraw()) : (this.yAxisLeft.removeGroup(n), this.legendLeft.removeGroup(n), this.legendLeft.redraw()), delete this.groups[n]);
};
Ne.prototype._updateGroup = function(n, e) {
  Object.prototype.hasOwnProperty.call(this.groups, e) ? (this.groups[e].update(n), this.groups[e].options.yAxisOrientation == "right" ? (this.yAxisRight.updateGroup(e, this.groups[e]), this.legendRight.updateGroup(e, this.groups[e]), this.yAxisLeft.removeGroup(e), this.legendLeft.removeGroup(e)) : (this.yAxisLeft.updateGroup(e, this.groups[e]), this.legendLeft.updateGroup(e, this.groups[e]), this.yAxisRight.removeGroup(e), this.legendRight.removeGroup(e))) : (this.groups[e] = new Ur(n, e, this.options, this.groupsUsingDefaultStyles), this.groups[e].options.yAxisOrientation == "right" ? (this.yAxisRight.addGroup(e, this.groups[e]), this.legendRight.addGroup(e, this.groups[e])) : (this.yAxisLeft.addGroup(e, this.groups[e]), this.legendLeft.addGroup(e, this.groups[e]))), this.legendLeft.redraw(), this.legendRight.redraw();
};
Ne.prototype._updateAllGroupData = function(n, e) {
  if (this.itemsData != null) {
    var t = {}, r = this.itemsData.get(), s = this.itemsData.idProp, a = {};
    n && Ae(n).call(n, function(x) {
      a[x] = x;
    });
    for (var o = {}, l = 0; l < r.length; l++) {
      var c = r[l], d = c.group;
      d == null && (d = ga), Object.prototype.hasOwnProperty.call(o, d) ? o[d]++ : o[d] = 1;
    }
    var f = {};
    if (!e && n) {
      for (d in this.groups)
        if (Object.prototype.hasOwnProperty.call(this.groups, d)) {
          _ = this.groups[d];
          var p = _.getItems();
          t[d] = ie(p).call(p, function(x) {
            return f[x[s]] = x[s], x[s] !== a[x[s]];
          });
          var m = o[d];
          o[d] -= t[d].length, t[d].length < m && (t[d][m - 1] = {});
        }
    }
    for (l = 0; l < r.length; l++)
      if (c = r[l], d = c.group, d == null && (d = ga), !(!e && n && c[s] !== a[c[s]] && Object.prototype.hasOwnProperty.call(f, c[s]))) {
        Object.prototype.hasOwnProperty.call(t, d) || (t[d] = new Array(o[d]));
        var g = E.bridgeObject(c);
        g.x = E.convert(c.x, "Date"), g.end = E.convert(c.end, "Date"), g.orginalY = c.y, g.y = Number(c.y), g[s] = c[s];
        var b = t[d].length - o[d]--;
        t[d][b] = g;
      }
    for (d in this.groups)
      !Object.prototype.hasOwnProperty.call(this.groups, d) || Object.prototype.hasOwnProperty.call(t, d) || (t[d] = new Array(0));
    for (d in t)
      if (Object.prototype.hasOwnProperty.call(t, d))
        if (t[d].length == 0)
          Object.prototype.hasOwnProperty.call(this.groups, d) && this._removeGroup(d);
        else {
          var _ = void 0;
          this.groupsData != null && (_ = this.groupsData.get(d)), _ == null && (_ = {
            id: d,
            content: this.options.defaultGroup + d
          }), this._updateGroup(_, d), this.groups[d].setItems(t[d]);
        }
    this.forceGraphUpdate = !0, this.body.emitter.emit("_change", {
      queue: !0
    });
  }
};
Ne.prototype.redraw = function() {
  var n = !1;
  this.props.width = this.dom.frame.offsetWidth, this.props.height = this.body.domProps.centerContainer.height - this.body.domProps.border.top - this.body.domProps.border.bottom, n = this._isResized() || n;
  var e = this.body.range.end - this.body.range.start, t = e != this.lastVisibleInterval;
  if (this.lastVisibleInterval = e, n == !0) {
    var r;
    this.svg.style.width = E.option.asSize(3 * this.props.width), this.svg.style.left = E.option.asSize(-this.props.width), (Fe(r = this.options.height + "").call(r, "%") != -1 || this.updateSVGheightOnResize == !0) && (this.updateSVGheight = !0);
  }
  if (this.updateSVGheight == !0 ? (this.options.graphHeight != this.props.height + "px" && (this.options.graphHeight = this.props.height + "px", this.svg.style.height = this.props.height + "px"), this.updateSVGheight = !1) : this.svg.style.height = ("" + this.options.graphHeight).replace("px", "") + "px", n == !0 || t == !0 || this.abortedGraphUpdate == !0 || this.forceGraphUpdate == !0)
    n = this._updateGraph() || n, this.forceGraphUpdate = !1, this.lastStart = this.body.range.start, this.svg.style.left = -this.props.width + "px";
  else if (this.lastStart != 0) {
    var s = this.body.range.start - this.lastStart, a = this.body.range.end - this.body.range.start;
    if (this.props.width != 0) {
      var o = this.props.width / a, l = s * o;
      this.svg.style.left = -this.props.width - l + "px";
    }
  }
  return this.legendLeft.redraw(), this.legendRight.redraw(), n;
};
Ne.prototype._getSortedGroupIds = function() {
  var n = [];
  for (var e in this.groups)
    if (Object.prototype.hasOwnProperty.call(this.groups, e)) {
      var t = this.groups[e];
      t.visible == !0 && (this.options.groups.visibility[e] === void 0 || this.options.groups.visibility[e] == !0) && n.push({
        id: e,
        zIndex: t.options.zIndex
      });
    }
  E.insertSort(n, function(a, o) {
    var l = a.zIndex, c = o.zIndex;
    return l === void 0 && (l = 0), c === void 0 && (c = 0), l == c ? 0 : l < c ? -1 : 1;
  });
  for (var r = new Array(n.length), s = 0; s < n.length; s++)
    r[s] = n[s].id;
  return r;
};
Ne.prototype._updateGraph = function() {
  if (Lr(this.svgElements), this.props.width != 0 && this.itemsData != null) {
    var n, e, t = {}, r = !1, s = this.body.util.toGlobalTime(-this.body.domProps.root.width), a = this.body.util.toGlobalTime(2 * this.body.domProps.root.width), o = this._getSortedGroupIds();
    if (o.length > 0) {
      var l = {};
      for (this._getRelevantData(o, l, s, a), this._applySampling(o, l), e = 0; e < o.length; e++)
        this._convertXcoordinates(l[o[e]]);
      if (this._getYRanges(o, l, t), r = this._updateYAxis(o, t), r == !0)
        return zt(this.svgElements), this.abortedGraphUpdate = !0, !0;
      this.abortedGraphUpdate = !1;
      var c = void 0;
      for (e = 0; e < o.length; e++)
        n = this.groups[o[e]], this.options.stack === !0 && this.options.style === "line" && (n.options.excludeFromStacking == null || !n.options.excludeFromStacking) && (c != null && (this._stack(l[n.id], l[c.id]), n.options.shaded.enabled == !0 && n.options.shaded.orientation !== "group" && (n.options.shaded.orientation == "top" && c.options.shaded.orientation !== "group" ? (c.options.shaded.orientation = "group", c.options.shaded.groupId = n.id) : (n.options.shaded.orientation = "group", n.options.shaded.groupId = c.id))), c = n), this._convertYcoordinates(l[o[e]], n);
      var d = {};
      for (e = 0; e < o.length; e++)
        if (n = this.groups[o[e]], n.options.style === "line" && n.options.shaded.enabled == !0) {
          var f = l[o[e]];
          if (f == null || f.length == 0)
            continue;
          if (Object.prototype.hasOwnProperty.call(d, o[e]) || (d[o[e]] = vt.calcPath(f, n)), n.options.shaded.orientation === "group") {
            var p = n.options.shaded.groupId;
            if (Fe(o).call(o, p) === -1) {
              console.log(n.id + ": Unknown shading group target given:" + p);
              continue;
            }
            Object.prototype.hasOwnProperty.call(d, p) || (d[p] = vt.calcPath(l[p], this.groups[p])), vt.drawShading(d[o[e]], n, d[p], this.framework);
          } else
            vt.drawShading(d[o[e]], n, void 0, this.framework);
        }
      for (At.draw(o, l, this.framework), e = 0; e < o.length; e++)
        if (n = this.groups[o[e]], l[o[e]].length > 0)
          switch (n.options.style) {
            case "line":
              Object.prototype.hasOwnProperty.call(d, o[e]) || (d[o[e]] = vt.calcPath(l[o[e]], n)), vt.draw(d[o[e]], n, this.framework);
            // eslint-disable-next-line no-fallthrough
            case "point":
            // eslint-disable-next-line no-fallthrough
            case "points":
              (n.options.style == "point" || n.options.style == "points" || n.options.drawPoints.enabled == !0) && Cs.draw(l[o[e]], n, this.framework);
              break;
          }
    }
  }
  return zt(this.svgElements), !1;
};
Ne.prototype._stack = function(n, e) {
  var t, r, s, a, o;
  t = 0;
  for (var l = 0; l < n.length; l++) {
    a = void 0, o = void 0;
    for (var c = t; c < e.length; c++)
      if (e[c].x === n[l].x) {
        a = e[c], o = e[c], t = c;
        break;
      } else if (e[c].x > n[l].x) {
        o = e[c], c == 0 ? a = o : a = e[c - 1], t = c;
        break;
      }
    o === void 0 && (a = e[e.length - 1], o = e[e.length - 1]), r = o.x - a.x, s = o.y - a.y, r == 0 ? n[l].y = n[l].orginalY + o.y : n[l].y = n[l].orginalY + s / r * (n[l].x - a.x) + a.y;
  }
};
Ne.prototype._getRelevantData = function(n, e, t, r) {
  var s, a, o, l;
  if (n.length > 0)
    for (a = 0; a < n.length; a++) {
      s = this.groups[n[a]];
      var c = s.getItems();
      if (gt(s.options) == !0) {
        var d = function(g, b) {
          return g.getTime() == b.getTime() ? 0 : g < b ? -1 : 1;
        }, f = Math.max(0, E.binarySearchValue(c, t, "x", "before", d)), p = Math.min(c.length, E.binarySearchValue(c, r, "x", "after", d) + 1);
        p <= 0 && (p = c.length);
        var m = new Array(p - f);
        for (o = f; o < p; o++)
          l = s.itemsData[o], m[o - f] = l;
        e[n[a]] = m;
      } else
        e[n[a]] = s.itemsData;
    }
};
Ne.prototype._applySampling = function(n, e) {
  var t;
  if (n.length > 0) {
    for (var r = 0; r < n.length; r++)
      if (t = this.groups[n[r]], t.options.sampling == !0) {
        var s = e[n[r]];
        if (s.length > 0) {
          var a = 1, o = s.length, l = this.body.util.toGlobalScreen(s[s.length - 1].x) - this.body.util.toGlobalScreen(s[0].x), c = o / l;
          a = Math.min(Math.ceil(0.2 * o), Math.max(1, Math.round(c)));
          for (var d = new Array(o), f = 0; f < o; f += a) {
            var p = Math.round(f / a);
            d[p] = s[f];
          }
          e[n[r]] = Mt(d).call(d, 0, Math.round(o / a));
        }
      }
  }
};
Ne.prototype._getYRanges = function(n, e, t) {
  var r, s, a, o = [], l = [], c;
  if (n.length > 0) {
    for (a = 0; a < n.length; a++)
      r = e[n[a]], c = this.groups[n[a]].options, r.length > 0 && (s = this.groups[n[a]], c.stack === !0 && c.style === "bar" ? c.yAxisOrientation === "left" ? o = rt(o).call(o, r) : l = rt(l).call(l, r) : t[n[a]] = s.getYRange(r, n[a]));
    At.getStackedYRange(o, t, n, "__barStackLeft", "left"), At.getStackedYRange(l, t, n, "__barStackRight", "right");
  }
};
Ne.prototype._updateYAxis = function(n, e) {
  var t = !1, r = !1, s = !1, a = 1e9, o = 1e9, l = -1e9, c = -1e9, d, f;
  if (n.length > 0) {
    for (let g = 0; g < n.length; g++) {
      var p = this.groups[n[g]];
      p && p.options.yAxisOrientation != "right" ? (r = !0, a = 1e9, l = -1e9) : p && p.options.yAxisOrientation && (s = !0, o = 1e9, c = -1e9);
    }
    for (i = 0; i < n.length; i++)
      !Object.prototype.hasOwnProperty.call(e, n[i]) || e[n[i]].ignore === !0 || (d = e[n[i]].min, f = e[n[i]].max, e[n[i]].yAxisOrientation != "right" ? (r = !0, a = a > d ? d : a, l = l < f ? f : l) : (s = !0, o = o > d ? d : o, c = c < f ? f : c));
    r == !0 && this.yAxisLeft.setRange(a, l), s == !0 && this.yAxisRight.setRange(o, c);
  }
  t = this._toggleAxisVisiblity(r, this.yAxisLeft) || t, t = this._toggleAxisVisiblity(s, this.yAxisRight) || t, s == !0 && r == !0 ? (this.yAxisLeft.drawIcons = !0, this.yAxisRight.drawIcons = !0) : (this.yAxisLeft.drawIcons = !1, this.yAxisRight.drawIcons = !1), this.yAxisRight.master = !r, this.yAxisRight.masterAxis = this.yAxisLeft, this.yAxisRight.master == !1 ? (s == !0 ? this.yAxisLeft.lineOffset = this.yAxisRight.width : this.yAxisLeft.lineOffset = 0, t = this.yAxisLeft.redraw() || t, t = this.yAxisRight.redraw() || t) : t = this.yAxisRight.redraw() || t;
  var m = ["__barStackLeft", "__barStackRight", "__lineStackLeft", "__lineStackRight"];
  for (i = 0; i < m.length; i++)
    Fe(n).call(n, m[i]) != -1 && Mt(n).call(n, Fe(n).call(n, m[i]), 1);
  return t;
};
Ne.prototype._toggleAxisVisiblity = function(n, e) {
  var t = !1;
  return n == !1 ? e.dom.frame.parentNode && e.hidden == !1 && (e.hide(), t = !0) : !e.dom.frame.parentNode && e.hidden == !0 && (e.show(), t = !0), t;
};
Ne.prototype._convertXcoordinates = function(n) {
  for (var e = this.body.util.toScreen, t = 0; t < n.length; t++)
    n[t].screen_x = e(n[t].x) + this.props.width, n[t].screen_y = n[t].y, n[t].end != null ? n[t].screen_end = e(n[t].end) + this.props.width : n[t].screen_end = void 0;
};
Ne.prototype._convertYcoordinates = function(n, e) {
  var t = this.yAxisLeft, r = Number(this.svg.style.height.replace("px", ""));
  e.options.yAxisOrientation == "right" && (t = this.yAxisRight);
  for (var s = 0; s < n.length; s++)
    n[s].screen_y = Math.round(t.convertValue(n[s].y));
  e.setZeroPosition(Math.min(r, t.convertValue(0)));
};
class ss extends zr {
  /**
   * @param {{dom: Object, domProps: Object, emitter: Emitter, range: Range}} body
   * @param {Object} [options]        See TimeAxis.setOptions for the available
   *                                  options.
   * @constructor TimeAxis
   * @extends Component
   */
  constructor(e, t) {
    super(), this.dom = {
      foreground: null,
      lines: [],
      majorTexts: [],
      minorTexts: [],
      redundant: {
        lines: [],
        majorTexts: [],
        minorTexts: []
      }
    }, this.props = {
      range: {
        start: 0,
        end: 0,
        minimumStep: 0
      },
      lineTop: 0
    }, this.defaultOptions = {
      orientation: {
        axis: "bottom"
      },
      // axis orientation: 'top' or 'bottom'
      showMinorLabels: !0,
      showMajorLabels: !0,
      showWeekScale: !1,
      maxMinorChars: 7,
      format: E.extend({}, ri.FORMAT),
      moment: Et,
      timeAxis: null
    }, this.options = E.extend({}, this.defaultOptions), this.body = e, this._create(), this.setOptions(t);
  }
  /**
   * Set options for the TimeAxis.
   * Parameters will be merged in current options.
   * @param {Object} options  Available options:
   *                          {string} [orientation.axis]
   *                          {boolean} [showMinorLabels]
   *                          {boolean} [showMajorLabels]
   *                          {boolean} [showWeekScale]
   */
  setOptions(e) {
    e && (E.selectiveExtend(["showMinorLabels", "showMajorLabels", "showWeekScale", "maxMinorChars", "hiddenDates", "timeAxis", "moment", "rtl"], this.options, e), E.selectiveDeepExtend(["format"], this.options, e), "orientation" in e && (typeof e.orientation == "string" ? this.options.orientation.axis = e.orientation : typeof e.orientation == "object" && "axis" in e.orientation && (this.options.orientation.axis = e.orientation.axis)), "locale" in e && (typeof Et.locale == "function" ? Et.locale(e.locale) : Et.lang(e.locale)));
  }
  /**
   * Create the HTML DOM for the TimeAxis
   */
  _create() {
    this.dom.foreground = document.createElement("div"), this.dom.background = document.createElement("div"), this.dom.foreground.className = "vis-time-axis vis-foreground", this.dom.background.className = "vis-time-axis vis-background";
  }
  /**
   * Destroy the TimeAxis
   */
  destroy() {
    this.dom.foreground.parentNode && this.dom.foreground.parentNode.removeChild(this.dom.foreground), this.dom.background.parentNode && this.dom.background.parentNode.removeChild(this.dom.background), this.body = null;
  }
  /**
   * Repaint the component
   * @return {boolean} Returns true if the component is resized
   */
  redraw() {
    const e = this.props, t = this.dom.foreground, r = this.dom.background, s = this.options.orientation.axis == "top" ? this.body.dom.top : this.body.dom.bottom, a = t.parentNode !== s;
    this._calculateCharSize();
    const o = this.options.showMinorLabels && this.options.orientation.axis !== "none", l = this.options.showMajorLabels && this.options.orientation.axis !== "none";
    e.minorLabelHeight = o ? e.minorCharHeight : 0, e.majorLabelHeight = l ? e.majorCharHeight : 0, e.height = e.minorLabelHeight + e.majorLabelHeight, e.width = t.offsetWidth, e.minorLineHeight = this.body.domProps.root.height - e.majorLabelHeight - (this.options.orientation.axis == "top" ? this.body.domProps.bottom.height : this.body.domProps.top.height), e.minorLineWidth = 1, e.majorLineHeight = e.minorLineHeight + e.majorLabelHeight, e.majorLineWidth = 1;
    const c = t.nextSibling, d = r.nextSibling;
    return t.parentNode && t.parentNode.removeChild(t), r.parentNode && r.parentNode.removeChild(r), t.style.height = "".concat(this.props.height, "px"), this._repaintLabels(), c ? s.insertBefore(t, c) : s.appendChild(t), d ? this.body.dom.backgroundVertical.insertBefore(r, d) : this.body.dom.backgroundVertical.appendChild(r), this._isResized() || a;
  }
  /**
   * Repaint major and minor text labels and vertical grid lines
   * @private
   */
  _repaintLabels() {
    const e = this.options.orientation.axis, t = E.convert(this.body.range.start, "Number"), r = E.convert(this.body.range.end, "Number"), s = this.body.util.toTime((this.props.minorCharWidth || 10) * this.options.maxMinorChars).valueOf();
    let a = s - is(this.options.moment, this.body.hiddenDates, this.body.range, s);
    a -= this.body.util.toTime(0).valueOf();
    const o = new ri(new Date(t), new Date(r), a, this.body.hiddenDates, this.options);
    o.setMoment(this.options.moment), this.options.format && o.setFormat(this.options.format), this.options.timeAxis && o.setScale(this.options.timeAxis), this.step = o;
    const l = this.dom;
    l.redundant.lines = l.lines, l.redundant.majorTexts = l.majorTexts, l.redundant.minorTexts = l.minorTexts, l.lines = [], l.majorTexts = [], l.minorTexts = [];
    let c, d, f, p, m, g, b = 0, _, x, T, S = 0;
    const k = 1e3;
    let D;
    for (o.start(), d = o.getCurrent(), p = this.body.util.toScreen(d); o.hasNext() && S < k; ) {
      switch (S++, m = o.isMajor(), D = o.getClassName(), c = d, f = p, o.next(), d = o.getCurrent(), p = this.body.util.toScreen(d), _ = b, b = p - f, o.scale) {
        case "week":
          g = !0;
          break;
        default:
          g = b >= _ * 0.4;
          break;
      }
      if (this.options.showMinorLabels && g) {
        var I = this._repaintMinorText(f, o.getLabelMinor(c), e, D);
        I.style.width = "".concat(b, "px");
      }
      m && this.options.showMajorLabels ? (f > 0 && (T == null && (T = f), I = this._repaintMajorText(f, o.getLabelMajor(c), e, D)), x = this._repaintMajorLine(f, b, e, D)) : g ? x = this._repaintMinorLine(f, b, e, D) : x && (x.style.width = "".concat(Vt(x.style.width) + b, "px"));
    }
    if (S === k && !XE && (console.warn("Something is wrong with the Timeline scale. Limited drawing of grid lines to ".concat(k, " lines.")), XE = !0), this.options.showMajorLabels) {
      const C = this.body.util.toTime(0), P = o.getLabelMajor(C), R = P.length * (this.props.majorCharWidth || 10) + 10;
      (T == null || R < T) && this._repaintMajorText(0, P, e, D);
    }
    B(E).call(E, this.dom.redundant, (C) => {
      for (; C.length; ) {
        const P = C.pop();
        P && P.parentNode && P.parentNode.removeChild(P);
      }
    });
  }
  /**
   * Create a minor label for the axis at position x
   * @param {number} x
   * @param {string} text
   * @param {string} orientation   "top" or "bottom" (default)
   * @param {string} className
   * @return {Element} Returns the HTML element of the created label
   * @private
   */
  _repaintMinorText(e, t, r, s) {
    let a = this.dom.redundant.minorTexts.shift();
    if (!a) {
      const l = document.createTextNode("");
      a = document.createElement("div"), a.appendChild(l), this.dom.foreground.appendChild(a);
    }
    this.dom.minorTexts.push(a), a.innerHTML = E.xss(t);
    let o = r == "top" ? this.props.majorLabelHeight : 0;
    return this._setXY(a, e, o), a.className = "vis-text vis-minor ".concat(s), a;
  }
  /**
   * Create a Major label for the axis at position x
   * @param {number} x
   * @param {string} text
   * @param {string} orientation   "top" or "bottom" (default)
   * @param {string} className
   * @return {Element} Returns the HTML element of the created label
   * @private
   */
  _repaintMajorText(e, t, r, s) {
    let a = this.dom.redundant.majorTexts.shift();
    if (!a) {
      const l = document.createElement("div");
      a = document.createElement("div"), a.appendChild(l), this.dom.foreground.appendChild(a);
    }
    a.childNodes[0].innerHTML = E.xss(t), a.className = "vis-text vis-major ".concat(s);
    let o = r == "top" ? 0 : this.props.minorLabelHeight;
    return this._setXY(a, e, o), this.dom.majorTexts.push(a), a;
  }
  /**
   * sets xy
   * @param {string} label
   * @param {number} x
   * @param {number} y
   * @private
   */
  _setXY(e, t, r) {
    var s;
    const a = this.options.rtl ? t * -1 : t;
    e.style.transform = rt(s = "translate(".concat(a, "px, ")).call(s, r, "px)");
  }
  /**
   * Create a minor line for the axis at position x
   * @param {number} left
   * @param {number} width
   * @param {string} orientation   "top" or "bottom" (default)
   * @param {string} className
   * @return {Element} Returns the created line
   * @private
   */
  _repaintMinorLine(e, t, r, s) {
    var a;
    let o = this.dom.redundant.lines.shift();
    o || (o = document.createElement("div"), this.dom.background.appendChild(o)), this.dom.lines.push(o);
    const l = this.props;
    o.style.width = "".concat(t, "px"), o.style.height = "".concat(l.minorLineHeight, "px");
    let c = r == "top" ? l.majorLabelHeight : this.body.domProps.top.height, d = e - l.minorLineWidth / 2;
    return this._setXY(o, d, c), o.className = rt(a = "vis-grid ".concat(this.options.rtl ? "vis-vertical-rtl" : "vis-vertical", " vis-minor ")).call(a, s), o;
  }
  /**
   * Create a Major line for the axis at position x
   * @param {number} left
   * @param {number} width
   * @param {string} orientation   "top" or "bottom" (default)
   * @param {string} className
   * @return {Element} Returns the created line
   * @private
   */
  _repaintMajorLine(e, t, r, s) {
    var a;
    let o = this.dom.redundant.lines.shift();
    o || (o = document.createElement("div"), this.dom.background.appendChild(o)), this.dom.lines.push(o);
    const l = this.props;
    o.style.width = "".concat(t, "px"), o.style.height = "".concat(l.majorLineHeight, "px");
    let c = r == "top" ? 0 : this.body.domProps.top.height, d = e - l.majorLineWidth / 2;
    return this._setXY(o, d, c), o.className = rt(a = "vis-grid ".concat(this.options.rtl ? "vis-vertical-rtl" : "vis-vertical", " vis-major ")).call(a, s), o;
  }
  /**
   * Determine the size of text on the axis (both major and minor axis).
   * The size is calculated only once and then cached in this.props.
   * @private
   */
  _calculateCharSize() {
    this.dom.measureCharMinor || (this.dom.measureCharMinor = document.createElement("DIV"), this.dom.measureCharMinor.className = "vis-text vis-minor vis-measure", this.dom.measureCharMinor.style.position = "absolute", this.dom.measureCharMinor.appendChild(document.createTextNode("0")), this.dom.foreground.appendChild(this.dom.measureCharMinor)), this.props.minorCharHeight = this.dom.measureCharMinor.clientHeight, this.props.minorCharWidth = this.dom.measureCharMinor.clientWidth, this.dom.measureCharMajor || (this.dom.measureCharMajor = document.createElement("DIV"), this.dom.measureCharMajor.className = "vis-text vis-major vis-measure", this.dom.measureCharMajor.style.position = "absolute", this.dom.measureCharMajor.appendChild(document.createTextNode("0")), this.dom.foreground.appendChild(this.dom.measureCharMajor)), this.props.majorCharHeight = this.dom.measureCharMajor.clientHeight, this.props.majorCharWidth = this.dom.measureCharMajor.clientWidth;
  }
}
var XE = !1, fp, ZE;
function eG() {
  if (ZE) return fp;
  ZE = 1, WD();
  var n = /* @__PURE__ */ je();
  return fp = n.setInterval, fp;
}
var pp, QE;
function tG() {
  return QE || (QE = 1, pp = /* @__PURE__ */ eG()), pp;
}
var rG = tG(), iG = /* @__PURE__ */ Q(rG);
function hk(n, e) {
  e.inputHandler = function(t) {
    t.isFirst && e(t);
  }, n.on("hammer.input", e.inputHandler);
}
function nG(n, e) {
  return e.inputHandler = function(t) {
    t.isFinal && e(t);
  }, n.on("hammer.input", e.inputHandler);
}
function sG(n) {
  const e = "pan-y";
  return n.getTouchAction = function() {
    return [e];
  }, n;
}
function rr(n) {
  var e, t;
  this.active = !1, this.dom = {
    container: n
  }, this.dom.overlay = document.createElement("div"), this.dom.overlay.className = "vis-overlay", this.dom.container.appendChild(this.dom.overlay), this.hammer = Tt(this.dom.overlay), this.hammer.on("tap", N(e = this._onTapOverlay).call(e, this));
  var r = this, s = ["tap", "doubletap", "press", "pinch", "pan", "panstart", "panmove", "panend"];
  B(s).call(s, function(a) {
    r.hammer.on(a, function(o) {
      o.stopPropagation();
    });
  }), document && document.body && (this.onClick = function(a) {
    aG(a.target, n) || r.deactivate();
  }, document.body.addEventListener("click", this.onClick)), this.keycharm !== void 0 && this.keycharm.destroy(), this.keycharm = fD(), this.escListener = N(t = this.deactivate).call(t, this);
}
xm(rr.prototype);
rr.current = null;
rr.prototype.destroy = function() {
  this.deactivate(), this.dom.overlay.parentNode.removeChild(this.dom.overlay), this.onClick && document.body.removeEventListener("click", this.onClick), this.keycharm !== void 0 && this.keycharm.destroy(), this.keycharm = null, this.hammer.destroy(), this.hammer = null;
};
rr.prototype.activate = function() {
  var n;
  rr.current && rr.current.deactivate(), rr.current = this, this.active = !0, this.dom.overlay.style.display = "none", E.addClassName(this.dom.container, "vis-active"), this.emit("change"), this.emit("activate"), N(n = this.keycharm).call(n, "esc", this.escListener);
};
rr.prototype.deactivate = function() {
  rr.current === this && (rr.current = null), this.active = !1, this.dom.overlay.style.display = "", E.removeClassName(this.dom.container, "vis-active"), this.keycharm.unbind("esc", this.escListener), this.emit("change"), this.emit("deactivate");
};
rr.prototype._onTapOverlay = function(n) {
  this.activate(), n.stopPropagation();
};
function aG(n, e) {
  for (; n; ) {
    if (n === e)
      return !0;
    n = n.parentNode;
  }
  return !1;
}
class xi {
  /**
   * Create the main DOM for the Core: a root panel containing left, right,
   * top, bottom, content, and background panel.
   * @param {Element} container  The container element where the Core will
   *                             be attached.
   * @protected
   */
  _create(e) {
    var t, r, s;
    this.dom = {}, this.dom.container = e, this.dom.container.style.position = "relative", this.dom.root = document.createElement("div"), this.dom.background = document.createElement("div"), this.dom.backgroundVertical = document.createElement("div"), this.dom.backgroundHorizontal = document.createElement("div"), this.dom.centerContainer = document.createElement("div"), this.dom.leftContainer = document.createElement("div"), this.dom.rightContainer = document.createElement("div"), this.dom.center = document.createElement("div"), this.dom.left = document.createElement("div"), this.dom.right = document.createElement("div"), this.dom.top = document.createElement("div"), this.dom.bottom = document.createElement("div"), this.dom.shadowTop = document.createElement("div"), this.dom.shadowBottom = document.createElement("div"), this.dom.shadowTopLeft = document.createElement("div"), this.dom.shadowBottomLeft = document.createElement("div"), this.dom.shadowTopRight = document.createElement("div"), this.dom.shadowBottomRight = document.createElement("div"), this.dom.rollingModeBtn = document.createElement("div"), this.dom.loadingScreen = document.createElement("div"), this.dom.root.className = "vis-timeline", this.dom.background.className = "vis-panel vis-background", this.dom.backgroundVertical.className = "vis-panel vis-background vis-vertical", this.dom.backgroundHorizontal.className = "vis-panel vis-background vis-horizontal", this.dom.centerContainer.className = "vis-panel vis-center", this.dom.leftContainer.className = "vis-panel vis-left", this.dom.rightContainer.className = "vis-panel vis-right", this.dom.top.className = "vis-panel vis-top", this.dom.bottom.className = "vis-panel vis-bottom", this.dom.left.className = "vis-content", this.dom.center.className = "vis-content", this.dom.right.className = "vis-content", this.dom.shadowTop.className = "vis-shadow vis-top", this.dom.shadowBottom.className = "vis-shadow vis-bottom", this.dom.shadowTopLeft.className = "vis-shadow vis-top", this.dom.shadowBottomLeft.className = "vis-shadow vis-bottom", this.dom.shadowTopRight.className = "vis-shadow vis-top", this.dom.shadowBottomRight.className = "vis-shadow vis-bottom", this.dom.rollingModeBtn.className = "vis-rolling-mode-btn", this.dom.loadingScreen.className = "vis-loading-screen", this.dom.root.appendChild(this.dom.background), this.dom.root.appendChild(this.dom.backgroundVertical), this.dom.root.appendChild(this.dom.backgroundHorizontal), this.dom.root.appendChild(this.dom.centerContainer), this.dom.root.appendChild(this.dom.leftContainer), this.dom.root.appendChild(this.dom.rightContainer), this.dom.root.appendChild(this.dom.top), this.dom.root.appendChild(this.dom.bottom), this.dom.root.appendChild(this.dom.rollingModeBtn), this.dom.centerContainer.appendChild(this.dom.center), this.dom.leftContainer.appendChild(this.dom.left), this.dom.rightContainer.appendChild(this.dom.right), this.dom.centerContainer.appendChild(this.dom.shadowTop), this.dom.centerContainer.appendChild(this.dom.shadowBottom), this.dom.leftContainer.appendChild(this.dom.shadowTopLeft), this.dom.leftContainer.appendChild(this.dom.shadowBottomLeft), this.dom.rightContainer.appendChild(this.dom.shadowTopRight), this.dom.rightContainer.appendChild(this.dom.shadowBottomRight), this.props = {
      root: {},
      background: {},
      centerContainer: {},
      leftContainer: {},
      rightContainer: {},
      center: {},
      left: {},
      right: {},
      top: {},
      bottom: {},
      border: {},
      scrollTop: 0,
      scrollTopMin: 0
    }, this.on("rangechange", () => {
      this.initialDrawDone === !0 && this._redraw();
    }), this.on("rangechanged", () => {
      this.initialRangeChangeDone || (this.initialRangeChangeDone = !0);
    }), this.on("touch", N(t = this._onTouch).call(t, this)), this.on("panmove", N(r = this._onDrag).call(r, this));
    const a = this;
    this._origRedraw = N(s = this._redraw).call(s, this), this._redraw = E.throttle(this._origRedraw), this.on("_change", (b) => {
      a.itemSet && a.itemSet.initialItemSetDrawn && b && b.queue == !0 ? a._redraw() : a._origRedraw();
    }), this.hammer = new Tt(this.dom.root);
    const o = this.hammer.get("pinch").set({
      enable: !0
    });
    o && sG(o), this.hammer.get("pan").set({
      threshold: 5,
      direction: Tt.DIRECTION_ALL
    }), this.timelineListeners = {};
    const l = [
      "tap",
      "doubletap",
      "press",
      "pinch",
      "pan",
      "panstart",
      "panmove",
      "panend"
      // TODO: cleanup
      //'touch', 'pinch',
      //'tap', 'doubletap', 'hold',
      //'dragstart', 'drag', 'dragend',
      //'mousewheel', 'DOMMouseScroll' // DOMMouseScroll is needed for Firefox
    ];
    B(l).call(l, (b) => {
      const _ = (x) => {
        a.isActive() && a.emit(b, x);
      };
      a.hammer.on(b, _), a.timelineListeners[b] = _;
    }), hk(this.hammer, (b) => {
      a.emit("touch", b);
    }), nG(this.hammer, (b) => {
      a.emit("release", b);
    });
    function c(b) {
      if (!this.isActive()) return;
      if (this.emit("mousewheel", b), this.options.preferZoom) {
        if (!this.options.zoomKey || b[this.options.zoomKey]) return;
      } else if (this.options.zoomKey && b[this.options.zoomKey]) return;
      if (!this.options.verticalScroll && !this.options.horizontalScroll) return;
      let _ = 0, x = 0;
      "detail" in b && (x = b.detail * -1), "wheelDelta" in b && (x = b.wheelDelta), "wheelDeltaY" in b && (x = b.wheelDeltaY), "wheelDeltaX" in b && (_ = b.wheelDeltaX * -1), "axis" in b && b.axis === b.HORIZONTAL_AXIS && (_ = x * -1, x = 0), "deltaY" in b && (x = b.deltaY * -1), "deltaX" in b && (_ = b.deltaX);
      var T = 40, S = 800;
      b.deltaMode && (b.deltaMode === 1 ? (_ *= T, x *= T) : (_ *= T, x *= S));
      const k = this.options.verticalScroll, D = Math.abs(x) >= Math.abs(_), I = this.options.horizontalScroll && this.options.horizontalScrollKey && b[this.options.horizontalScrollKey];
      if (k && D && !I) {
        const C = this.props.scrollTop, P = C + x;
        this._setScrollTop(P) !== C && (this._redraw(), this.emit("scroll", b), b.preventDefault());
        return;
      }
      if (this.options.horizontalScroll) {
        this.range.stopRolling();
        let P = (D ? x : _) / 120 * (this.range.end - this.range.start) / 20;
        this.options.horizontalScrollInvert && D && (P = -P);
        const R = this.range.start + P, M = this.range.end + P, q = {
          animation: !1,
          byUser: !0,
          event: b
        };
        this.range.setRange(R, M, q), b.preventDefault();
      }
    }
    const d = "onwheel" in document.createElement("div") ? "wheel" : document.onmousewheel !== void 0 ? "mousewheel" : (
      // DOMMouseScroll - Older Firefox versions use "DOMMouseScroll"
      // onmousewheel - All the use "onmousewheel"
      this.dom.centerContainer.addEventListener ? "DOMMouseScroll" : "onmousewheel"
    );
    this.dom.top.addEventListener, this.dom.bottom.addEventListener, this.dom.centerContainer.addEventListener(d, N(c).call(c, this), !1), this.dom.top.addEventListener(d, N(c).call(c, this), !1), this.dom.bottom.addEventListener(d, N(c).call(c, this), !1);
    function f(b) {
      if (a.options.verticalScroll && (b.preventDefault(), a.isActive())) {
        const _ = -b.target.scrollTop;
        a._setScrollTop(_), a._redraw(), a.emit("scrollSide", b);
      }
    }
    this.dom.left.parentNode.addEventListener("scroll", N(f).call(f, this)), this.dom.right.parentNode.addEventListener("scroll", N(f).call(f, this));
    let p = !1;
    function m(b) {
      var _;
      if (b.preventDefault && (a.emit("dragover", a.getEventProperties(b)), b.preventDefault()), Fe(_ = b.target.className).call(_, "timeline") > -1 && !p)
        return b.dataTransfer.dropEffect = "move", p = !0, !1;
    }
    function g(b) {
      b.preventDefault && b.preventDefault(), b.stopPropagation && b.stopPropagation();
      try {
        var _ = JSON.parse(b.dataTransfer.getData("text"));
        if (!_ || !_.content) return;
      } catch {
        return !1;
      }
      return p = !1, b.center = {
        x: b.clientX,
        y: b.clientY
      }, _.target !== "item" ? a.itemSet._onAddItem(b) : a.itemSet._onDropObjectOnItem(b), a.emit("drop", a.getEventProperties(b)), !1;
    }
    if (this.dom.center.addEventListener("dragover", N(m).call(m, this), !1), this.dom.center.addEventListener("drop", N(g).call(g, this), !1), this.customTimes = [], this.touch = {}, this.redrawCount = 0, this.initialDrawDone = !1, this.initialRangeChangeDone = !1, !e) throw new Error("No container provided");
    e.appendChild(this.dom.root), e.appendChild(this.dom.loadingScreen);
  }
  /**
   * Set options. Options will be passed to all components loaded in the Timeline.
   * @param {Object} [options]
   *                           {String} orientation
   *                              Vertical orientation for the Timeline,
   *                              can be 'bottom' (default) or 'top'.
   *                           {string | number} width
   *                              Width for the timeline, a number in pixels or
   *                              a css string like '1000px' or '75%'. '100%' by default.
   *                           {string | number} height
   *                              Fixed height for the Timeline, a number in pixels or
   *                              a css string like '400px' or '75%'. If undefined,
   *                              The Timeline will automatically size such that
   *                              its contents fit.
   *                           {string | number} minHeight
   *                              Minimum height for the Timeline, a number in pixels or
   *                              a css string like '400px' or '75%'.
   *                           {string | number} maxHeight
   *                              Maximum height for the Timeline, a number in pixels or
   *                              a css string like '400px' or '75%'.
   *                           {number | Date | string} start
   *                              Start date for the visible window
   *                           {number | Date | string} end
   *                              End date for the visible window
   */
  setOptions(e) {
    var t;
    if (e) {
      const o = ["width", "height", "minHeight", "maxHeight", "autoResize", "start", "end", "clickToUse", "dataAttributes", "hiddenDates", "locale", "locales", "moment", "preferZoom", "rtl", "zoomKey", "horizontalScroll", "horizontalScrollKey", "horizontalScrollInvert", "verticalScroll", "longSelectPressTime", "snap"];
      if (E.selectiveExtend(o, this.options, e), this.dom.rollingModeBtn.style.visibility = "hidden", this.options.rtl && (this.dom.container.style.direction = "rtl", this.dom.backgroundVertical.className = "vis-panel vis-background vis-vertical-rtl"), this.options.verticalScroll && (this.options.rtl ? this.dom.rightContainer.className = "vis-panel vis-right vis-vertical-scroll" : this.dom.leftContainer.className = "vis-panel vis-left vis-vertical-scroll"), typeof this.options.orientation != "object" && (this.options.orientation = {
        item: void 0,
        axis: void 0
      }), "orientation" in e && (typeof e.orientation == "string" ? this.options.orientation = {
        item: e.orientation,
        axis: e.orientation
      } : typeof e.orientation == "object" && ("item" in e.orientation && (this.options.orientation.item = e.orientation.item), "axis" in e.orientation && (this.options.orientation.axis = e.orientation.axis))), this.options.orientation.axis === "both") {
        if (!this.timeAxis2) {
          const l = this.timeAxis2 = new ss(this.body, this.options);
          l.setOptions = (c) => {
            const d = c ? E.extend({}, c) : {};
            d.orientation = "top", ss.prototype.setOptions.call(l, d);
          }, this.components.push(l);
        }
      } else if (this.timeAxis2) {
        var r;
        const l = Fe(r = this.components).call(r, this.timeAxis2);
        if (l !== -1) {
          var s;
          Mt(s = this.components).call(s, l, 1);
        }
        this.timeAxis2.destroy(), this.timeAxis2 = null;
      }
      typeof e.drawPoints == "function" && (e.drawPoints = {
        onRender: e.drawPoints
      }), "hiddenDates" in this.options && Ga(this.options.moment, this.body, this.options.hiddenDates), "clickToUse" in e && (e.clickToUse ? this.activator || (this.activator = new rr(this.dom.root)) : this.activator && (this.activator.destroy(), delete this.activator)), this._initAutoResize();
    }
    if (B(t = this.components).call(t, (o) => o.setOptions(e)), "configure" in e) {
      var a;
      this.configurator || (this.configurator = this._createConfigurator()), this.configurator.setOptions(e.configure);
      const o = E.deepExtend({}, this.options);
      B(a = this.components).call(a, (l) => {
        E.deepExtend(o, l.options);
      }), this.configurator.setModuleOptions({
        global: o
      });
    }
    this._redraw();
  }
  /**
   * Returns true when the Timeline is active.
   * @returns {boolean}
   */
  isActive() {
    return !this.activator || this.activator.active;
  }
  /**
   * Destroy the Core, clean up all DOM elements and event listeners.
   */
  destroy() {
    var e;
    this.setItems(null), this.setGroups(null), this.off(), this._stopAutoResize(), this.dom.root.parentNode && this.dom.root.parentNode.removeChild(this.dom.root), this.dom = null, this.activator && (this.activator.destroy(), delete this.activator);
    for (const t in this.timelineListeners)
      Object.prototype.hasOwnProperty.call(this.timelineListeners, t) && delete this.timelineListeners[t];
    this.timelineListeners = null, this.hammer && this.hammer.destroy(), this.hammer = null, B(e = this.components).call(e, (t) => t.destroy()), this.body = null;
  }
  /**
   * Set a custom time bar
   * @param {Date} time
   * @param {number} [id=undefined] Optional id of the custom time bar to be adjusted.
   */
  setCustomTime(e, t) {
    var r;
    const s = ie(r = this.customTimes).call(r, (a) => t === a.options.id);
    if (s.length === 0)
      throw new Error("No custom time bar found with id ".concat(Yt(t)));
    s.length > 0 && s[0].setCustomTime(e);
  }
  /**
   * Retrieve the current custom time.
   * @param {number} [id=undefined]    Id of the custom time bar.
   * @return {Date | undefined} customTime
   */
  getCustomTime(e) {
    var t;
    const r = ie(t = this.customTimes).call(t, (s) => s.options.id === e);
    if (r.length === 0)
      throw new Error("No custom time bar found with id ".concat(Yt(e)));
    return r[0].getCustomTime();
  }
  /**
   * Set a custom marker for the custom time bar.
   * @param {string} [title] Title of the custom marker.
   * @param {number} [id=undefined] Id of the custom marker.
   * @param {boolean} [editable=false] Make the custom marker editable.
   */
  setCustomTimeMarker(e, t, r) {
    var s;
    const a = ie(s = this.customTimes).call(s, (o) => o.options.id === t);
    if (a.length === 0)
      throw new Error("No custom time bar found with id ".concat(Yt(t)));
    a.length > 0 && a[0].setCustomMarker(e, r);
  }
  /**
   * Set a custom title for the custom time bar.
   * @param {string} [title] Custom title
   * @param {number} [id=undefined]    Id of the custom time bar.
   * @returns {*}
   */
  setCustomTimeTitle(e, t) {
    var r;
    const s = ie(r = this.customTimes).call(r, (a) => a.options.id === t);
    if (s.length === 0)
      throw new Error("No custom time bar found with id ".concat(Yt(t)));
    if (s.length > 0)
      return s[0].setCustomTitle(e);
  }
  /**
   * Retrieve meta information from an event.
   * Should be overridden by classes extending Core
   * @param {Event} event
   * @return {Object} An object with related information.
   */
  getEventProperties(e) {
    return {
      event: e
    };
  }
  /**
   * Add custom vertical bar
   * @param {Date | string | number} [time]  A Date, unix timestamp, or
   *                                         ISO date string. Time point where
   *                                         the new bar should be placed.
   *                                         If not provided, `new Date()` will
   *                                         be used.
   * @param {number | string} [id=undefined] Id of the new bar. Optional
   * @return {number | string}               Returns the id of the new bar
   */
  addCustomTime(e, t) {
    var r;
    const s = e !== void 0 ? E.convert(e, "Date") : /* @__PURE__ */ new Date();
    if (bm(r = this.customTimes).call(r, (l) => l.options.id === t))
      throw new Error("A custom time with id ".concat(Yt(t), " already exists"));
    const o = new Ha(this.body, E.extend({}, this.options, {
      time: s,
      id: t,
      snap: this.itemSet ? this.itemSet.options.snap : this.options.snap
    }));
    return this.customTimes.push(o), this.components.push(o), this._redraw(), t;
  }
  /**
   * Remove previously added custom bar
   * @param {int} id ID of the custom bar to be removed
   * [at]returns {boolean} True if the bar exists and is removed, false otherwise
   */
  removeCustomTime(e) {
    var t;
    const r = ie(t = this.customTimes).call(t, (s) => s.options.id === e);
    if (r.length === 0)
      throw new Error("No custom time bar found with id ".concat(Yt(e)));
    B(r).call(r, (s) => {
      var a, o, l, c;
      Mt(a = this.customTimes).call(a, Fe(o = this.customTimes).call(o, s), 1), Mt(l = this.components).call(l, Fe(c = this.components).call(c, s), 1), s.destroy();
    });
  }
  /**
   * Get the id's of the currently visible items.
   * @returns {Array} The ids of the visible items
   */
  getVisibleItems() {
    return this.itemSet && this.itemSet.getVisibleItems() || [];
  }
  /**
   * Get the id's of the items at specific time, where a click takes place on the timeline.
   * @param {Date} timeOfEvent The point in time to query items.
   * @returns {Array} The ids of all items in existence at the time of event.
   */
  getItemsAtCurrentTime(e) {
    return this.time = e, this.itemSet && this.itemSet.getItemsAtCurrentTime(this.time) || [];
  }
  /**
   * Get the id's of the currently visible groups.
   * @returns {Array} The ids of the visible groups
   */
  getVisibleGroups() {
    return this.itemSet && this.itemSet.getVisibleGroups() || [];
  }
  /**
   * Set Core window such that it fits all items
   * @param {Object} [options]  Available options:
   *                                `animation: boolean | {duration: number, easingFunction: string}`
   *                                    If true (default), the range is animated
   *                                    smoothly to the new window. An object can be
   *                                    provided to specify duration and easing function.
   *                                    Default duration is 500 ms, and default easing
   *                                    function is 'easeInOutQuad'.
   * @param {function} [callback] a callback funtion to be executed at the end of this function
   */
  fit(e, t) {
    const r = this.getDataRange();
    if (r.min === null && r.max === null)
      return;
    const s = r.max - r.min, a = new Date(r.min.valueOf() - s * 0.01), o = new Date(r.max.valueOf() + s * 0.01), l = e && e.animation !== void 0 ? e.animation : !0;
    this.range.setRange(a, o, {
      animation: l
    }, t);
  }
  /**
   * Calculate the data range of the items start and end dates
   * [at]returns {{min: [Date], max: [Date]}}
   * @protected
   */
  getDataRange() {
    throw new Error("Cannot invoke abstract method getDataRange");
  }
  /**
   * Set the visible window. Both parameters are optional, you can change only
   * start or only end. Syntax:
   *
   *     TimeLine.setWindow(start, end)
   *     TimeLine.setWindow(start, end, options)
   *     TimeLine.setWindow(range)
   *
   * Where start and end can be a Date, number, or string, and range is an
   * object with properties start and end.
   *
   * @param {Date | number | string | Object} [start] Start date of visible window
   * @param {Date | number | string} [end]            End date of visible window
   * @param {Object} [options]  Available options:
   *                                `animation: boolean | {duration: number, easingFunction: string}`
   *                                    If true (default), the range is animated
   *                                    smoothly to the new window. An object can be
   *                                    provided to specify duration and easing function.
   *                                    Default duration is 500 ms, and default easing
   *                                    function is 'easeInOutQuad'.
   * @param {function} [callback] a callback funtion to be executed at the end of this function
   */
  setWindow(e, t, r, s) {
    typeof arguments[2] == "function" && (s = arguments[2], r = {});
    let a, o;
    arguments.length == 1 ? (o = arguments[0], a = o.animation !== void 0 ? o.animation : !0, this.range.setRange(o.start, o.end, {
      animation: a
    })) : arguments.length == 2 && typeof arguments[1] == "function" ? (o = arguments[0], s = arguments[1], a = o.animation !== void 0 ? o.animation : !0, this.range.setRange(o.start, o.end, {
      animation: a
    }, s)) : (a = r && r.animation !== void 0 ? r.animation : !0, this.range.setRange(e, t, {
      animation: a
    }, s));
  }
  /**
   * Move the window such that given time is centered on screen.
   * @param {Date | number | string} time
   * @param {Object} [options]  Available options:
   *                                `animation: boolean | {duration: number, easingFunction: string}`
   *                                    If true (default), the range is animated
   *                                    smoothly to the new window. An object can be
   *                                    provided to specify duration and easing function.
   *                                    Default duration is 500 ms, and default easing
   *                                    function is 'easeInOutQuad'.
   * @param {function} [callback] a callback funtion to be executed at the end of this function
   */
  moveTo(e, t, r) {
    typeof arguments[1] == "function" && (r = arguments[1], t = {});
    const s = this.range.end - this.range.start, a = E.convert(e, "Date").valueOf(), o = a - s / 2, l = a + s / 2, c = t && t.animation !== void 0 ? t.animation : !0;
    this.range.setRange(o, l, {
      animation: c
    }, r);
  }
  /**
   * Get the visible window
   * @return {{start: Date, end: Date}}   Visible range
   */
  getWindow() {
    const e = this.range.getRange();
    return {
      start: new Date(e.start),
      end: new Date(e.end)
    };
  }
  /**
   * Zoom in the window such that given time is centered on screen.
   * @param {number} percentage - must be between [0..1]
   * @param {Object} [options]  Available options:
   *                                `animation: boolean | {duration: number, easingFunction: string}`
   *                                    If true (default), the range is animated
   *                                    smoothly to the new window. An object can be
   *                                    provided to specify duration and easing function.
   *                                    Default duration is 500 ms, and default easing
   *                                    function is 'easeInOutQuad'.
   * @param {function} [callback] a callback funtion to be executed at the end of this function
   */
  zoomIn(e, t, r) {
    if (!e || e < 0 || e > 1) return;
    typeof arguments[1] == "function" && (r = arguments[1], t = {});
    const s = this.getWindow(), a = s.start.valueOf(), o = s.end.valueOf(), l = o - a, c = l / (1 + e), d = (l - c) / 2, f = a + d, p = o - d;
    this.setWindow(f, p, t, r);
  }
  /**
   * Zoom out the window such that given time is centered on screen.
   * @param {number} percentage - must be between [0..1]
   * @param {Object} [options]  Available options:
   *                                `animation: boolean | {duration: number, easingFunction: string}`
   *                                    If true (default), the range is animated
   *                                    smoothly to the new window. An object can be
   *                                    provided to specify duration and easing function.
   *                                    Default duration is 500 ms, and default easing
   *                                    function is 'easeInOutQuad'.
   * @param {function} [callback] a callback funtion to be executed at the end of this function
   */
  zoomOut(e, t, r) {
    if (!e || e < 0 || e > 1) return;
    typeof arguments[1] == "function" && (r = arguments[1], t = {});
    const s = this.getWindow(), a = s.start.valueOf(), o = s.end.valueOf(), l = o - a, c = a - l * e / 2, d = o + l * e / 2;
    this.setWindow(c, d, t, r);
  }
  /**
   * Force a redraw. Can be overridden by implementations of Core
   *
   * Note: this function will be overridden on construction with a trottled version
   */
  redraw() {
    this._redraw();
  }
  /**
   * Redraw for internal use. Redraws all components. See also the public
   * method redraw.
   * @protected
   */
  _redraw() {
    var e;
    this.redrawCount++;
    const t = this.dom;
    if (!t || !t.container || t.root.offsetWidth == 0) return;
    let r = !1;
    const s = this.options, a = this.props;
    rs(this.options.moment, this.body, this.options.hiddenDates), s.orientation == "top" ? (E.addClassName(t.root, "vis-top"), E.removeClassName(t.root, "vis-bottom")) : (E.removeClassName(t.root, "vis-top"), E.addClassName(t.root, "vis-bottom")), s.rtl ? (E.addClassName(t.root, "vis-rtl"), E.removeClassName(t.root, "vis-ltr")) : (E.addClassName(t.root, "vis-ltr"), E.removeClassName(t.root, "vis-rtl")), t.root.style.maxHeight = E.option.asSize(s.maxHeight, ""), t.root.style.minHeight = E.option.asSize(s.minHeight, ""), t.root.style.width = E.option.asSize(s.width, "");
    const o = t.root.offsetWidth;
    a.border.left = 1, a.border.right = 1, a.border.top = 1, a.border.bottom = 1, a.center.height = t.center.offsetHeight, a.left.height = t.left.offsetHeight, a.right.height = t.right.offsetHeight, a.top.height = t.top.clientHeight || -a.border.top, a.bottom.height = Math.round(t.bottom.getBoundingClientRect().height) || t.bottom.clientHeight || -a.border.bottom;
    const l = Math.max(a.left.height, a.center.height, a.right.height), c = a.top.height + l + a.bottom.height + a.border.top + a.border.bottom;
    t.root.style.height = E.option.asSize(s.height, "".concat(c, "px")), a.root.height = t.root.offsetHeight, a.background.height = a.root.height;
    const d = a.root.height - a.top.height - a.bottom.height;
    a.centerContainer.height = d, a.leftContainer.height = d, a.rightContainer.height = a.leftContainer.height, a.root.width = o, a.background.width = a.root.width, this.initialDrawDone || (a.scrollbarWidth = E.getScrollBarWidth());
    const f = t.leftContainer.clientWidth, p = t.rightContainer.clientWidth;
    s.verticalScroll ? s.rtl ? (a.left.width = f || -a.border.left, a.right.width = p + a.scrollbarWidth || -a.border.right) : (a.left.width = f + a.scrollbarWidth || -a.border.left, a.right.width = p || -a.border.right) : (a.left.width = f || -a.border.left, a.right.width = p || -a.border.right), this._setDOM();
    let m = this._updateScrollTop();
    s.orientation.item != "top" && (m += Math.max(a.centerContainer.height - a.center.height - a.border.top - a.border.bottom, 0)), t.center.style.transform = "translateY(".concat(m, "px)");
    const g = a.scrollTop == 0 ? "hidden" : "", b = a.scrollTop == a.scrollTopMin ? "hidden" : "";
    t.shadowTop.style.visibility = g, t.shadowBottom.style.visibility = b, t.shadowTopLeft.style.visibility = g, t.shadowBottomLeft.style.visibility = b, t.shadowTopRight.style.visibility = g, t.shadowBottomRight.style.visibility = b, s.verticalScroll && (t.rightContainer.className = "vis-panel vis-right vis-vertical-scroll", t.leftContainer.className = "vis-panel vis-left vis-vertical-scroll", t.shadowTopRight.style.visibility = "hidden", t.shadowBottomRight.style.visibility = "hidden", t.shadowTopLeft.style.visibility = "hidden", t.shadowBottomLeft.style.visibility = "hidden", t.left.style.top = "0px", t.right.style.top = "0px"), (!s.verticalScroll || a.center.height < a.centerContainer.height) && (t.left.style.top = "".concat(m, "px"), t.right.style.top = "".concat(m, "px"), t.rightContainer.className = t.rightContainer.className.replace(new RegExp("(?:^|\\s)vis-vertical-scroll(?:\\s|$)"), " "), t.leftContainer.className = t.leftContainer.className.replace(new RegExp("(?:^|\\s)vis-vertical-scroll(?:\\s|$)"), " "), a.left.width = f || -a.border.left, a.right.width = p || -a.border.right, this._setDOM());
    const _ = a.center.height > a.centerContainer.height;
    this.hammer.get("pan").set({
      direction: _ ? Tt.DIRECTION_ALL : Tt.DIRECTION_HORIZONTAL
    }), this.hammer.get("press").set({
      time: this.options.longSelectPressTime
    }), B(e = this.components).call(e, (T) => {
      r = T.redraw() || r;
    });
    const x = 5;
    if (r)
      if (this.redrawCount < x) {
        this.body.emitter.emit("_change");
        return;
      } else
        console.log("WARNING: infinite loop in redraw?");
    else
      this.redrawCount = 0;
    this.body.emitter.emit("changed");
  }
  /**
   * sets the basic DOM components needed for the timeline\graph2d
   */
  _setDOM() {
    const e = this.props, t = this.dom;
    e.leftContainer.width = e.left.width, e.rightContainer.width = e.right.width;
    const r = e.root.width - e.left.width - e.right.width;
    e.center.width = r, e.centerContainer.width = r, e.top.width = r, e.bottom.width = r, t.background.style.height = "".concat(e.background.height, "px"), t.backgroundVertical.style.height = "".concat(e.background.height, "px"), t.backgroundHorizontal.style.height = "".concat(e.centerContainer.height, "px"), t.centerContainer.style.height = "".concat(e.centerContainer.height, "px"), t.leftContainer.style.height = "".concat(e.leftContainer.height, "px"), t.rightContainer.style.height = "".concat(e.rightContainer.height, "px"), t.background.style.width = "".concat(e.background.width, "px"), t.backgroundVertical.style.width = "".concat(e.centerContainer.width, "px"), t.backgroundHorizontal.style.width = "".concat(e.background.width, "px"), t.centerContainer.style.width = "".concat(e.center.width, "px"), t.top.style.width = "".concat(e.top.width, "px"), t.bottom.style.width = "".concat(e.bottom.width, "px"), t.background.style.left = "0", t.background.style.top = "0", t.backgroundVertical.style.left = "".concat(e.left.width + e.border.left, "px"), t.backgroundVertical.style.top = "0", t.backgroundHorizontal.style.left = "0", t.backgroundHorizontal.style.top = "".concat(e.top.height, "px"), t.centerContainer.style.left = "".concat(e.left.width, "px"), t.centerContainer.style.top = "".concat(e.top.height, "px"), t.leftContainer.style.left = "0", t.leftContainer.style.top = "".concat(e.top.height, "px"), t.rightContainer.style.left = "".concat(e.left.width + e.center.width, "px"), t.rightContainer.style.top = "".concat(e.top.height, "px"), t.top.style.left = "".concat(e.left.width, "px"), t.top.style.top = "0", t.bottom.style.left = "".concat(e.left.width, "px"), t.bottom.style.top = "".concat(e.top.height + e.centerContainer.height, "px"), t.center.style.left = "0", t.left.style.left = "0", t.right.style.left = "0";
  }
  /**
   * Set a current time. This can be used for example to ensure that a client's
   * time is synchronized with a shared server time.
   * Only applicable when option `showCurrentTime` is true.
   * @param {Date | string | number} time     A Date, unix timestamp, or
   *                                          ISO date string.
   */
  setCurrentTime(e) {
    if (!this.currentTime)
      throw new Error("Option showCurrentTime must be true");
    this.currentTime.setCurrentTime(e);
  }
  /**
   * Get the current time.
   * Only applicable when option `showCurrentTime` is true.
   * @return {Date} Returns the current time.
   */
  getCurrentTime() {
    if (!this.currentTime)
      throw new Error("Option showCurrentTime must be true");
    return this.currentTime.getCurrentTime();
  }
  /**
   * Convert a position on screen (pixels) to a datetime
   * @param {int}     x    Position on the screen in pixels
   * @return {Date}   time The datetime the corresponds with given position x
   * @protected
   * TODO: move this function to Range
   */
  _toTime(e) {
    return Vp(this, e, this.props.center.width);
  }
  /**
   * Convert a position on the global screen (pixels) to a datetime
   * @param {int}     x    Position on the screen in pixels
   * @return {Date}   time The datetime the corresponds with given position x
   * @protected
   * TODO: move this function to Range
   */
  _toGlobalTime(e) {
    return Vp(this, e, this.props.root.width);
  }
  /**
   * Convert a datetime (Date object) into a position on the screen
   * @param {Date}   time A date
   * @return {int}   x    The position on the screen in pixels which corresponds
   *                      with the given date.
   * @protected
   * TODO: move this function to Range
   */
  _toScreen(e) {
    return Up(this, e, this.props.center.width);
  }
  /**
   * Convert a datetime (Date object) into a position on the root
   * This is used to get the pixel density estimate for the screen, not the center panel
   * @param {Date}   time A date
   * @return {int}   x    The position on root in pixels which corresponds
   *                      with the given date.
   * @protected
   * TODO: move this function to Range
   */
  _toGlobalScreen(e) {
    return Up(this, e, this.props.root.width);
  }
  /**
   * Initialize watching when option autoResize is true
   * @private
   */
  _initAutoResize() {
    this.options.autoResize == !0 ? this._startAutoResize() : this._stopAutoResize();
  }
  /**
   * Watch for changes in the size of the container. On resize, the Panel will
   * automatically redraw itself.
   * @private
   */
  _startAutoResize() {
    const e = this;
    this._stopAutoResize(), this._onResize = () => {
      if (e.options.autoResize != !0) {
        e._stopAutoResize();
        return;
      }
      if (e.dom.root) {
        const t = e.dom.root.offsetHeight, r = e.dom.root.offsetWidth;
        (r != e.props.lastWidth || t != e.props.lastHeight) && (e.props.lastWidth = r, e.props.lastHeight = t, e.props.scrollbarWidth = E.getScrollBarWidth(), e.body.emitter.emit("_change"));
      }
    }, window.addEventListener("resize", this._onResize), e.dom.root && (e.props.lastWidth = e.dom.root.offsetWidth, e.props.lastHeight = e.dom.root.offsetHeight), this.watchTimer = iG(this._onResize, 1e3);
  }
  /**
   * Stop watching for a resize of the frame.
   * @private
   */
  _stopAutoResize() {
    this.watchTimer && (clearInterval(this.watchTimer), this.watchTimer = void 0), this._onResize && (window.removeEventListener("resize", this._onResize), this._onResize = null);
  }
  /**
   * Start moving the timeline vertically
   * @private
   */
  _onTouch() {
    this.touch.allowDragging = !0, this.touch.initialScrollTop = this.props.scrollTop;
  }
  /**
   * Start moving the timeline vertically
   * @private
   */
  _onPinch() {
    this.touch.allowDragging = !1;
  }
  /**
   * Move the timeline vertically
   * @param {Event} event
   * @private
   */
  _onDrag(e) {
    if (!e || !this.touch.allowDragging) return;
    const t = e.deltaY, r = this._getScrollTop(), s = this._setScrollTop(this.touch.initialScrollTop + t);
    this.options.verticalScroll && (this.dom.left.parentNode.scrollTop = -this.props.scrollTop, this.dom.right.parentNode.scrollTop = -this.props.scrollTop), s != r && this.emit("verticalDrag");
  }
  /**
   * Apply a scrollTop
   * @param {number} scrollTop
   * @returns {number} scrollTop  Returns the applied scrollTop
   * @private
   */
  _setScrollTop(e) {
    return this.props.scrollTop = e, this._updateScrollTop(), this.props.scrollTop;
  }
  /**
   * Update the current scrollTop when the height of  the containers has been changed
   * @returns {number} scrollTop  Returns the applied scrollTop
   * @private
   */
  _updateScrollTop() {
    const e = Math.min(this.props.centerContainer.height - this.props.border.top - this.props.border.bottom - this.props.center.height, 0);
    return e != this.props.scrollTopMin && (this.options.orientation.item != "top" && (this.props.scrollTop += e - this.props.scrollTopMin), this.props.scrollTopMin = e), this.props.scrollTop > 0 && (this.props.scrollTop = 0), this.props.scrollTop < e && (this.props.scrollTop = e), this.options.verticalScroll && (this.dom.left.parentNode.scrollTop = -this.props.scrollTop, this.dom.right.parentNode.scrollTop = -this.props.scrollTop), this.props.scrollTop;
  }
  /**
   * Get the current scrollTop
   * @returns {number} scrollTop
   * @private
   */
  _getScrollTop() {
    return this.props.scrollTop;
  }
  /**
   * Load a configurator
   * [at]returns {Object}
   * @private
   */
  _createConfigurator() {
    throw new Error("Cannot invoke abstract method _createConfigurator");
  }
}
xm(xi.prototype);
var oG = {
  black: "#000000",
  navy: "#000080",
  darkblue: "#00008B",
  mediumblue: "#0000CD",
  blue: "#0000FF",
  darkgreen: "#006400",
  green: "#008000",
  teal: "#008080",
  darkcyan: "#008B8B",
  deepskyblue: "#00BFFF",
  darkturquoise: "#00CED1",
  mediumspringgreen: "#00FA9A",
  lime: "#00FF00",
  springgreen: "#00FF7F",
  aqua: "#00FFFF",
  cyan: "#00FFFF",
  midnightblue: "#191970",
  dodgerblue: "#1E90FF",
  lightseagreen: "#20B2AA",
  forestgreen: "#228B22",
  seagreen: "#2E8B57",
  darkslategray: "#2F4F4F",
  limegreen: "#32CD32",
  mediumseagreen: "#3CB371",
  turquoise: "#40E0D0",
  royalblue: "#4169E1",
  steelblue: "#4682B4",
  darkslateblue: "#483D8B",
  mediumturquoise: "#48D1CC",
  indigo: "#4B0082",
  darkolivegreen: "#556B2F",
  cadetblue: "#5F9EA0",
  cornflowerblue: "#6495ED",
  mediumaquamarine: "#66CDAA",
  dimgray: "#696969",
  slateblue: "#6A5ACD",
  olivedrab: "#6B8E23",
  slategray: "#708090",
  lightslategray: "#778899",
  mediumslateblue: "#7B68EE",
  lawngreen: "#7CFC00",
  chartreuse: "#7FFF00",
  aquamarine: "#7FFFD4",
  maroon: "#800000",
  purple: "#800080",
  olive: "#808000",
  gray: "#808080",
  skyblue: "#87CEEB",
  lightskyblue: "#87CEFA",
  blueviolet: "#8A2BE2",
  darkred: "#8B0000",
  darkmagenta: "#8B008B",
  saddlebrown: "#8B4513",
  darkseagreen: "#8FBC8F",
  lightgreen: "#90EE90",
  mediumpurple: "#9370D8",
  darkviolet: "#9400D3",
  palegreen: "#98FB98",
  darkorchid: "#9932CC",
  yellowgreen: "#9ACD32",
  sienna: "#A0522D",
  brown: "#A52A2A",
  darkgray: "#A9A9A9",
  lightblue: "#ADD8E6",
  greenyellow: "#ADFF2F",
  paleturquoise: "#AFEEEE",
  lightsteelblue: "#B0C4DE",
  powderblue: "#B0E0E6",
  firebrick: "#B22222",
  darkgoldenrod: "#B8860B",
  mediumorchid: "#BA55D3",
  rosybrown: "#BC8F8F",
  darkkhaki: "#BDB76B",
  silver: "#C0C0C0",
  mediumvioletred: "#C71585",
  indianred: "#CD5C5C",
  peru: "#CD853F",
  chocolate: "#D2691E",
  tan: "#D2B48C",
  lightgrey: "#D3D3D3",
  palevioletred: "#D87093",
  thistle: "#D8BFD8",
  orchid: "#DA70D6",
  goldenrod: "#DAA520",
  crimson: "#DC143C",
  gainsboro: "#DCDCDC",
  plum: "#DDA0DD",
  burlywood: "#DEB887",
  lightcyan: "#E0FFFF",
  lavender: "#E6E6FA",
  darksalmon: "#E9967A",
  violet: "#EE82EE",
  palegoldenrod: "#EEE8AA",
  lightcoral: "#F08080",
  khaki: "#F0E68C",
  aliceblue: "#F0F8FF",
  honeydew: "#F0FFF0",
  azure: "#F0FFFF",
  sandybrown: "#F4A460",
  wheat: "#F5DEB3",
  beige: "#F5F5DC",
  whitesmoke: "#F5F5F5",
  mintcream: "#F5FFFA",
  ghostwhite: "#F8F8FF",
  salmon: "#FA8072",
  antiquewhite: "#FAEBD7",
  linen: "#FAF0E6",
  lightgoldenrodyellow: "#FAFAD2",
  oldlace: "#FDF5E6",
  red: "#FF0000",
  fuchsia: "#FF00FF",
  magenta: "#FF00FF",
  deeppink: "#FF1493",
  orangered: "#FF4500",
  tomato: "#FF6347",
  hotpink: "#FF69B4",
  coral: "#FF7F50",
  darkorange: "#FF8C00",
  lightsalmon: "#FFA07A",
  orange: "#FFA500",
  lightpink: "#FFB6C1",
  pink: "#FFC0CB",
  gold: "#FFD700",
  peachpuff: "#FFDAB9",
  navajowhite: "#FFDEAD",
  moccasin: "#FFE4B5",
  bisque: "#FFE4C4",
  mistyrose: "#FFE4E1",
  blanchedalmond: "#FFEBCD",
  papayawhip: "#FFEFD5",
  lavenderblush: "#FFF0F5",
  seashell: "#FFF5EE",
  cornsilk: "#FFF8DC",
  lemonchiffon: "#FFFACD",
  floralwhite: "#FFFAF0",
  snow: "#FFFAFA",
  yellow: "#FFFF00",
  lightyellow: "#FFFFE0",
  ivory: "#FFFFF0",
  white: "#FFFFFF"
};
class lG {
  /**
   * @param {number} [pixelRatio=1]
   */
  constructor() {
    let e = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : 1;
    this.pixelRatio = e, this.generated = !1, this.centerCoordinates = {
      x: 289 / 2,
      y: 289 / 2
    }, this.r = 289 * 0.49, this.color = {
      r: 255,
      g: 255,
      b: 255,
      a: 1
    }, this.hueCircle = void 0, this.initialColor = {
      r: 255,
      g: 255,
      b: 255,
      a: 1
    }, this.previousColor = void 0, this.applied = !1, this.updateCallback = () => {
    }, this.closeCallback = () => {
    }, this._create();
  }
  /**
   * this inserts the colorPicker into a div from the DOM
   * @param {Element} container
   */
  insertTo(e) {
    this.hammer !== void 0 && (this.hammer.destroy(), this.hammer = void 0), this.container = e, this.container.appendChild(this.frame), this._bindHammer(), this._setSize();
  }
  /**
   * the callback is executed on apply and save. Bind it to the application
   * @param {function} callback
   */
  setUpdateCallback(e) {
    if (typeof e == "function")
      this.updateCallback = e;
    else
      throw new Error("Function attempted to set as colorPicker update callback is not a function.");
  }
  /**
   * the callback is executed on apply and save. Bind it to the application
   * @param {function} callback
   */
  setCloseCallback(e) {
    if (typeof e == "function")
      this.closeCallback = e;
    else
      throw new Error("Function attempted to set as colorPicker closing callback is not a function.");
  }
  /**
   *
   * @param {string} color
   * @returns {String}
   * @private
   */
  _isColorString(e) {
    if (typeof e == "string")
      return oG[e];
  }
  /**
   * Set the color of the colorPicker
   * Supported formats:
   * 'red'                   --> HTML color string
   * '#ffffff'               --> hex string
   * 'rgb(255,255,255)'      --> rgb string
   * 'rgba(255,255,255,1.0)' --> rgba string
   * {r:255,g:255,b:255}     --> rgb object
   * {r:255,g:255,b:255,a:1.0} --> rgba object
   * @param {string|Object} color
   * @param {boolean} [setInitial=true]
   */
  setColor(e) {
    let t = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : !0;
    if (e === "none")
      return;
    let r;
    var s = this._isColorString(e);
    if (s !== void 0 && (e = s), E.isString(e) === !0) {
      if (E.isValidRGB(e) === !0) {
        let a = e.substr(4).substr(0, e.length - 5).split(",");
        r = {
          r: a[0],
          g: a[1],
          b: a[2],
          a: 1
        };
      } else if (E.isValidRGBA(e) === !0) {
        let a = e.substr(5).substr(0, e.length - 6).split(",");
        r = {
          r: a[0],
          g: a[1],
          b: a[2],
          a: a[3]
        };
      } else if (E.isValidHex(e) === !0) {
        let a = E.hexToRGB(e);
        r = {
          r: a.r,
          g: a.g,
          b: a.b,
          a: 1
        };
      }
    } else if (e instanceof Object && e.r !== void 0 && e.g !== void 0 && e.b !== void 0) {
      let a = e.a !== void 0 ? e.a : "1.0";
      r = {
        r: e.r,
        g: e.g,
        b: e.b,
        a
      };
    }
    if (r === void 0)
      throw new Error("Unknown color passed to the colorPicker. Supported are strings: rgb, hex, rgba. Object: rgb ({r:r,g:g,b:b,[a:a]}). Supplied: " + Yt(e));
    this._setColor(r, t);
  }
  /**
   * this shows the color picker.
   * The hue circle is constructed once and stored.
   */
  show() {
    this.closeCallback !== void 0 && (this.closeCallback(), this.closeCallback = void 0), this.applied = !1, this.frame.style.display = "block", this._generateHueCircle();
  }
  // ------------------------------------------ PRIVATE ----------------------------- //
  /**
   * Hide the picker. Is called by the cancel button.
   * Optional boolean to store the previous color for easy access later on.
   * @param {boolean} [storePrevious=true]
   * @private
   */
  _hide() {
    (arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : !0) === !0 && (this.previousColor = E.extend({}, this.color)), this.applied === !0 && this.updateCallback(this.initialColor), this.frame.style.display = "none", Dt(() => {
      this.closeCallback !== void 0 && (this.closeCallback(), this.closeCallback = void 0);
    }, 0);
  }
  /**
   * bound to the save button. Saves and hides.
   * @private
   */
  _save() {
    this.updateCallback(this.color), this.applied = !1, this._hide();
  }
  /**
   * Bound to apply button. Saves but does not close. Is undone by the cancel button.
   * @private
   */
  _apply() {
    this.applied = !0, this.updateCallback(this.color), this._updatePicker(this.color);
  }
  /**
   * load the color from the previous session.
   * @private
   */
  _loadLast() {
    this.previousColor !== void 0 ? this.setColor(this.previousColor, !1) : alert("There is no last color to load...");
  }
  /**
   * set the color, place the picker
   * @param {Object} rgba
   * @param {boolean} [setInitial=true]
   * @private
   */
  _setColor(e) {
    (arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : !0) === !0 && (this.initialColor = E.extend({}, e)), this.color = e;
    let r = E.RGBToHSV(e.r, e.g, e.b), s = 2 * Math.PI, a = this.r * r.s, o = this.centerCoordinates.x + a * Math.sin(s * r.h), l = this.centerCoordinates.y + a * Math.cos(s * r.h);
    this.colorPickerSelector.style.left = o - 0.5 * this.colorPickerSelector.clientWidth + "px", this.colorPickerSelector.style.top = l - 0.5 * this.colorPickerSelector.clientHeight + "px", this._updatePicker(e);
  }
  /**
   * bound to opacity control
   * @param {number} value
   * @private
   */
  _setOpacity(e) {
    this.color.a = e / 100, this._updatePicker(this.color);
  }
  /**
   * bound to brightness control
   * @param {number} value
   * @private
   */
  _setBrightness(e) {
    let t = E.RGBToHSV(this.color.r, this.color.g, this.color.b);
    t.v = e / 100;
    let r = E.HSVToRGB(t.h, t.s, t.v);
    r.a = this.color.a, this.color = r, this._updatePicker();
  }
  /**
   * update the color picker. A black circle overlays the hue circle to mimic the brightness decreasing.
   * @param {Object} rgba
   * @private
   */
  _updatePicker() {
    let e = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : this.color, t = E.RGBToHSV(e.r, e.g, e.b), r = this.colorPickerCanvas.getContext("2d");
    this.pixelRation === void 0 && (this.pixelRatio = (window.devicePixelRatio || 1) / (r.webkitBackingStorePixelRatio || r.mozBackingStorePixelRatio || r.msBackingStorePixelRatio || r.oBackingStorePixelRatio || r.backingStorePixelRatio || 1)), r.setTransform(this.pixelRatio, 0, 0, this.pixelRatio, 0, 0);
    let s = this.colorPickerCanvas.clientWidth, a = this.colorPickerCanvas.clientHeight;
    r.clearRect(0, 0, s, a), r.putImageData(this.hueCircle, 0, 0), r.fillStyle = "rgba(0,0,0," + (1 - t.v) + ")", r.circle(this.centerCoordinates.x, this.centerCoordinates.y, this.r), nO(r).call(r), this.brightnessRange.value = 100 * t.v, this.opacityRange.value = 100 * e.a, this.initialColorDiv.style.backgroundColor = "rgba(" + this.initialColor.r + "," + this.initialColor.g + "," + this.initialColor.b + "," + this.initialColor.a + ")", this.newColorDiv.style.backgroundColor = "rgba(" + this.color.r + "," + this.color.g + "," + this.color.b + "," + this.color.a + ")";
  }
  /**
   * used by create to set the size of the canvas.
   * @private
   */
  _setSize() {
    this.colorPickerCanvas.style.width = "100%", this.colorPickerCanvas.style.height = "100%", this.colorPickerCanvas.width = 289 * this.pixelRatio, this.colorPickerCanvas.height = 289 * this.pixelRatio;
  }
  /**
   * create all dom elements
   * TODO: cleanup, lots of similar dom elements
   * @private
   */
  _create() {
    var e, t, r, s;
    if (this.frame = document.createElement("div"), this.frame.className = "vis-color-picker", this.colorPickerDiv = document.createElement("div"), this.colorPickerSelector = document.createElement("div"), this.colorPickerSelector.className = "vis-selector", this.colorPickerDiv.appendChild(this.colorPickerSelector), this.colorPickerCanvas = document.createElement("canvas"), this.colorPickerDiv.appendChild(this.colorPickerCanvas), this.colorPickerCanvas.getContext) {
      let o = this.colorPickerCanvas.getContext("2d");
      this.pixelRatio = (window.devicePixelRatio || 1) / (o.webkitBackingStorePixelRatio || o.mozBackingStorePixelRatio || o.msBackingStorePixelRatio || o.oBackingStorePixelRatio || o.backingStorePixelRatio || 1), this.colorPickerCanvas.getContext("2d").setTransform(this.pixelRatio, 0, 0, this.pixelRatio, 0, 0);
    } else {
      let o = document.createElement("DIV");
      o.style.color = "red", o.style.fontWeight = "bold", o.style.padding = "10px", o.innerHTML = "Error: your browser does not support HTML canvas", this.colorPickerCanvas.appendChild(o);
    }
    this.colorPickerDiv.className = "vis-color", this.opacityDiv = document.createElement("div"), this.opacityDiv.className = "vis-opacity", this.brightnessDiv = document.createElement("div"), this.brightnessDiv.className = "vis-brightness", this.arrowDiv = document.createElement("div"), this.arrowDiv.className = "vis-arrow", this.opacityRange = document.createElement("input");
    try {
      this.opacityRange.type = "range", this.opacityRange.min = "0", this.opacityRange.max = "100";
    } catch {
    }
    this.opacityRange.value = "100", this.opacityRange.className = "vis-range", this.brightnessRange = document.createElement("input");
    try {
      this.brightnessRange.type = "range", this.brightnessRange.min = "0", this.brightnessRange.max = "100";
    } catch {
    }
    this.brightnessRange.value = "100", this.brightnessRange.className = "vis-range", this.opacityDiv.appendChild(this.opacityRange), this.brightnessDiv.appendChild(this.brightnessRange);
    var a = this;
    this.opacityRange.onchange = function() {
      a._setOpacity(this.value);
    }, this.opacityRange.oninput = function() {
      a._setOpacity(this.value);
    }, this.brightnessRange.onchange = function() {
      a._setBrightness(this.value);
    }, this.brightnessRange.oninput = function() {
      a._setBrightness(this.value);
    }, this.brightnessLabel = document.createElement("div"), this.brightnessLabel.className = "vis-label vis-brightness", this.brightnessLabel.innerHTML = "brightness:", this.opacityLabel = document.createElement("div"), this.opacityLabel.className = "vis-label vis-opacity", this.opacityLabel.innerHTML = "opacity:", this.newColorDiv = document.createElement("div"), this.newColorDiv.className = "vis-new-color", this.newColorDiv.innerHTML = "new", this.initialColorDiv = document.createElement("div"), this.initialColorDiv.className = "vis-initial-color", this.initialColorDiv.innerHTML = "initial", this.cancelButton = document.createElement("div"), this.cancelButton.className = "vis-button vis-cancel", this.cancelButton.innerHTML = "cancel", this.cancelButton.onclick = N(e = this._hide).call(e, this, !1), this.applyButton = document.createElement("div"), this.applyButton.className = "vis-button vis-apply", this.applyButton.innerHTML = "apply", this.applyButton.onclick = N(t = this._apply).call(t, this), this.saveButton = document.createElement("div"), this.saveButton.className = "vis-button vis-save", this.saveButton.innerHTML = "save", this.saveButton.onclick = N(r = this._save).call(r, this), this.loadButton = document.createElement("div"), this.loadButton.className = "vis-button vis-load", this.loadButton.innerHTML = "load last", this.loadButton.onclick = N(s = this._loadLast).call(s, this), this.frame.appendChild(this.colorPickerDiv), this.frame.appendChild(this.arrowDiv), this.frame.appendChild(this.brightnessLabel), this.frame.appendChild(this.brightnessDiv), this.frame.appendChild(this.opacityLabel), this.frame.appendChild(this.opacityDiv), this.frame.appendChild(this.newColorDiv), this.frame.appendChild(this.initialColorDiv), this.frame.appendChild(this.cancelButton), this.frame.appendChild(this.applyButton), this.frame.appendChild(this.saveButton), this.frame.appendChild(this.loadButton);
  }
  /**
   * bind hammer to the color picker
   * @private
   */
  _bindHammer() {
    this.drag = {}, this.pinch = {}, this.hammer = new Tt(this.colorPickerCanvas), this.hammer.get("pinch").set({
      enable: !0
    }), hk(this.hammer, (e) => {
      this._moveSelector(e);
    }), this.hammer.on("tap", (e) => {
      this._moveSelector(e);
    }), this.hammer.on("panstart", (e) => {
      this._moveSelector(e);
    }), this.hammer.on("panmove", (e) => {
      this._moveSelector(e);
    }), this.hammer.on("panend", (e) => {
      this._moveSelector(e);
    });
  }
  /**
   * generate the hue circle. This is relatively heavy (200ms) and is done only once on the first time it is shown.
   * @private
   */
  _generateHueCircle() {
    if (this.generated === !1) {
      let e = this.colorPickerCanvas.getContext("2d");
      this.pixelRation === void 0 && (this.pixelRatio = (window.devicePixelRatio || 1) / (e.webkitBackingStorePixelRatio || e.mozBackingStorePixelRatio || e.msBackingStorePixelRatio || e.oBackingStorePixelRatio || e.backingStorePixelRatio || 1)), e.setTransform(this.pixelRatio, 0, 0, this.pixelRatio, 0, 0);
      let t = this.colorPickerCanvas.clientWidth, r = this.colorPickerCanvas.clientHeight;
      e.clearRect(0, 0, t, r);
      let s, a, o, l;
      this.centerCoordinates = {
        x: t * 0.5,
        y: r * 0.5
      }, this.r = 0.49 * t;
      let c = 2 * Math.PI / 360, d = 1 / 360, f = 1 / this.r, p;
      for (o = 0; o < 360; o++)
        for (l = 0; l < this.r; l++)
          s = this.centerCoordinates.x + l * Math.sin(c * o), a = this.centerCoordinates.y + l * Math.cos(c * o), p = E.HSVToRGB(o * d, l * f, 1), e.fillStyle = "rgb(" + p.r + "," + p.g + "," + p.b + ")", e.fillRect(s - 0.5, a - 0.5, 2, 2);
      e.strokeStyle = "rgba(0,0,0,1)", e.circle(this.centerCoordinates.x, this.centerCoordinates.y, this.r), e.stroke(), this.hueCircle = e.getImageData(0, 0, t, r);
    }
    this.generated = !0;
  }
  /**
   * move the selector. This is called by hammer functions.
   *
   * @param {Event}  event   The event
   * @private
   */
  _moveSelector(e) {
    let t = this.colorPickerDiv.getBoundingClientRect(), r = e.center.x - t.left, s = e.center.y - t.top, a = 0.5 * this.colorPickerDiv.clientHeight, o = 0.5 * this.colorPickerDiv.clientWidth, l = r - o, c = s - a, d = Math.atan2(l, c), f = 0.98 * Math.min(Math.sqrt(l * l + c * c), o), p = Math.cos(d) * f + a, m = Math.sin(d) * f + o;
    this.colorPickerSelector.style.top = p - 0.5 * this.colorPickerSelector.clientHeight + "px", this.colorPickerSelector.style.left = m - 0.5 * this.colorPickerSelector.clientWidth + "px";
    let g = d / (2 * Math.PI);
    g = g < 0 ? g + 1 : g;
    let b = f / this.r, _ = E.RGBToHSV(this.color.r, this.color.g, this.color.b);
    _.h = g, _.s = b;
    let x = E.HSVToRGB(_.h, _.s, _.v);
    x.a = this.color.a, this.color = x, this.initialColorDiv.style.backgroundColor = "rgba(" + this.initialColor.r + "," + this.initialColor.g + "," + this.initialColor.b + "," + this.initialColor.a + ")", this.newColorDiv.style.backgroundColor = "rgba(" + this.color.r + "," + this.color.g + "," + this.color.b + "," + this.color.a + ")";
  }
}
class fk {
  /**
   * @param {Object} parentModule        | the location where parentModule.setOptions() can be called
   * @param {Object} defaultContainer    | the default container of the module
   * @param {Object} configureOptions    | the fully configured and predefined options set found in allOptions.js
   * @param {number} pixelRatio          | canvas pixel ratio
   */
  constructor(e, t, r) {
    let s = arguments.length > 3 && arguments[3] !== void 0 ? arguments[3] : 1;
    this.parent = e, this.changedOptions = [], this.container = t, this.allowCreation = !1, this.options = {}, this.initialized = !1, this.popupCounter = 0, this.defaultOptions = {
      enabled: !1,
      filter: !0,
      container: void 0,
      showButton: !0
    }, E.extend(this.options, this.defaultOptions), this.configureOptions = r, this.moduleOptions = {}, this.domElements = [], this.popupDiv = {}, this.popupLimit = 5, this.popupHistory = {}, this.colorPicker = new lG(s), this.wrapper = void 0;
  }
  /**
   * refresh all options.
   * Because all modules parse their options by themselves, we just use their options. We copy them here.
   *
   * @param {Object} options
   */
  setOptions(e) {
    if (e !== void 0) {
      this.popupHistory = {}, this._removePopup();
      let t = !0;
      if (typeof e == "string")
        this.options.filter = e;
      else if (ye(e))
        this.options.filter = e.join();
      else if (typeof e == "object") {
        if (e == null)
          throw new TypeError("options cannot be null");
        e.container !== void 0 && (this.options.container = e.container), ie(e) !== void 0 && (this.options.filter = ie(e)), e.showButton !== void 0 && (this.options.showButton = e.showButton), e.enabled !== void 0 && (t = e.enabled);
      } else typeof e == "boolean" ? (this.options.filter = !0, t = e) : typeof e == "function" && (this.options.filter = e, t = !0);
      ie(this.options) === !1 && (t = !1), this.options.enabled = t;
    }
    this._clean();
  }
  /**
   *
   * @param {Object} moduleOptions
   */
  setModuleOptions(e) {
    this.moduleOptions = e, this.options.enabled === !0 && (this._clean(), this.options.container !== void 0 && (this.container = this.options.container), this._create());
  }
  /**
   * Create all DOM elements
   * @private
   */
  _create() {
    this._clean(), this.changedOptions = [];
    let e = ie(this.options), t = 0, r = !1;
    for (let s in this.configureOptions)
      Object.prototype.hasOwnProperty.call(this.configureOptions, s) && (this.allowCreation = !1, r = !1, typeof e == "function" ? (r = e(s, []), r = r || this._handleObject(this.configureOptions[s], [s], !0)) : (e === !0 || Fe(e).call(e, s) !== -1) && (r = !0), r !== !1 && (this.allowCreation = !0, t > 0 && this._makeItem([]), this._makeHeader(s), this._handleObject(this.configureOptions[s], [s])), t++);
    this._makeButton(), this._push();
  }
  /**
   * draw all DOM elements on the screen
   * @private
   */
  _push() {
    this.wrapper = document.createElement("div"), this.wrapper.className = "vis-configuration-wrapper", this.container.appendChild(this.wrapper);
    for (var e = 0; e < this.domElements.length; e++)
      this.wrapper.appendChild(this.domElements[e]);
    this._showPopupIfNeeded();
  }
  /**
   * delete all DOM elements
   * @private
   */
  _clean() {
    for (var e = 0; e < this.domElements.length; e++)
      this.wrapper.removeChild(this.domElements[e]);
    this.wrapper !== void 0 && (this.container.removeChild(this.wrapper), this.wrapper = void 0), this.domElements = [], this._removePopup();
  }
  /**
   * get the value from the actualOptions if it exists
   * @param {array} path    | where to look for the actual option
   * @returns {*}
   * @private
   */
  _getValue(e) {
    let t = this.moduleOptions;
    for (let r = 0; r < e.length; r++)
      if (t[e[r]] !== void 0)
        t = t[e[r]];
      else {
        t = void 0;
        break;
      }
    return t;
  }
  /**
   * all option elements are wrapped in an item
   * @param {Array} path    | where to look for the actual option
   * @param {Array.<Element>} domElements
   * @returns {number}
   * @private
   */
  _makeItem(e) {
    if (this.allowCreation === !0) {
      let a = document.createElement("div");
      a.className = "vis-configuration vis-config-item vis-config-s" + e.length;
      for (var t = arguments.length, r = new Array(t > 1 ? t - 1 : 0), s = 1; s < t; s++)
        r[s - 1] = arguments[s];
      return B(r).call(r, (o) => {
        a.appendChild(o);
      }), this.domElements.push(a), this.domElements.length;
    }
    return 0;
  }
  /**
   * header for major subjects
   * @param {string} name
   * @private
   */
  _makeHeader(e) {
    let t = document.createElement("div");
    t.className = "vis-configuration vis-config-header", t.innerHTML = E.xss(e), this._makeItem([], t);
  }
  /**
   * make a label, if it is an object label, it gets different styling.
   * @param {string} name
   * @param {array} path    | where to look for the actual option
   * @param {string} objectLabel
   * @returns {HTMLElement}
   * @private
   */
  _makeLabel(e, t) {
    let r = arguments.length > 2 && arguments[2] !== void 0 ? arguments[2] : !1, s = document.createElement("div");
    return s.className = "vis-configuration vis-config-label vis-config-s" + t.length, r === !0 ? s.innerHTML = E.xss("<i><b>" + e + ":</b></i>") : s.innerHTML = E.xss(e + ":"), s;
  }
  /**
   * make a dropdown list for multiple possible string optoins
   * @param {Array.<number>} arr
   * @param {number} value
   * @param {array} path    | where to look for the actual option
   * @private
   */
  _makeDropdown(e, t, r) {
    let s = document.createElement("select");
    s.className = "vis-configuration vis-config-select";
    let a = 0;
    t !== void 0 && Fe(e).call(e, t) !== -1 && (a = Fe(e).call(e, t));
    for (let c = 0; c < e.length; c++) {
      let d = document.createElement("option");
      d.value = e[c], c === a && (d.selected = "selected"), d.innerHTML = e[c], s.appendChild(d);
    }
    let o = this;
    s.onchange = function() {
      o._update(this.value, r);
    };
    let l = this._makeLabel(r[r.length - 1], r);
    this._makeItem(r, l, s);
  }
  /**
   * make a range object for numeric options
   * @param {Array.<number>} arr
   * @param {number} value
   * @param {array} path    | where to look for the actual option
   * @private
   */
  _makeRange(e, t, r) {
    let s = e[0], a = e[1], o = e[2], l = e[3], c = document.createElement("input");
    c.className = "vis-configuration vis-config-range";
    try {
      c.type = "range", c.min = a, c.max = o;
    } catch {
    }
    c.step = l;
    let d = "", f = 0;
    if (t !== void 0) {
      let _ = 1.2;
      t < 0 && t * _ < a ? (c.min = Math.ceil(t * _), f = c.min, d = "range increased") : t / _ < a && (c.min = Math.ceil(t / _), f = c.min, d = "range increased"), t * _ > o && o !== 1 && (c.max = Math.ceil(t * _), f = c.max, d = "range increased"), c.value = t;
    } else
      c.value = s;
    let p = document.createElement("input");
    p.className = "vis-configuration vis-config-rangeinput", p.value = Number(c.value);
    var m = this;
    c.onchange = function() {
      p.value = this.value, m._update(Number(this.value), r);
    }, c.oninput = function() {
      p.value = this.value;
    };
    let g = this._makeLabel(r[r.length - 1], r), b = this._makeItem(r, g, c, p);
    d !== "" && this.popupHistory[b] !== f && (this.popupHistory[b] = f, this._setupPopup(d, b));
  }
  /**
   * make a button object
   * @private
   */
  _makeButton() {
    if (this.options.showButton === !0) {
      let e = document.createElement("div");
      e.className = "vis-configuration vis-config-button", e.innerHTML = "generate options", e.onclick = () => {
        this._printOptions();
      }, e.onmouseover = () => {
        e.className = "vis-configuration vis-config-button hover";
      }, e.onmouseout = () => {
        e.className = "vis-configuration vis-config-button";
      }, this.optionsContainer = document.createElement("div"), this.optionsContainer.className = "vis-configuration vis-config-option-container", this.domElements.push(this.optionsContainer), this.domElements.push(e);
    }
  }
  /**
   * prepare the popup
   * @param {string} string
   * @param {number} index
   * @private
   */
  _setupPopup(e, t) {
    if (this.initialized === !0 && this.allowCreation === !0 && this.popupCounter < this.popupLimit) {
      let r = document.createElement("div");
      r.id = "vis-configuration-popup", r.className = "vis-configuration-popup", r.innerHTML = E.xss(e), r.onclick = () => {
        this._removePopup();
      }, this.popupCounter += 1, this.popupDiv = {
        html: r,
        index: t
      };
    }
  }
  /**
   * remove the popup from the dom
   * @private
   */
  _removePopup() {
    this.popupDiv.html !== void 0 && (this.popupDiv.html.parentNode.removeChild(this.popupDiv.html), clearTimeout(this.popupDiv.hideTimeout), clearTimeout(this.popupDiv.deleteTimeout), this.popupDiv = {});
  }
  /**
   * Show the popup if it is needed.
   * @private
   */
  _showPopupIfNeeded() {
    if (this.popupDiv.html !== void 0) {
      let t = this.domElements[this.popupDiv.index].getBoundingClientRect();
      this.popupDiv.html.style.left = t.left + "px", this.popupDiv.html.style.top = t.top - 30 + "px", document.body.appendChild(this.popupDiv.html), this.popupDiv.hideTimeout = Dt(() => {
        this.popupDiv.html.style.opacity = 0;
      }, 1500), this.popupDiv.deleteTimeout = Dt(() => {
        this._removePopup();
      }, 1800);
    }
  }
  /**
   * make a checkbox for boolean options.
   * @param {number} defaultValue
   * @param {number} value
   * @param {array} path    | where to look for the actual option
   * @private
   */
  _makeCheckbox(e, t, r) {
    var s = document.createElement("input");
    s.type = "checkbox", s.className = "vis-configuration vis-config-checkbox", s.checked = e, t !== void 0 && (s.checked = t, t !== e && (typeof e == "object" ? t !== e.enabled && this.changedOptions.push({
      path: r,
      value: t
    }) : this.changedOptions.push({
      path: r,
      value: t
    })));
    let a = this;
    s.onchange = function() {
      a._update(this.checked, r);
    };
    let o = this._makeLabel(r[r.length - 1], r);
    this._makeItem(r, o, s);
  }
  /**
   * make a text input field for string options.
   * @param {number} defaultValue
   * @param {number} value
   * @param {array} path    | where to look for the actual option
   * @private
   */
  _makeTextInput(e, t, r) {
    var s = document.createElement("input");
    s.type = "text", s.className = "vis-configuration vis-config-text", s.value = t, t !== e && this.changedOptions.push({
      path: r,
      value: t
    });
    let a = this;
    s.onchange = function() {
      a._update(this.value, r);
    };
    let o = this._makeLabel(r[r.length - 1], r);
    this._makeItem(r, o, s);
  }
  /**
   * make a color field with a color picker for color fields
   * @param {Array.<number>} arr
   * @param {number} value
   * @param {array} path    | where to look for the actual option
   * @private
   */
  _makeColorField(e, t, r) {
    let s = e[1], a = document.createElement("div");
    t = t === void 0 ? s : t, t !== "none" ? (a.className = "vis-configuration vis-config-colorBlock", a.style.backgroundColor = t) : a.className = "vis-configuration vis-config-colorBlock none", t = t === void 0 ? s : t, a.onclick = () => {
      this._showColorPicker(t, a, r);
    };
    let o = this._makeLabel(r[r.length - 1], r);
    this._makeItem(r, o, a);
  }
  /**
   * used by the color buttons to call the color picker.
   * @param {number} value
   * @param {HTMLElement} div
   * @param {array} path    | where to look for the actual option
   * @private
   */
  _showColorPicker(e, t, r) {
    t.onclick = function() {
    }, this.colorPicker.insertTo(t), this.colorPicker.show(), this.colorPicker.setColor(e), this.colorPicker.setUpdateCallback((s) => {
      let a = "rgba(" + s.r + "," + s.g + "," + s.b + "," + s.a + ")";
      t.style.backgroundColor = a, this._update(a, r);
    }), this.colorPicker.setCloseCallback(() => {
      t.onclick = () => {
        this._showColorPicker(e, t, r);
      };
    });
  }
  /**
   * parse an object and draw the correct items
   * @param {Object} obj
   * @param {array} [path=[]]    | where to look for the actual option
   * @param {boolean} [checkOnly=false]
   * @returns {boolean}
   * @private
   */
  _handleObject(e) {
    let t = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : [], r = arguments.length > 2 && arguments[2] !== void 0 ? arguments[2] : !1, s = !1, a = ie(this.options), o = !1;
    for (let l in e) {
      if (!Object.prototype.hasOwnProperty.call(e, l)) continue;
      s = !0;
      let c = e[l], d = E.copyAndExtendArray(t, l);
      if (typeof a == "function" && (s = a(l, t), s === !1 && !ye(c) && typeof c != "string" && typeof c != "boolean" && c instanceof Object && (this.allowCreation = !1, s = this._handleObject(c, d, !0), this.allowCreation = r === !1)), s !== !1) {
        o = !0;
        let f = this._getValue(d);
        if (ye(c))
          this._handleArray(c, f, d);
        else if (typeof c == "string")
          this._makeTextInput(c, f, d);
        else if (typeof c == "boolean")
          this._makeCheckbox(c, f, d);
        else if (c instanceof Object) {
          let p = !0;
          if (Fe(t).call(t, "physics") !== -1 && this.moduleOptions.physics.solver !== l && (p = !1), p === !0)
            if (c.enabled !== void 0) {
              let m = E.copyAndExtendArray(d, "enabled"), g = this._getValue(m);
              if (g === !0) {
                let b = this._makeLabel(l, d, !0);
                this._makeItem(d, b), o = this._handleObject(c, d) || o;
              } else
                this._makeCheckbox(c, g, d);
            } else {
              let m = this._makeLabel(l, d, !0);
              this._makeItem(d, m), o = this._handleObject(c, d) || o;
            }
        } else
          console.error("dont know how to handle", c, l, d);
      }
    }
    return o;
  }
  /**
   * handle the array type of option
   * @param {Array.<number>} arr
   * @param {number} value
   * @param {array} path    | where to look for the actual option
   * @private
   */
  _handleArray(e, t, r) {
    typeof e[0] == "string" && e[0] === "color" ? (this._makeColorField(e, t, r), e[1] !== t && this.changedOptions.push({
      path: r,
      value: t
    })) : typeof e[0] == "string" ? (this._makeDropdown(e, t, r), e[0] !== t && this.changedOptions.push({
      path: r,
      value: t
    })) : typeof e[0] == "number" && (this._makeRange(e, t, r), e[0] !== t && this.changedOptions.push({
      path: r,
      value: Number(t)
    }));
  }
  /**
   * called to update the network with the new settings.
   * @param {number} value
   * @param {array} path    | where to look for the actual option
   * @private
   */
  _update(e, t) {
    let r = this._constructOptions(e, t);
    this.parent.body && this.parent.body.emitter && this.parent.body.emitter.emit && this.parent.body.emitter.emit("configChange", r), this.initialized = !0, this.parent.setOptions(r);
  }
  /**
   *
   * @param {string|Boolean} value
   * @param {Array.<string>} path
   * @param {{}} optionsObj
   * @returns {{}}
   * @private
   */
  _constructOptions(e, t) {
    let r = arguments.length > 2 && arguments[2] !== void 0 ? arguments[2] : {}, s = r;
    e = e === "true" ? !0 : e, e = e === "false" ? !1 : e;
    for (let a = 0; a < t.length; a++)
      t[a] !== "global" && (s[t[a]] === void 0 && (s[t[a]] = {}), a !== t.length - 1 ? s = s[t[a]] : s[t[a]] = e);
    return r;
  }
  /**
   * @private
   */
  _printOptions() {
    let e = this.getOptions();
    this.optionsContainer.innerHTML = "<pre>var options = " + Yt(e, null, 2) + "</pre>";
  }
  /**
   *
   * @returns {{}} options
   */
  getOptions() {
    let e = {};
    for (var t = 0; t < this.changedOptions.length; t++)
      this._constructOptions(this.changedOptions[t].value, this.changedOptions[t].path, e);
    return e;
  }
}
let Fn = !1, JE;
const ya = "background: #FFeeee; color: #dd0000";
class Ue {
  /**
   * Main function to be called
   * @param {Object} options
   * @param {Object} referenceOptions
   * @param {Object} subObject
   * @returns {boolean}
   * @static
   */
  static validate(e, t, r) {
    Fn = !1, JE = t;
    let s = t;
    return r !== void 0 && (s = t[r]), Ue.parse(e, s, []), Fn;
  }
  /**
   * Will traverse an object recursively and check every value
   * @param {Object} options
   * @param {Object} referenceOptions
   * @param {array} path    | where to look for the actual option
   * @static
   */
  static parse(e, t, r) {
    for (let s in e)
      Object.prototype.hasOwnProperty.call(e, s) && Ue.check(s, e, t, r);
  }
  /**
   * Check every value. If the value is an object, call the parse function on that object.
   * @param {string} option
   * @param {Object} options
   * @param {Object} referenceOptions
   * @param {array} path    | where to look for the actual option
   * @static
   */
  static check(e, t, r, s) {
    if (r[e] === void 0 && r.__any__ === void 0) {
      Ue.getSuggestion(e, r, s);
      return;
    }
    let a = e, o = !0;
    r[e] === void 0 && r.__any__ !== void 0 && (a = "__any__", o = Ue.getType(t[e]) === "object");
    let l = r[a];
    o && l.__type__ !== void 0 && (l = l.__type__), Ue.checkFields(e, t, r, a, l, s);
  }
  /**
   *
   * @param {string}  option           | the option property
   * @param {Object}  options          | The supplied options object
   * @param {Object}  referenceOptions | The reference options containing all options and their allowed formats
   * @param {string}  referenceOption  | Usually this is the same as option, except when handling an __any__ tag.
   * @param {string}  refOptionObj     | This is the type object from the reference options
   * @param {Array}   path             | where in the object is the option
   * @static
   */
  static checkFields(e, t, r, s, a, o) {
    let l = function(f) {
      console.log("%c" + f + Ue.printLocation(o, e), ya);
    }, c = Ue.getType(t[e]), d = a[c];
    d !== void 0 ? Ue.getType(d) === "array" && Fe(d).call(d, t[e]) === -1 ? (l('Invalid option detected in "' + e + '". Allowed values are:' + Ue.print(d) + ' not "' + t[e] + '". '), Fn = !0) : c === "object" && s !== "__any__" && (o = E.copyAndExtendArray(o, e), Ue.parse(t[e], r[s], o)) : a.any === void 0 && (l('Invalid type received for "' + e + '". Expected: ' + Ue.print(ct(a)) + ". Received [" + c + '] "' + t[e] + '"'), Fn = !0);
  }
  /**
   *
   * @param {Object|boolean|number|string|Array.<number>|Date|Node|Moment|undefined|null} object
   * @returns {string}
   * @static
   */
  static getType(e) {
    var t = typeof e;
    return t === "object" ? e === null ? "null" : e instanceof Boolean ? "boolean" : e instanceof Number ? "number" : e instanceof String ? "string" : ye(e) ? "array" : e instanceof Date ? "date" : e.nodeType !== void 0 ? "dom" : e._isAMomentObject === !0 ? "moment" : "object" : t === "number" ? "number" : t === "boolean" ? "boolean" : t === "string" ? "string" : t === void 0 ? "undefined" : t;
  }
  /**
   * @param {string} option
   * @param {Object} options
   * @param {Array.<string>} path
   * @static
   */
  static getSuggestion(e, t, r) {
    let s = Ue.findInOptions(e, t, r, !1), a = Ue.findInOptions(e, JE, [], !0), o = 8, l = 4, c;
    s.indexMatch !== void 0 ? c = " in " + Ue.printLocation(s.path, e, "") + 'Perhaps it was incomplete? Did you mean: "' + s.indexMatch + `"?

` : a.distance <= l && s.distance > a.distance ? c = " in " + Ue.printLocation(s.path, e, "") + "Perhaps it was misplaced? Matching option found at: " + Ue.printLocation(a.path, a.closestMatch, "") : s.distance <= o ? c = '. Did you mean "' + s.closestMatch + '"?' + Ue.printLocation(s.path, e) : c = ". Did you mean one of these: " + Ue.print(ct(t)) + Ue.printLocation(r, e), console.log('%cUnknown option detected: "' + e + '"' + c, ya), Fn = !0;
  }
  /**
   * traverse the options in search for a match.
   * @param {string} option
   * @param {Object} options
   * @param {Array} path    | where to look for the actual option
   * @param {boolean} [recursive=false]
   * @returns {{closestMatch: string, path: Array, distance: number}}
   * @static
   */
  static findInOptions(e, t, r) {
    let s = arguments.length > 3 && arguments[3] !== void 0 ? arguments[3] : !1, a = 1e9, o = "", l = [], c = e.toLowerCase(), d;
    for (let p in t) {
      if (!Object.prototype.hasOwnProperty.call(t, p)) continue;
      let m;
      if (t[p].__type__ !== void 0 && s === !0) {
        let g = Ue.findInOptions(e, t[p], E.copyAndExtendArray(r, p));
        a > g.distance && (o = g.closestMatch, l = g.path, a = g.distance, d = g.indexMatch);
      } else {
        var f;
        Fe(f = p.toLowerCase()).call(f, c) !== -1 && (d = p), m = Ue.levenshteinDistance(e, p), a > m && (o = p, l = E.copyArray(r), a = m);
      }
    }
    return {
      closestMatch: o,
      path: l,
      distance: a,
      indexMatch: d
    };
  }
  /**
   * @param {Array.<string>} path
   * @param {Object} option
   * @param {string} prefix
   * @returns {String}
   * @static
   */
  static printLocation(e, t) {
    let s = `

` + (arguments.length > 2 && arguments[2] !== void 0 ? arguments[2] : `Problem value found at: 
`) + `options = {
`;
    for (let a = 0; a < e.length; a++) {
      for (let o = 0; o < a + 1; o++)
        s += "  ";
      s += e[a] + `: {
`;
    }
    for (let a = 0; a < e.length + 1; a++)
      s += "  ";
    s += t + `
`;
    for (let a = 0; a < e.length + 1; a++) {
      for (let o = 0; o < e.length - a; o++)
        s += "  ";
      s += `}
`;
    }
    return s + `

`;
  }
  /**
   * @param {Object} options
   * @returns {String}
   * @static
   */
  static print(e) {
    return Yt(e).replace(/(")|(\[)|(\])|(,"__type__")/g, "").replace(/(,)/g, ", ");
  }
  /**
   *  Compute the edit distance between the two given strings
   * http://en.wikibooks.org/wiki/Algorithm_Implementation/Strings/Levenshtein_distance#JavaScript
   *
   * Copyright (c) 2011 Andrei Mackenzie
   *
   * Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
   *
   * The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
   *
   * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
   *
   * @param {string} a
   * @param {string} b
   * @returns {Array.<Array.<number>>}}
   * @static
   */
  static levenshteinDistance(e, t) {
    if (e.length === 0) return t.length;
    if (t.length === 0) return e.length;
    var r = [], s;
    for (s = 0; s <= t.length; s++)
      r[s] = [s];
    var a;
    for (a = 0; a <= e.length; a++)
      r[0][a] = a;
    for (s = 1; s <= t.length; s++)
      for (a = 1; a <= e.length; a++)
        t.charAt(s - 1) == e.charAt(a - 1) ? r[s][a] = r[s - 1][a - 1] : r[s][a] = Math.min(
          r[s - 1][a - 1] + 1,
          // substitution
          Math.min(
            r[s][a - 1] + 1,
            // insertion
            r[s - 1][a] + 1
          )
        );
    return r[t.length][e.length];
  }
}
const me = "string", Ce = "boolean", qe = "number", uG = "array", zi = "date", ze = "object", cG = "dom", Ui = "moment", eD = "any", dG = {
  configure: {
    enabled: {
      boolean: Ce
    },
    filter: {
      boolean: Ce,
      function: "function"
    },
    container: {
      dom: cG
    },
    __type__: {
      object: ze,
      boolean: Ce,
      function: "function"
    }
  },
  //globals :
  alignCurrentTime: {
    string: me,
    undefined: "undefined"
  },
  yAxisOrientation: {
    string: ["left", "right"]
  },
  defaultGroup: {
    string: me
  },
  sort: {
    boolean: Ce
  },
  sampling: {
    boolean: Ce
  },
  stack: {
    boolean: Ce
  },
  graphHeight: {
    string: me,
    number: qe
  },
  shaded: {
    enabled: {
      boolean: Ce
    },
    orientation: {
      string: ["bottom", "top", "zero", "group"]
    },
    // top, bottom, zero, group
    groupId: {
      object: ze
    },
    __type__: {
      boolean: Ce,
      object: ze
    }
  },
  style: {
    string: ["line", "bar", "points"]
  },
  // line, bar
  barChart: {
    width: {
      number: qe
    },
    minWidth: {
      number: qe
    },
    sideBySide: {
      boolean: Ce
    },
    align: {
      string: ["left", "center", "right"]
    },
    __type__: {
      object: ze
    }
  },
  interpolation: {
    enabled: {
      boolean: Ce
    },
    parametrization: {
      string: ["centripetal", "chordal", "uniform"]
    },
    // uniform (alpha = 0.0), chordal (alpha = 1.0), centripetal (alpha = 0.5)
    alpha: {
      number: qe
    },
    __type__: {
      object: ze,
      boolean: Ce
    }
  },
  drawPoints: {
    enabled: {
      boolean: Ce
    },
    onRender: {
      function: "function"
    },
    size: {
      number: qe
    },
    style: {
      string: ["square", "circle"]
    },
    // square, circle
    __type__: {
      object: ze,
      boolean: Ce,
      function: "function"
    }
  },
  dataAxis: {
    showMinorLabels: {
      boolean: Ce
    },
    showMajorLabels: {
      boolean: Ce
    },
    showWeekScale: {
      boolean: Ce
    },
    icons: {
      boolean: Ce
    },
    width: {
      string: me,
      number: qe
    },
    visible: {
      boolean: Ce
    },
    alignZeros: {
      boolean: Ce
    },
    left: {
      range: {
        min: {
          number: qe,
          undefined: "undefined"
        },
        max: {
          number: qe,
          undefined: "undefined"
        },
        __type__: {
          object: ze
        }
      },
      format: {
        function: "function"
      },
      title: {
        text: {
          string: me,
          number: qe,
          undefined: "undefined"
        },
        style: {
          string: me,
          undefined: "undefined"
        },
        __type__: {
          object: ze
        }
      },
      __type__: {
        object: ze
      }
    },
    right: {
      range: {
        min: {
          number: qe,
          undefined: "undefined"
        },
        max: {
          number: qe,
          undefined: "undefined"
        },
        __type__: {
          object: ze
        }
      },
      format: {
        function: "function"
      },
      title: {
        text: {
          string: me,
          number: qe,
          undefined: "undefined"
        },
        style: {
          string: me,
          undefined: "undefined"
        },
        __type__: {
          object: ze
        }
      },
      __type__: {
        object: ze
      }
    },
    __type__: {
      object: ze
    }
  },
  legend: {
    enabled: {
      boolean: Ce
    },
    icons: {
      boolean: Ce
    },
    left: {
      visible: {
        boolean: Ce
      },
      position: {
        string: ["top-right", "bottom-right", "top-left", "bottom-left"]
      },
      __type__: {
        object: ze
      }
    },
    right: {
      visible: {
        boolean: Ce
      },
      position: {
        string: ["top-right", "bottom-right", "top-left", "bottom-left"]
      },
      __type__: {
        object: ze
      }
    },
    __type__: {
      object: ze,
      boolean: Ce
    }
  },
  groups: {
    visibility: {
      any: eD
    },
    __type__: {
      object: ze
    }
  },
  autoResize: {
    boolean: Ce
  },
  throttleRedraw: {
    number: qe
  },
  // TODO: DEPRICATED see https://github.com/almende/vis/issues/2511
  clickToUse: {
    boolean: Ce
  },
  end: {
    number: qe,
    date: zi,
    string: me,
    moment: Ui
  },
  format: {
    minorLabels: {
      millisecond: {
        string: me,
        undefined: "undefined"
      },
      second: {
        string: me,
        undefined: "undefined"
      },
      minute: {
        string: me,
        undefined: "undefined"
      },
      hour: {
        string: me,
        undefined: "undefined"
      },
      weekday: {
        string: me,
        undefined: "undefined"
      },
      day: {
        string: me,
        undefined: "undefined"
      },
      week: {
        string: me,
        undefined: "undefined"
      },
      month: {
        string: me,
        undefined: "undefined"
      },
      quarter: {
        string: me,
        undefined: "undefined"
      },
      year: {
        string: me,
        undefined: "undefined"
      },
      __type__: {
        object: ze
      }
    },
    majorLabels: {
      millisecond: {
        string: me,
        undefined: "undefined"
      },
      second: {
        string: me,
        undefined: "undefined"
      },
      minute: {
        string: me,
        undefined: "undefined"
      },
      hour: {
        string: me,
        undefined: "undefined"
      },
      weekday: {
        string: me,
        undefined: "undefined"
      },
      day: {
        string: me,
        undefined: "undefined"
      },
      week: {
        string: me,
        undefined: "undefined"
      },
      month: {
        string: me,
        undefined: "undefined"
      },
      quarter: {
        string: me,
        undefined: "undefined"
      },
      year: {
        string: me,
        undefined: "undefined"
      },
      __type__: {
        object: ze
      }
    },
    __type__: {
      object: ze
    }
  },
  moment: {
    function: "function"
  },
  height: {
    string: me,
    number: qe
  },
  hiddenDates: {
    start: {
      date: zi,
      number: qe,
      string: me,
      moment: Ui
    },
    end: {
      date: zi,
      number: qe,
      string: me,
      moment: Ui
    },
    repeat: {
      string: me
    },
    __type__: {
      object: ze,
      array: uG
    }
  },
  locale: {
    string: me
  },
  locales: {
    __any__: {
      any: eD
    },
    __type__: {
      object: ze
    }
  },
  max: {
    date: zi,
    number: qe,
    string: me,
    moment: Ui
  },
  maxHeight: {
    number: qe,
    string: me
  },
  maxMinorChars: {
    number: qe
  },
  min: {
    date: zi,
    number: qe,
    string: me,
    moment: Ui
  },
  minHeight: {
    number: qe,
    string: me
  },
  moveable: {
    boolean: Ce
  },
  multiselect: {
    boolean: Ce
  },
  orientation: {
    string: me
  },
  showCurrentTime: {
    boolean: Ce
  },
  showMajorLabels: {
    boolean: Ce
  },
  showMinorLabels: {
    boolean: Ce
  },
  showWeekScale: {
    boolean: Ce
  },
  snap: {
    function: "function",
    null: "null"
  },
  start: {
    date: zi,
    number: qe,
    string: me,
    moment: Ui
  },
  timeAxis: {
    scale: {
      string: me,
      undefined: "undefined"
    },
    step: {
      number: qe,
      undefined: "undefined"
    },
    __type__: {
      object: ze
    }
  },
  width: {
    string: me,
    number: qe
  },
  zoomable: {
    boolean: Ce
  },
  zoomKey: {
    string: ["ctrlKey", "altKey", "metaKey", ""]
  },
  zoomMax: {
    number: qe
  },
  zoomMin: {
    number: qe
  },
  zIndex: {
    number: qe
  },
  __type__: {
    object: ze
  }
}, hG = {
  global: {
    alignCurrentTime: ["none", "year", "month", "quarter", "week", "isoWeek", "day", "date", "hour", "minute", "second"],
    //yAxisOrientation: ['left','right'], // TDOO: enable as soon as Grahp2d doesn't crash when changing this on the fly
    sort: !0,
    sampling: !0,
    stack: !1,
    shaded: {
      enabled: !1,
      orientation: ["zero", "top", "bottom", "group"]
      // zero, top, bottom
    },
    style: ["line", "bar", "points"],
    // line, bar
    barChart: {
      width: [50, 5, 100, 5],
      minWidth: [50, 5, 100, 5],
      sideBySide: !1,
      align: ["left", "center", "right"]
      // left, center, right
    },
    interpolation: {
      enabled: !0,
      parametrization: ["centripetal", "chordal", "uniform"]
      // uniform (alpha = 0.0), chordal (alpha = 1.0), centripetal (alpha = 0.5)
    },
    drawPoints: {
      enabled: !0,
      size: [6, 2, 30, 1],
      style: ["square", "circle"]
      // square, circle
    },
    dataAxis: {
      showMinorLabels: !0,
      showMajorLabels: !0,
      showWeekScale: !1,
      icons: !1,
      width: [40, 0, 200, 1],
      visible: !0,
      alignZeros: !0,
      left: {
        //range: {min:'undefined': 'undefined'ined,max:'undefined': 'undefined'ined},
        //format: function (value) {return value;},
        title: {
          text: "",
          style: ""
        }
      },
      right: {
        //range: {min:'undefined': 'undefined'ined,max:'undefined': 'undefined'ined},
        //format: function (value) {return value;},
        title: {
          text: "",
          style: ""
        }
      }
    },
    legend: {
      enabled: !1,
      icons: !0,
      left: {
        visible: !0,
        position: ["top-right", "bottom-right", "top-left", "bottom-left"]
        // top/bottom - left,right
      },
      right: {
        visible: !0,
        position: ["top-right", "bottom-right", "top-left", "bottom-left"]
        // top/bottom - left,right
      }
    },
    autoResize: !0,
    clickToUse: !1,
    end: "",
    format: {
      minorLabels: {
        millisecond: "SSS",
        second: "s",
        minute: "HH:mm",
        hour: "HH:mm",
        weekday: "ddd D",
        day: "D",
        week: "w",
        month: "MMM",
        quarter: "[Q]Q",
        year: "YYYY"
      },
      majorLabels: {
        millisecond: "HH:mm:ss",
        second: "D MMMM HH:mm",
        minute: "ddd D MMMM",
        hour: "ddd D MMMM",
        weekday: "MMMM YYYY",
        day: "MMMM YYYY",
        week: "MMMM YYYY",
        month: "YYYY",
        quarter: "YYYY",
        year: ""
      }
    },
    height: "",
    locale: "",
    max: "",
    maxHeight: "",
    maxMinorChars: [7, 0, 20, 1],
    min: "",
    minHeight: "",
    moveable: !0,
    orientation: ["both", "bottom", "top"],
    showCurrentTime: !1,
    showMajorLabels: !0,
    showMinorLabels: !0,
    showWeekScale: !1,
    start: "",
    width: "100%",
    zoomable: !0,
    zoomKey: ["ctrlKey", "altKey", "metaKey", ""],
    zoomMax: [31536e10, 10, 31536e10, 1],
    zoomMin: [10, 10, 31536e10, 1],
    zIndex: 0
  }
};
class Ps extends zr {
  /**
   * @param {{dom: Object, domProps: Object, emitter: Emitter}} body
   * @param {Object} [options]    See description at Range.setOptions
   * @constructor Range
   * @extends Component
   */
  constructor(e, t) {
    var r, s, a, o, l, c, d;
    super();
    const f = Et().hours(0).minutes(0).seconds(0).milliseconds(0), p = f.clone().add(-3, "days").valueOf(), m = f.clone().add(3, "days").valueOf();
    this.millisecondsPerPixelCache = void 0, t === void 0 ? (this.start = p, this.end = m) : (this.start = t.start || p, this.end = t.end || m), this.rolling = !1, this.body = e, this.deltaDifference = 0, this.scaleOffset = 0, this.startToFront = !1, this.endToFront = !0, this.defaultOptions = {
      rtl: !1,
      start: null,
      end: null,
      moment: Et,
      direction: "horizontal",
      // 'horizontal' or 'vertical'
      moveable: !0,
      zoomable: !0,
      min: null,
      max: null,
      zoomMin: 10,
      // milliseconds
      zoomMax: 1e3 * 60 * 60 * 24 * 365 * 1e4,
      // milliseconds
      rollingMode: {
        follow: !1,
        offset: 0.5
      }
    }, this.options = E.extend({}, this.defaultOptions), this.props = {
      touch: {}
    }, this.animationTimer = null, this.body.emitter.on("panstart", N(r = this._onDragStart).call(r, this)), this.body.emitter.on("panmove", N(s = this._onDrag).call(s, this)), this.body.emitter.on("panend", N(a = this._onDragEnd).call(a, this)), this.body.emitter.on("mousewheel", N(o = this._onMouseWheel).call(o, this)), this.body.emitter.on("touch", N(l = this._onTouch).call(l, this)), this.body.emitter.on("pinch", N(c = this._onPinch).call(c, this)), this.body.dom.rollingModeBtn.addEventListener("click", N(d = this.startRolling).call(d, this)), this.setOptions(t);
  }
  /**
   * Set options for the range controller
   * @param {Object} options      Available options:
   *                              {number | Date | String} start  Start date for the range
   *                              {number | Date | String} end    End date for the range
   *                              {number} min    Minimum value for start
   *                              {number} max    Maximum value for end
   *                              {number} zoomMin    Set a minimum value for
   *                                                  (end - start).
   *                              {number} zoomMax    Set a maximum value for
   *                                                  (end - start).
   *                              {boolean} moveable Enable moving of the range
   *                                                 by dragging. True by default
   *                              {boolean} zoomable Enable zooming of the range
   *                                                 by pinching/scrolling. True by default
   */
  setOptions(e) {
    if (e) {
      const t = ["animation", "direction", "min", "max", "zoomMin", "zoomMax", "moveable", "zoomable", "moment", "activate", "hiddenDates", "zoomKey", "zoomFriction", "rtl", "showCurrentTime", "rollingMode", "horizontalScroll", "horizontalScrollKey", "horizontalScrollInvert", "verticalScroll"];
      E.selectiveExtend(t, this.options, e), e.rollingMode && e.rollingMode.follow && this.startRolling(), ("start" in e || "end" in e) && this.setRange(e.start, e.end);
    }
  }
  /**
   * Start auto refreshing the current time bar
   */
  startRolling() {
    const e = this;
    function t() {
      e.stopRolling(), e.rolling = !0;
      let r = e.end - e.start;
      const s = E.convert(/* @__PURE__ */ new Date(), "Date").valueOf(), a = e.options.rollingMode && e.options.rollingMode.offset || 0.5, o = s - r * a, l = s + r * (1 - a), c = {
        animation: !1
      };
      e.setRange(o, l, c), r = 1 / e.conversion(e.body.domProps.center.width).scale / 10, r < 30 && (r = 30), r > 1e3 && (r = 1e3), e.body.dom.rollingModeBtn.style.visibility = "hidden", e.currentTimeTimer = Dt(t, r);
    }
    t();
  }
  /**
   * Stop auto refreshing the current time bar
   */
  stopRolling() {
    this.currentTimeTimer !== void 0 && (clearTimeout(this.currentTimeTimer), this.rolling = !1, this.body.dom.rollingModeBtn.style.visibility = "visible");
  }
  /**
   * Set a new start and end range
   * @param {Date | number | string} start
   * @param {Date | number | string} end
   * @param {Object} options      Available options:
   *                              {boolean | {duration: number, easingFunction: string}} [animation=false]
   *                                    If true, the range is animated
   *                                    smoothly to the new window. An object can be
   *                                    provided to specify duration and easing function.
   *                                    Default duration is 500 ms, and default easing
   *                                    function is 'easeInOutQuad'.
   *                              {boolean} [byUser=false]
   *                              {Event}  event  Mouse event
   * @param {Function} callback     a callback function to be executed at the end of this function
   * @param {Function} frameCallback    a callback function executed each frame of the range animation.
   *                                    The callback will be passed three parameters:
   *                                    {number} easeCoefficient    an easing coefficent
   *                                    {boolean} willDraw          If true the caller will redraw after the callback completes
   *                                    {boolean} done              If true then animation is ending after the current frame
   * @return {void}
   */
  setRange(e, t, r, s, a) {
    r || (r = {}), r.byUser !== !0 && (r.byUser = !1);
    const o = this, l = e != null ? E.convert(e, "Date").valueOf() : null, c = t != null ? E.convert(t, "Date").valueOf() : null;
    if (this._cancelAnimation(), this.millisecondsPerPixelCache = void 0, r.animation) {
      const p = this.start, m = this.end, g = typeof r.animation == "object" && "duration" in r.animation ? r.animation.duration : 500, b = typeof r.animation == "object" && "easingFunction" in r.animation ? r.animation.easingFunction : "easeInOutQuad", _ = E.easingFunctions[b];
      if (!_) {
        var d;
        throw new Error(rt(d = "Unknown easing function ".concat(Yt(b), ". Choose from: ")).call(d, ct(E.easingFunctions).join(", ")));
      }
      const x = Ut();
      let T = !1;
      const S = () => {
        if (!o.props.touch.dragging) {
          const D = Ut() - x, I = _(D / g), C = D > g, P = C || l === null ? l : p + (l - p) * I, R = C || c === null ? c : m + (c - m) * I;
          f = o._applyRange(P, R), rs(o.options.moment, o.body, o.options.hiddenDates), T = T || f;
          const M = {
            start: new Date(o.start),
            end: new Date(o.end),
            byUser: r.byUser,
            event: r.event
          };
          if (a && a(I, f, C), f && o.body.emitter.emit("rangechange", M), C) {
            if (T && (o.body.emitter.emit("rangechanged", M), s))
              return s();
          } else
            o.animationTimer = Dt(S, 20);
        }
      };
      return S();
    } else {
      var f = this._applyRange(l, c);
      if (rs(this.options.moment, this.body, this.options.hiddenDates), f) {
        const p = {
          start: new Date(this.start),
          end: new Date(this.end),
          byUser: r.byUser,
          event: r.event
        };
        if (this.body.emitter.emit("rangechange", p), clearTimeout(o.timeoutID), o.timeoutID = Dt(() => {
          o.body.emitter.emit("rangechanged", p);
        }, 200), s)
          return s();
      }
    }
  }
  /**
   * Get the number of milliseconds per pixel.
   *
   * @returns {undefined|number}
   */
  getMillisecondsPerPixel() {
    return this.millisecondsPerPixelCache === void 0 && (this.millisecondsPerPixelCache = (this.end - this.start) / this.body.dom.center.clientWidth), this.millisecondsPerPixelCache;
  }
  /**
   * Stop an animation
   * @private
   */
  _cancelAnimation() {
    this.animationTimer && (clearTimeout(this.animationTimer), this.animationTimer = null);
  }
  /**
   * Set a new start and end range. This method is the same as setRange, but
   * does not trigger a range change and range changed event, and it returns
   * true when the range is changed
   * @param {number} [start]
   * @param {number} [end]
   * @return {boolean} changed
   * @private
   */
  _applyRange(e, t) {
    let r = e != null ? E.convert(e, "Date").valueOf() : this.start, s = t != null ? E.convert(t, "Date").valueOf() : this.end;
    const a = this.options.max != null ? E.convert(this.options.max, "Date").valueOf() : null, o = this.options.min != null ? E.convert(this.options.min, "Date").valueOf() : null;
    let l;
    if (isNaN(r) || r === null)
      throw new Error('Invalid start "'.concat(e, '"'));
    if (isNaN(s) || s === null)
      throw new Error('Invalid end "'.concat(t, '"'));
    if (s < r && (s = r), o !== null && r < o && (l = o - r, r += l, s += l, a != null && s > a && (s = a)), a !== null && s > a && (l = s - a, r -= l, s -= l, o != null && r < o && (r = o)), this.options.zoomMin !== null) {
      let d = va(this.options.zoomMin);
      d < 0 && (d = 0), s - r < d && (this.end - this.start === d && r >= this.start - 0.5 && s <= this.end ? (r = this.start, s = this.end) : (l = d - (s - r), r -= l / 2, s += l / 2));
    }
    if (this.options.zoomMax !== null) {
      let d = va(this.options.zoomMax);
      d < 0 && (d = 0), s - r > d && (this.end - this.start === d && r < this.start && s > this.end ? (r = this.start, s = this.end) : (l = s - r - d, r += l / 2, s -= l / 2));
    }
    const c = this.start != r || this.end != s;
    return !(r >= this.start && r <= this.end || s >= this.start && s <= this.end) && !(this.start >= r && this.start <= s || this.end >= r && this.end <= s) && this.body.emitter.emit("checkRangedItems"), this.start = r, this.end = s, c;
  }
  /**
   * Retrieve the current range.
   * @return {Object} An object with start and end properties
   */
  getRange() {
    return {
      start: this.start,
      end: this.end
    };
  }
  /**
   * Calculate the conversion offset and scale for current range, based on
   * the provided width
   * @param {number} width
   * @param {number} [totalHidden=0]
   * @returns {{offset: number, scale: number}} conversion
   */
  conversion(e, t) {
    return Ps.conversion(this.start, this.end, e, t);
  }
  /**
   * Static method to calculate the conversion offset and scale for a range,
   * based on the provided start, end, and width
   * @param {number} start
   * @param {number} end
   * @param {number} width
   * @param {number} [totalHidden=0]
   * @returns {{offset: number, scale: number}} conversion
   */
  static conversion(e, t, r, s) {
    return s === void 0 && (s = 0), r != 0 && t - e != 0 ? {
      offset: e,
      scale: r / (t - e - s)
    } : {
      offset: 0,
      scale: 1
    };
  }
  /**
   * Start dragging horizontally or vertically
   * @param {Event} event
   * @private
   */
  _onDragStart(e) {
    this.deltaDifference = 0, this.previousDelta = 0, this.options.moveable && this._isInsideRange(e) && this.props.touch.allowDragging && (this.stopRolling(), this.props.touch.start = this.start, this.props.touch.end = this.end, this.props.touch.dragging = !0, this.body.dom.root && (this.body.dom.root.style.cursor = "move"));
  }
  /**
   * Perform dragging operation
   * @param {Event} event
   * @private
   */
  _onDrag(e) {
    if (!e || !this.props.touch.dragging || !this.options.moveable || !this.props.touch.allowDragging) return;
    const t = this.options.direction;
    tD(t);
    let r = t == "horizontal" ? e.deltaX : e.deltaY;
    r -= this.deltaDifference;
    let s = this.props.touch.end - this.props.touch.start;
    const a = sn(this.body.hiddenDates, this.start, this.end);
    s -= a;
    const o = t == "horizontal" ? this.body.domProps.center.width : this.body.domProps.center.height;
    let l;
    this.options.rtl ? l = r / o * s : l = -r / o * s;
    const c = this.props.touch.start + l, d = this.props.touch.end + l, f = Qr(this.body.hiddenDates, c, this.previousDelta - r, !0), p = Qr(this.body.hiddenDates, d, this.previousDelta - r, !0);
    if (f != c || p != d) {
      this.deltaDifference += r, this.props.touch.start = f, this.props.touch.end = p, this._onDrag(e);
      return;
    }
    this.previousDelta = r, this._applyRange(c, d);
    const m = new Date(this.start), g = new Date(this.end);
    this.body.emitter.emit("rangechange", {
      start: m,
      end: g,
      byUser: !0,
      event: e
    }), this.body.emitter.emit("panmove");
  }
  /**
   * Stop dragging operation
   * @param {event} event
   * @private
   */
  _onDragEnd(e) {
    this.props.touch.dragging && this.options.moveable && this.props.touch.allowDragging && (this.props.touch.dragging = !1, this.body.dom.root && (this.body.dom.root.style.cursor = "auto"), this.body.emitter.emit("rangechanged", {
      start: new Date(this.start),
      end: new Date(this.end),
      byUser: !0,
      event: e
    }));
  }
  /**
   * Event handler for mouse wheel event, used to zoom
   * Code from http://adomas.org/javascript-mouse-wheel/
   * @param {Event} event
   * @private
   */
  _onMouseWheel(e) {
    let t = 0;
    if (e.wheelDelta ? t = e.wheelDelta / 120 : e.detail ? t = -e.detail / 3 : e.deltaY && (t = -e.deltaY / 3), !(this.options.zoomKey && !e[this.options.zoomKey] && this.options.zoomable || !this.options.zoomable && this.options.moveable) && this.options.zoomable && this.options.moveable && this._isInsideRange(e) && t) {
      const r = this.options.zoomFriction || 5;
      let s;
      t < 0 ? s = 1 - t / r : s = 1 / (1 + t / r);
      let a;
      if (this.rolling) {
        const o = this.options.rollingMode && this.options.rollingMode.offset || 0.5;
        a = this.start + (this.end - this.start) * o;
      } else {
        const o = this.getPointer({
          x: e.clientX,
          y: e.clientY
        }, this.body.dom.center);
        a = this._pointerToDate(o);
      }
      this.zoom(s, a, t, e), e.preventDefault();
    }
  }
  /**
   * Start of a touch gesture
   * @param {Event} event
   * @private
   */
  _onTouch(e) {
    this.props.touch.start = this.start, this.props.touch.end = this.end, this.props.touch.allowDragging = !0, this.props.touch.center = null, this.props.touch.centerDate = null, this.scaleOffset = 0, this.deltaDifference = 0, E.preventDefault(e);
  }
  /**
   * Handle pinch event
   * @param {Event} event
   * @private
   */
  _onPinch(e) {
    if (!(this.options.zoomable && this.options.moveable)) return;
    E.preventDefault(e), this.props.touch.allowDragging = !1, this.props.touch.center || (this.props.touch.center = this.getPointer(e.center, this.body.dom.center), this.props.touch.centerDate = this._pointerToDate(this.props.touch.center)), this.stopRolling();
    const t = 1 / (e.scale + this.scaleOffset), r = this.props.touch.centerDate, s = sn(this.body.hiddenDates, this.start, this.end), a = is(this.options.moment, this.body.hiddenDates, this, r), o = s - a;
    let l = r - a + (this.props.touch.start - (r - a)) * t, c = r + o + (this.props.touch.end - (r + o)) * t;
    this.startToFront = 1 - t <= 0, this.endToFront = t - 1 <= 0;
    const d = Qr(this.body.hiddenDates, l, 1 - t, !0), f = Qr(this.body.hiddenDates, c, t - 1, !0);
    (d != l || f != c) && (this.props.touch.start = d, this.props.touch.end = f, this.scaleOffset = 1 - e.scale, l = d, c = f);
    const p = {
      animation: !1,
      byUser: !0,
      event: e
    };
    this.setRange(l, c, p), this.startToFront = !1, this.endToFront = !0;
  }
  /**
   * Test whether the mouse from a mouse event is inside the visible window,
   * between the current start and end date
   * @param {Object} event
   * @return {boolean} Returns true when inside the visible window
   * @private
   */
  _isInsideRange(e) {
    const t = e.center ? e.center.x : e.clientX, r = this.body.dom.centerContainer.getBoundingClientRect(), s = this.options.rtl ? t - r.left : r.right - t, a = this.body.util.toTime(s);
    return a >= this.start && a <= this.end;
  }
  /**
   * Helper function to calculate the center date for zooming
   * @param {{x: number, y: number}} pointer
   * @return {number} date
   * @private
   */
  _pointerToDate(e) {
    let t;
    const r = this.options.direction;
    if (tD(r), r == "horizontal")
      return this.body.util.toTime(e.x).valueOf();
    {
      const s = this.body.domProps.center.height;
      return t = this.conversion(s), e.y / t.scale + t.offset;
    }
  }
  /**
   * Get the pointer location relative to the location of the dom element
   * @param {{x: number, y: number}} touch
   * @param {Element} element   HTML DOM element
   * @return {{x: number, y: number}} pointer
   * @private
   */
  getPointer(e, t) {
    const r = t.getBoundingClientRect();
    return this.options.rtl ? {
      x: r.right - e.x,
      y: e.y - r.top
    } : {
      x: e.x - r.left,
      y: e.y - r.top
    };
  }
  /**
   * Zoom the range the given scale in or out. Start and end date will
   * be adjusted, and the timeline will be redrawn. You can optionally give a
   * date around which to zoom.
   * For example, try scale = 0.9 or 1.1
   * @param {number} scale      Scaling factor. Values above 1 will zoom out,
   *                            values below 1 will zoom in.
   * @param {number} [center]   Value representing a date around which will
   *                            be zoomed.
   * @param {number} delta
   * @param {Event} event
   */
  zoom(e, t, r, s) {
    t == null && (t = (this.start + this.end) / 2);
    const a = sn(this.body.hiddenDates, this.start, this.end), o = is(this.options.moment, this.body.hiddenDates, this, t), l = a - o;
    let c = t - o + (this.start - (t - o)) * e, d = t + l + (this.end - (t + l)) * e;
    this.startToFront = !(r > 0), this.endToFront = !(-r > 0);
    const f = Qr(this.body.hiddenDates, c, r, !0), p = Qr(this.body.hiddenDates, d, -r, !0);
    (f != c || p != d) && (c = f, d = p);
    const m = {
      animation: !1,
      byUser: !0,
      event: s
    };
    this.setRange(c, d, m), this.startToFront = !1, this.endToFront = !0;
  }
  /**
   * Move the range with a given delta to the left or right. Start and end
   * value will be adjusted. For example, try delta = 0.1 or -0.1
   * @param {number}  delta     Moving amount. Positive value will move right,
   *                            negative value will move left
   */
  move(e) {
    const t = this.end - this.start, r = this.start + t * e, s = this.end + t * e;
    this.start = r, this.end = s;
  }
  /**
   * Move the range to a new center point
   * @param {number} moveTo      New center point of the range
   */
  moveTo(e) {
    const r = (this.start + this.end) / 2 - e, s = this.start - r, a = this.end - r, o = {
      animation: !1,
      byUser: !0,
      event: null
    };
    this.setRange(s, a, o);
  }
  /**
   * Destroy the Range
   */
  destroy() {
    this.stopRolling();
  }
}
function tD(n) {
  if (n != "horizontal" && n != "vertical")
    throw new TypeError('Unknown direction "'.concat(n, '". Choose "horizontal" or "vertical".'));
}
function Tr(n, e, t, r) {
  var s, a, o, l, c, d, f;
  if (!(ye(t) || Hr(t)) && t instanceof Object) {
    var p = r;
    r = t, t = p;
  }
  r && r.throttleRedraw && console.warn('Graph2d option "throttleRedraw" is DEPRICATED and no longer supported. It will be removed in the next MAJOR release.');
  var m = this;
  this.defaultOptions = {
    start: null,
    end: null,
    autoResize: !0,
    orientation: {
      axis: "bottom",
      // axis orientation: 'bottom', 'top', or 'both'
      item: "bottom"
      // not relevant for Graph2d
    },
    moment: Et,
    width: null,
    height: null,
    maxHeight: null,
    minHeight: null
  }, this.options = E.deepExtend({}, this.defaultOptions), this._create(n), this.components = [], this.body = {
    dom: this.dom,
    domProps: this.props,
    emitter: {
      on: N(s = this.on).call(s, this),
      off: N(a = this.off).call(a, this),
      emit: N(o = this.emit).call(o, this)
    },
    hiddenDates: [],
    util: {
      getScale() {
        return m.timeAxis.step.scale;
      },
      getStep() {
        return m.timeAxis.step.step;
      },
      toScreen: N(l = m._toScreen).call(l, m),
      toGlobalScreen: N(c = m._toGlobalScreen).call(c, m),
      // this refers to the root.width
      toTime: N(d = m._toTime).call(d, m),
      toGlobalTime: N(f = m._toGlobalTime).call(f, m)
    }
  }, this.range = new Ps(this.body), this.components.push(this.range), this.body.range = this.range, this.timeAxis = new ss(this.body), this.components.push(this.timeAxis), this.currentTime = new Um(this.body), this.components.push(this.currentTime), this.linegraph = new Ne(this.body), this.components.push(this.linegraph), this.itemsData = null, this.groupsData = null, this.on("tap", function(g) {
    m.emit("click", m.getEventProperties(g));
  }), this.on("doubletap", function(g) {
    m.emit("doubleClick", m.getEventProperties(g));
  }), this.dom.root.oncontextmenu = function(g) {
    m.emit("contextmenu", m.getEventProperties(g));
  }, this.initialFitDone = !1, this.on("changed", function() {
    if (m.itemsData != null) {
      if (!m.initialFitDone && !m.options.rollingMode)
        if (m.initialFitDone = !0, m.options.start != null || m.options.end != null) {
          if (m.options.start == null || m.options.end == null)
            var g = m.getItemRange();
          var b = m.options.start != null ? m.options.start : g.min, _ = m.options.end != null ? m.options.end : g.max;
          m.setWindow(b, _, {
            animation: !1
          });
        } else
          m.fit({
            animation: !1
          });
      !m.initialDrawDone && (m.initialRangeChangeDone || !m.options.start && !m.options.end || m.options.rollingMode) && (m.initialDrawDone = !0, m.dom.root.style.visibility = "visible", m.dom.loadingScreen.parentNode.removeChild(m.dom.loadingScreen), m.options.onInitialDrawComplete && Dt(() => m.options.onInitialDrawComplete(), 0));
    }
  }), r && this.setOptions(r), t && this.setGroups(t), e && this.setItems(e), this._redraw();
}
Tr.prototype = new xi();
Tr.prototype.setOptions = function(n) {
  Ue.validate(n, dG) === !0 && console.log("%cErrors have been found in the supplied options object.", ya), xi.prototype.setOptions.call(this, n);
};
Tr.prototype.setItems = function(n) {
  var e = this.itemsData == null, t;
  if (n ? Hr(n) ? t = cn(n) : t = cn(new Oi(n)) : t = null, this.itemsData && this.itemsData.dispose(), this.itemsData = t, this.linegraph && this.linegraph.setItems(t != null ? t.rawDS : null), e)
    if (this.options.start != null || this.options.end != null) {
      var r = this.options.start != null ? this.options.start : null, s = this.options.end != null ? this.options.end : null;
      this.setWindow(r, s, {
        animation: !1
      });
    } else
      this.fit({
        animation: !1
      });
};
Tr.prototype.setGroups = function(n) {
  var e;
  n ? Hr(n) ? e = n : e = new Oi(n) : e = null, this.groupsData = e, this.linegraph.setGroups(e);
};
Tr.prototype.getLegend = function(n, e, t) {
  return e === void 0 && (e = 15), t === void 0 && (t = 15), this.linegraph.groups[n] !== void 0 ? this.linegraph.groups[n].getLegend(e, t) : "cannot find group:'" + n + "'";
};
Tr.prototype.isGroupVisible = function(n) {
  return this.linegraph.groups[n] !== void 0 ? this.linegraph.groups[n].visible && (this.linegraph.options.groups.visibility[n] === void 0 || this.linegraph.options.groups.visibility[n] == !0) : !1;
};
Tr.prototype.getDataRange = function() {
  var n = null, e = null;
  for (var t in this.linegraph.groups)
    if (!(!Object.prototype.hasOwnProperty.call(this.linegraph.groups, t) || this.linegraph.groups[t].visible !== !0))
      for (var r = 0; r < this.linegraph.groups[t].itemsData.length; r++) {
        var s = this.linegraph.groups[t].itemsData[r], a = E.convert(s.x, "Date").valueOf();
        n = n == null || n > a ? a : n, e = e == null || e < a ? a : e;
      }
  return {
    min: n != null ? new Date(n) : null,
    max: e != null ? new Date(e) : null
  };
};
Tr.prototype.getEventProperties = function(n) {
  var e = n.center ? n.center.x : n.clientX, t = n.center ? n.center.y : n.clientY, r = e - E.getAbsoluteLeft(this.dom.centerContainer), s = t - E.getAbsoluteTop(this.dom.centerContainer), a = this._toTime(r), o = Ha.customTimeFromTarget(n), l = E.getTarget(n), c = null;
  E.hasParent(l, this.timeAxis.dom.foreground) || this.timeAxis2 && E.hasParent(l, this.timeAxis2.dom.foreground) ? c = "axis" : E.hasParent(l, this.linegraph.yAxisLeft.dom.frame) || E.hasParent(l, this.linegraph.yAxisRight.dom.frame) ? c = "data-axis" : E.hasParent(l, this.linegraph.legendLeft.dom.frame) || E.hasParent(l, this.linegraph.legendRight.dom.frame) ? c = "legend" : o != null ? c = "custom-time" : E.hasParent(l, this.currentTime.bar) ? c = "current-time" : E.hasParent(l, this.dom.center) && (c = "background");
  var d = [], f = this.linegraph.yAxisLeft, p = this.linegraph.yAxisRight;
  return !f.hidden && this.itemsData.length > 0 && d.push(f.screenToValue(s)), !p.hidden && this.itemsData.length > 0 && d.push(p.screenToValue(s)), {
    event: n,
    customTime: o ? o.options.id : null,
    what: c,
    pageX: n.srcEvent ? n.srcEvent.pageX : n.pageX,
    pageY: n.srcEvent ? n.srcEvent.pageY : n.pageY,
    x: r,
    y: s,
    time: a,
    value: d
  };
};
Tr.prototype._createConfigurator = function() {
  return new fk(this, this.dom.container, hG);
};
const pe = "string", he = "boolean", Ye = "number", rD = "array", Vi = "date", tt = "object", fG = "dom", Ki = "moment", iD = "any", pG = {
  configure: {
    enabled: {
      boolean: he
    },
    filter: {
      boolean: he,
      function: "function"
    },
    container: {
      dom: fG
    },
    __type__: {
      object: tt,
      boolean: he,
      function: "function"
    }
  },
  //globals :
  align: {
    string: pe
  },
  alignCurrentTime: {
    string: pe,
    undefined: "undefined"
  },
  rtl: {
    boolean: he,
    undefined: "undefined"
  },
  rollingMode: {
    follow: {
      boolean: he
    },
    offset: {
      number: Ye,
      undefined: "undefined"
    },
    __type__: {
      object: tt
    }
  },
  onTimeout: {
    timeoutMs: {
      number: Ye
    },
    callback: {
      function: "function"
    },
    __type__: {
      object: tt
    }
  },
  verticalScroll: {
    boolean: he,
    undefined: "undefined"
  },
  horizontalScroll: {
    boolean: he,
    undefined: "undefined"
  },
  horizontalScrollKey: {
    string: pe,
    undefined: "undefined"
  },
  horizontalScrollInvert: {
    boolean: he,
    undefined: "undefined"
  },
  autoResize: {
    boolean: he
  },
  throttleRedraw: {
    number: Ye
  },
  // TODO: DEPRICATED see https://github.com/almende/vis/issues/2511
  clickToUse: {
    boolean: he
  },
  dataAttributes: {
    string: pe,
    array: rD
  },
  editable: {
    add: {
      boolean: he,
      undefined: "undefined"
    },
    remove: {
      boolean: he,
      undefined: "undefined"
    },
    updateGroup: {
      boolean: he,
      undefined: "undefined"
    },
    updateTime: {
      boolean: he,
      undefined: "undefined"
    },
    overrideItems: {
      boolean: he,
      undefined: "undefined"
    },
    __type__: {
      boolean: he,
      object: tt
    }
  },
  end: {
    number: Ye,
    date: Vi,
    string: pe,
    moment: Ki
  },
  format: {
    minorLabels: {
      millisecond: {
        string: pe,
        undefined: "undefined"
      },
      second: {
        string: pe,
        undefined: "undefined"
      },
      minute: {
        string: pe,
        undefined: "undefined"
      },
      hour: {
        string: pe,
        undefined: "undefined"
      },
      weekday: {
        string: pe,
        undefined: "undefined"
      },
      day: {
        string: pe,
        undefined: "undefined"
      },
      week: {
        string: pe,
        undefined: "undefined"
      },
      month: {
        string: pe,
        undefined: "undefined"
      },
      year: {
        string: pe,
        undefined: "undefined"
      },
      __type__: {
        object: tt,
        function: "function"
      }
    },
    majorLabels: {
      millisecond: {
        string: pe,
        undefined: "undefined"
      },
      second: {
        string: pe,
        undefined: "undefined"
      },
      minute: {
        string: pe,
        undefined: "undefined"
      },
      hour: {
        string: pe,
        undefined: "undefined"
      },
      weekday: {
        string: pe,
        undefined: "undefined"
      },
      day: {
        string: pe,
        undefined: "undefined"
      },
      week: {
        string: pe,
        undefined: "undefined"
      },
      month: {
        string: pe,
        undefined: "undefined"
      },
      year: {
        string: pe,
        undefined: "undefined"
      },
      __type__: {
        object: tt,
        function: "function"
      }
    },
    __type__: {
      object: tt
    }
  },
  moment: {
    function: "function"
  },
  groupHeightMode: {
    string: pe
  },
  groupOrder: {
    string: pe,
    function: "function"
  },
  groupEditable: {
    add: {
      boolean: he,
      undefined: "undefined"
    },
    remove: {
      boolean: he,
      undefined: "undefined"
    },
    order: {
      boolean: he,
      undefined: "undefined"
    },
    __type__: {
      boolean: he,
      object: tt
    }
  },
  groupOrderSwap: {
    function: "function"
  },
  height: {
    string: pe,
    number: Ye
  },
  hiddenDates: {
    start: {
      date: Vi,
      number: Ye,
      string: pe,
      moment: Ki
    },
    end: {
      date: Vi,
      number: Ye,
      string: pe,
      moment: Ki
    },
    repeat: {
      string: pe
    },
    __type__: {
      object: tt,
      array: rD
    }
  },
  itemsAlwaysDraggable: {
    item: {
      boolean: he,
      undefined: "undefined"
    },
    range: {
      boolean: he,
      undefined: "undefined"
    },
    __type__: {
      boolean: he,
      object: tt
    }
  },
  limitSize: {
    boolean: he
  },
  locale: {
    string: pe
  },
  locales: {
    __any__: {
      any: iD
    },
    __type__: {
      object: tt
    }
  },
  longSelectPressTime: {
    number: Ye
  },
  margin: {
    axis: {
      number: Ye
    },
    item: {
      horizontal: {
        number: Ye,
        undefined: "undefined"
      },
      vertical: {
        number: Ye,
        undefined: "undefined"
      },
      __type__: {
        object: tt,
        number: Ye
      }
    },
    __type__: {
      object: tt,
      number: Ye
    }
  },
  max: {
    date: Vi,
    number: Ye,
    string: pe,
    moment: Ki
  },
  maxHeight: {
    number: Ye,
    string: pe
  },
  maxMinorChars: {
    number: Ye
  },
  min: {
    date: Vi,
    number: Ye,
    string: pe,
    moment: Ki
  },
  minHeight: {
    number: Ye,
    string: pe
  },
  moveable: {
    boolean: he
  },
  multiselect: {
    boolean: he
  },
  multiselectPerGroup: {
    boolean: he
  },
  onAdd: {
    function: "function"
  },
  onDropObjectOnItem: {
    function: "function"
  },
  onUpdate: {
    function: "function"
  },
  onMove: {
    function: "function"
  },
  onMoving: {
    function: "function"
  },
  onRemove: {
    function: "function"
  },
  onAddGroup: {
    function: "function"
  },
  onMoveGroup: {
    function: "function"
  },
  onRemoveGroup: {
    function: "function"
  },
  onInitialDrawComplete: {
    function: "function"
  },
  order: {
    function: "function"
  },
  orientation: {
    axis: {
      string: pe,
      undefined: "undefined"
    },
    item: {
      string: pe,
      undefined: "undefined"
    },
    __type__: {
      string: pe,
      object: tt
    }
  },
  selectable: {
    boolean: he
  },
  sequentialSelection: {
    boolean: he
  },
  showCurrentTime: {
    boolean: he
  },
  showMajorLabels: {
    boolean: he
  },
  showMinorLabels: {
    boolean: he
  },
  showWeekScale: {
    boolean: he
  },
  stack: {
    boolean: he
  },
  stackSubgroups: {
    boolean: he
  },
  cluster: {
    maxItems: {
      number: Ye,
      undefined: "undefined"
    },
    titleTemplate: {
      string: pe,
      undefined: "undefined"
    },
    clusterCriteria: {
      function: "function",
      undefined: "undefined"
    },
    showStipes: {
      boolean: he,
      undefined: "undefined"
    },
    fitOnDoubleClick: {
      boolean: he,
      undefined: "undefined"
    },
    __type__: {
      boolean: he,
      object: tt
    }
  },
  snap: {
    function: "function",
    null: "null"
  },
  start: {
    date: Vi,
    number: Ye,
    string: pe,
    moment: Ki
  },
  template: {
    function: "function"
  },
  loadingScreenTemplate: {
    function: "function"
  },
  groupTemplate: {
    function: "function"
  },
  visibleFrameTemplate: {
    string: pe,
    function: "function"
  },
  showTooltips: {
    boolean: he
  },
  tooltip: {
    followMouse: {
      boolean: he
    },
    overflowMethod: {
      string: ["cap", "flip", "none"]
    },
    delay: {
      number: Ye
    },
    template: {
      function: "function"
    },
    __type__: {
      object: tt
    }
  },
  tooltipOnItemUpdateTime: {
    template: {
      function: "function"
    },
    __type__: {
      boolean: he,
      object: tt
    }
  },
  timeAxis: {
    scale: {
      string: pe,
      undefined: "undefined"
    },
    step: {
      number: Ye,
      undefined: "undefined"
    },
    __type__: {
      object: tt
    }
  },
  type: {
    string: pe
  },
  width: {
    string: pe,
    number: Ye
  },
  preferZoom: {
    boolean: he
  },
  zoomable: {
    boolean: he
  },
  zoomKey: {
    string: ["ctrlKey", "altKey", "shiftKey", "metaKey", ""]
  },
  zoomFriction: {
    number: Ye
  },
  zoomMax: {
    number: Ye
  },
  zoomMin: {
    number: Ye
  },
  xss: {
    disabled: {
      boolean: he
    },
    filterOptions: {
      __any__: {
        any: iD
      },
      __type__: {
        object: tt
      }
    },
    __type__: {
      object: tt
    }
  },
  __type__: {
    object: tt
  }
}, mG = {
  global: {
    align: ["center", "left", "right"],
    alignCurrentTime: ["none", "year", "month", "quarter", "week", "isoWeek", "day", "date", "hour", "minute", "second"],
    direction: !1,
    autoResize: !0,
    clickToUse: !1,
    // dataAttributes: ['all'], // FIXME: can be 'all' or string[]
    editable: {
      add: !1,
      remove: !1,
      updateGroup: !1,
      updateTime: !1
    },
    end: "",
    format: {
      minorLabels: {
        millisecond: "SSS",
        second: "s",
        minute: "HH:mm",
        hour: "HH:mm",
        weekday: "ddd D",
        day: "D",
        week: "w",
        month: "MMM",
        year: "YYYY"
      },
      majorLabels: {
        millisecond: "HH:mm:ss",
        second: "D MMMM HH:mm",
        minute: "ddd D MMMM",
        hour: "ddd D MMMM",
        weekday: "MMMM YYYY",
        day: "MMMM YYYY",
        week: "MMMM YYYY",
        month: "YYYY",
        year: ""
      }
    },
    groupHeightMode: ["auto", "fixed", "fitItems"],
    //groupOrder: {string, 'function': 'function'},
    groupsDraggable: !1,
    height: "",
    //hiddenDates: {object, array},
    locale: "",
    longSelectPressTime: 251,
    margin: {
      axis: [20, 0, 100, 1],
      item: {
        horizontal: [10, 0, 100, 1],
        vertical: [10, 0, 100, 1]
      }
    },
    max: "",
    maxHeight: "",
    maxMinorChars: [7, 0, 20, 1],
    min: "",
    minHeight: "",
    moveable: !1,
    multiselect: !1,
    multiselectPerGroup: !1,
    //onAdd: {'function': 'function'},
    //onUpdate: {'function': 'function'},
    //onMove: {'function': 'function'},
    //onMoving: {'function': 'function'},
    //onRename: {'function': 'function'},
    //order: {'function': 'function'},
    orientation: {
      axis: ["both", "bottom", "top"],
      item: ["bottom", "top"]
    },
    preferZoom: !1,
    selectable: !0,
    showCurrentTime: !1,
    showMajorLabels: !0,
    showMinorLabels: !0,
    stack: !0,
    stackSubgroups: !0,
    cluster: !1,
    //snap: {'function': 'function', nada},
    start: "",
    //template: {'function': 'function'},
    //timeAxis: {
    //  scale: ['millisecond', 'second', 'minute', 'hour', 'weekday', 'day', 'week', 'month', 'year'],
    //  step: [1, 1, 10, 1]
    //},
    showTooltips: !0,
    tooltip: {
      followMouse: !1,
      overflowMethod: "flip",
      delay: [500, 0, 99999, 100]
    },
    tooltipOnItemUpdateTime: !1,
    type: ["box", "point", "range", "background"],
    width: "100%",
    zoomable: !0,
    zoomKey: ["ctrlKey", "altKey", "shiftKey", "metaKey", ""],
    zoomMax: [31536e10, 10, 31536e10, 1],
    zoomMin: [10, 10, 31536e10, 1],
    xss: {
      disabled: !1
    }
  }
};
class Vm extends xi {
  /**
   * @param {HTMLElement} container
   * @param {vis.DataSet | vis.DataView | Array} [items]
   * @param {vis.DataSet | vis.DataView | Array} [groups]
   * @param {Object} [options]  See Timeline.setOptions for the available options.
   * @constructor Timeline
   */
  constructor(e, t, r, s) {
    var a, o, l, c, d, f, p;
    if (super(), this.initTime = /* @__PURE__ */ new Date(), this.itemsDone = !1, !(this instanceof Vm))
      throw new SyntaxError("Constructor must be called with the new operator");
    if (!(ye(r) || Hr(r)) && r instanceof Object) {
      const x = s;
      s = r, r = x;
    }
    s && s.throttleRedraw && console.warn('Timeline option "throttleRedraw" is DEPRICATED and no longer supported. It will be removed in the next MAJOR release.');
    const m = this;
    if (this.defaultOptions = {
      autoResize: !0,
      longSelectPressTime: 251,
      orientation: {
        axis: "bottom",
        // axis orientation: 'bottom', 'top', or 'both'
        item: "bottom"
        // not relevant
      },
      moment: Et
    }, this.options = E.deepExtend({}, this.defaultOptions), s && E.setupXSSProtection(s.xss), this._create(e), !s || s && typeof s.rtl > "u") {
      this.dom.root.style.visibility = "hidden";
      let x, T = this.dom.root;
      for (; !x && T; )
        x = window.getComputedStyle(T, null).direction, T = T.parentElement;
      this.options.rtl = x && x.toLowerCase() == "rtl";
    } else
      this.options.rtl = s.rtl;
    s && (s.rollingMode && (this.options.rollingMode = s.rollingMode), s.onInitialDrawComplete && (this.options.onInitialDrawComplete = s.onInitialDrawComplete), s.onTimeout && (this.options.onTimeout = s.onTimeout), s.loadingScreenTemplate && (this.options.loadingScreenTemplate = s.loadingScreenTemplate));
    const g = document.createElement("div");
    if (this.options.loadingScreenTemplate) {
      var b;
      const x = N(b = this.options.loadingScreenTemplate).call(b, this), T = x(this.dom.loadingScreen);
      T instanceof Object && !(T instanceof Element) ? x(g) : T instanceof Element ? (g.innerHTML = "", g.appendChild(T)) : T != null && (g.innerHTML = E.xss(T));
    }
    this.dom.loadingScreen.appendChild(g), this.components = [], this.body = {
      dom: this.dom,
      domProps: this.props,
      emitter: {
        on: N(a = this.on).call(a, this),
        off: N(o = this.off).call(o, this),
        emit: N(l = this.emit).call(l, this)
      },
      hiddenDates: [],
      util: {
        getScale() {
          return m.timeAxis.step.scale;
        },
        getStep() {
          return m.timeAxis.step.step;
        },
        toScreen: N(c = m._toScreen).call(c, m),
        toGlobalScreen: N(d = m._toGlobalScreen).call(d, m),
        // this refers to the root.width
        toTime: N(f = m._toTime).call(f, m),
        toGlobalTime: N(p = m._toGlobalTime).call(p, m)
      }
    }, this.range = new Ps(this.body, this.options), this.components.push(this.range), this.body.range = this.range, this.timeAxis = new ss(this.body, this.options), this.timeAxis2 = null, this.components.push(this.timeAxis), this.currentTime = new Um(this.body, this.options), this.components.push(this.currentTime), this.itemSet = new ii(this.body, this.options), this.components.push(this.itemSet), this.itemsData = null, this.groupsData = null;
    function _(x, T) {
      m.hasListeners(x) && m.emit(x, m.getEventProperties(T));
    }
    this.dom.root.onclick = (x) => _("click", x), this.dom.root.ondblclick = (x) => _("doubleClick", x), this.dom.root.oncontextmenu = (x) => _("contextmenu", x), this.dom.root.onmouseover = (x) => _("mouseOver", x), window.PointerEvent ? (this.dom.root.onpointerdown = (x) => _("mouseDown", x), this.dom.root.onpointermove = (x) => _("mouseMove", x), this.dom.root.onpointerup = (x) => _("mouseUp", x)) : (this.dom.root.onmousemove = (x) => _("mouseMove", x), this.dom.root.onmousedown = (x) => _("mouseDown", x), this.dom.root.onmouseup = (x) => _("mouseUp", x)), this.initialFitDone = !1, this.on("changed", () => {
      if (m.itemsData != null) {
        if (!m.initialFitDone && !m.options.rollingMode)
          if (m.initialFitDone = !0, m.options.start != null || m.options.end != null) {
            if (m.options.start == null || m.options.end == null)
              var x = m.getItemRange();
            const T = m.options.start != null ? m.options.start : x.min, S = m.options.end != null ? m.options.end : x.max;
            m.setWindow(T, S, {
              animation: !1
            });
          } else
            m.fit({
              animation: !1
            });
        !m.initialDrawDone && (m.initialRangeChangeDone || !m.options.start && !m.options.end || m.options.rollingMode) && (m.initialDrawDone = !0, m.itemSet.initialDrawDone = !0, m.dom.root.style.visibility = "visible", m.dom.loadingScreen.parentNode.removeChild(m.dom.loadingScreen), m.options.onInitialDrawComplete && Dt(() => m.options.onInitialDrawComplete(), 0));
      }
    }), this.on("destroyTimeline", () => {
      m.destroy();
    }), s && this.setOptions(s), this.body.emitter.on("fit", (x) => {
      this._onFit(x), this.redraw();
    }), r && this.setGroups(r), t && this.setItems(t), this._redraw();
  }
  /**
   * Load a configurator
   * @return {Object}
   * @private
   */
  _createConfigurator() {
    return new fk(this, this.dom.container, mG);
  }
  /**
   * Force a redraw. The size of all items will be recalculated.
   * Can be useful to manually redraw when option autoResize=false and the window
   * has been resized, or when the items CSS has been changed.
   *
   * Note: this function will be overridden on construction with a trottled version
   */
  redraw() {
    this.itemSet && this.itemSet.markDirty({
      refreshItems: !0
    }), this._redraw();
  }
  /**
   * Remove an item from the group
   * @param {object} options
   */
  setOptions(e) {
    if (Ue.validate(e, pG) === !0 && console.log("%cErrors have been found in the supplied options object.", ya), xi.prototype.setOptions.call(this, e), "type" in e && e.type !== this.options.type) {
      this.options.type = e.type;
      const r = this.itemsData;
      if (r) {
        const s = this.getSelection();
        this.setItems(null), this.setItems(r.rawDS), this.setSelection(s);
      }
    }
  }
  /**
   * Set items
   * @param {vis.DataSet | Array | null} items
   */
  setItems(e) {
    this.itemsDone = !1;
    let t;
    e ? Hr(e) ? t = cn(e) : t = cn(new Oi(e)) : t = null, this.itemsData && this.itemsData.dispose(), this.itemsData = t, this.itemSet && this.itemSet.setItems(t != null ? t.rawDS : null);
  }
  /**
   * Set groups
   * @param {vis.DataSet | Array} groups
   */
  setGroups(e) {
    let t;
    const r = (s) => s.visible !== !1;
    e ? (ye(e) && (e = new Oi(e)), t = new ja(e, {
      filter: r
    })) : t = null, this.groupsData != null && typeof this.groupsData.setData == "function" && this.groupsData.setData(null), this.groupsData = t, this.itemSet.setGroups(t);
  }
  /**
   * Set both items and groups in one go
   * @param {{items: (Array | vis.DataSet), groups: (Array | vis.DataSet)}} data
   */
  setData(e) {
    e && e.groups && this.setGroups(e.groups), e && e.items && this.setItems(e.items);
  }
  /**
   * Set selected items by their id. Replaces the current selection
   * Unknown id's are silently ignored.
   * @param {string[] | string} [ids]  An array with zero or more id's of the items to be
   *                                selected. If ids is an empty array, all items will be
   *                                unselected.
   * @param {Object} [options]      Available options:
   *                                `focus: boolean`
   *                                    If true, focus will be set to the selected item(s)
   *                                `animation: boolean | {duration: number, easingFunction: string}`
   *                                    If true (default), the range is animated
   *                                    smoothly to the new window. An object can be
   *                                    provided to specify duration and easing function.
   *                                    Default duration is 500 ms, and default easing
   *                                    function is 'easeInOutQuad'.
   *                                    Only applicable when option focus is true.
   */
  setSelection(e, t) {
    this.itemSet && this.itemSet.setSelection(e), t && t.focus && this.focus(e, t);
  }
  /**
   * Get the selected items by their id
   * @return {Array} ids  The ids of the selected items
   */
  getSelection() {
    return this.itemSet && this.itemSet.getSelection() || [];
  }
  /**
   * Adjust the visible window such that the selected item (or multiple items)
   * are centered on screen.
   * @param {string | String[]} id     An item id or array with item ids
   * @param {Object} [options]      Available options:
   *                                `animation: boolean | {duration: number, easingFunction: string}`
   *                                    If true (default), the range is animated
   *                                    smoothly to the new window. An object can be
   *                                    provided to specify duration and easing function.
   *                                    Default duration is 500 ms, and default easing
   *                                    function is 'easeInOutQuad'.
   *                                `zoom: boolean`
   *                                    If true (default), the timeline will
   *                                    zoom on the element after focus it.
   */
  focus(e, t) {
    if (!this.itemsData || e == null) return;
    const r = ye(e) ? e : [e], s = this.itemsData.get(r);
    let a = null, o = null;
    if (B(s).call(s, (l) => {
      const c = l.start.valueOf(), d = "end" in l ? l.end.valueOf() : l.start.valueOf();
      (a === null || c < a) && (a = c), (o === null || d > o) && (o = d);
    }), a !== null && o !== null) {
      const l = this, c = this.itemSet.items[r[0]];
      let d = this._getScrollTop() * -1, f = null;
      const p = (S, k, D) => {
        const I = nD(l, c);
        if (I === !1 || (f || (f = I), f.itemTop == I.itemTop && !f.shouldScroll))
          return;
        f.itemTop != I.itemTop && I.shouldScroll && (f = I, d = l._getScrollTop() * -1);
        const C = d, P = f.scrollOffset, R = D ? P : C + (P - C) * S;
        l._setScrollTop(-R), k || l._redraw();
      }, m = () => {
        const S = nD(l, c);
        S.shouldScroll && S.itemTop != f.itemTop && (l._setScrollTop(-S.scrollOffset), l._redraw());
      }, g = () => {
        m(), Dt(m, 100);
      }, b = t && t.zoom !== void 0 ? t.zoom : !0, _ = (a + o) / 2, x = b ? (o - a) * 1.1 : Math.max(this.range.end - this.range.start, (o - a) * 1.1), T = t && t.animation !== void 0 ? t.animation : !0;
      T || (f = {
        shouldScroll: !1,
        scrollOffset: -1,
        itemTop: -1
      }), this.range.stopRolling(), this.range.setRange(_ - x / 2, _ + x / 2, {
        animation: T
      }, g, p);
    }
  }
  /**
   * Set Timeline window such that it fits all items
   * @param {Object} [options]  Available options:
   *                                `animation: boolean | {duration: number, easingFunction: string}`
   *                                    If true (default), the range is animated
   *                                    smoothly to the new window. An object can be
   *                                    provided to specify duration and easing function.
   *                                    Default duration is 500 ms, and default easing
   *                                    function is 'easeInOutQuad'.
   * @param {function} [callback]
   */
  fit(e, t) {
    const r = e && e.animation !== void 0 ? e.animation : !0;
    let s;
    this.itemsData.length === 1 && this.itemsData.get()[0].end === void 0 ? (s = this.getDataRange(), this.moveTo(s.min.valueOf(), {
      animation: r
    }, t)) : (s = this.getItemRange(), this.range.setRange(s.min, s.max, {
      animation: r
    }, t));
  }
  /**
   * Determine the range of the items, taking into account their actual width
   * and a margin of 10 pixels on both sides.
   *
   * @returns {{min: Date, max: Date}}
   */
  getItemRange() {
    const e = this.getDataRange();
    let t = e.min !== null ? e.min.valueOf() : null, r = e.max !== null ? e.max.valueOf() : null, s = null, a = null;
    if (t != null && r != null) {
      let o = r - t;
      o <= 0 && (o = 10);
      const l = o / this.props.center.width, c = {};
      let d = 0;
      if (B(E).call(E, this.itemSet.items, (p, m) => {
        p.groupShowing && (c[m] = p.redraw(!0), d = c[m].length);
      }), d > 0)
        for (let p = 0; p < d; p++)
          B(E).call(E, c, (m) => {
            m[p]();
          });
      if (B(E).call(E, this.itemSet.items, (p) => {
        const m = mp(p), g = vp(p);
        let b, _;
        this.options.rtl ? (b = m - (p.getWidthRight() + 10) * l, _ = g + (p.getWidthLeft() + 10) * l) : (b = m - (p.getWidthLeft() + 10) * l, _ = g + (p.getWidthRight() + 10) * l), b < t && (t = b, s = p), _ > r && (r = _, a = p);
      }), s && a) {
        const p = s.getWidthLeft() + 10, m = a.getWidthRight() + 10, g = this.props.center.width - p - m;
        g > 0 && (this.options.rtl ? (t = mp(s) - m * o / g, r = vp(a) + p * o / g) : (t = mp(s) - p * o / g, r = vp(a) + m * o / g));
      }
    }
    return {
      min: t != null ? new Date(t) : null,
      max: r != null ? new Date(r) : null
    };
  }
  /**
   * Calculate the data range of the items start and end dates
   * @returns {{min: Date, max: Date}}
   */
  getDataRange() {
    let e = null, t = null;
    if (this.itemsData) {
      var r;
      B(r = this.itemsData).call(r, (s) => {
        const a = E.convert(s.start, "Date").valueOf(), o = E.convert(s.end != null ? s.end : s.start, "Date").valueOf();
        (e === null || a < e) && (e = a), (t === null || o > t) && (t = o);
      });
    }
    return {
      min: e != null ? new Date(e) : null,
      max: t != null ? new Date(t) : null
    };
  }
  /**
   * Generate Timeline related information from an event
   * @param {Event} event
   * @return {Object} An object with related information, like on which area
   *                  The event happened, whether clicked on an item, etc.
   */
  getEventProperties(e) {
    const t = e.center ? e.center.x : e.clientX, r = e.center ? e.center.y : e.clientY, s = this.dom.centerContainer.getBoundingClientRect(), a = this.options.rtl ? s.right - t : t - s.left, o = r - s.top, l = this.itemSet.itemFromTarget(e), c = this.itemSet.groupFromTarget(e), d = Ha.customTimeFromTarget(e), f = this.itemSet.options.snap || null, p = this.body.util.getScale(), m = this.body.util.getStep(), g = this._toTime(a), b = f ? f(g, p, m) : g, _ = E.getTarget(e);
    let x = null;
    return l != null ? x = "item" : d != null ? x = "custom-time" : E.hasParent(_, this.timeAxis.dom.foreground) || this.timeAxis2 && E.hasParent(_, this.timeAxis2.dom.foreground) ? x = "axis" : E.hasParent(_, this.itemSet.dom.labelSet) ? x = "group-label" : E.hasParent(_, this.currentTime.bar) ? x = "current-time" : E.hasParent(_, this.dom.center) && (x = "background"), {
      event: e,
      item: l ? l.id : null,
      isCluster: l ? !!l.isCluster : !1,
      items: l ? l.items || [] : null,
      group: c ? c.groupId : null,
      customTime: d ? d.options.id : null,
      what: x,
      pageX: e.srcEvent ? e.srcEvent.pageX : e.pageX,
      pageY: e.srcEvent ? e.srcEvent.pageY : e.pageY,
      x: a,
      y: o,
      time: g,
      snappedTime: b
    };
  }
  /**
   * Toggle Timeline rolling mode
   */
  toggleRollingMode() {
    this.range.rolling ? this.range.stopRolling() : (this.options.rollingMode == null && this.setOptions(this.options), this.range.startRolling());
  }
  /**
   * redraw
   * @private
   */
  _redraw() {
    xi.prototype._redraw.call(this);
  }
  /**
   * on fit callback
   * @param {object} args
   * @private
   */
  _onFit(e) {
    const t = e.start, r = e.end, s = e.animation;
    r ? this.range.setRange(t, r, {
      animation: s
    }) : this.moveTo(t.valueOf(), {
      animation: s
    });
  }
}
function mp(n) {
  return E.convert(n.data.start, "Date").valueOf();
}
function vp(n) {
  const e = n.data.end != null ? n.data.end : n.data.start;
  return E.convert(e, "Date").valueOf();
}
function nD(n, e) {
  if (!e.parent)
    return !1;
  const t = n.options.rtl ? n.props.rightContainer.height : n.props.leftContainer.height, r = n.props.center.height, s = e.parent;
  let a = s.top, o = !0;
  const l = n.timeAxis.options.orientation.axis, c = () => l == "bottom" ? s.height - e.top - e.height : e.top, d = n._getScrollTop() * -1, f = a + c(), p = e.height;
  return f < d ? a + t <= a + c() + p && (a += c() - n.itemSet.options.margin.item.vertical) : f + p > d + t ? a += c() + p - t + n.itemSet.options.margin.item.vertical : o = !1, a = Math.min(a, r - t), {
    shouldScroll: o,
    scrollOffset: a,
    itemTop: f
  };
}
const vG = jO();
Et.locale(vG);
const gG = {
  Core: xi,
  DateUtil: GH,
  Range: Ps,
  stack: VY,
  TimeStep: ri,
  components: {
    items: {
      Item: Li,
      BackgroundItem: li,
      BoxItem: ak,
      ClusterItem: Rs,
      PointItem: ok,
      RangeItem: ti
    },
    BackgroundGroup: Ym,
    Component: zr,
    CurrentTime: Um,
    CustomTime: Ha,
    DataAxis: Wp,
    DataScale: sk,
    GraphGroup: Ur,
    Group: ma,
    ItemSet: ii,
    Legend: Gt,
    LineGraph: Ne,
    TimeAxis: ss
  }
}, yG = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  DOMutil: MY,
  DataSet: Oi,
  DataView: ja,
  Graph2d: Tr,
  Hammer: Tt,
  Queue: $a,
  Timeline: Vm,
  keycharm: fD,
  moment: Et,
  timeline: gG,
  util: FO
}, Symbol.toStringTag, { value: "Module" }));
