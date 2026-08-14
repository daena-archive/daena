import { CHOICE_SYSTEM_IDS } from "./choice.ts";
import { INVENTORY_SYSTEM_IDS } from "./inventory.ts";
import { GRAMMAR_SYSTEM_IDS } from "./types.ts";

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

const SPECIALIZED_SYSTEM_IDS = new Set<string>([...CHOICE_SYSTEM_IDS, ...INVENTORY_SYSTEM_IDS]);

export const GRAMMAR_VALUE_SCHEMA = {
  $schema: "https://json-schema.org/draft/2020-12/schema",
  $id: "daena.language.grammar",
  oneOf: [
    ...CHOICE_SYSTEM_IDS.map((id) => systemBranch(id, { oneOf: [emptyConfig, CHOICE_CONFIG[id]] })),
    ...INVENTORY_SYSTEM_IDS.map((id) => systemBranch(id, { oneOf: [emptyConfig, INVENTORY_CONFIG[id]] })),
    ...GRAMMAR_SYSTEM_IDS.filter((id) => !SPECIALIZED_SYSTEM_IDS.has(id)).map((id) =>
      systemBranch(id, {
        oneOf: [
          emptyConfig,
          id === "pronouns.personal" || id === "verbs.argument-indexing"
            ? id === "verbs.argument-indexing"
              ? {
                  type: "object",
                  additionalProperties: false,
                  required: ["participants", "axes", "cells"],
                  properties: {
                    participants: enumString(["none", "subject", "object", "subject-object", "other"]),
                    representation: enumString([
                      "endings",
                      "prefixes",
                      "full-forms",
                      "auxiliaries",
                      "flexible-table",
                      "custom",
                    ]),
                    axes: { type: "array", items: axis },
                    cells: { type: "array", items: cell },
                    flexibleNotes: { type: "string" },
                    agreementRecordId: { type: "string" },
                  },
                }
              : paradigmConfig
            : {
                type: "object",
                minProperties: 1,
                additionalProperties: true,
                properties: {
                  position: { type: "string" },
                  strategy: { type: "string" },
                  strategies: stringArray(),
                  categories: { type: "array", items: category },
                  cases: { type: "array" },
                  classes: { type: "array" },
                  articles: { type: "array" },
                  axes: { type: "array", items: axis },
                  cells: { type: "array", items: cell },
                  distances: stringArray(),
                  behaviors: stringArray(),
                  behavior: { type: "string" },
                  kind: { type: "string" },
                  participants: { type: "string" },
                  interrogatives: { type: "array" },
                  forms: { type: "array" },
                  particle: { type: "string" },
                  placement: { type: "string" },
                  marker: { type: "string" },
                  construction: { type: "string" },
                },
              },
        ],
      }),
    ),
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
