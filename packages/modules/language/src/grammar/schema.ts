import { CHOICE_SYSTEM_IDS } from "./choice.ts";
import { INVENTORY_SYSTEM_IDS } from "./inventory.ts";
import { STRATEGY_SYSTEM_IDS } from "./strategy.ts";
import { CLAUSE_SYSTEM_IDS } from "./clause.ts";
import { PARADIGM_SYSTEM_IDS } from "./paradigm.ts";

const example = {
  type: "object",
  additionalProperties: false,
  required: ["id", "text"],
  properties: {
    id: { type: "string" },
    text: { type: "string" },
    translation: { type: "string" },
    gloss: { type: "string" },
    notes: { type: "string" },
  },
};

const link = {
  type: "object",
  additionalProperties: false,
  required: ["id", "kind", "targetId"],
  properties: {
    id: { type: "string" },
    kind: { type: "string", enum: ["lexeme", "lexeme-example", "sample", "paradigm"] },
    targetId: { type: "string" },
    secondaryId: { type: "string" },
    label: { type: "string" },
  },
};

const examples = { type: "array", items: example };
const links = { type: "array", items: link };

function systemBranch(systemId: string, config: Record<string, unknown>) {
  return {
    type: "object",
    additionalProperties: false,
    required: ["recordKind", "schemaVersion", "systemId", "status", "config", "notes", "examples", "links"],
    properties: {
      recordKind: { const: "system" },
      schemaVersion: { const: 1 },
      systemId: { const: systemId },
      status: { type: "string", enum: ["unconfigured", "configured", "not-used"] },
      config: config,
      notes: { type: "string" },
      examples,
      links,
    },
  };
}

const emptyConfig = { type: "object", additionalProperties: false, properties: {} };

const axis = {
  type: "object",
  additionalProperties: false,
  required: ["id", "label", "values"],
  properties: {
    id: { type: "string" },
    label: { type: "string" },
    values: {
      type: "array",
      items: {
        type: "object",
        additionalProperties: false,
        required: ["id", "label"],
        properties: {
          id: { type: "string" },
          label: { type: "string" },
          description: { type: "string" },
        },
      },
    },
  },
};

const cell = {
  type: "object",
  additionalProperties: false,
  required: ["id", "coordinates", "state"],
  properties: {
    id: { type: "string" },
    coordinates: { type: "object", additionalProperties: { type: "string" } },
    state: { type: "string", enum: ["form", "same-as", "zero", "not-applicable"] },
    form: { type: "string" },
    alternateForms: { type: "array", items: { type: "string" } },
    sameAsCellId: { type: "string" },
    notes: { type: "string" },
    exampleId: { type: "string" },
  },
};

const paradigmConfig = {
  type: "object",
  additionalProperties: false,
  required: ["axes", "cells"],
  properties: { axes: { type: "array", items: axis }, cells: { type: "array", items: cell } },
};

const PARADIGM_CONFIG = {
  "pronouns.personal": paradigmConfig,
  "pronouns.demonstratives": {
    type: "object",
    additionalProperties: false,
    required: ["axes", "cells"],
    properties: {
      axes: { type: "array", items: axis },
      cells: { type: "array", items: cell },
      distances: stringArray(),
    },
  },
  "verbs.argument-indexing": {
    type: "object",
    additionalProperties: false,
    required: ["participants", "axes", "cells"],
    properties: {
      participants: enumString(["none", "subject", "object", "subject-object", "other"]),
      representation: enumString(["endings", "prefixes", "full-forms", "auxiliaries", "flexible-table", "custom"]),
      axes: { type: "array", items: axis },
      cells: { type: "array", items: cell },
      flexibleNotes: { type: "string" },
      agreementRecordId: { type: "string" },
    },
  },
} as const;

function enumString(values: string[]) {
  return { type: "string", enum: values };
}

function stringArray(values?: string[]) {
  return { type: "array", items: values ? enumString(values) : { type: "string" } };
}

function positionConfig(positions: string[]) {
  return {
    type: "object",
    additionalProperties: false,
    required: ["position", "alternatePositions"],
    properties: {
      position: enumString(positions),
      customPosition: { type: "string" },
      alternatePositions: stringArray(positions),
      conditions: { type: "string" },
    },
  };
}

const CHOICE_CONFIG = {
  "syntax.basic-word-order": {
    type: "object",
    additionalProperties: false,
    required: ["order", "influences"],
    properties: {
      order: enumString(["sov", "svo", "vso", "vos", "ovs", "osv", "flexible", "custom"]),
      customOrder: { type: "string" },
      strength: enumString(["strict", "strongly-preferred", "default-flexible", "context"]),
      influences: stringArray(["topic", "focus", "emphasis", "definiteness", "animacy", "discourse", "custom"]),
      customInfluence: { type: "string" },
      changeNotes: { type: "string" },
    },
  },
  "syntax.adjective-position": positionConfig(["before", "after", "either", "meaning-changes", "custom"]),
  "syntax.adpositions": {
    type: "object",
    additionalProperties: false,
    required: ["strategy"],
    properties: {
      strategy: enumString(["prepositions", "postpositions", "both", "other"]),
      distributionNotes: { type: "string" },
    },
  },
  "syntax.possessive-position": positionConfig([
    "possessor-before",
    "possessor-after",
    "either",
    "morphological",
    "multiple",
    "custom",
  ]),
  "syntax.relative-clause-position": positionConfig(["before", "after", "internally-headed", "multiple", "custom"]),
} as const;

const category = {
  type: "object",
  additionalProperties: false,
  required: ["id", "label"],
  properties: {
    id: { type: "string" },
    templateId: { type: "string" },
    label: { type: "string" },
    meaning: { type: "string" },
    marker: { type: "string" },
    position: { type: "string" },
    interaction: { type: "string" },
    notes: { type: "string" },
  },
};

const caseItem = {
  type: "object",
  additionalProperties: false,
  required: ["id", "name", "primaryFunction"],
  properties: {
    id: { type: "string" },
    templateId: { type: "string" },
    name: { type: "string" },
    abbreviation: { type: "string" },
    primaryFunction: { type: "string" },
    additionalFunctions: { type: "string" },
    marking: { type: "string" },
    notes: { type: "string" },
  },
};

const classItem = {
  type: "object",
  additionalProperties: false,
  required: ["id", "name"],
  properties: {
    id: { type: "string" },
    name: { type: "string" },
    abbreviation: { type: "string" },
    membership: { type: "string" },
    exceptions: { type: "string" },
  },
};

const INVENTORY_CONFIG = {
  "nouns.number": {
    type: "object",
    additionalProperties: false,
    required: ["categories", "markingStrategies"],
    properties: {
      categories: { type: "array", items: category },
      markingStrategies: stringArray(["affix", "separate-word", "stem-change", "multiple", "unmarked", "custom"]),
    },
  },
  "nouns.case": {
    type: "object",
    additionalProperties: false,
    required: ["cases"],
    properties: { cases: { type: "array", items: caseItem } },
  },
  "nouns.classes": {
    type: "object",
    additionalProperties: false,
    required: ["kind", "classes"],
    properties: {
      kind: enumString(["gender", "noun-class", "custom"]),
      classes: { type: "array", items: classItem },
    },
  },
  "verbs.tense": {
    type: "object",
    additionalProperties: false,
    required: ["categories"],
    properties: { categories: { type: "array", items: category } },
  },
  "verbs.aspect": {
    type: "object",
    additionalProperties: false,
    required: ["categories"],
    properties: { categories: { type: "array", items: category } },
  },
  "verbs.mood": {
    type: "object",
    additionalProperties: false,
    required: ["categories"],
    properties: { categories: { type: "array", items: category } },
  },
} as const;

const article = {
  type: "object",
  additionalProperties: false,
  required: ["id", "form"],
  properties: {
    id: { type: "string" },
    form: { type: "string" },
    position: { type: "string" },
    notes: { type: "string" },
  },
};

const negativeForm = {
  type: "object",
  additionalProperties: false,
  required: ["id", "form"],
  properties: {
    id: { type: "string" },
    form: { type: "string" },
    conditions: { type: "string" },
    notes: { type: "string" },
  },
};

const STRATEGY_CONFIG = {
  "nouns.definiteness": {
    type: "object",
    additionalProperties: false,
    required: ["strategies", "articles"],
    properties: {
      strategies: stringArray([
        "definite-article",
        "indefinite-article",
        "both",
        "affixes",
        "demonstratives",
        "context",
        "other",
      ]),
      articles: { type: "array", items: article },
    },
  },
  "nouns.possession": {
    type: "object",
    additionalProperties: false,
    required: ["strategies"],
    properties: {
      strategies: stringArray([
        "possessive-pronouns",
        "genitive",
        "possessor-marking",
        "possessed-marking",
        "linking-particle",
        "word-order",
        "multiple",
      ]),
      alienability: { type: "boolean" },
      alienabilityNotes: { type: "string" },
    },
  },
  "verbs.marking-strategy": {
    type: "object",
    additionalProperties: false,
    required: ["strategies"],
    properties: {
      strategies: stringArray([
        "invariant",
        "prefixes",
        "suffixes",
        "other-affixes",
        "stem-changes",
        "auxiliaries",
        "particles",
        "multiple",
        "custom",
      ]),
      customStrategy: { type: "string" },
    },
  },
  "verbs.negative-forms": {
    type: "object",
    additionalProperties: false,
    required: ["strategies", "forms"],
    properties: {
      strategies: stringArray([
        "affix",
        "negative-auxiliary",
        "special-verb",
        "stem-change",
        "none",
        "multiple",
        "custom",
      ]),
      forms: { type: "array", items: negativeForm },
    },
  },
  "modifiers.adjective-behavior": {
    type: "object",
    additionalProperties: false,
    required: ["behaviors", "agreementRecordIds"],
    properties: {
      behaviors: stringArray(["invariant", "agree-with-noun", "verb-like", "noun-like", "multiple-classes", "custom"]),
      customBehavior: { type: "string" },
      agreementRecordIds: stringArray(),
    },
  },
  "modifiers.comparative": {
    type: "object",
    additionalProperties: false,
    required: ["strategies"],
    properties: {
      strategies: stringArray(["synthetic", "particle", "affix", "exceed", "special", "multiple", "custom"]),
      marker: { type: "string" },
      construction: { type: "string" },
    },
  },
  "modifiers.superlative": {
    type: "object",
    additionalProperties: false,
    required: ["strategies"],
    properties: {
      strategies: stringArray(["dedicated", "intensifier", "comparative", "definite", "none", "custom"]),
      marker: { type: "string" },
      construction: { type: "string" },
    },
  },
} as const;

const interrogative = {
  type: "object",
  additionalProperties: false,
  required: ["id", "meaning"],
  properties: {
    id: { type: "string" },
    meaning: { type: "string" },
    form: { type: "string" },
    lexemeId: { type: "string" },
  },
};

const CLAUSE_CONFIG = {
  "clauses.yes-no-questions": {
    type: "object",
    additionalProperties: false,
    required: ["strategies"],
    properties: {
      strategies: stringArray([
        "intonation",
        "particle",
        "word-order",
        "verb-morphology",
        "auxiliary",
        "multiple",
        "custom",
      ]),
      particle: { type: "string" },
      placement: enumString(["clause-initial", "clause-final", "before-verb", "after-verb", "other"]),
    },
  },
  "clauses.content-questions": {
    type: "object",
    additionalProperties: false,
    required: ["behavior", "interrogatives"],
    properties: {
      behavior: enumString(["in-situ", "fronted", "fixed-position", "special-structure", "mixed", "custom"]),
      customBehavior: { type: "string" },
      interrogatives: { type: "array", items: interrogative },
    },
  },
  "clauses.imperatives": {
    type: "object",
    additionalProperties: false,
    required: ["strategies"],
    properties: {
      strategies: stringArray([
        "bare-verb",
        "special-form",
        "particle",
        "auxiliary",
        "word-order",
        "multiple",
        "custom",
      ]),
      numberDistinction: { type: "boolean" },
      polarityDistinction: { type: "boolean" },
      politenessDistinction: { type: "boolean" },
    },
  },
  "clauses.negation": {
    type: "object",
    additionalProperties: false,
    required: ["strategies"],
    properties: {
      strategies: stringArray(["particle", "affix", "auxiliary", "special-verb", "multiple", "custom"]),
      particle: { type: "string" },
      placement: enumString(["clause-initial", "clause-final", "before-verb", "after-verb", "other"]),
      negativeQuestions: { type: "string" },
      negativeImperatives: { type: "string" },
    },
  },
  "clauses.relative-clauses": {
    type: "object",
    additionalProperties: false,
    required: ["strategies"],
    properties: {
      strategies: stringArray([
        "relative-pronoun",
        "complementizer",
        "gap",
        "resumptive",
        "internally-headed",
        "multiple",
        "custom",
      ]),
      headBehavior: { type: "string" },
      resumptives: { type: "string" },
    },
  },
} as const;

export const GRAMMAR_VALUE_SCHEMA = {
  $schema: "https://json-schema.org/draft/2020-12/schema",
  $id: "daena.language.grammar",
  oneOf: [
    ...CHOICE_SYSTEM_IDS.map((id) => systemBranch(id, { oneOf: [emptyConfig, CHOICE_CONFIG[id]] })),
    ...INVENTORY_SYSTEM_IDS.map((id) => systemBranch(id, { oneOf: [emptyConfig, INVENTORY_CONFIG[id]] })),
    ...STRATEGY_SYSTEM_IDS.map((id) => systemBranch(id, { oneOf: [emptyConfig, STRATEGY_CONFIG[id]] })),
    ...CLAUSE_SYSTEM_IDS.map((id) => systemBranch(id, { oneOf: [emptyConfig, CLAUSE_CONFIG[id]] })),
    ...PARADIGM_SYSTEM_IDS.map((id) => systemBranch(id, { oneOf: [emptyConfig, PARADIGM_CONFIG[id]] })),
    {
      type: "object",
      additionalProperties: false,
      required: [
        "recordKind",
        "schemaVersion",
        "title",
        "controller",
        "target",
        "features",
        "behavior",
        "notes",
        "examples",
        "links",
      ],
      properties: {
        recordKind: { const: "agreement" },
        schemaVersion: { const: 1 },
        title: { type: "string" },
        controller: {
          type: "object",
          additionalProperties: false,
          required: ["kind"],
          properties: {
            kind: enumString(["subject", "object", "noun", "possessor", "custom"]),
            customLabel: { type: "string" },
          },
        },
        target: {
          type: "object",
          additionalProperties: false,
          required: ["kind"],
          properties: {
            kind: enumString(["verb", "adjective", "article", "pronoun", "participle", "custom"]),
            customLabel: { type: "string" },
          },
        },
        features: {
          type: "array",
          items: {
            type: "object",
            additionalProperties: false,
            required: ["label"],
            properties: {
              sourceSystemId: { type: "string" },
              categoryId: { type: "string" },
              label: { type: "string" },
            },
          },
        },
        behavior: enumString(["full", "partial", "conditional"]),
        defaultForm: { type: "string" },
        conditions: { type: "string" },
        exceptions: { type: "string" },
        notes: { type: "string" },
        examples,
        links,
      },
    },
    {
      type: "object",
      additionalProperties: false,
      required: ["recordKind", "schemaVersion", "title", "tags", "body", "examples", "links"],
      properties: {
        recordKind: { const: "custom-rule" },
        schemaVersion: { const: 1 },
        title: { type: "string" },
        tags: stringArray(),
        body: { type: "string" },
        examples,
        links,
      },
    },
    {
      type: "object",
      additionalProperties: false,
      required: ["recordKind", "schemaVersion", "sectionId", "status"],
      properties: {
        recordKind: { const: "section-state" },
        schemaVersion: { const: 1 },
        sectionId: { const: "agreement" },
        status: { const: "not-used" },
        note: { type: "string" },
      },
    },
  ],
};
