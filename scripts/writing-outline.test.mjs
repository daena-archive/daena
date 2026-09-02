import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  appearanceLabel,
  buildManuscriptOutline,
  collectPartOfEdges,
  containmentNames,
  isManuscriptType,
  outlineHasNesting,
  parentByChild,
  partOfEdgesFromRelationships,
  WRITING_FEATURES,
  WRITING_PART_OF,
} from "../src/lib/writing/outline.ts";
import { contributedRelationshipFields } from "../src/lib/modules/contributed-fields.ts";

const series = { id: "series", name: "The Cycle", entity_type: "daena.writing:manuscript" };
const book = { id: "book", name: "Book 1", entity_type: "daena.writing:manuscript" };
const ch4 = { id: "ch4", name: "Chapter 4", entity_type: "daena.writing:manuscript" };
const ch6 = { id: "ch6", name: "Chapter 6", entity_type: "daena.writing:manuscript" };
const sketch = { id: "sketch", name: "Napkin idea", entity_type: "daena.writing:manuscript" };

const edges = partOfEdgesFromRelationships([
  { id: "e1", source_id: "book", target_id: "series", relationship_type: WRITING_PART_OF, metadata: '{"order":1}' },
  { id: "e2", source_id: "ch4", target_id: "book", relationship_type: WRITING_PART_OF, metadata: { order: 4 } },
  { id: "e3", source_id: "ch6", target_id: "book", relationship_type: WRITING_PART_OF, metadata: { order: 6 } },
]);

const nested = buildManuscriptOutline([series, book, ch4, ch6, sketch], [], edges);
assert.equal(outlineHasNesting(nested), true);
assert.deepEqual(
  nested.map((node) => node.id),
  ["series", "sketch"],
);
assert.equal(nested[0].children[0].id, "book");
assert.deepEqual(
  nested[0].children[0].children.map((node) => node.id),
  ["ch4", "ch6"],
);

const flat = buildManuscriptOutline([sketch], [], []);
assert.equal(outlineHasNesting(flat), false);
assert.deepEqual(
  flat.map((node) => node.id),
  ["sketch"],
);

const childOnPage = buildManuscriptOutline([ch4], [book], edges);
assert.equal(childOnPage[0].id, "ch4");
assert.equal(childOnPage[0].children.length, 0);

const names = new Map([
  ["series", "The Cycle"],
  ["book", "Book 1"],
  ["ch4", "Chapter 4"],
]);
assert.equal(appearanceLabel(containmentNames("ch4", parentByChild(edges), names)), "The Cycle · Book 1 · Chapter 4");

assert.equal(isManuscriptType("daena.writing:manuscript"), true);
assert.equal(isManuscriptType("daena.lore:person"), false);

const queried = await collectPartOfEdges(["book"], async (ids) =>
  edges.filter((edge) => ids.includes(edge.sourceId) || ids.includes(edge.targetId)),
);
assert.equal(queried.length >= 2, true);

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const lore = JSON.parse(readFileSync(join(root, "packages/modules/lore/manifest.json"), "utf8"));
const writing = JSON.parse(readFileSync(join(root, "packages/modules/writing/manifest.json"), "utf8"));
const writingEnabled = { ...writing, enabled: true };
const person = "daena.lore:person";
const types = new Set([
  person,
  "daena.lore:place",
  "daena.lore:faction",
  "daena.lore:artifact",
  "daena.lore:culture",
  "daena.lore:concept",
  "manuscript",
  "daena.writing:manuscript",
]);
const onPerson = contributedRelationshipFields(lore, person, [lore, writingEnabled], types);
assert.equal(
  onPerson.some((field) => field.key === "appearances" && field.relationshipType === WRITING_FEATURES),
  true,
  "Lore people receive Writing appearances",
);
const onManuscript = contributedRelationshipFields(writingEnabled, "manuscript", [writingEnabled], types);
const keys = onManuscript.filter((field) => field.type === "relationship").map((field) => field.key);
assert.equal(keys.includes("parent"), true);
assert.equal(keys.includes("contents"), true);
assert.equal(keys.includes("revises"), true);
assert.equal(keys.includes("features"), true);
assert.equal(writing.migrations[0].id, "writing-v1");
assert.equal(writing.version, "0.1.0");

console.log("writing outline and contribution checks passed");
