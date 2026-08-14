import assert from "node:assert/strict";
import {
  GRAMMAR_CATALOG,
  GRAMMAR_SECTIONS,
  GRAMMAR_SYSTEM_IDS,
  GRAMMAR_VALUE_SCHEMA,
  WORD_ORDER_OPTIONS,
  applyAdjectivePosition,
  applyAdpositions,
  applyBasicWordOrder,
  applyPossessivePosition,
  applyRelativeClausePosition,
  addCase,
  addNounClass,
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
  moveNumberCategory,
  NUMBER_TEMPLATES,
  referencedCategoryIds,
  removeCase,
  removeNumberCategory,
  setNounClassKind,
  toggleNumberMarking,
  toggleNumberTemplate,
  toggleTamTemplate,
  updateNumberCategory,
  DEFINITENESS_OPTIONS,
  addArticle,
  moveArticle,
  setAlienability,
  setAlienabilityNotes,
  setCustomVerbMarking,
  toggleAdjectiveBehavior,
  toggleAgreementRecord,
  toggleDefinitenessStrategy,
  toggleDegreeStrategy,
  toggleNegativeStrategy,
  togglePossessionStrategy,
  toggleVerbMarking,
  updateArticle,
  YES_NO_OPTIONS,
  moveInterrogative,
  setContentBehavior,
  setImperativeDistinction,
  setNegationParticle,
  setYesNoParticle,
  setYesNoPlacement,
  toggleImperativeStrategy,
  toggleInterrogative,
  toggleNegationStrategy,
  toggleRelativization,
  toggleYesNoStrategy,
  updateInterrogative,
  CHOICE_SYSTEM_IDS,
  INVENTORY_SYSTEM_IDS,
  STRATEGY_SYSTEM_IDS,
  CLAUSE_SYSTEM_IDS,
  PARADIGM_SYSTEM_IDS,
  DISTANCE_VALUES,
  NUMBER_VALUES,
  PERSON_VALUES,
  setArgumentParticipants,
  toggleAxisValue,
  toggleDistance,
  updateParadigmCell,
  emptyAgreementRecord,
  emptyAgreementSectionState,
  emptyCustomRule,
  offeredAgreementGroups,
  openAgreementEditor,
  setAgreementBehavior,
  setAgreementController,
  setAgreementTarget,
  summarizeAgreement,
  toggleAgreementGroup,
  addCustomAgreementFeature,
  CUSTOM_RULE_TAGS,
  toggleCustomRuleTag,
  GRAMMAR_STARTER_STEPS,
  nextStarterSystem,
  remainingStarterSystems,
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
assert.equal(summarizeSystem("nouns.number", setSystemStatus(emptySystemRecord("nouns.number"), "configured")), "Configured");

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

assert.deepEqual(
  WORD_ORDER_OPTIONS.map((item) => item.value),
  ["sov", "svo", "vso", "vos", "ovs", "osv", "flexible", "custom"],
);

const wordOrder = applyBasicWordOrder(setSystemStatus(emptySystemRecord("syntax.basic-word-order"), "configured"), {
  order: "sov",
  strength: "strict",
});
assert.equal(configuredMinimum("syntax.basic-word-order", wordOrder.config), true);
assert.equal(summarizeSystem("syntax.basic-word-order", wordOrder), "SOV · Strict");
assert.equal(validateGrammarDraft(wordOrder).length, 0);
assert.equal(matchesSchema(serializeGrammarRecord(wordOrder), GRAMMAR_VALUE_SCHEMA), true);

const customOrder = applyBasicWordOrder(wordOrder, { order: "custom" });
assert.equal(configuredMinimum("syntax.basic-word-order", customOrder.config), false);
assert.equal(validateGrammarDraft(customOrder)[0].path, "customOrder");
const namedCustom = applyBasicWordOrder(customOrder, { customOrder: "Topic first, then verb." });
assert.equal(summarizeSystem("syntax.basic-word-order", namedCustom), "Topic first, then verb. · Strict");
assert.equal(namedCustom.config.order, "custom");

const flexible = applyBasicWordOrder(namedCustom, { order: "flexible", toggleInfluence: "topic" });
assert.equal(flexible.config.customOrder, undefined);
assert.deepEqual(flexible.config.influences, ["topic"]);
const withFocus = applyBasicWordOrder(flexible, { toggleInfluence: "focus" });
assert.deepEqual(withFocus.config.influences, ["topic", "focus"]);
const withoutTopic = applyBasicWordOrder(withFocus, { toggleInfluence: "topic" });
assert.deepEqual(withoutTopic.config.influences, ["focus"]);
const notFlexible = applyBasicWordOrder(withoutTopic, { order: "svo" });
assert.deepEqual(notFlexible.config.influences, []);

const adjective = applyAdjectivePosition(setSystemStatus(emptySystemRecord("syntax.adjective-position"), "configured"), {
  position: "before",
});
assert.equal(summarizeSystem("syntax.adjective-position", adjective), "Before noun");
const adjectiveAlt = applyAdjectivePosition(adjective, { toggleAlternate: "after", conditions: "Poetry allows both." });
assert.deepEqual(adjectiveAlt.config.alternatePositions, ["after"]);
const adjectiveMoved = applyAdjectivePosition(adjectiveAlt, { position: "after" });
assert.deepEqual(adjectiveMoved.config.alternatePositions, []);
const adjectiveCustom = applyAdjectivePosition(adjective, { position: "custom" });
assert.equal(validateGrammarDraft(adjectiveCustom)[0].path, "customPosition");

const possessive = applyPossessivePosition(setSystemStatus(emptySystemRecord("syntax.possessive-position"), "configured"), {
  position: "possessor-before",
});
assert.equal(summarizeSystem("syntax.possessive-position", possessive), "Possessor before noun");

const relative = applyRelativeClausePosition(
  setSystemStatus(emptySystemRecord("syntax.relative-clause-position"), "configured"),
  { position: "internally-headed" },
);
assert.equal(summarizeSystem("syntax.relative-clause-position", relative), "Internally headed");

const adpositions = applyAdpositions(setSystemStatus(emptySystemRecord("syntax.adpositions"), "configured"), {
  strategy: "both",
  distributionNotes: "Time uses prepositions; space uses postpositions.",
});
assert.equal(summarizeSystem("syntax.adpositions", adpositions), "Both");
assert.match(adpositions.config.distributionNotes, /Time/);
const prepositions = applyAdpositions(adpositions, { strategy: "prepositions" });
assert.equal(prepositions.config.distributionNotes, undefined);

const choiceApi = fakeGrammarApi();
const savedChoice = await persistGrammarRecord(choiceApi, owner, { draft: wordOrder });
assert.equal(savedChoice.ok, true);
assert.equal(savedChoice.record.value.config.order, "sov");
assert.match(sectionCardSummary(savedChoice.index, "syntax").detail, /1 system/);
assert.equal(grammarGlance(savedChoice.index).find((row) => row.label === "Basic word order").value, "SOV · Strict");

const adjectiveSaved = await persistGrammarRecord(choiceApi, owner, { draft: adjectiveAlt });
assert.equal(adjectiveSaved.ok, true);
assert.equal(
  grammarGlance(adjectiveSaved.index).find((row) => row.label === "Adjective position").value,
  "Before noun",
);

assert.deepEqual(
  NUMBER_TEMPLATES.map((item) => item.id),
  ["singular", "plural", "dual", "trial", "paucal", "collective", "custom"],
);

let number = toggleNumberTemplate(setSystemStatus(emptySystemRecord("nouns.number"), "configured"), "singular").draft;
number = toggleNumberTemplate(number, "plural").draft;
const singularId = number.config.categories[0].id;
const pluralId = number.config.categories[1].id;
assert.equal(number.config.categories[0].templateId, "singular");
number = updateNumberCategory(number, singularId, { label: "Sg", marker: "-∅" });
assert.equal(number.config.categories[0].id, singularId);
assert.equal(number.config.categories[0].templateId, "singular");
number = moveNumberCategory(number, pluralId, -1);
assert.deepEqual(
  number.config.categories.map((item) => item.id),
  [pluralId, singularId],
);
number = toggleNumberMarking(number, "affix");
assert.equal(summarizeSystem("nouns.number", number), "Plural, Sg · affix");
assert.equal(validateGrammarDraft(number).length, 0);
assert.equal(matchesSchema(serializeGrammarRecord(number), GRAMMAR_VALUE_SCHEMA), true);

const agreementRef = {
  id: "agr-1",
  value: {
    recordKind: "agreement",
    schemaVersion: 1,
    title: "Noun → Adj",
    controller: { kind: "noun" },
    target: { kind: "adjective" },
    features: [{ sourceSystemId: "nouns.number", categoryId: singularId, label: "Number" }],
    behavior: "full",
    notes: "",
    examples: [],
    links: [],
  },
};
const referencedIndex = indexGrammarRecords([{ id: "num-1", value: number }, agreementRef]);
const referenced = referencedCategoryIds(referencedIndex, "nouns.number");
assert.equal(referenced.has(singularId), true);
const blocked = removeNumberCategory(number, singularId, { referenced });
assert.equal(blocked.blocked.id, singularId);
assert.deepEqual(
  blocked.draft.config.categories.map((item) => item.id),
  [pluralId, singularId],
);
const forced = removeNumberCategory(number, singularId, { referenced, force: true });
assert.equal(forced.blocked, undefined);
assert.deepEqual(
  forced.draft.config.categories.map((item) => item.id),
  [pluralId],
);
assert.equal(brokenAgreementFeatures(indexGrammarRecords([{ id: "num-1", value: forced.draft }, agreementRef])).length, 1);

let cases = addCase(setSystemStatus(emptySystemRecord("nouns.case"), "configured"), "nominative");
const nomId = cases.config.cases[0].id;
cases = addCase(cases, "accusative");
assert.equal(cases.config.cases[0].id, nomId);
assert.equal(cases.config.cases[0].primaryFunction, "Subject");
assert.match(summarizeSystem("nouns.case", cases), /2 cases/);
const customCase = addCase(setSystemStatus(emptySystemRecord("nouns.case"), "configured"), "custom");
assert.equal(configuredMinimum("nouns.case", customCase.config), false);
assert.equal(removeCase(cases, nomId).draft.config.cases.length, 1);

let classes = setNounClassKind(setSystemStatus(emptySystemRecord("nouns.classes"), "configured"), "gender");
assert.equal(configuredMinimum("nouns.classes", classes.config), false);
classes = addNounClass(classes, "Masculine");
classes = addNounClass(classes, "Feminine");
assert.equal(summarizeSystem("nouns.classes", classes), "gender · Masculine, Feminine");
assert.equal(validateGrammarDraft(classes).length, 0);

let tense = toggleTamTemplate(setSystemStatus(emptySystemRecord("verbs.tense"), "configured"), "past").draft;
tense = toggleTamTemplate(tense, "future").draft;
assert.equal(summarizeSystem("verbs.tense", tense), "Past, Future");
assert.equal(tense.config.categories[0].id !== tense.config.categories[1].id, true);

const inventoryApi = fakeGrammarApi();
const savedNumber = await persistGrammarRecord(inventoryApi, owner, { draft: number });
assert.equal(savedNumber.ok, true);
assert.equal(savedNumber.record.value.config.categories[0].id, pluralId);
assert.match(sectionCardSummary(savedNumber.index, "nouns").detail, /1 system/);
assert.match(grammarGlance(savedNumber.index).find((row) => row.label === "Number").value, /Plural/);

assert.deepEqual(
  DEFINITENESS_OPTIONS.map((item) => item.value),
  ["definite-article", "indefinite-article", "both", "affixes", "demonstratives", "context", "other"],
);

let definiteness = toggleDefinitenessStrategy(setSystemStatus(emptySystemRecord("nouns.definiteness"), "configured"), "definite-article");
definiteness = addArticle(definiteness, "le");
const articleId = definiteness.config.articles[0].id;
definiteness = addArticle(definiteness, "la");
const secondArticle = definiteness.config.articles[1].id;
definiteness = updateArticle(definiteness, articleId, { position: "before" });
assert.equal(definiteness.config.articles[0].id, articleId);
definiteness = moveArticle(definiteness, secondArticle, -1);
assert.deepEqual(
  definiteness.config.articles.map((item) => item.id),
  [secondArticle, articleId],
);
assert.equal(summarizeSystem("nouns.definiteness", definiteness), "Definite article");
assert.equal(validateGrammarDraft(definiteness).length, 0);
assert.equal(matchesSchema(serializeGrammarRecord(definiteness), GRAMMAR_VALUE_SCHEMA), true);

let possession = togglePossessionStrategy(setSystemStatus(emptySystemRecord("nouns.possession"), "configured"), "genitive");
possession = setAlienability(possession, true);
possession = setAlienabilityNotes(possession, "Body parts are inalienable.");
assert.equal(possession.config.alienability, true);
assert.equal(summarizeSystem("nouns.possession", possession), "Genitive marking");

let marking = toggleVerbMarking(setSystemStatus(emptySystemRecord("verbs.marking-strategy"), "configured"), "custom");
assert.equal(configuredMinimum("verbs.marking-strategy", marking.config), false);
assert.equal(validateGrammarDraft(marking)[0].path, "customStrategy");
marking = setCustomVerbMarking(marking, "Tone on the verb stem.");
assert.equal(summarizeSystem("verbs.marking-strategy", marking), "Tone on the verb stem.");
marking = toggleVerbMarking(marking, "suffixes");
assert.match(summarizeSystem("verbs.marking-strategy", marking), /Suffixes/);

let negative = toggleNegativeStrategy(setSystemStatus(emptySystemRecord("verbs.negative-forms"), "configured"), "affix");
assert.equal(summarizeSystem("verbs.negative-forms", negative), "Affix");

let adjectives = toggleAdjectiveBehavior(setSystemStatus(emptySystemRecord("modifiers.adjective-behavior"), "configured"), "agree-with-noun");
adjectives = toggleAgreementRecord(adjectives, "agr-1");
assert.deepEqual(adjectives.config.agreementRecordIds, ["agr-1"]);
adjectives = toggleAdjectiveBehavior(adjectives, "agree-with-noun");
assert.deepEqual(adjectives.config.agreementRecordIds, []);
adjectives = toggleAdjectiveBehavior(adjectives, "invariant");
assert.equal(summarizeSystem("modifiers.adjective-behavior", adjectives), "Invariant");

let comparative = toggleDegreeStrategy(setSystemStatus(emptySystemRecord("modifiers.comparative"), "configured"), "particle");
assert.equal(summarizeSystem("modifiers.comparative", comparative), "Comparative particle");
let superlative = toggleDegreeStrategy(setSystemStatus(emptySystemRecord("modifiers.superlative"), "configured"), "none");
assert.equal(summarizeSystem("modifiers.superlative", superlative), "No dedicated superlative");

const strategyApi = fakeGrammarApi();
const savedDefiniteness = await persistGrammarRecord(strategyApi, owner, { draft: definiteness });
assert.equal(savedDefiniteness.ok, true);
assert.equal(savedDefiniteness.record.value.config.articles[0].id, secondArticle);
assert.match(sectionCardSummary(savedDefiniteness.index, "nouns").detail, /1 system/);

assert.deepEqual(
  YES_NO_OPTIONS.map((item) => item.value),
  ["intonation", "particle", "word-order", "verb-morphology", "auxiliary", "multiple", "custom"],
);

let yesNo = toggleYesNoStrategy(setSystemStatus(emptySystemRecord("clauses.yes-no-questions"), "configured"), "particle");
assert.equal(configuredMinimum("clauses.yes-no-questions", yesNo.config), false);
assert.equal(validateGrammarDraft(yesNo)[0].path, "particle");
yesNo = setYesNoParticle(yesNo, "ma");
yesNo = setYesNoPlacement(yesNo, "clause-final");
assert.equal(summarizeSystem("clauses.yes-no-questions", yesNo), "Question particle · “ma”");
assert.equal(validateGrammarDraft(yesNo).length, 0);
assert.equal(matchesSchema(serializeGrammarRecord(yesNo), GRAMMAR_VALUE_SCHEMA), true);

let content = setContentBehavior(setSystemStatus(emptySystemRecord("clauses.content-questions"), "configured"), "custom");
assert.equal(validateGrammarDraft(content)[0].path, "customBehavior");
content = setContentBehavior(setSystemStatus(emptySystemRecord("clauses.content-questions"), "configured"), "in-situ");
content = toggleInterrogative(content, "who");
content = toggleInterrogative(content, "what");
const whoId = content.config.interrogatives[0].id;
content = updateInterrogative(content, whoId, { form: "ke" });
assert.equal(content.config.interrogatives[0].id, whoId);
content = moveInterrogative(content, content.config.interrogatives[1].id, -1);
assert.equal(content.config.interrogatives[1].id, whoId);
assert.equal(summarizeSystem("clauses.content-questions", content), "Remain in normal position · what, who");

let imperatives = toggleImperativeStrategy(setSystemStatus(emptySystemRecord("clauses.imperatives"), "configured"), "bare-verb");
imperatives = setImperativeDistinction(imperatives, "numberDistinction", true);
assert.equal(summarizeSystem("clauses.imperatives", imperatives), "Bare verb");
assert.equal(imperatives.config.numberDistinction, true);

let negation = toggleNegationStrategy(setSystemStatus(emptySystemRecord("clauses.negation"), "configured"), "particle");
assert.equal(validateGrammarDraft(negation)[0].path, "particle");
negation = setNegationParticle(negation, "ne");
assert.equal(summarizeSystem("clauses.negation", negation), "Particle · “ne”");

let relativeClauses = toggleRelativization(setSystemStatus(emptySystemRecord("clauses.relative-clauses"), "configured"), "gap");
assert.equal(summarizeSystem("clauses.relative-clauses", relativeClauses), "Gap");

const clauseApi = fakeGrammarApi();
const savedYesNo = await persistGrammarRecord(clauseApi, owner, { draft: yesNo });
assert.equal(savedYesNo.ok, true);
assert.equal(savedYesNo.record.value.config.particle, "ma");
assert.equal(grammarGlance(savedYesNo.index).find((row) => row.label === "Questions").value, "Question particle · “ma”");

assert.equal(
  CHOICE_SYSTEM_IDS.length + INVENTORY_SYSTEM_IDS.length + STRATEGY_SYSTEM_IDS.length + CLAUSE_SYSTEM_IDS.length + PARADIGM_SYSTEM_IDS.length,
  GRAMMAR_SYSTEM_IDS.length,
);

let personal = toggleAxisValue(setSystemStatus(emptySystemRecord("pronouns.personal"), "configured"), "person", PERSON_VALUES[0]).draft;
personal = toggleAxisValue(personal, "person", PERSON_VALUES[1]).draft;
personal = toggleAxisValue(personal, "number", NUMBER_VALUES[0]).draft;
const firstCell = personal.config.cells.find((cell) => cell.coordinates.person === "person-1" && cell.coordinates.number === "number-sg");
assert.equal(Boolean(firstCell), true);
personal = updateParadigmCell(personal, firstCell.id, { form: "yo" });
personal = toggleAxisValue(personal, "person", PERSON_VALUES[2]).draft;
const preserved = personal.config.cells.find((cell) => cell.coordinates.person === "person-1" && cell.coordinates.number === "number-sg");
assert.equal(preserved.id, firstCell.id);
assert.equal(preserved.form, "yo");
assert.match(summarizeSystem("pronouns.personal", personal), /1st/);
const blockedAxis = toggleAxisValue(personal, "person", PERSON_VALUES[0]);
assert.equal(blockedAxis.draft.config.cells.some((cell) => cell.form === "yo"), true);
assert.equal(blockedAxis.blocked.populated > 0, true);
const forcedAxis = toggleAxisValue(personal, "person", PERSON_VALUES[0], { force: true });
assert.equal(forcedAxis.blocked, undefined);
assert.equal(forcedAxis.draft.config.cells.some((cell) => cell.form === "yo"), false);
assert.equal(validateGrammarDraft(personal).length, 0);
assert.equal(matchesSchema(serializeGrammarRecord(personal), GRAMMAR_VALUE_SCHEMA), true);

let demonstratives = toggleDistance(setSystemStatus(emptySystemRecord("pronouns.demonstratives"), "configured"), DISTANCE_VALUES[0]).draft;
demonstratives = toggleDistance(demonstratives, DISTANCE_VALUES[1]).draft;
assert.deepEqual(demonstratives.config.distances, ["distance-proximal", "distance-distal"]);
assert.equal(demonstratives.config.axes[0].id, "distance");
assert.equal(demonstratives.config.cells.length, 2);
assert.equal(summarizeSystem("pronouns.demonstratives", demonstratives), "Proximal / Distal");
assert.equal(matchesSchema(serializeGrammarRecord(demonstratives), GRAMMAR_VALUE_SCHEMA), true);

let indexing = setArgumentParticipants(setSystemStatus(emptySystemRecord("verbs.argument-indexing"), "configured"), "none");
assert.equal(configuredMinimum("verbs.argument-indexing", indexing.config), true);
assert.equal(indexing.config.participants, "none");
assert.equal(indexing.config.cells.length, 0);
assert.equal(summarizeSystem("verbs.argument-indexing", indexing), "Does not index participants");
indexing = setArgumentParticipants(indexing, "subject");
assert.equal(indexing.config.axes.some((axis) => axis.id === "person"), true);
assert.equal(indexing.config.cells.length > 0, true);
assert.equal(matchesSchema(serializeGrammarRecord(indexing), GRAMMAR_VALUE_SCHEMA), true);

const paradigmApi = fakeGrammarApi();
const savedPersonal = await persistGrammarRecord(paradigmApi, owner, { draft: personal });
assert.equal(savedPersonal.ok, true);
assert.equal(savedPersonal.record.value.config.cells.find((cell) => cell.id === firstCell.id).form, "yo");

const numberGroups = offeredAgreementGroups(indexGrammarRecords([{ id: "num-1", value: number }]));
const numberGroup = numberGroups.find((group) => group.id === "nouns.number");
assert.equal(numberGroup.label, "Number");
let subjectVerb = toggleAgreementGroup(emptyAgreementRecord(), numberGroup);
assert.equal(subjectVerb.features.some((item) => item.categoryId === pluralId), true);
number = updateNumberCategory(number, pluralId, { label: "Many" });
assert.equal(subjectVerb.features.find((item) => item.categoryId === pluralId).categoryId, pluralId);
assert.equal(subjectVerb.features.find((item) => item.categoryId === pluralId).label, "Plural");
const renamedIndex = indexGrammarRecords([{ id: "num-1", value: number }, { id: "agr-1", value: subjectVerb }]);
assert.equal(brokenAgreementFeatures(renamedIndex).length, 0);
const missingNumber = removeNumberCategory(number, pluralId, { force: true }).draft;
assert.equal(
  brokenAgreementFeatures(indexGrammarRecords([{ id: "num-1", value: missingNumber }, { id: "agr-1", value: subjectVerb }])).length,
  1,
);

subjectVerb = setAgreementController(subjectVerb, "custom");
assert.equal(validateGrammarDraft(subjectVerb)[0].path, "controllerCustom");
subjectVerb = setAgreementController(emptyAgreementRecord(), "subject");
subjectVerb = setAgreementTarget(subjectVerb, "verb");
subjectVerb = setAgreementBehavior(subjectVerb, "partial");
assert.equal(summarizeAgreement(subjectVerb), "Subject → Verb");
let nounAdjective = setAgreementTarget(setAgreementController(emptyAgreementRecord(), "noun"), "adjective");
nounAdjective = addCustomAgreementFeature(nounAdjective, "Honorific");
assert.equal(summarizeAgreement(nounAdjective), "Noun → Adjective · Honorific");
assert.equal(matchesSchema(serializeGrammarRecord(subjectVerb), GRAMMAR_VALUE_SCHEMA), true);
assert.equal(matchesSchema(serializeGrammarRecord(nounAdjective), GRAMMAR_VALUE_SCHEMA), true);

const openedAgreement = openAgreementEditor(emptyGrammarUiState().index);
assert.equal(openedAgreement.draft.recordKind, "agreement");
assert.equal(openedAgreement.originSection, "agreement");

assert.deepEqual(CUSTOM_RULE_TAGS.slice(0, 3), ["syntax", "morphology", "phonology interaction"]);
let rule = toggleCustomRuleTag({ ...emptyCustomRule(), title: "Switch-reference" }, "discourse");
assert.deepEqual(rule.tags, ["discourse"]);
rule = toggleCustomRuleTag(rule, "discourse");
assert.deepEqual(rule.tags, []);

const unusedSeed = {
  id: "section-state-1",
  ownerEntityId: owner,
  collection: "grammar",
  value: emptyAgreementSectionState("No agreement."),
  revision: "rev-1",
  createdAt: "t",
  updatedAt: "t",
};
const agreementApi = fakeGrammarApi([unusedSeed]);
const savedAgreement = await persistGrammarRecord(agreementApi, owner, { draft: subjectVerb });
assert.equal(savedAgreement.ok, true);
assert.equal(savedAgreement.index.sectionStates.size, 0);
assert.equal(savedAgreement.index.agreements.length, 1);
assert.equal(savedAgreement.index.agreements[0].value.title, "Subject → Verb");
assert.match(sectionCardSummary(savedAgreement.index, "agreement").detail, /1 system/);

assert.deepEqual(GRAMMAR_STARTER_STEPS, [
  "syntax.basic-word-order",
  "syntax.adjective-position",
  "nouns.number",
  "pronouns.personal",
  "verbs.tense",
  "clauses.yes-no-questions",
  "clauses.negation",
]);
assert.equal(GRAMMAR_CATALOG.every((item) => item.scope === "initial"), true);
assert.deepEqual(remainingStarterSystems(emptyGrammarUiState().index), [...GRAMMAR_STARTER_STEPS]);
const starterIndex = indexGrammarRecords([
  {
    id: "wo",
    value: {
      ...emptySystemRecord("syntax.basic-word-order", "configured"),
      config: fixtures["syntax.basic-word-order"],
    },
  },
]);
assert.equal(nextStarterSystem(starterIndex), "syntax.adjective-position");
assert.equal(nextStarterSystem(starterIndex, "syntax.adjective-position"), "nouns.number");
assert.equal(remainingStarterSystems(starterIndex).includes("syntax.basic-word-order"), false);
const dismissed = emptyGrammarUiState();
dismissed.starterDismissed = true;
assert.equal(dismissed.starterDismissed, true);
assert.equal(dismissed.editing, null);

console.log("language grammar helpers ok");
