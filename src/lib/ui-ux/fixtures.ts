/**
 * Representative UI/UX scenarios for Houses, Tree, entity lifecycle, and schema.
 * These are review fixtures for later slices — not a second data model.
 */

import { ENTITY_ACTIONS, MUTATION_STATUS, MUTATION_STATUS_MESSAGES, TREE_KEYBOARD, TREE_SCOPES } from "./vocabulary.ts";

export type FixtureEntity = {
  id: string;
  name: string;
  type: string;
  revision: string;
  deleted?: boolean;
  fields?: Record<string, unknown>;
};

export type FixtureRelationship = {
  id: string;
  sourceId: string;
  targetId: string;
  type: string;
  revision: string;
  metadata?: Record<string, unknown>;
  /** Present when the edge is intentionally invalid for warning/recovery tests. */
  malformed?: boolean;
  malformedReason?: string;
};

export type FixtureSchemaOverlay = {
  moduleId: string;
  revision: string;
  customEntityTypes?: Array<{ id: string; name: string }>;
  customFields?: Array<{ key: string; label: string; type: string; entityTypes?: string[] }>;
  customTemplates?: Array<{ id: string; name: string; entityType: string }>;
  disabledFields?: string[];
  disabledEntityTypes?: string[];
};

export type UiUxScenario = {
  id: string;
  title: string;
  purpose: string;
  surfaces: string[];
  entities: FixtureEntity[];
  relationships: FixtureRelationship[];
  schemaOverlays?: FixtureSchemaOverlay[];
  /** When set, synthesize additional entities for scale tests. */
  scale?: { personCount: number; houseCount: number };
  expectedObservations: string[];
  mutationProbe?: {
    kind: "revision-conflict" | "membership-remove" | "schema-save";
    expectedStatus: typeof MUTATION_STATUS.conflict | typeof MUTATION_STATUS.saving | typeof MUTATION_STATUS.failed;
    expectedMessageIncludes?: string;
  };
};

function person(id: string, name: string, revision = "1"): FixtureEntity {
  return { id, name, type: "daena.lore:person", revision };
}

function house(id: string, name: string, revision = "1"): FixtureEntity {
  return { id, name, type: "daena.houses:house", revision };
}

function parent(
  id: string,
  sourceId: string,
  targetId: string,
  kind = "biological",
  revision = "1",
): FixtureRelationship {
  return {
    id,
    sourceId,
    targetId,
    type: "family_parent_of",
    revision,
    metadata: { kind },
  };
}

function partner(
  id: string,
  sourceId: string,
  targetId: string,
  kind = "marriage",
  status = "active",
  revision = "1",
): FixtureRelationship {
  return {
    id,
    sourceId,
    targetId,
    type: "family_partner_with",
    revision,
    metadata: { kind, status },
  };
}

function membership(
  id: string,
  personId: string,
  houseId: string,
  role: string,
  revision = "1",
  extras: Record<string, unknown> = {},
): FixtureRelationship {
  return {
    id,
    sourceId: personId,
    targetId: houseId,
    type: "family_member_of",
    revision,
    metadata: { role, ...extras },
  };
}

/** Empty project: every workspace shows empty states and always-visible New. */
export const EMPTY_PROJECT: UiUxScenario = {
  id: "empty-project",
  title: "Empty project",
  purpose: "Baseline empty states for collection, Tree landing, and schema tabs.",
  surfaces: [
    "workspace.lore.library",
    "workspace.timeline.events",
    "workspace.writing.manuscripts",
    "workspace.language.collection",
    "workspace.maps.collection",
    "workspace.houses.collection",
    "workspace.houses.tree.landing",
    "project.fields.types",
    "project.fields.fields",
    "project.fields.templates",
  ],
  entities: [],
  relationships: [],
  expectedObservations: [
    "Each normal workspace exposes a visible New action.",
    "Tree landing exposes New person and New house even when lists are empty.",
    "Fields & Types Types/Fields/Templates tabs render with package defaults and no project customizations.",
    "Archive section is empty.",
  ],
};

/**
 * Disconnected House: members with no kinship edges between them.
 * UI must say "N family groups" instead of looking like a layout bug.
 */
export const DISCONNECTED_HOUSE: UiUxScenario = {
  id: "disconnected-house",
  title: "Disconnected House members",
  purpose: "House Tree with three members and zero kinship edges between them.",
  surfaces: ["workspace.houses.tree.open-house", "workspace.houses.collection"],
  entities: [
    house("house-ash", "House Ash"),
    person("p-aria", "Aria"),
    person("p-borin", "Borin"),
    person("p-cela", "Cela"),
  ],
  relationships: [
    membership("m1", "p-aria", "house-ash", "head"),
    membership("m2", "p-borin", "house-ash", "member"),
    membership("m3", "p-cela", "house-ash", "heir"),
  ],
  expectedObservations: [
    "Members-only scope shows three person cards and no kinship edges.",
    "UI reports 3 family groups (or equivalent disconnected-component messaging).",
    "Role badges distinguish head and heir from ordinary membership.",
    "Fit frames all disconnected groups.",
  ],
};

/** One person belongs to multiple houses with distinct roles. */
export const MULTIPLE_MEMBERSHIPS: UiUxScenario = {
  id: "multiple-memberships",
  title: "Multiple house memberships",
  purpose: "Person dock and House dock must show every membership with role and notes.",
  surfaces: ["workspace.houses.tree.open-person", "workspace.houses.tree.open-house"],
  entities: [
    house("house-ash", "House Ash"),
    house("house-oak", "House Oak"),
    person("p-aria", "Aria"),
    person("p-dara", "Dara"),
  ],
  relationships: [
    membership("m-ash-head", "p-aria", "house-ash", "head", "1", { notes: "Founding line" }),
    membership("m-oak-consort", "p-aria", "house-oak", "consort", "1", {
      customLabel: "Guest consort",
      notes: "By marriage alliance",
    }),
    membership("m-oak-member", "p-dara", "house-oak", "member"),
    partner("r-partner", "p-aria", "p-dara", "marriage", "active"),
  ],
  expectedObservations: [
    "Aria lists House Ash (head) and House Oak (consort / Guest consort).",
    "Removing Aria from House Oak deletes only the membership relationship.",
    "Confirmation copy states the person remains in Lore.",
    "Default create-membership role may be member, but authors can change it.",
  ],
};

/** Malformed / skipped edge for warning investigation. */
export const MALFORMED_EDGE: UiUxScenario = {
  id: "malformed-edge",
  title: "Malformed family edge",
  purpose: "Tree warnings must surface skipped edges beyond a hidden count.",
  surfaces: ["workspace.houses.tree.open-person"],
  entities: [person("p-root", "Root"), person("p-parent", "Parent"), person("p-missing", "Missing Endpoint")],
  relationships: [
    parent("ok-parent", "p-parent", "p-root"),
    {
      id: "bad-self-parent",
      sourceId: "p-root",
      targetId: "p-root",
      type: "family_parent_of",
      revision: "1",
      metadata: { kind: "biological" },
      malformed: true,
      malformedReason: "self-parent",
    },
    {
      id: "bad-dangling",
      sourceId: "p-root",
      targetId: "entity-does-not-exist",
      type: "family_parent_of",
      revision: "1",
      metadata: { kind: "biological" },
      malformed: true,
      malformedReason: "missing-endpoint",
    },
  ],
  expectedObservations: [
    "Valid parent edge still renders.",
    "Malformed edges are skipped and listed with reasons, not only a count in settings.",
    "Canvas remains interactive for the valid neighborhood.",
  ],
};

/** Custom schema overlay with live entities using the custom type/field. */
export const CUSTOM_SCHEMA_LIVE_DATA: UiUxScenario = {
  id: "custom-schema-live-data",
  title: "Custom schema with live data",
  purpose: "Schema preview must count entities and field values before risky removes.",
  surfaces: ["project.fields.types", "project.fields.fields", "project.fields.templates", "workspace.lore.library"],
  entities: [
    {
      id: "custom-1",
      name: "Order of Embers",
      type: "daena.lore:knightly-order",
      revision: "3",
      fields: { motto: "Ash remembers", founded: "Year 12" },
    },
    {
      id: "custom-2",
      name: "Order of Rivers",
      type: "daena.lore:knightly-order",
      revision: "1",
      fields: { motto: "Flow onward" },
    },
    person("p-keeper", "Keeper"),
  ],
  relationships: [],
  schemaOverlays: [
    {
      moduleId: "daena.lore",
      revision: "overlay-rev-7",
      customEntityTypes: [{ id: "daena.lore:knightly-order", name: "Knightly order" }],
      customFields: [
        {
          key: "motto",
          label: "Motto",
          type: "text",
          entityTypes: ["daena.lore:knightly-order"],
        },
        {
          key: "founded",
          label: "Founded",
          type: "text",
          entityTypes: ["daena.lore:knightly-order"],
        },
      ],
      customTemplates: [
        {
          id: "knightly-order",
          name: "Knightly order",
          entityType: "daena.lore:knightly-order",
        },
      ],
    },
  ],
  expectedObservations: [
    "Types tab shows Knightly order as Project custom with entity count 2.",
    "Removing the type requires reassignment; never silently broaden motto/founded to all types.",
    "Preview reports field-value counts before Save.",
    "Overlay save uses expectedRevision overlay-rev-7 and a request ID.",
  ],
};

/** Simulated concurrent editors for relationship and schema overlay. */
export const REVISION_CONFLICT: UiUxScenario = {
  id: "revision-conflict",
  title: "Simulated revision conflict",
  purpose: "Conflict UI keeps the draft and offers reload / review / retry.",
  surfaces: ["workspace.houses.tree.relationship-dock", "project.fields.types"],
  entities: [person("p-a", "Asha", "4"), person("p-b", "Bram", "2"), house("house-ash", "House Ash", "1")],
  relationships: [
    partner("rel-1", "p-a", "p-b", "marriage", "active", "2"),
    membership("m1", "p-a", "house-ash", "member", "1"),
  ],
  schemaOverlays: [
    {
      moduleId: "daena.houses",
      revision: "overlay-rev-3",
      customFields: [{ key: "sigil", label: "Sigil", type: "text", entityTypes: ["daena.houses:house"] }],
    },
  ],
  expectedObservations: [
    "Relationship save with stale expectedRevision shows conflict status, not a silent overwrite.",
    "Actions include Reload current values and Review draft.",
    "Schema overlay save with stale revision keeps the draft and offers compare/reload/reapply.",
    `Status vocabulary uses ${MUTATION_STATUS.conflict} and ${MUTATION_STATUS_MESSAGES.conflictReload}.`,
  ],
  mutationProbe: {
    kind: "revision-conflict",
    expectedStatus: MUTATION_STATUS.conflict,
    expectedMessageIncludes: MUTATION_STATUS_MESSAGES.revisionConflictCode,
  },
};

/** Large project scale envelope — synthesize rather than ship 10k rows. */
export const LARGE_PROJECT: UiUxScenario = {
  id: "large-project",
  title: "Large project scale envelope",
  purpose: "Pickers and collections must stay backend-paged; never load all entities into memory.",
  surfaces: [
    "workspace.lore.library",
    "workspace.houses.tree.landing",
    "picker.async-entity",
    "workspace.houses.tree.root-picker",
  ],
  entities: [house("house-seed", "House Seed"), person("p-seed", "Seed Root")],
  relationships: [membership("m-seed", "p-seed", "house-seed", "founder")],
  scale: { personCount: 10_000, houseCount: 200 },
  expectedObservations: [
    "Opening a picker issues EntityListQuery pages with text search and type scopes.",
    "Interactive collections do not materialize the full entities array for filtering.",
    "Stale request tokens cannot replace newer search results.",
    "Tree landing person/house lists remain paged.",
  ],
};

export const UI_UX_SCENARIOS: UiUxScenario[] = [
  EMPTY_PROJECT,
  LARGE_PROJECT,
  DISCONNECTED_HOUSE,
  MULTIPLE_MEMBERSHIPS,
  MALFORMED_EDGE,
  CUSTOM_SCHEMA_LIVE_DATA,
  REVISION_CONFLICT,
];

export const SURFACE_IDS = [
  "workspace.lore.library",
  "workspace.lore.entity-editor",
  "workspace.timeline.events",
  "workspace.timeline.calendars",
  "workspace.writing.manuscripts",
  "workspace.writing.reference",
  "workspace.language.collection",
  "workspace.language.overview",
  "workspace.maps.collection",
  "workspace.maps.editor",
  "workspace.houses.collection",
  "workspace.houses.tree.landing",
  "workspace.houses.tree.open-person",
  "workspace.houses.tree.open-house",
  "workspace.houses.tree.relationship-dock",
  "project.fields.plugin-list",
  "project.fields.types",
  "project.fields.fields",
  "project.fields.templates",
  "project.archive",
  "dialog.create-template",
  "dialog.edit-identity",
  "dialog.archive-confirm",
  "picker.async-entity",
  "workspace.houses.tree.root-picker",
] as const;

export type SurfaceId = (typeof SURFACE_IDS)[number];

/** Target create defaults by workspace (plan §4.3). */
export const CONTEXTUAL_NEW_DEFAULTS = {
  lore: "first-enabled-lore-template",
  timeline: "current-events-or-calendars-tab",
  writing: "current-manuscripts-or-reference-tab",
  language: "language-template",
  houses: "house-template",
  "houses.tree": [ENTITY_ACTIONS.newPerson, ENTITY_ACTIONS.newHouse],
  maps: "provider-menu",
} as const;

export const TREE_SCOPE_DEFAULT = TREE_SCOPES.membersOnly.id;

export function synthesizeLargeProjectPeople(count: number, prefix = "person"): FixtureEntity[] {
  const out: FixtureEntity[] = [];
  const width = Math.max(4, String(count).length);
  for (let index = 1; index <= count; index += 1) {
    const id = `${prefix}-${String(index).padStart(width, "0")}`;
    out.push(person(id, `Person ${index}`, "1"));
  }
  return out;
}

export function synthesizeLargeProjectHouses(count: number, prefix = "house"): FixtureEntity[] {
  const out: FixtureEntity[] = [];
  const width = Math.max(3, String(count).length);
  for (let index = 1; index <= count; index += 1) {
    const id = `${prefix}-${String(index).padStart(width, "0")}`;
    out.push(house(id, `House ${index}`, "1"));
  }
  return out;
}

export function scenarioById(id: string): UiUxScenario | undefined {
  return UI_UX_SCENARIOS.find((scenario) => scenario.id === id);
}

/** Stable export for keyboard contract reviews. */
export const TREE_KEYBOARD_CONTRACT = TREE_KEYBOARD;
