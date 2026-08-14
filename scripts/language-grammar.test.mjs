import assert from "node:assert/strict";
import {
  GRAMMAR_CATALOG,
  GRAMMAR_SECTIONS,
  GRAMMAR_SYSTEM_IDS,
  GRAMMAR_VALUE_SCHEMA,
  assertCatalogComplete,
  brokenAgreementFeatures,
  configuredMinimum,
  emptySystemRecord,
  grammarGlance,
  grammarStatusLabel,
  indexGrammarRecords,
  normalizeGrammarRecord,
  searchGrammar,
  sectionCardSummary,
  serializeGrammarRecord,
  summarizeSystem,
  validateGrammarDraft,
  emptyGrammarUiState,
  isGrammarDirty,
  openSystemEditor,
  persistGrammarRecord,
  isStaleRevisionError,
  setSystemStatus,
  confirmGrammarLeave,
  loadGrammarIndex,
} from "../packages/modules/language/src/grammar.ts";

function matchesSchema(value, schema, defs = schema.$defs ?? {}) {
  if (schema.$ref) {
    const key = String(schema.$ref).replace("#/$defs/", "");
    return matchesSchema(value, defs[key], defs);
  }
  if (schema.const !== undefined) return value === schema.const;
  if (schema.enum) return schema.enum.includes(value);
  if (schema.oneOf) return schema.oneOf.some((branch) => matchesSchema(value, branch, defs));
  const type = schema.type;
  if (type === "string") return typeof value === "string";
  if (type === "number") return typeof value === "number";
  if (type === "boolean") return typeof value === "boolean";
  if (type === "array") {
    if (!Array.isArray(value)) return false;
    return schema.items ? value.every((item) => matchesSchema(item, schema.items, defs)) : true;
  }
  if (type === "object") {
    if (!value || typeof value !== "object" || Array.isArray(value)) return false;
    const required = schema.required ?? [];
    for (const key of required) if (!(key in value)) return false;
    if (schema.additionalProperties === false) {
      const allowed = new Set(Object.keys(schema.properties ?? {}));
      for (const key of Object.keys(value)) if (!allowed.has(key)) return false;
    }
    for (const [key, child] of Object.entries(schema.properties ?? {})) {
      if (key in value && !matchesSchema(value[key], child, defs)) return false;
    }
    if (schema.additionalProperties && typeof schema.additionalProperties === "object") {
      const known = new Set(Object.keys(schema.properties ?? {}));
      for (const [key, child] of Object.entries(value)) {
        if (!known.has(key) && !matchesSchema(child, schema.additionalProperties, defs)) return false;
      }
    }
    return true;
  }
  return true;
}

assertCatalogComplete();
assert.equal(new Set(GRAMMAR_SYSTEM_IDS).size, GRAMMAR_SYSTEM_IDS.length);
assert.equal(GRAMMAR_CATALOG.length, GRAMMAR_SYSTEM_IDS.length);
for (const system of GRAMMAR_CATALOG) {
  assert.ok(system.searchAliases.length > 0, system.id);
  assert.ok(system.hint.length > 0, system.id);
}

const legacy = normalizeGrammarRecord({ title: "Tense", section: "verb", body: "Past.", links: [] });
assert.equal(legacy.ok, false);
assert.equal(legacy.issues[0].code, "legacy-topic");
assert.equal(matchesSchema({ title: "Tense", section: "verb", body: "Past.", links: [] }, GRAMMAR_VALUE_SCHEMA), false);

assert.equal(normalizeGrammarRecord({ recordKind: "system", schemaVersion: 1, systemId: "mystery", status: "configured" }).ok, false);
assert.equal(normalizeGrammarRecord({ recordKind: "note", schemaVersion: 1 }).ok, false);
assert.equal(normalizeGrammarRecord({ recordKind: "custom-rule", schemaVersion: 1, tags: [], body: "" }).ok, false);

const unknownSystem = { recordKind: "system", schemaVersion: 1, systemId: "syntax.banana", status: "configured", config: {}, notes: "", examples: [], links: [] };
assert.equal(normalizeGrammarRecord(unknownSystem).ok, false);

function ok(value) {
  const result = normalizeGrammarRecord(value);
  assert.equal(result.ok, true, result.ok ? "" : result.issues.map((item) => item.message).join("; "));
  assert.equal(matchesSchema(result.record, GRAMMAR_VALUE_SCHEMA), true, JSON.stringify(result.record));
  const serialized = serializeGrammarRecord(result.record);
  assert.deepEqual(normalizeGrammarRecord(serialized).record, result.record);
  return result.record;
}

const fixtures = {
  "syntax.basic-word-order": { order: "sov", strength: "default-flexible", influences: ["topic"], changeNotes: "Focus can front." },
  "syntax.adjective-position": { position: "before", alternatePositions: [], conditions: "" },
  "syntax.adpositions": { strategy: "postpositions", distributionNotes: "Case may co-occur." },
  "syntax.possessive-position": { position: "possessor-before", alternatePositions: [] },
  "syntax.relative-clause-position": { position: "after", alternatePositions: [] },
  "nouns.number": {
    categories: [
      { id: "singular", templateId: "singular", label: "Singular" },
      { id: "plural", templateId: "plural", label: "Plural", marker: "-n", position: "suffix" },
    ],
    markingStrategies: ["affix"],
  },
  "nouns.case": {
    cases: [
      { id: "nom", templateId: "nominative", name: "Nominative", abbreviation: "NOM", primaryFunction: "Subject" },
      { id: "acc", templateId: "accusative", name: "Accusative", abbreviation: "ACC", primaryFunction: "Direct object" },
    ],
  },
  "nouns.classes": { kind: "gender", classes: [{ id: "m", name: "Masculine" }, { id: "f", name: "Feminine" }] },
  "nouns.definiteness": { strategies: ["definite-article"], articles: [{ id: "def", form: "le", position: "before" }] },
  "nouns.possession": { strategies: ["genitive"] },
  "pronouns.personal": {
    axes: [
      { id: "person", label: "Person", values: [{ id: "1", label: "1st" }, { id: "2", label: "2nd" }] },
      { id: "number", label: "Number", values: [{ id: "sg", label: "Singular" }, { id: "pl", label: "Plural" }] },
    ],
    cells: [
      { id: "1sg", coordinates: { person: "1", number: "sg" }, state: "form", form: "na" },
      { id: "1pl", coordinates: { person: "1", number: "pl" }, state: "form", form: "nar" },
      { id: "2sg", coordinates: { person: "2", number: "sg" }, state: "same-as", sameAsCellId: "1sg" },
      { id: "2pl", coordinates: { person: "2", number: "pl" }, state: "zero" },
    ],
  },
  "pronouns.demonstratives": {
    distances: ["proximal", "distal"],
    axes: [{ id: "distance", label: "Distance", values: [{ id: "proximal", label: "this" }, { id: "distal", label: "that" }] }],
    cells: [{ id: "prox", coordinates: { distance: "proximal" }, state: "form", form: "si" }],
  },
  "verbs.marking-strategy": { strategies: ["suffixes"] },
  "verbs.tense": { categories: [{ id: "past", templateId: "past", label: "Past", marker: "-ka" }] },
  "verbs.aspect": { categories: [{ id: "pfv", templateId: "perfective", label: "Perfective" }] },
  "verbs.mood": { categories: [{ id: "ind", templateId: "indicative", label: "Indicative" }] },
  "verbs.argument-indexing": {
    participants: "subject",
    representation: "endings",
    axes: [{ id: "person", label: "Person", values: [{ id: "1", label: "1st" }] }],
    cells: [{ id: "c1", coordinates: { person: "1" }, state: "form", form: "-m" }],
  },
  "verbs.negative-forms": { strategies: ["affix"], forms: [{ id: "neg", form: "-na" }] },
  "modifiers.adjective-behavior": { behaviors: ["invariant"], agreementRecordIds: [] },
  "modifiers.comparative": { strategies: ["particle"], marker: "more" },
  "modifiers.superlative": { strategies: ["dedicated"], marker: "-est" },
  "clauses.yes-no-questions": { strategies: ["particle"], particle: "ma", placement: "clause-final" },
  "clauses.content-questions": { behavior: "in-situ", interrogatives: [{ id: "who", meaning: "who", form: "ke" }] },
  "clauses.imperatives": { strategies: ["bare-verb"], numberDistinction: true },
  "clauses.negation": { strategies: ["particle"], particle: "ne", placement: "before-verb" },
  "clauses.relative-clauses": { strategies: ["gap"], headBehavior: "post-nominal" },
};

for (const systemId of GRAMMAR_SYSTEM_IDS) {
  const unconfigured = ok(emptySystemRecord(systemId));
  assert.equal(unconfigured.status, "unconfigured");
  assert.deepEqual(unconfigured.config, {});
  const unused = ok({ ...emptySystemRecord(systemId, "not-used"), notes: "Word order covers this." });
  assert.equal(unused.status, "not-used");
  assert.deepEqual(unused.config, {});
  const configured = ok({
    recordKind: "system",
    schemaVersion: 1,
    systemId,
    status: "configured",
    config: fixtures[systemId],
    notes: "",
    examples: [{ id: "ex1", text: "Nar bel tor.", translation: "I eat bread.", gloss: "1sg bread eat" }],
    links: [{ id: "l1", kind: "lexeme", targetId: "f4c4f6b9-7c1e-4b8a-9d2e-0a3b5c7d9e11", label: "nar" }],
  });
  assert.equal(configured.status, "configured");
  assert.equal(configuredMinimum(systemId, configured.config), true, systemId);
  assert.notEqual(summarizeSystem(systemId, configured), grammarStatusLabel("unconfigured"));
}

const incomplete = normalizeGrammarRecord({
  recordKind: "system",
  schemaVersion: 1,
  systemId: "syntax.basic-word-order",
  status: "configured",
  config: {},
  notes: "",
  examples: [],
  links: [],
});
assert.equal(incomplete.ok, true);
assert.equal(incomplete.issues.some((item) => item.code === "configured-minimum"), true);

const trimmed = ok({
  recordKind: "system",
  schemaVersion: 1,
  systemId: "syntax.adjective-position",
  status: "configured",
  config: { position: " after ", alternatePositions: ["mystery"] },
  notes: "  note  ",
  examples: [{ text: " house red " }],
  links: [{ kind: "lexeme", targetId: "  abc  ", label: " x " }, { kind: "mystery", targetId: "abc" }],
});
assert.equal(trimmed.config.position, "after");
assert.deepEqual(trimmed.config.alternatePositions, []);
assert.equal(trimmed.notes, "note");
assert.equal(trimmed.examples[0].text, "house red");
assert.equal(trimmed.links.length, 1);

const agreement = ok({
  recordKind: "agreement",
  schemaVersion: 1,
  title: " Subject → Verb ",
  controller: { kind: "subject" },
  target: { kind: "verb" },
  features: [{ sourceSystemId: "nouns.number", categoryId: "plural", label: "Number" }],
  behavior: "partial",
  notes: "Third-person verbs agree in number only.",
  examples: [],
  links: [],
});
assert.equal(agreement.title, "Subject → Verb");

const custom = ok({
  recordKind: "custom-rule",
  schemaVersion: 1,
  title: " Switch-reference ",
  tags: ["discourse", ""],
  body: "Same-subject marking on chained verbs.\r\n",
  examples: [],
  links: [{ kind: "paradigm", targetId: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee" }],
});
assert.equal(custom.title, "Switch-reference");
assert.equal(custom.body.includes("\r"), false);

const sectionState = ok({ recordKind: "section-state", schemaVersion: 1, sectionId: "agreement", status: "not-used", note: "No agreement." });
assert.equal(sectionState.sectionId, "agreement");

const numberRecord = {
  id: "11111111-1111-1111-1111-111111111111",
  value: {
    recordKind: "system",
    schemaVersion: 1,
    systemId: "nouns.number",
    status: "configured",
    config: fixtures["nouns.number"],
    notes: "",
    examples: [],
    links: [],
  },
};
const duplicateA = { id: "22222222-2222-2222-2222-222222222222", value: emptySystemRecord("syntax.basic-word-order", "configured") };
duplicateA.value = {
  ...emptySystemRecord("syntax.basic-word-order", "configured"),
  config: fixtures["syntax.basic-word-order"],
};
const duplicateB = {
  id: "33333333-3333-3333-3333-333333333333",
  value: { ...emptySystemRecord("syntax.basic-word-order", "not-used"), notes: "ignored winner" },
};
const indexed = indexGrammarRecords([
  numberRecord,
  duplicateA,
  duplicateB,
  { id: "44444444-4444-4444-4444-444444444444", value: agreement },
  { id: "55555555-5555-5555-5555-555555555555", value: custom },
  { id: "66666666-6666-6666-6666-666666666666", value: { title: "old", section: "other", body: "", links: [] } },
]);
assert.equal(indexed.duplicates.get("syntax.basic-word-order")?.length, 2);
assert.equal(indexed.systems.has("syntax.basic-word-order"), false);
assert.equal(indexed.systems.get("nouns.number")?.value.status, "configured");
assert.equal(indexed.rejected.length, 1);
assert.ok(indexed.diagnostics.some((item) => item.code === "duplicate-system"));

const broken = brokenAgreementFeatures(
  indexGrammarRecords([
    { id: "a", value: agreement },
    {
      id: "b",
      value: {
        recordKind: "system",
        schemaVersion: 1,
        systemId: "nouns.number",
        status: "configured",
        config: { categories: [{ id: "singular", label: "Singular" }], markingStrategies: [] },
        notes: "",
        examples: [],
        links: [],
      },
    },
  ]),
);
assert.ok(broken.some((item) => item.code === "broken-reference"));

const emptyIndex = indexGrammarRecords([]);
for (const section of GRAMMAR_SECTIONS) {
  const card = sectionCardSummary(emptyIndex, section.id);
  if (section.id === "agreement") assert.equal(card.detail, "None configured");
  else if (section.id === "other") assert.equal(card.detail, "No custom rules");
  else assert.equal(card.detail, "None configured");
}
const glance = grammarGlance(emptyIndex);
assert.ok(glance.every((row) => row.value === "Not configured"));

const populated = indexGrammarRecords([
  {
    id: "w",
    value: {
      recordKind: "system",
      schemaVersion: 1,
      systemId: "syntax.basic-word-order",
      status: "configured",
      config: fixtures["syntax.basic-word-order"],
      notes: "",
      examples: [],
      links: [],
    },
  },
  {
    id: "c",
    value: {
      recordKind: "system",
      schemaVersion: 1,
      systemId: "nouns.case",
      status: "not-used",
      config: {},
      notes: "Roles come from word order.",
      examples: [],
      links: [],
    },
  },
]);
assert.match(summarizeSystem("syntax.basic-word-order", populated.systems.get("syntax.basic-word-order").value), /SOV/);
assert.match(sectionCardSummary(populated, "syntax").detail, /1 system/);
assert.match(grammarGlance(populated).find((row) => row.label === "Case system").value, /Not used/);

const hits = searchGrammar("ergative", emptyIndex);
assert.equal(hits[0].systemId, "nouns.case");
assert.equal(hits[0].status, "unconfigured");
assert.ok(searchGrammar("questions", emptyIndex).some((item) => item.systemId === "clauses.yes-no-questions"));
assert.ok(searchGrammar("switch-reference", indexed).some((item) => item.kind === "custom-rule"));

assert.equal(GRAMMAR_SECTIONS.map((item) => item.id).join(","), "syntax,nouns,pronouns,verbs,modifiers,clauses,agreement,other");

const owner = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
function fakeGrammarApi(seed = []) {
  const store = new Map(seed.map((record) => [record.id, { ...record }]));
  return {
    store,
    async list(_collection, ownerEntityId, query = {}) {
      const all = [...store.values()].filter((item) => item.ownerEntityId === ownerEntityId);
      const offset = query.offset ?? 0;
      return all.slice(offset, offset + (query.limit ?? 100));
    },
    async create(_collection, ownerEntityId, value) {
      const record = {
        id: crypto.randomUUID(),
        collection: "grammar",
        ownerEntityId,
        value,
        createdAt: "t",
        updatedAt: "t",
        revision: "rev-1",
      };
      store.set(record.id, record);
      return record;
    },
    async update(_collection, id, _owner, value, options) {
      const current = store.get(id);
      if (!current) throw new Error("missing");
      if (current.revision !== options.expectedRevision) {
        throw new Error(`module record revision conflict: expected ${options.expectedRevision}, current ${current.revision}`);
      }
      current.value = value;
      current.revision = `rev-${store.size + 2}`;
      current.updatedAt = "t2";
      return { ...current };
    },
    async delete(_collection, id, _owner, options) {
      const current = store.get(id);
      if (!current) throw new Error("missing");
      if (current.revision !== options.expectedRevision) {
        throw new Error(`module record revision conflict: expected ${options.expectedRevision}, current ${current.revision}`);
      }
      store.delete(id);
    },
  };
}

assert.equal(isStaleRevisionError(new Error("module record revision conflict: expected a, current b")), true);
assert.equal(isStaleRevisionError(new Error("nope")), false);

const emptyUi = emptyGrammarUiState();
const opened = openSystemEditor(emptyUi.index, "nouns.case");
assert.equal(opened.draft.status, "unconfigured");
assert.equal(isGrammarDirty(opened), false);
const dupIndex = indexGrammarRecords([
  { id: "d1", value: emptySystemRecord("nouns.case", "not-used") },
  { id: "d2", value: emptySystemRecord("nouns.case", "not-used") },
]);
assert.equal(openSystemEditor(dupIndex, "nouns.case").locked, true);
opened.draft = setSystemStatus(opened.draft, "not-used");
opened.draft.notes = "Word order covers this.";
assert.equal(isGrammarDirty(opened), true);
assert.equal(confirmGrammarLeave(opened, () => false), false);
assert.equal(confirmGrammarLeave(opened, () => true), true);
assert.equal(validateGrammarDraft(opened.draft).length, 0);
assert.equal(
  validateGrammarDraft(setSystemStatus(emptySystemRecord("nouns.case"), "configured")).some((item) => item.code === "configured-minimum"),
  true,
);

const api = fakeGrammarApi();
const saved = await persistGrammarRecord(api, owner, opened);
assert.equal(saved.ok, true);
assert.equal(saved.record.value.status, "not-used");
assert.equal(saved.index.systems.get("nouns.case").value.status, "not-used");
assert.match(sectionCardSummary(saved.index, "nouns").detail, /not used|0 system/i);
assert.equal(grammarGlance(saved.index).find((row) => row.label === "Case system").value.startsWith("Not used"), true);

const staleApi = fakeGrammarApi([saved.record]);
staleApi.store.get(saved.record.id).revision = "rev-other";
const stale = await persistGrammarRecord(staleApi, owner, {
  recordId: saved.record.id,
  revision: "rev-1",
  draft: saved.record.value,
});
assert.equal(stale.ok, false);
assert.equal(stale.stale, true);
assert.equal(stale.stored.revision, "rev-other");

const paged = fakeGrammarApi();
for (let index = 0; index < 120; index += 1) {
  await paged.create("grammar", owner, {
    recordKind: "custom-rule",
    schemaVersion: 1,
    title: `Rule ${index}`,
    tags: [],
    body: "",
    examples: [],
    links: [],
  });
}
const loaded = await loadGrammarIndex(paged, owner);
assert.equal(loaded.records.length, 120);
assert.equal(loaded.index.customRules.length, 120);

console.log("language grammar helpers ok");
